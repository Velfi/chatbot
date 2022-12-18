pub mod call;
mod db;
mod state;
pub mod text;

use super::env::Env;
use super::strategy::Strategy;
use super::{Event, EventRx, EventTx};
use crate::message::Message;
use crate::openai_api::fetch_response_to_prompt;
use crate::Args;
use db::{
    begin_new_conversation, commit_conversation_to_database,
    load_previous_conversation_from_database, save_database_to_file,
};
use rusqlite::Connection;
pub use state::State;
use std::mem;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, instrument, trace};

enum Inner {
    // The app is currently waiting for a response from OpenAI
    BotsTurn,
    LoadingBotResponse {
        start_time: Instant,
        rx: mpsc::Receiver<Message>,
    },
    TakingAWhileToLoadBotResponse {
        start_time: Instant,
        rx: mpsc::Receiver<Message>,
    },
    UsersTurn,
}

#[instrument(name = "backend tick", skip(state, strategy))]
pub async fn tick(state: &mut State, strategy: impl Strategy) -> Result<(), anyhow::Error> {
    trace!("checking for received events...");
    strategy.handle_backend_events(state).await?;

    trace!("driving state machine...");
    match &mut state.inner {
        Inner::BotsTurn => {
            trace!("handling bot's turn...");
            let id = state.conversation.len() as u64;
            let prompt = create_prompt_from_messages(
                state.env.starting_prompt(),
                &state.conversation,
                state.env.prompt_context_length(),
            );
            let req = fetch_response_to_prompt(
                id,
                prompt,
                state.env.their_name().to_owned(),
                state.env.openai_model_name().to_owned(),
                state.env.token_limit(),
            );
            let (tx, rx) = mpsc::channel(1);

            tokio::spawn(async move {
                // TODO don't unwrap here
                let response = req.await.unwrap();
                tx.send(response).await.unwrap();
            });

            state.inner = Inner::LoadingBotResponse {
                start_time: Instant::now(),
                rx,
            };

            Ok(())
        }
        Inner::LoadingBotResponse { start_time, rx } => {
            trace!("loading bot response...");
            if start_time.elapsed() > state.env.expected_response_time() {
                trace!(
                    "{} is taking longer than {:?} to respond",
                    state.env.their_name(),
                    state.env.expected_response_time()
                );
                let start_time = *start_time;
                let rx = mem::replace(rx, mpsc::channel(1).1);
                state.inner = Inner::TakingAWhileToLoadBotResponse { start_time, rx };

                return Ok(());
            }

            state
                .frontend_tx
                .send(Event::StatusUpdated(
                    "Waiting for bot's response".to_owned(),
                ))
                .map_err(|e| {
                    anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                })?;

            // TODO this code is copied in the below handler, how can this be avoided?
            if let Some(message) = check_for_bot_response(state.env.their_name(), rx) {
                state.conversation.push(message);
                state
                    .frontend_tx
                    .send(Event::ConversationUpdated(state.conversation.clone()))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state
                    .frontend_tx
                    .send(Event::StatusUpdated(format!(
                        "Bot responded in {:?}",
                        start_time.elapsed()
                    )))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state.inner = Inner::UsersTurn;
            }

            Ok(())
        }
        Inner::TakingAWhileToLoadBotResponse { start_time, rx } => {
            trace!("loading bot response (taking a while)...");
            state
                .frontend_tx
                .send(Event::StatusUpdated(format!(
                    "Waiting for bot's response, It's taking a while ({}s)",
                    start_time.elapsed().as_secs()
                )))
                .map_err(|e| {
                    anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                })?;

            if let Some(message) = check_for_bot_response(state.env.their_name(), rx) {
                debug!("received response from {}", state.env.their_name());
                state.conversation.push(message);
                state
                    .frontend_tx
                    .send(Event::ConversationUpdated(state.conversation.clone()))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {}", e)
                    })?;
                state
                    .frontend_tx
                    .send(Event::StatusUpdated(format!(
                        "Bot slowly responded in {:?}",
                        start_time.elapsed()
                    )))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                    })?;
                state.inner = Inner::UsersTurn;
            }

            Ok(())
        }
        Inner::UsersTurn => {
            // The backend has nothing to do but wait for a response from the user
            Ok(())
        }
    }
}

pub fn quit(state: State) -> Result<(), anyhow::Error> {
    let mut conn = state.conn;
    commit_conversation_to_database(&mut conn, state.env.starting_prompt(), &state.conversation)?;
    save_database_to_file(&conn, state.env.database_file_path())?;

    Ok(())
}

fn create_prompt_from_messages(
    starting_prompt: &str,
    messages: &[Message],
    prompt_context_length: usize,
) -> String {
    let mut message_iter = messages.iter();
    while message_iter.len() > prompt_context_length {
        // Skip messages until we've limited ourselves to the last <PROMPT_MESSAGES_TO_SEND> messages
        let _ = message_iter.next();
    }

    let messages_len = message_iter.clone().map(|m| m.content.len()).sum::<usize>();
    // Really, the final prompt will be longer than this due to also including names and timestamps,
    // but this is a good starting point.
    let mut prompt = String::with_capacity(messages_len + starting_prompt.len());

    if !starting_prompt.is_empty() {
        prompt.push_str(starting_prompt);
        prompt.push_str("\n\n")
    }

    for message in message_iter {
        prompt.push_str(format!("{}:\n{}\n\n", &message.sender, &message.content).as_str());
    }

    prompt
}

#[instrument(skip(rx))]
fn check_for_bot_response(their_name: &str, rx: &mut mpsc::Receiver<Message>) -> Option<Message> {
    match rx.try_recv() {
        Ok(message) => {
            debug!("received response from {their_name}",);
            trace!(
                message.timestamp = message.timestamp.to_rfc2822().as_str(),
                message.id = message.id,
                message.content = message.content,
                "bot sent message"
            );

            Some(message)
        }
        Err(e) => match e {
            mpsc::error::TryRecvError::Empty => {
                trace!("no response from {their_name} yet");
                None
            }
            mpsc::error::TryRecvError::Disconnected => {
                unreachable!("The request task can't close this mpsc before sending the response")
            }
        },
    }
}

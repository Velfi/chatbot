use super::db::{
    begin_new_conversation, commit_conversation_to_database,
    load_previous_conversation_from_database, save_database_to_file,
};
use super::{Event, EventRx, EventTx};
use crate::app::env::Env;
use crate::message::Message;
use crate::openai_api::fetch_response_to_prompt;
use crate::Args;
use rusqlite::Connection;
use std::mem;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, instrument, trace};

pub struct State {
    pub _app_tx: EventTx,
    pub conn: Connection,
    pub conversation: Vec<Message>,
    pub frontend_tx: EventTx,
    pub inner: Inner,
    pub rx: EventRx,
    pub env: Arc<Env>,
}

impl State {
    pub async fn new(
        rx: EventRx,
        frontend_tx: EventTx,
        app_tx: EventTx,
        env: Arc<Env>,
        args: &Args,
    ) -> Result<Self, anyhow::Error> {
        let (conn, previous_conversation) = if args.resume() {
            load_previous_conversation_from_database(env.database_file_path())?
        } else {
            begin_new_conversation(env.database_file_path())?
        };

        let is_users_turn = previous_conversation.is_empty()
            || previous_conversation.last().unwrap().sender == env.their_name();

        let inner = if is_users_turn {
            Inner::UsersTurn
        } else {
            Inner::BotsTurn
        };

        frontend_tx
            .send(Event::ConversationUpdated(previous_conversation.clone()))
            .map_err(|e| anyhow::anyhow!("Failed to send conversation to frontend: {e}"))?;
        frontend_tx
            .send(Event::StatusUpdated(format!(
                "{} is ready to chat. Please type your input and press ENTER",
                env.their_name()
            )))
            .map_err(|e| anyhow::anyhow!("Failed to send status to frontend: {e}"))?;

        Ok(Self {
            _app_tx: app_tx,
            conn,
            conversation: previous_conversation,
            frontend_tx,
            inner,
            rx,
            env,
        })
    }
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

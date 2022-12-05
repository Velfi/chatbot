mod db;

use super::env::Env;
use super::{Event, EventRx, EventTx};
use crate::message::Message;
use crate::openai_api::fetch_response_to_prompt;
use db::{
    commit_conversation_to_database, load_previous_conversation_from_database,
    save_database_to_file,
};
use rusqlite::Connection;
use std::mem;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, instrument, trace};

// The Unpin in this feels wrong but I'm not sure

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

pub struct BackendState {
    _app_tx: EventTx,
    conn: Connection,
    conversation: Vec<Message>,
    frontend_tx: EventTx,
    inner: Inner,
    rx: EventRx,
    env: Arc<Env>,
}

impl BackendState {
    pub(super) async fn new(
        rx: EventRx,
        frontend_tx: EventTx,
        app_tx: EventTx,
        env: Arc<Env>,
    ) -> Result<Self, anyhow::Error> {
        let (conn, previous_conversation) =
            load_previous_conversation_from_database(env.database_file_path())?;

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

    #[instrument(name = "backend tick", skip(self))]
    pub(super) async fn tick(&mut self) -> Result<(), anyhow::Error> {
        trace!("checking for received events...");
        loop {
            match self.rx.try_recv() {
                Ok(event) => match event {
                    Event::Quit => {
                        // App will call the quit method. We can't call it because it consumes self.
                    }
                    Event::UserMessage(content) => {
                        let message = Message {
                            sender: self.env.your_name().to_owned(),
                            content,
                            timestamp: chrono::Utc::now(),
                            id: self.conversation.len() as u64,
                        };
                        trace!(
                            message.timestamp = message.timestamp.to_rfc2822().as_str(),
                            message.id = message.id,
                            message.content = message.content,
                            "user sent message"
                        );

                        self.conversation.push(message);
                        // Immediately send the conversation to the frontend so that the user's
                        // message will be displayed immediately, instead of after the bot responds.
                        self.frontend_tx
                            .send(Event::ConversationUpdated(self.conversation.clone()))
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "failed to notify frontend of conversation update: {e}"
                                )
                            })?;

                        self.inner = Inner::BotsTurn;
                    }
                    _ => {}
                },
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => break,
                    mpsc::error::TryRecvError::Disconnected => {
                        unreachable!(
                            "The frontend will never disconnect from the backend while ticking"
                        )
                    }
                },
            }
        }

        trace!("driving state machine...");
        match &mut self.inner {
            Inner::BotsTurn => {
                trace!("handling bot's turn...");
                let id = self.conversation.len() as u64;
                let prompt = create_prompt_from_messages(
                    self.env.starting_prompt(),
                    &self.conversation,
                    self.env.prompt_context_length(),
                );
                let req = fetch_response_to_prompt(
                    id,
                    prompt,
                    self.env.their_name().to_owned(),
                    self.env.openai_model_name().to_owned(),
                    self.env.token_limit(),
                );
                let (tx, rx) = mpsc::channel(1);

                tokio::spawn(async move {
                    // TODO don't unwrap here
                    let response = req.await.unwrap();
                    tx.send(response).await.unwrap();
                });

                self.inner = Inner::LoadingBotResponse {
                    start_time: Instant::now(),
                    rx,
                };

                Ok(())
            }
            Inner::LoadingBotResponse { start_time, rx } => {
                trace!("loading bot response...");
                if start_time.elapsed() > self.env.expected_response_time() {
                    trace!(
                        "{} is taking longer than {:?} to respond",
                        self.env.their_name(),
                        self.env.expected_response_time()
                    );
                    let start_time = *start_time;
                    let rx = mem::replace(rx, mpsc::channel(1).1);
                    self.inner = Inner::TakingAWhileToLoadBotResponse { start_time, rx };

                    return Ok(());
                }

                self.frontend_tx
                    .send(Event::StatusUpdated(
                        "Waiting for bot's response".to_owned(),
                    ))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                    })?;

                // TODO this code is copied in the below handler, how can this be avoided?
                if let Some(message) = check_for_bot_response(self.env.their_name(), rx) {
                    self.conversation.push(message);
                    self.frontend_tx
                        .send(Event::ConversationUpdated(self.conversation.clone()))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                        })?;
                    self.frontend_tx
                        .send(Event::StatusUpdated(format!(
                            "Bot responded in {:?}",
                            start_time.elapsed()
                        )))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                        })?;
                    self.inner = Inner::UsersTurn;
                }

                Ok(())
            }
            Inner::TakingAWhileToLoadBotResponse { start_time, rx } => {
                trace!("loading bot response (taking a while)...");
                self.frontend_tx
                    .send(Event::StatusUpdated(format!(
                        "Waiting for bot's response, It's taking a while ({}s)",
                        start_time.elapsed().as_secs()
                    )))
                    .map_err(|e| {
                        anyhow::anyhow!("failed to notify frontend of status update: {}", e)
                    })?;

                if let Some(message) = check_for_bot_response(self.env.their_name(), rx) {
                    debug!("received response from {}", self.env.their_name());
                    self.conversation.push(message);
                    self.frontend_tx
                        .send(Event::ConversationUpdated(self.conversation.clone()))
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "failed to notify frontend of conversation update: {}",
                                e
                            )
                        })?;
                    self.frontend_tx
                        .send(Event::StatusUpdated(format!(
                            "Bot slowly responded in {:?}",
                            start_time.elapsed()
                        )))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                        })?;
                    self.inner = Inner::UsersTurn;
                }

                Ok(())
            }
            Inner::UsersTurn => {
                // The backend has nothing to do but wait for a response from the user
                Ok(())
            }
        }
    }

    pub async fn quit(self) -> Result<(), anyhow::Error> {
        let mut conn = self.conn;
        commit_conversation_to_database(
            &mut conn,
            &self.env.starting_prompt(),
            &self.conversation,
        )?;
        save_database_to_file(&mut conn, self.env.database_file_path())?;

        Ok(())
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
        prompt.push_str(&starting_prompt);
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

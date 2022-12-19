use super::db::{begin_new_conversation, load_previous_conversation_from_database};
use super::{Event, EventRx, EventTx};
use crate::app::{env::Env, TurnToSpeak};
use crate::message::{create_prompt_from_messages, Message};
use crate::openai_api::fetch_response_to_prompt;
use crate::Args;
use rusqlite::Connection;
use std::mem;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::oneshot;
use tracing::{debug, instrument, trace};

pub enum Inner {
    SendRequest,
    // The app is currently waiting for a response from OpenAI
    LoadingBotResponse {
        start_time: Instant,
        rx: oneshot::Receiver<Message>,
    },
    TakingAWhileToLoadBotResponse {
        start_time: Instant,
        rx: oneshot::Receiver<Message>,
    },
}

pub struct State {
    pub _app_tx: EventTx,
    pub conn: Connection,
    pub conversation: Vec<Message>,
    pub frontend_tx: EventTx,
    pub inner: Inner,
    pub rx: EventRx,
    pub env: Arc<Env>,
    pub turn_to_speak: TurnToSpeak,
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

        let turn_to_speak = if previous_conversation.is_empty()
            || previous_conversation.last().unwrap().sender == env.their_name()
        {
            TurnToSpeak::User
        } else {
            TurnToSpeak::Bot
        };

        // Can we combine these two and send a slice or something instead?
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
            inner: Inner::SendRequest,
            rx,
            env,
            turn_to_speak,
        })
    }

    pub fn run_bot_response_state_machine(&mut self) -> Result<(), anyhow::Error> {
        match &mut self.inner {
            Inner::SendRequest => {
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
                let (tx, rx) = oneshot::channel();

                tokio::spawn(async move {
                    // TODO don't unwrap here
                    let response = req.await.unwrap();
                    tx.send(response).unwrap();
                });

                self.inner = Inner::LoadingBotResponse {
                    start_time: Instant::now(),
                    rx,
                };
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
                    // TODO is this really necessary?
                    let rx = mem::replace(rx, oneshot::channel().1);
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
                    self.turn_to_speak = TurnToSpeak::User;
                    self.inner = Inner::SendRequest;
                }
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
                    self.turn_to_speak = TurnToSpeak::User;
                    self.inner = Inner::SendRequest;
                }
            }
        }

        Ok(())
    }
}

#[instrument(skip(rx))]
fn check_for_bot_response(
    their_name: &str,
    rx: &mut oneshot::Receiver<Message>,
) -> Option<Message> {
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
            oneshot::error::TryRecvError::Empty => {
                trace!("no response from {their_name} yet");
                None
            }
            oneshot::error::TryRecvError::Closed => {
                unreachable!("The request task can't close this mpsc before sending the response")
            }
        },
    }
}

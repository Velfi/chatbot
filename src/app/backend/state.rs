use super::db::{begin_new_conversation, load_previous_conversation_from_database};
use super::{Event, EventRx, EventTx};
use crate::app::{env::Env, TurnToSpeak};
use crate::message::Message;
use crate::Args;
use rusqlite::Connection;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};
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

    pub fn handle_backend_events(&mut self) -> Result<(), anyhow::Error> {
        trace!("handling backend events...");

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

                        self.turn_to_speak = TurnToSpeak::Bot;
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

        Ok(())
    }
}

#[instrument(skip(rx))]
pub fn check_for_bot_response(
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

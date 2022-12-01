use tokio::sync::mpsc;
use tracing::trace;

use super::{Event, EventRx, EventTx};
use crate::message::Message;
use crate::{openai_api, BOT_NAME, USER_NAME};
use std::thread;
use std::time::{Duration, Instant};

enum Inner {
    // The app is currently waiting for a response from OpenAI
    BotsTurn,
    LoadingBotResponse(Instant),
    TakingAWhileToLoadBotResponse,
    BotResponseReady(Message),
    UsersTurn,
    // The app is committing the current state to disk and finishing up with anything else it needs
    // to do before exiting
    Quitting,
    ReadyToQuit,
}

pub struct BackendState {
    app_tx: EventTx,
    frontend_tx: EventTx,
    rx: EventRx,
    openai_client: openai_api::Client,
    pub messages: Vec<Message>,
    inner: Inner,
}

impl BackendState {
    pub(super) async fn new(
        rx: EventRx,
        frontend_tx: EventTx,
        app_tx: EventTx,
    ) -> Result<Self, anyhow::Error> {
        let previous_conversation =
            load_previous_conversation_from_database().unwrap_or_else(|| load_test_conversation());
        let is_users_turn = previous_conversation.is_empty()
            || previous_conversation.last().unwrap().sender == BOT_NAME;

        let inner = if is_users_turn {
            Inner::UsersTurn
        } else {
            Inner::BotsTurn
        };

        frontend_tx
            .send(Event::ConversationUpdated(previous_conversation.clone()))
            .map_err(|e| anyhow::anyhow!("Failed to send conversation to frontend: {e}"))?;
        frontend_tx
            .send(Event::StatusUpdated("Ready to chat".to_string()))
            .map_err(|e| anyhow::anyhow!("Failed to send status to frontend: {e}"))?;

        Ok(Self {
            app_tx,
            frontend_tx,
            rx,
            openai_client: openai_api::Client::new(),
            messages: previous_conversation,
            inner,
        })
    }

    pub(super) async fn tick(&mut self) -> Result<(), anyhow::Error> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => match event {
                    Event::Quit => {
                        // App will call the quit method. We can't call it because it consumes self.
                    }
                    Event::UserMessage(content) => {
                        let message = Message {
                            sender: USER_NAME.to_owned(),
                            content,
                            timestamp: chrono::Utc::now(),
                            id: self.messages.len() as u64,
                        };
                        trace!(
                            message.timestamp = message.timestamp.to_rfc2822().as_str(),
                            message.id = message.id,
                            message.content = message.content,
                            "user sent message"
                        );

                        self.messages.push(message);
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

        Ok(())
    }

    //     match &mut self.inner {
    //         Inner::BotsTurn => {
    //             self.inner = Inner::LoadingBotResponse(Instant::now());
    //         }
    //         Inner::LoadingBotResponse(start_time) => {
    //             // Load the bot's response
    //             let message = self
    //                 .openai_client
    //                 .fetch_next_response_to_messages(self.next_message_id(), &self.messages)
    //                 .await;
    //             self.messages.push(message);
    //             self.inner = Inner::UsersTurn;
    //         }
    //         Inner::Quitting => {
    //             // TODO save the state to disk

    //             self.inner = Inner::ReadyToQuit;
    //         }
    //         _ => {
    //             // Do nothing
    //         }
    //     }

    //     Ok(())
    // }

    pub async fn quit(self) -> Result<(), anyhow::Error> {
        // TODO save the state to disk

        Ok(())
    }

    // pub fn user_is_typing(&mut self) -> bool {
    //     matches!(self.inner, Inner::UserIsTyping)
    // }

    // pub fn status_message(&self) -> &'static str {
    //     match self.inner {
    //         Inner::BotIsTyping => "Bot is typing...",
    //         Inner::BotResponseReady => "Posting Bot's response...",
    //         Inner::UserIsTyping => "User is typing...",
    //         Inner::Quitting => "Quitting...",
    //         Inner::ReadyToQuit => "Ready to quit",
    //     }
    // }

    // // This works by checking the length of messages. It's definitely not going to work if multiple
    // // things are trying to create their own messages at the same time.
    // pub fn next_message_id(&self) -> u64 {
    //     self.messages.len() as u64
    // }

    // pub fn send_message(&mut self, message: Message) {
    //     self.messages.push(message.clone());

    //     match self.inner {
    //         Inner::BotIsTyping => {
    //             self.inner = Inner::UserIsTyping;
    //         }
    //         Inner::UserIsTyping => {
    //             self.inner = Inner::BotIsTyping;
    //         }
    //         _ => unreachable!("A message was sent out of turn. This is a bug."),
    //     }
    // }
}

fn load_previous_conversation_from_database() -> Option<Vec<Message>> {
    // TODO load messages from database
    None
}

fn load_test_conversation() -> Vec<Message> {
    let mut messages = Vec::new();
    messages.push(Message {
        id: 1,
        sender: USER_NAME.to_string(),
        content: "Hello bot.".to_string(),
        timestamp: chrono::Utc::now(),
    });
    // These sleeps ensure the timestamps will be different
    thread::sleep(Duration::from_millis(100));

    messages.push(Message {
        id: 2,
        sender: BOT_NAME.to_string(),
        content: "Hello user.".to_string(),
        timestamp: chrono::Utc::now(),
    });
    thread::sleep(Duration::from_millis(100));

    messages.push(Message {
        id: 3,
        sender: USER_NAME.to_string(),
        content: "How are you?".to_string(),
        timestamp: chrono::Utc::now(),
    });
    thread::sleep(Duration::from_millis(100));

    messages.push(Message {
        id: 4,
        sender: BOT_NAME.to_string(),
        content: "I'm fine, thanks. How are you?".to_string(),
        timestamp: chrono::Utc::now(),
    });
    thread::sleep(Duration::from_millis(100));

    messages.push(Message {
        id: 5,
        sender: USER_NAME.to_string(),
        content: "I'm fine too. Goodbye for now, bot.".to_string(),
        timestamp: chrono::Utc::now(),
    });
    thread::sleep(Duration::from_millis(100));

    messages.push(Message {
        id: 6,
        sender: BOT_NAME.to_string(),
        content: "Goodbye user.".to_string(),
        timestamp: chrono::Utc::now(),
    });

    messages
}

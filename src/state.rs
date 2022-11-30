use std::thread;
use std::time::Duration;

use crate::message::Message;
use crate::{BOT_NAME, USER_NAME};

#[derive(Debug, Clone, PartialEq)]
enum Inner {
    // The app is currently waiting for a response from OpenAI
    BotIsTyping,
    // Contains the `Message` sent by the bot
    BotSentMessage(Message),
    UserIsTyping,
    // Contains the `Message` sent by the bot
    UserSentMessage(Message),
    // The app is committing the current state to disk and finishing up with anything else it needs
    // to do before exiting
    Quitting,
}

pub struct State {
    pub messages: Vec<Message>,
    pub should_quit: bool,
    inner: Inner,
}

impl State {
    pub fn load() -> Self {
        Self::new_from_database().unwrap_or_else(Self::new_testing)
    }

    pub fn user_is_typing(&mut self) -> bool {
        self.inner == Inner::UserIsTyping
    }

    pub fn status_message(&self) -> &'static str {
        match self.inner {
            Inner::BotIsTyping => "Bot is typing...",
            Inner::BotSentMessage(_) => "Bot sent a message",
            Inner::UserIsTyping => "User is typing...",
            Inner::UserSentMessage(_) => "User sent a message",
            Inner::Quitting => "Quitting...",
        }
    }

    // This works by checking the length of messages. It's definitely not going to work if multiple
    // things are trying to create their own messages at the same time.
    pub fn next_message_id(&self) -> u64 {
        self.messages.len() as u64
    }

    pub fn send_message(&mut self, message: Message) {
        self.messages.push(message.clone());

        match self.inner {
            Inner::BotIsTyping => {
                self.inner = Inner::BotSentMessage(message);
            }
            Inner::UserIsTyping => {
                self.inner = Inner::UserSentMessage(message);
            }
            _ => unreachable!("A message was sent out of turn. This is a bug."),
        }
    }

    fn new_from_database() -> Option<Self> {
        // TODO load messages from database
        None
    }

    fn _new_empty() -> Self {
        Self {
            messages: Vec::new(),
            should_quit: false,
            inner: Inner::UserIsTyping,
        }
    }

    fn new_testing() -> Self {
        let mut messages = Vec::new();
        messages.push(Message {
            id: 1,
            sender: USER_NAME.to_string(),
            content: "Hello bot.".to_string(),
            timestamp: chrono::Utc::now(),
        });
        // These sleeps ensure the timestamps will be different
        thread::sleep(Duration::from_millis(10));

        messages.push(Message {
            id: 2,
            sender: BOT_NAME.to_string(),
            content: "Hello user.".to_string(),
            timestamp: chrono::Utc::now(),
        });
        thread::sleep(Duration::from_millis(10));

        messages.push(Message {
            id: 3,
            sender: USER_NAME.to_string(),
            content: "How are you?".to_string(),
            timestamp: chrono::Utc::now(),
        });
        thread::sleep(Duration::from_millis(10));

        messages.push(Message {
            id: 4,
            sender: BOT_NAME.to_string(),
            content: "I'm fine, thanks. How are you?".to_string(),
            timestamp: chrono::Utc::now(),
        });
        thread::sleep(Duration::from_millis(10));

        messages.push(Message {
            id: 5,
            sender: USER_NAME.to_string(),
            content: "I'm fine too. Goodbye for now, bot.".to_string(),
            timestamp: chrono::Utc::now(),
        });
        thread::sleep(Duration::from_millis(10));

        messages.push(Message {
            id: 6,
            sender: BOT_NAME.to_string(),
            content: "Goodbye user.".to_string(),
            timestamp: chrono::Utc::now(),
        });

        Self {
            messages,
            should_quit: false,
            inner: Inner::UserIsTyping,
        }
    }
}

use chrono::{DateTime, Utc};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl PartialEq for Message {
    // Messages only have a unique ID relative to a single conversation, so don't go comparing
    // messages from different conversations.
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Message {
    // Messages should be orderable by their ID because this app is effectively synchronous but we
    // sort based on timestamp anyways.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

pub fn create_prompt_from_messages(
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

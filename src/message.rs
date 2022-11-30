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

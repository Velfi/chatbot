use std::path::Path;

use crate::message::Message;
use anyhow::Context;
use rusqlite::{params, Connection};
use tracing::{info, debug};

pub fn load_previous_conversation_from_database(
    path: &Path,
) -> Result<(Connection, Vec<Message>), anyhow::Error> {
    // find the database file and load it
    Connection::open(path)
        .context("failed to load database from disk")
        .and_then(|conn| {
            // TODO log possible failuers with `error!()`
            conn.query_row(
                // TODO do I want ASC or DESC here?
                "SELECT id FROM conversations ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .context("failed to load previous conversation ID from database")
            .map(|id| (conn, id))
        })
        .and_then(|(conn, id)| {
            // TODO log possible failuers with `error!()`
            get_messages_by_conversation_id(&conn, id).map(|messages| (conn, messages))
        })
        .or_else(|e| {
            info!("failed to load database from disk: {}", e);
            info!("creating new database and returning empty conversation");
            initialize_database()
                .context("failed to initialize database")
                .map(|conn| (conn, Vec::new()))
        })
}

fn get_messages_by_conversation_id(
    conn: &Connection,
    conversation_id: i64,
) -> Result<Vec<Message>, anyhow::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, sender, content, created_at, conversation
            FROM messages
            WHERE conversation = ?1
        ",
    )?;
    let rows = stmt
        .query_map([conversation_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                sender: row.get(1)?,
                content: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })
        .context("failed to load messages from database")?;

    // TODO is there a fancier way to do this with a `collect()`?
    let mut messages = Vec::new();
    for row in rows {
        messages.push(row?);
    }

    Ok(messages)
}

fn initialize_database() -> Result<Connection, anyhow::Error> {
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE conversations (
            id         INTEGER PRIMARY KEY,
            created_at TEXT NOT NULL,
            prompt     TEXT NOT NULL
        )",
        (),
    )
    .context("creating conversations table")?;

    // Messages keep track of their conversation instead of the other way around. Is that really
    // stupid?
    conn.execute(
        "CREATE TABLE messages (
            id                        INTEGER PRIMARY KEY,
            sender                    TEXT NOT NULL,
            content                   TEXT NOT NULL,
            created_at                TEXT NOT NULL,
            conversation              INTEGER NOT NULL,
            FOREIGN KEY(conversation) REFERENCES conversations(id)
        )",
        (),
    )
    .context("creating messages table")?;

    Ok(conn)
}

pub fn commit_conversation_to_database(
    conn: &mut Connection,
    prompt: &str,
    conversation: &[Message],
) -> Result<(), anyhow::Error> {
    if conversation.is_empty() {
        debug!("no conversations to commit to DB, returning early");
        return Ok(());
    }

    let tx = conn.transaction().context("starting transaction")?;

    tx.execute(
        "INSERT INTO conversations (created_at, prompt) VALUES (?1, ?2)",
        (conversation[0].timestamp, prompt),
    )
    .context("inserting conversation into database")?;
    let conversation_id = tx.last_insert_rowid();

    {
        let mut stmt = tx
        .prepare(
            "INSERT INTO messages (sender, content, created_at, conversation) VALUES (?1, ?2, ?3, ?4)",
        )
        .context("preparing statement to insert messages into database")?;

        for message in conversation {
            stmt.execute(params![
                message.sender,
                message.content,
                message.timestamp,
                conversation_id
            ])
            .context("inserting message into database")?;
        }
    }

    tx.commit().context("committing transaction")?;

    Ok(())
}

pub fn save_database_to_file(conn: &Connection, path: &Path) -> Result<(), anyhow::Error> {
    // TODO add a fancy progress indicator
    conn.backup(rusqlite::DatabaseName::Main, path, None)
        .context("saving database to disk")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{thread, time::Duration};

    const USER_NAME: &str = "test_user";
    const BOT_NAME: &str = "test_bot";
    const PROMPT: &str = "test prompt";
    const DB_PATH: &str = "test.db";

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

    #[test]
    fn test_e2e() {
        let db_path = Path::new(DB_PATH);

        // Load a test conversation, commit it to the DB, and write the DB to disk.
        let mut conn = initialize_database().unwrap();
        let messages = load_test_conversation();
        commit_conversation_to_database(&mut conn, PROMPT, &messages).unwrap();
        save_database_to_file(&conn, db_path).unwrap();
        conn.close().unwrap();

        // Load the DB from disk and make sure the conversation matches the test conversation.
        let (conn, messages_from_db) = load_previous_conversation_from_database(db_path).unwrap();
        assert_eq!(messages, messages_from_db);
        conn.close().unwrap();

        // Clean up the DB file for future tests.
        std::fs::remove_file(DB_PATH).unwrap();
    }
}

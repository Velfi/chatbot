use super::State;
use crate::{
    app::{Event, TurnToSpeak},
    message::Message,
};
use tokio::sync::mpsc;
use tracing::trace;

pub fn handle_backend_events(state: &mut State) -> Result<(), anyhow::Error> {
    trace!("handling backend events...");

    loop {
        match state.rx.try_recv() {
            Ok(event) => match event {
                Event::Quit => {
                    // App will call the quit method. We can't call it because it consumes state.
                }
                Event::UserMessage(content) => {
                    let message = Message {
                        sender: state.env.your_name().to_owned(),
                        content,
                        timestamp: chrono::Utc::now(),
                        id: state.conversation.len() as u64,
                    };
                    trace!(
                        message.timestamp = message.timestamp.to_rfc2822().as_str(),
                        message.id = message.id,
                        message.content = message.content,
                        "user sent message"
                    );

                    state.conversation.push(message);
                    // Immediately send the conversation to the frontend so that the user's
                    // message will be displayed immediately, instead of after the bot responds.
                    state
                        .frontend_tx
                        .send(Event::ConversationUpdated(state.conversation.clone()))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to notify frontend of conversation update: {e}")
                        })?;

                    state.turn_to_speak = TurnToSpeak::Bot;
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

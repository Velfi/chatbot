pub mod call;
mod state;
pub mod text;

use super::strategy::Strategy;
use super::TurnToSpeak;
use crate::{app::Event, message::Message};
use anyhow::Context;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
pub use state::State;
use tokio::sync::mpsc;
use tracing::{instrument, trace};

#[instrument(name = "frontend tick", skip(state, strategy))]
pub(super) async fn tick(state: &mut State, strategy: impl Strategy) -> Result<(), anyhow::Error> {
    trace!("handling user input...");
    strategy.handle_user_input(state)?;

    trace!("checking for received events...");
    loop {
        match state.rx.try_recv() {
            Ok(event) => match event {
                Event::Quit => {
                    // App will call the quit method. We can't call it because it consumes `self`.
                }
                Event::ConversationUpdated(conversation) => {
                    // If the last sender is the bot, it's the user's turn to speak and vice versa.
                    match conversation.last() {
                        Some(Message { sender, .. }) if sender == state.env.your_name() => {
                            trace!("it's {}'s turn to speak", state.env.their_name());
                            state.turn_to_speak = TurnToSpeak::Bot;
                        }
                        _ => {
                            trace!("it's {}'s turn to speak", state.env.your_name());
                            state.turn_to_speak = TurnToSpeak::User;
                        }
                    }

                    state.widget_state.conversation = conversation;
                }
                Event::StatusUpdated(status) => {
                    state.widget_state.status = status;
                }
                _ => {}
            },
            Err(e) => match e {
                mpsc::error::TryRecvError::Empty => break,
                mpsc::error::TryRecvError::Disconnected => {
                    unreachable!(
                        "The backend will never disconnect from the frontend while ticking"
                    )
                }
            },
        }
    }

    trace!("redrawing terminal...");
    // We always redraw because the user may have resized the window or scrolled the conversation
    strategy.redraw_terminal(state)?;

    Ok(())
}

pub fn quit(mut state: State) -> Result<(), anyhow::Error> {
    trace!("tearing down terminal");

    disable_raw_mode().context("disabling raw mode")?;
    execute!(
        state.terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    state.terminal.show_cursor().context("frontend quitting")
}

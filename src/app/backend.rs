pub mod call;
mod db;
mod state;
pub mod text;

use super::strategy::Strategy;
use super::TurnToSpeak;
use super::{Event, EventRx, EventTx};
use db::{commit_conversation_to_database, save_database_to_file};
pub use state::State;
use tracing::{instrument, trace};

#[instrument(name = "backend tick", skip(state, strategy))]
pub async fn tick(state: &mut State, strategy: impl Strategy) -> Result<(), anyhow::Error> {
    trace!("checking for received events...");
    strategy.handle_backend_events(state)?;

    trace!("driving state machine...");
    match state.turn_to_speak {
        TurnToSpeak::Bot => state.run_bot_response_state_machine(),
        TurnToSpeak::User => {
            // The backend has nothing to do but wait for a response from the user
            Ok(())
        }
    }
}

pub fn quit(state: State) -> Result<(), anyhow::Error> {
    let mut conn = state.conn;
    commit_conversation_to_database(&mut conn, state.env.starting_prompt(), &state.conversation)?;
    save_database_to_file(&conn, state.env.database_file_path())?;

    Ok(())
}

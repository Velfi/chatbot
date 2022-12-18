use super::State;
use tracing::trace;

pub fn handle_user_input(state: &mut State) -> Result<(), anyhow::Error> {
    trace!("handling user input...");
    Ok(())
}

pub fn redraw_terminal(state: &mut State) -> Result<(), anyhow::Error> {
    trace!("redrawing terminal...");
    Ok(())
}

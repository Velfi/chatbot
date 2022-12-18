use super::State;
use tracing::trace;

pub fn handle_backend_events(state: &mut State) -> Result<(), anyhow::Error> {
    trace!("checking for received events...");
    Ok(())
}

use super::{backend, frontend};
use crate::args::Action;

pub trait Strategy {
    fn handle_user_input(&self, state: &mut frontend::State) -> Result<(), anyhow::Error>;
    fn redraw_terminal(&self, state: &mut frontend::State) -> Result<(), anyhow::Error>;
    fn handle_backend_events(&self, state: &mut backend::State) -> Result<(), anyhow::Error>;
    fn run_bot_state_machine(&self, state: &mut backend::State) -> Result<(), anyhow::Error>;
}

impl Strategy for Action {
    fn handle_user_input(&self, state: &mut frontend::State) -> Result<(), anyhow::Error> {
        match self {
            Action::Call => frontend::call::handle_user_input(state),
            Action::Text => frontend::text::handle_user_input(state),
        }
    }

    fn redraw_terminal(&self, state: &mut frontend::State) -> Result<(), anyhow::Error> {
        match self {
            Action::Call => frontend::call::redraw_terminal(state),
            Action::Text => frontend::text::redraw_terminal(state),
        }
    }

    fn handle_backend_events(&self, state: &mut backend::State) -> Result<(), anyhow::Error> {
        match self {
            Action::Call => backend::call::handle_backend_events(state),
            Action::Text => backend::text::handle_backend_events(state),
        }
    }

    fn run_bot_state_machine(&self, state: &mut backend::State) -> Result<(), anyhow::Error> {
        match self {
            Action::Call => backend::call::run_bot_state_machine(state),
            Action::Text => backend::text::run_bot_state_machine(state),
        }
    }
}

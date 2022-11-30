pub mod app;
pub mod message;
pub mod openai_api;
pub mod state;

use crate::app::App;
use crate::state::State;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Stdout};
use tracing::trace;
use tui::{backend::CrosstermBackend, Terminal};

const USER_NAME: &str = "User";
const BOT_NAME: &str = "Bot";

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("failed to read .env file, please create one");
    tracing_subscriber::fmt::init();

    let mut terminal = setup_terminal()?;
    let state = State::load();
    let app = App::builder().state(state).terminal(&mut terminal).build();

    app.run_until_exit()?;
    teardown_terminal(terminal)?;

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, std::io::Error> {
    trace!("setting up terminal");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn teardown_terminal(
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), std::io::Error> {
    trace!("tearing down terminal");

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

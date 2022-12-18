use super::TurnToSpeak;
use crate::{
    app::{env::Env, EventRx, EventTx},
    args::Args,
    message::Message,
};
use crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use std::sync::Arc;
use tracing::trace;
use tui::{backend::CrosstermBackend, Terminal};
use tui_textarea::TextArea;

// TODO Can these use Cows instead?
struct WidgetState {
    pub conversation: Vec<Message>,
    pub status: String,
    pub textarea: TextArea<'static>,
}

pub struct State {
    pub app_tx: EventTx,
    pub backend_tx: EventTx,
    pub rx: EventRx,
    pub widget_state: WidgetState,
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub turn_to_speak: TurnToSpeak,
    pub env: Arc<Env>,
}

impl State {
    pub async fn new(
        rx: EventRx,
        backend_tx: EventTx,
        app_tx: EventTx,
        env: Arc<Env>,
        args: &Args,
    ) -> Result<Self, anyhow::Error> {
        trace!("setting up terminal");

        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let widget_state = WidgetState {
            conversation: Vec::new(),
            status: "loading the chatbot...".to_owned(),
            textarea: TextArea::default(),
        };

        Ok(Self {
            app_tx,
            backend_tx,
            rx,
            widget_state,
            terminal: Terminal::new(backend)?,
            turn_to_speak: TurnToSpeak::User,
            env,
        })
    }
}

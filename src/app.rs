mod ui;

use crate::{message::Message, state::State, USER_NAME};
use std::io::Stdout;
use tracing::trace;
use tui::{backend::CrosstermBackend, Terminal};
use tui_textarea::{Input, Key, TextArea};
use ui::{build_layout_chunks, build_messages_widget, build_status_widget};

pub struct App<'t> {
    state: State,
    terminal: &'t mut Terminal<CrosstermBackend<Stdout>>,
}

impl<'t, 'b> App<'t> {
    pub fn builder() -> Builder<'b> {
        Builder::new()
    }

    pub fn run_until_exit(mut self) -> Result<(), anyhow::Error> {
        let mut textarea = TextArea::default();

        loop {
            if self.state.should_quit {
                trace!("quitting");
                break;
            }

            self.terminal.draw(|f| {
                let chunks = build_layout_chunks(f);

                let messages_widget = build_messages_widget(&self.state);
                f.render_widget(messages_widget, chunks[0]);

                f.render_widget(textarea.widget(), chunks[1]);

                let status_widget = build_status_widget(&self.state);
                f.render_widget(status_widget, chunks[2]);
            })?;

            match crossterm::event::read()?.into() {
                Input { key: Key::Esc, .. } => {
                    self.state.should_quit = true;
                }
                Input {
                    key: Key::Enter, ..
                } => {
                    if self.state.user_is_typing() {
                        let content = textarea.into_lines().remove(0);
                        let message = Message {
                            sender: USER_NAME.to_owned(),
                            content,
                            timestamp: chrono::Utc::now(),
                            id: self.state.next_message_id(),
                        };
                        trace!(
                            message.timestamp = message.timestamp.to_rfc2822().as_str(),
                            message.id = message.id,
                            message.content = message.content,
                            "user sent message"
                        );

                        self.state.send_message(message);
                        // Clear the textarea by replacing it with a new one.
                        textarea = TextArea::default();
                    }
                }
                // Ignore these keyboard shortcuts
                Input {
                    key: Key::Char('m'),
                    ctrl: true,
                    alt: false,
                } => continue,
                // All other inputs are passed to the textarea input handler
                input => {
                    // Ignore user input unless it's their turn to type.
                    if self.state.user_is_typing() {
                        textarea.input(input);
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct Builder<'t> {
    state: Option<State>,
    terminal: Option<&'t mut Terminal<CrosstermBackend<Stdout>>>,
}

impl<'t> Builder<'t> {
    pub fn new() -> Self {
        Self {
            state: None,
            terminal: None,
        }
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = Some(state);
        self
    }

    pub fn terminal(mut self, terminal: &'t mut Terminal<CrosstermBackend<Stdout>>) -> Self {
        self.terminal = Some(terminal);
        self
    }

    pub fn build(self) -> App<'t> {
        App {
            state: self.state.expect("state is required"),
            terminal: self.terminal.expect("terminal is required"),
        }
    }
}

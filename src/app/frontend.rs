use crate::message::Message;
use anyhow::Context;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{borrow::Cow, io, sync::Arc};
use tokio::sync::mpsc;
use tracing::{debug, instrument, trace};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    Frame,
};
use tui::{backend::CrosstermBackend, Terminal};
use tui_textarea::{Input, Key, TextArea};

use super::{env::Env, Event, EventRx, EventTx};

enum Inner {
    AwaitingUserInput,
    AwaitingBotResponse,
}

// TODO Can these use Cows instead?
struct WidgetState {
    conversation: Vec<Message>,
    status: String,
    textarea: TextArea<'static>,
}

pub struct FrontendState {
    app_tx: EventTx,
    backend_tx: EventTx,
    rx: EventRx,
    widget_state: WidgetState,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    inner: Inner,
    env: Arc<Env>,
}

impl FrontendState {
    pub(super) async fn new(
        rx: EventRx,
        backend_tx: EventTx,
        app_tx: EventTx,
        env: Arc<Env>,
    ) -> Result<Self, anyhow::Error> {
        trace!("setting up terminal");

        enable_raw_mode()?;
        let mut stdout = io::stdout();
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
            inner: Inner::AwaitingUserInput,
            env,
        })
    }

    #[instrument(name = "frontend tick", skip(self))]
    pub(super) async fn tick(&mut self) -> Result<(), anyhow::Error> {
        trace!("handling user input...");
        while let Ok(true) = crossterm::event::poll(self.env.user_input_poll_duration()) {
            // This can potentially block although it shouldn't since I'm polling first. Still, I
            // feel weird about this and wonder if there's a better way.
            match crossterm::event::read()?.into() {
                Input { key: Key::Esc, .. } => {
                    self.app_tx
                        .send(Event::Quit)
                        .map_err(|e| anyhow::anyhow!("failed to send Quit event to app: {}", e))?;
                }
                Input {
                    key: Key::Enter, ..
                } => {
                    if self.widget_state.textarea.is_empty() {
                        debug!("user attempted to send message but it's empty");
                    } else if matches!(self.inner, Inner::AwaitingUserInput) {
                        debug!("sending message to backend after receiving Enter keypress");
                        // Clear the textarea by replacing it with a new one.
                        let content = std::mem::take(&mut self.widget_state.textarea)
                            .into_lines()
                            .remove(0);
                        self.backend_tx
                            .send(Event::UserMessage(content))
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "failed to send UserMessage event to backend: {}",
                                    e
                                )
                            })?;
                    } else {
                        debug!("user attempted to send message but it's not their turn");
                    }
                }
                // Ignore these keyboard shortcuts
                i @ Input {
                    key: Key::Char('m'),
                    ctrl: true,
                    alt: false,
                } => {
                    debug!("ignoring disabled keyboard shortcut {i:?}");
                }
                // All other inputs are passed to the textarea input handler
                input => {
                    // User input is always accepted but they can't send it until it's their turn to speak.
                    self.widget_state.textarea.input(input);
                }
            };
        }

        trace!("checking for received events...");
        loop {
            match self.rx.try_recv() {
                Ok(event) => match event {
                    Event::Quit => {
                        // App will call the quit method. We can't call it because it consumes `self`.
                    }
                    Event::ConversationUpdated(conversation) => {
                        // If the last sender is the bot, it's the user's turn to speak and vice versa.
                        match conversation.last() {
                            Some(Message { sender, .. }) if sender == self.env.your_name() => {
                                trace!("it's {}'s turn to speak", self.env.their_name());
                                self.inner = Inner::AwaitingBotResponse;
                            }
                            _ => {
                                trace!("it's {}'s turn to speak", self.env.your_name());
                                self.inner = Inner::AwaitingUserInput;
                            }
                        }

                        self.widget_state.conversation = conversation;
                    }
                    Event::StatusUpdated(status) => {
                        self.widget_state.status = status;
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
        self.terminal
            .draw(|f| {
                let chunks = build_layout_chunks(f);

                // TODO break this up into smaller functions
                if self.widget_state.conversation.is_empty() {
                    let p = Paragraph::new(Span::styled(
                        "This is a new conversation. Type your message and press Enter to start chatting.",
                        Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                    ))
                    .block(Block::default().borders(Borders::BOTTOM))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false });

                    f.render_widget(p, chunks[0]);
                } else {
                    let entries: Vec<_> = self.widget_state.conversation
                        .iter()
                        .flat_map(|m| {
                            [
                                Spans::from(vec![
                                    Span::styled(&m.sender, Style::default().add_modifier(Modifier::BOLD)),
                                    Span::raw(": "),
                                    Span::styled(
                                        m.timestamp.to_rfc2822(),
                                        Style::default()
                                            .fg(Color::Gray)
                                            .add_modifier(Modifier::ITALIC),
                                        ),
                                ]),
                                Spans::from(Span::raw(&m.content)),
                                // empty `Spans` to add a newline
                                Spans::default(),
                            ]
                        })
                        .collect();

                    let conversation_length = entries.len() as u16;
                    let bottom_of_conversation_block = chunks[0].bottom();

                    let scroll_offset = if bottom_of_conversation_block < conversation_length {
                        conversation_length - bottom_of_conversation_block + 1
                    } else {
                        0
                    };

                    let conversation = Paragraph::new(entries)
                    // TODO allow users to  scroll the conversation
                    // This will scroll down to the latest message in the conversation.
                    .scroll((scroll_offset, 0))
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::BOTTOM));

                    f.render_widget(conversation, chunks[0]);
                };

                f.render_widget(self.widget_state.textarea.widget(), chunks[1]);

                let status_widget = build_status_widget(self.widget_state.status.as_str().into());
                f.render_widget(status_widget, chunks[2]);
            })
            .map(|_| ())
            .context("failed to draw to terminal")?;

        Ok(())
    }

    pub async fn quit(self) -> Result<(), anyhow::Error> {
        Self::teardown_terminal(self.terminal).context("frontend quitting")
    }

    fn teardown_terminal(
        mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
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
}

fn build_layout_chunks<B: Backend>(f: &mut Frame<B>) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(70),
                Constraint::Percentage(20),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(f.size())
}

fn build_status_widget(status_message: Cow<'_, str>) -> impl Widget + '_ {
    let text = vec![Spans::from(Span::raw(status_message))];

    Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: false })
}

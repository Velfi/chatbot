use crate::{message::Message, BOT_NAME};
use anyhow::Context;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use tokio::sync::mpsc;
use tracing::trace;
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

use super::{Event, EventRx, EventTx};

enum Inner {
    AwaitingUserInput,
    AwaitingBotResponse,
}

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
}

impl FrontendState {
    pub(super) async fn new(
        rx: EventRx,
        backend_tx: EventTx,
        app_tx: EventTx,
    ) -> Result<Self, anyhow::Error> {
        trace!("setting up terminal");

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let widget_state = WidgetState {
            conversation: Vec::new(),
            status: "loading...".to_owned(),
            textarea: TextArea::default(),
        };

        Ok(Self {
            app_tx,
            backend_tx,
            rx,
            widget_state,
            terminal: Terminal::new(backend)?,
            inner: Inner::AwaitingUserInput,
        })
    }

    pub(super) async fn tick(&mut self) -> Result<(), anyhow::Error> {
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => {
                self.app_tx
                    .send(Event::Quit)
                    .map_err(|e| anyhow::anyhow!("failed to send Quit event to app: {}", e))?;
            }
            Input {
                key: Key::Enter, ..
            } => {
                if !self.widget_state.textarea.is_empty()
                    && matches!(self.inner, Inner::AwaitingUserInput)
                {
                    // Clear the textarea by replacing it with a new one.
                    let content = std::mem::take(&mut self.widget_state.textarea)
                        .into_lines()
                        .remove(0);
                    self.backend_tx
                        .send(Event::UserMessage(content))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to send UserMessage event to backend: {}", e)
                        })?;
                }
            }
            // Ignore these keyboard shortcuts
            Input {
                key: Key::Char('m'),
                ctrl: true,
                alt: false,
            } => (),
            // All other inputs are passed to the textarea input handler
            input => {
                // User input is always accepted but they can't send it until it's their turn to speak.
                self.widget_state.textarea.input(input);
            }
        };

        loop {
            match self.rx.try_recv() {
                Ok(event) => match event {
                    Event::Quit => {
                        // App will call the quit method. We can't call it because it consumes `self`.
                    }
                    Event::ConversationUpdated(conversation) => {
                        // If the last sender is the bot, it's the user's turn to speak and vice versa.
                        if conversation
                            .last()
                            .map(|m| m.sender == BOT_NAME)
                            .unwrap_or(false)
                        {
                            self.inner = Inner::AwaitingUserInput;
                        } else {
                            self.inner = Inner::AwaitingBotResponse;
                        };

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

        // We always redraw because the user may have resized the window or scrolled the conversation
        self.terminal
            .draw(|f| {
                let chunks = build_layout_chunks(f);

                let messages_widget = build_messages_widget(&self.widget_state.conversation);
                f.render_widget(messages_widget, chunks[0]);

                f.render_widget(self.widget_state.textarea.widget(), chunks[1]);

                let status_widget = build_status_widget(self.widget_state.status.clone());
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

fn build_messages_widget(messages: &[Message]) -> impl Widget + '_ {
    let text = messages
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
                // Empty line
                Spans::from(Span::raw("")),
            ]
        })
        .collect::<Vec<Spans>>();

    Paragraph::new(text)
        .block(Block::default().borders(Borders::BOTTOM))
        // .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
}

fn build_status_widget(status_message: String) -> impl Widget {
    let text = vec![Spans::from(Span::raw(status_message))];

    Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: false })
}

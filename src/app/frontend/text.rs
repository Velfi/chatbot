use super::{State, TurnToSpeak};
use crate::app::Event;
use anyhow::Context;
use std::borrow::Cow;
use tracing::debug;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    Frame,
};
use tui_textarea::{Input, Key};

pub fn handle_user_input(state: &mut State) -> Result<(), anyhow::Error> {
    while let Ok(true) = crossterm::event::poll(state.env.user_input_poll_duration()) {
        // This can potentially block although it shouldn't since I'm polling first. Still, I
        // feel weird about this and wonder if there's a better way.
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => {
                state
                    .app_tx
                    .send(Event::Quit)
                    .map_err(|e| anyhow::anyhow!("failed to send Quit event to app: {}", e))?;
            }
            Input {
                key: Key::Enter, ..
            } => {
                if state.widget_state.textarea.is_empty() {
                    debug!("user attempted to send message but it's empty");
                } else if matches!(state.turn_to_speak, TurnToSpeak::User) {
                    debug!("sending message to backend after receiving Enter keypress");
                    // Clear the textarea by replacing it with a new one.
                    let content = std::mem::take(&mut state.widget_state.textarea)
                        .into_lines()
                        .remove(0);
                    state
                        .backend_tx
                        .send(Event::UserMessage(content))
                        .map_err(|e| {
                            anyhow::anyhow!("failed to send UserMessage event to backend: {}", e)
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
                state.widget_state.textarea.input(input);
            }
        };
    }

    Ok(())
}

pub fn redraw_terminal(state: &mut State) -> Result<(), anyhow::Error> {
    state.terminal
            .draw(|f| {
                let chunks = build_layout_chunks(f);

                // TODO break this up into smaller functions
                if state.widget_state.conversation.is_empty() {
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
                    let entries: Vec<_> = state.widget_state.conversation
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

                f.render_widget(state.widget_state.textarea.widget(), chunks[1]);

                let status_widget = build_status_widget(state.widget_state.status.as_str().into());
                f.render_widget(status_widget, chunks[2]);
            })
            .map(|_| ())
            .context("failed to draw to terminal")
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

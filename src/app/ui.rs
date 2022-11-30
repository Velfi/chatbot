use crate::state::State;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    Frame,
};

pub(super) fn build_layout_chunks<B: Backend>(f: &mut Frame<B>) -> Vec<Rect> {
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

pub(super) fn build_messages_widget(s: &State) -> impl Widget + '_ {
    let text = s
        .messages
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

pub(super) fn build_status_widget(s: &State) -> impl Widget {
    let text = vec![Spans::from(Span::raw(s.status_message()))];

    Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: false })
}

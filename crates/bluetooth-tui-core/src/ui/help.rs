use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme;

const NAVIGATE: &[(&str, &str)] = &[
    ("↑ ↓", "move"),
    ("→", "drill into column"),
    ("←", "back one column"),
    ("tab", "cycle focus"),
    ("/", "cycle device filter"),
];

const VIEWS: &[(&str, &str)] = &[
    ("e", "events"),
    ("A", "adapter control"),
    ("v", "vendor chain"),
    ("f", "fullscreen detail"),
    (":", "command palette"),
];

const DEVICE_ACTIONS: &[(&str, &str)] = &[
    ("↵", "connect / disconnect"),
    ("p", "pair"),
    ("t T", "trust / untrust"),
    ("b B", "block / unblock"),
    ("a", "set alias"),
    ("F", "forget (removes keys)"),
];

const ADAPTER: &[(&str, &str)] = &[
    ("o", "power on / off"),
    ("s", "start / stop scan"),
    ("d", "discoverable"),
    ("k", "pairable"),
];

pub fn draw(frame: &mut Frame, area: Rect) {
    let modal = super::centered_rect(76, 24, area);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .style(Style::default().bg(theme::BG_MODAL))
        .title(Span::styled(
            " KEYMAP ",
            Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left = section("NAVIGATE", NAVIGATE)
        .into_iter()
        .chain(section("VIEWS", VIEWS))
        .collect::<Vec<_>>();
    let right = section("DEVICE ACTIONS", DEVICE_ACTIONS)
        .into_iter()
        .chain(section("ADAPTER", ADAPTER))
        .collect::<Vec<_>>();

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
}

fn section<'a>(title: &'a str, entries: &'a [(&str, &str)]) -> Vec<Line<'a>> {
    let mut lines = vec![
        Line::from(Span::styled(
            title,
            Style::default().fg(theme::TEXT_LABEL).add_modifier(Modifier::BOLD),
        )),
    ];
    for (key, desc) in entries {
        lines.push(Line::from(vec![
            Span::styled(format!("{key:<6}"), Style::default().fg(theme::AMBER)),
            Span::styled(*desc, Style::default().fg(theme::TEXT_VALUE)),
        ]));
    }
    lines.push(Line::default());
    lines
}

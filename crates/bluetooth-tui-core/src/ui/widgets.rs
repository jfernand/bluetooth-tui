use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, BannerKind, StatusBanner};
use bluetooth_driver::driver::BluetoothDriver;
use crate::theme;

/// The amber title bar every screen opens with: a label on the left, an
/// optional line of status chips on the right. Every screen routes through
/// here so the "bluetoothd unresponsive" badge shows up everywhere for
/// free, instead of every screen having to remember to render it.
pub fn header<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect, title: &str, right: Line<'_>) {
    let block = Block::default().style(Style::default().bg(theme::BG_BAR));
    frame.render_widget(block, area);

    let title = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default()
            .fg(theme::AMBER)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, inset(area, 1, 0));

    let mut spans = Vec::with_capacity(right.spans.len() + 2);
    if let Some(since) = app.bluez_unresponsive_since {
        spans.push(Span::styled(
            format!(" ⚠ BLUEZ NOT RESPONDING · {}s ", since.elapsed().as_secs()),
            Style::default()
                .fg(theme::ON_AMBER)
                .bg(theme::ERROR_FG)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw("  "));
    }
    spans.extend(right.spans);
    let right = Paragraph::new(Line::from(spans)).alignment(Alignment::Right);
    frame.render_widget(right, inset(area, 0, 1));
}

/// The single-line keybinding legend every screen ends with, e.g.
/// `↑↓ move   → drill   esc back`.
pub fn footer(frame: &mut Frame, area: Rect, hints: &[(&str, &str)]) {
    let block = Block::default().style(Style::default().bg(theme::BG_BAR));
    frame.render_widget(block, area);

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(*key, Style::default().fg(theme::AMBER)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, Style::default().fg(theme::TEXT_SECONDARY)));
    }
    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, inset(area, 1, 1));
}

/// A section label like `ADDRESS` or `STATE — EACH AXIS SEPARATE`.
pub fn section_title(text: &str) -> Line<'_> {
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(theme::TEXT_MUTED)
            .add_modifier(Modifier::BOLD),
    ))
}

/// A `label   value` row, as used throughout the detail panes.
pub fn field_line<'a>(label: &str, value: impl Into<Span<'a>>, label_width: usize) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label:<label_width$}"),
            Style::default().fg(theme::TEXT_LABEL),
        ),
        value.into(),
    ])
}

pub fn value(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(theme::TEXT_VALUE))
}

pub fn amber_value(text: impl Into<String>) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD),
    )
}

pub fn muted_value(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(theme::TEXT_MUTED))
}

/// The bordered error/info box used for structured `DriverError`
/// reporting and confirmations.
pub fn status_banner(banner: &StatusBanner) -> Paragraph<'_> {
    let (fg, border_color) = match banner.kind {
        BannerKind::Error => (theme::ERROR_FG, theme::ERROR_BORDER),
        BannerKind::Info => (theme::AMBER, theme::BORDER),
    };
    let bg = match banner.kind {
        BannerKind::Error => theme::ERROR_BG,
        BannerKind::Info => theme::BG_MODAL,
    };
    let lines = vec![
        Line::from(Span::styled(
            banner.title.as_str(),
            Style::default().fg(fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            banner.body.as_str(),
            Style::default().fg(theme::TEXT_VALUE),
        )),
    ];
    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg)),
    )
}

/// Confidence dots, e.g. `●●○` for 2 of 3.
pub fn confidence_dots(filled: u8, total: u8) -> Span<'static> {
    let mut s = String::new();
    for i in 0..total {
        s.push(if i < filled { '●' } else { '○' });
    }
    Span::styled(s, Style::default().fg(theme::AMBER))
}

fn inset(area: Rect, left: u16, right: u16) -> Rect {
    Rect::new(
        area.x + left,
        area.y,
        area.width.saturating_sub(left + right),
        area.height,
    )
}

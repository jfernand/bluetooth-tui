use bluetooth_driver::driver::{Adapter, BluetoothDriver};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme;

use super::widgets;

pub fn draw<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let Some(adapter) = app.current_adapter() else {
        widgets::header(frame, app, rows[0], "ADAPTER", Line::default());
        frame.render_widget(
            Paragraph::new("no adapter").style(Style::default().fg(theme::TEXT_MUTED)),
            rows[1],
        );
        return;
    };

    widgets::header(
        frame,
        app,
        rows[0],
        &format!("ADAPTER · {}", adapter.id()),
        Line::from(widgets::muted_value("esc back")),
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);

    draw_controls(frame, adapter, cols[0]);
    draw_properties(frame, adapter, cols[1]);

    let hints: &[(&str, &str)] = &[
        ("o", "power"),
        ("d", "discoverable"),
        ("k", "pairable"),
        ("s", "start/stop scan"),
        ("r", "refresh"),
        ("esc", "back"),
    ];
    widgets::footer(frame, rows[2], hints);

    if let Some(status) = &app.status {
        let banner_area = super::centered_rect(area.width.min(70), 4, area);
        frame.render_widget(widgets::status_banner(status), banner_area);
    }
}

fn toggle_line<'a>(label: &'a str, on: bool, detail: &'a str) -> Line<'a> {
    let state = if on {
        Span::styled("[ ON ]", Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ off ]", Style::default().fg(theme::TEXT_MUTED))
    };
    Line::from(vec![
        Span::raw(format!("{label:<16}")),
        state,
        Span::raw("  "),
        Span::styled(detail, Style::default().fg(theme::TEXT_MUTED)),
    ])
}

fn draw_controls(frame: &mut Frame, adapter: &impl Adapter, area: Rect) {
    let mut lines = vec![widgets::section_title("CONTROLS"), Line::default()];
    lines.push(toggle_line("Powered", adapter.is_powered(), "radio enabled"));
    lines.push(toggle_line(
        "Discoverable",
        adapter.is_discoverable(),
        "timeout 180s when enabled",
    ));
    lines.push(toggle_line("Pairable", adapter.is_pairable(), "no expiry"));
    let discovery_detail = if adapter.is_discovering() {
        "le + bredr"
    } else {
        "s to start"
    };
    lines.push(toggle_line("Discovery", adapter.is_discovering(), discovery_detail));

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_properties(frame: &mut Frame, adapter: &impl Adapter, area: Rect) {
    let lines = vec![
        widgets::section_title("PROPERTIES"),
        Line::default(),
        widgets::field_line("Address", widgets::value(adapter.address().to_string()), 14),
        widgets::field_line("Name", widgets::value(adapter.name().to_owned()), 14),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

use bluetooth_driver::driver::{Adapter, BluetoothDriver, Device};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::theme;

use super::widgets;

pub fn draw<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let scanning = app
        .current_adapter()
        .map(Adapter::is_discovering)
        .unwrap_or(false);
    let status = if scanning {
        let elapsed = app.scan_started_at.map(|t| t.elapsed().as_secs()).unwrap_or_default();
        Span::styled(
            format!("◐ SCANNING · {:02}:{:02}", elapsed / 60, elapsed % 60),
            Style::default().fg(theme::AMBER),
        )
    } else {
        Span::styled("STOPPED", Style::default().fg(theme::TEXT_MUTED))
    };
    widgets::header(frame, app, rows[0], "DISCOVERY", Line::from(status));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[1]);

    draw_list(frame, app, cols[0]);
    draw_candidate(frame, app, cols[1]);

    let hints: &[(&str, &str)] = &[
        ("s", "stop scan"),
        ("p", "pair"),
        ("c", "connect only"),
        ("v", "vendor"),
        ("esc", "back to devices"),
    ];
    widgets::footer(frame, rows[2], hints);

    if let Some(status) = &app.status {
        let banner_area = super::centered_rect(area.width.min(70), 4, area);
        frame.render_widget(widgets::status_banner(status), banner_area);
    }
}

fn draw_list<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let indices = app.visible_device_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|&i| {
            let d = &app.devices[i];
            let name = d
                .alias()
                .or(d.name())
                .map(str::to_owned)
                .unwrap_or_else(|| "Unknown".to_owned());
            let vendor = crate::app::quick_vendor_label(d).unwrap_or_else(|| "—".to_owned());
            let rssi = d.rssi().map(|r| r.0.to_string()).unwrap_or_default();
            ListItem::new(Line::from(vec![
                Span::raw(format!(" {name:<24.24} ")),
                Span::styled(
                    format!("{:<18} ", d.address().to_string()),
                    Style::default().fg(theme::TEXT_VALUE),
                ),
                Span::styled(format!("{vendor:<16.16} "), Style::default().fg(theme::TEXT_SECONDARY)),
                Span::styled(format!("{rssi:>5}"), Style::default().fg(theme::TEXT_SECONDARY)),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(pos) = app.selected_visible_position() {
        state = state.with_selected(Some(pos));
    }
    let list = List::new(items).highlight_style(Style::default().fg(theme::ON_AMBER).bg(theme::AMBER));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_candidate<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let Some(device) = app.selected_device() else {
        frame.render_widget(
            Paragraph::new("no devices found yet").style(Style::default().fg(theme::TEXT_MUTED)),
            area,
        );
        return;
    };

    let name = device
        .alias()
        .or(device.name())
        .map(str::to_owned)
        .unwrap_or_else(|| "Unknown".to_owned());

    let mut lines = vec![
        Line::from(Span::styled(
            name.to_uppercase(),
            Style::default().fg(theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        widgets::field_line("address", widgets::value(device.address().to_string()), 10),
    ];
    if let Some(rssi) = device.rssi() {
        lines.push(widgets::field_line("rssi", widgets::value(format!("{} dBm", rssi.0)), 10));
    }
    let uuids: Vec<String> = device.service_uuids().iter().map(ToString::to_string).collect();
    if !uuids.is_empty() {
        lines.push(widgets::field_line(
            "uuids",
            widgets::muted_value(format!("{} advertised", uuids.len())),
            10,
        ));
    }
    match crate::app::quick_vendor_label(device) {
        Some(v) => lines.push(widgets::field_line("vendor", widgets::amber_value(v), 10)),
        None => lines.push(widgets::field_line("vendor", widgets::muted_value("unresolved"), 10)),
    }

    frame.render_widget(Paragraph::new(lines), area);
}

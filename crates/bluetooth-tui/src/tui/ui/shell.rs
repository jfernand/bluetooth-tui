use bluetooth_driver::driver::{Adapter, Device};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::tui::app::{App, DeviceFilter, Focus};
use crate::tui::theme;

use super::widgets;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // breadcrumb
            Constraint::Min(3),    // columns
            Constraint::Length(1), // footer
        ])
        .split(area);

    draw_header(frame, app, rows[0]);
    draw_breadcrumb(frame, app, rows[1]);

    if app.fullscreen_detail {
        super::device_full::draw(frame, app, rows[2]);
    } else {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(28),
                Constraint::Length(58),
                Constraint::Min(30),
            ])
            .split(rows[2]);
        draw_adapters(frame, app, cols[0]);
        draw_devices(frame, app, cols[1]);
        draw_detail(frame, app, cols[2]);
    }

    let hints: &[(&str, &str)] = &[
        ("↑↓", "move"),
        ("→", "drill"),
        ("←", "back"),
        ("↵", "connect"),
        ("p", "pair"),
        ("s", "scan"),
        ("v", "vendor"),
        ("a", "alias"),
        ("t/T", "trust"),
        ("b/B", "block"),
        ("F", "forget"),
        ("e", "events"),
        ("A", "adapter"),
        ("/", "filter"),
        ("f", "fullscreen"),
        ("?", "help"),
        ("q", "quit"),
    ];
    widgets::footer(frame, rows[3], hints);

    if let Some(status) = &app.status {
        let banner_area = super::centered_rect(area.width.min(70), 4, area);
        frame.render_widget(widgets::status_banner(status), banner_area);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut right = vec![];
    if let Some(adapter) = app.current_adapter() {
        let power = if adapter.is_powered() {
            Span::styled("■ POWERED", Style::default().fg(theme::AMBER))
        } else {
            Span::styled("□ OFF", Style::default().fg(theme::TEXT_MUTED))
        };
        right.push(power);
        right.push(Span::raw("   "));
        right.push(widgets::muted_value(if adapter.is_discoverable() {
            "DISCOVERABLE ON"
        } else {
            "DISCOVERABLE OFF"
        }));
        right.push(Span::raw("   "));
        right.push(widgets::muted_value(if adapter.is_pairable() {
            "PAIRABLE ON"
        } else {
            "PAIRABLE OFF"
        }));
        if adapter.is_discovering() {
            right.push(Span::raw("   "));
            let elapsed = app
                .scan_started_at
                .map(|t| t.elapsed().as_secs())
                .unwrap_or_default();
            right.push(Span::styled(
                format!("◐ SCANNING {:02}:{:02}", elapsed / 60, elapsed % 60),
                Style::default().fg(theme::AMBER),
            ));
        }
    }
    right.push(Span::raw("   "));
    if app.unseen_events > 0 {
        right.push(Span::styled(
            format!(" EVENTS {} ", app.unseen_events),
            Style::default().fg(theme::ON_AMBER).bg(theme::AMBER),
        ));
    } else {
        right.push(widgets::muted_value(format!("events {}", app.events.len())));
    }

    widgets::header(frame, area, "BLUETOOTH EXPLORER", Line::from(right));
}

fn draw_breadcrumb(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![widgets::muted_value("adapters")];
    if let Some(adapter) = app.current_adapter() {
        spans.push(Span::styled(" / ", Style::default().fg(theme::TEXT_VERY_DIM)));
        spans.push(widgets::value(adapter.id().to_string()));
        spans.push(Span::styled(" / ", Style::default().fg(theme::TEXT_VERY_DIM)));
        spans.push(widgets::muted_value("devices"));
        if let Some(device) = app.selected_device() {
            spans.push(Span::styled(" / ", Style::default().fg(theme::TEXT_VERY_DIM)));
            spans.push(Span::styled(
                device.address().to_string(),
                Style::default().fg(theme::TEXT_PRIMARY),
            ));
        }
    }
    let block = Block::default().style(Style::default());
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_adapters(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" ADAPTERS ── {} ", app.adapters.len()),
            Style::default().fg(theme::TEXT_LABEL),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(6)])
        .split(inner);

    let items: Vec<ListItem> = app
        .adapters
        .iter()
        .map(|a| ListItem::new(Line::from(format!("{}  {}", a.id(), a.name()))))
        .collect();
    let mut state = ListState::default().with_selected(Some(app.adapter_idx));
    let highlight = focus_style(app.focus == Focus::Adapters);
    let list = List::new(items).highlight_style(highlight);
    frame.render_stateful_widget(list, rows[0], &mut state);

    if let Some(adapter) = app.current_adapter() {
        let lines = vec![
            widgets::section_title("ADAPTER"),
            widgets::field_line("address", widgets::value(adapter.address().to_string()), 8),
            widgets::field_line("devices", widgets::value(app.devices.len().to_string()), 8),
            widgets::field_line(
                "known",
                widgets::value(format!(
                    "{} paired",
                    app.devices.iter().filter(|d| d.is_paired()).count()
                )),
                8,
            ),
        ];
        frame.render_widget(Paragraph::new(lines), rows[1]);
    }
}

fn draw_devices(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" DEVICES ── {} ", app.visible_device_indices().len()),
            Style::default().fg(theme::TEXT_LABEL),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let filters = [
        DeviceFilter::All,
        DeviceFilter::Paired,
        DeviceFilter::Connected,
        DeviceFilter::Nearby,
        DeviceFilter::Blocked,
    ];
    let mut filter_spans = vec![Span::raw(" filter ")];
    for f in filters {
        let style = if f == app.device_filter {
            Style::default().fg(theme::ON_AMBER).bg(theme::AMBER)
        } else {
            Style::default().fg(theme::TEXT_SECONDARY)
        };
        filter_spans.push(Span::styled(format!(" {} ", f.label()), style));
    }
    frame.render_widget(Paragraph::new(Line::from(filter_spans)), rows[0]);

    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::BORDER_FAINT));
    let header_inner = header_block.inner(rows[1]);
    frame.render_widget(header_block, rows[1]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " NAME                     ADDR  P B C T X    RSSI",
            Style::default().fg(theme::TEXT_FAINT),
        )])),
        header_inner,
    );

    let indices = app.visible_device_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|&i| device_row(&app.devices[i]))
        .collect();
    let mut state = ListState::default();
    if !indices.is_empty() {
        state = state.with_selected(Some(app.device_idx.min(indices.len() - 1)));
    }
    let highlight = focus_style(app.focus == Focus::Devices);
    let list = List::new(items).highlight_style(highlight);
    frame.render_stateful_widget(list, rows[2], &mut state);
}

fn device_row(device: &bluetooth_driver::bluez::BluezDevice) -> ListItem<'static> {
    let name = device
        .alias()
        .or(device.name())
        .map(str::to_owned)
        .unwrap_or_else(|| device.address().to_string());
    let kind = match device.address_kind() {
        bluetooth_driver::driver::AddressKind::Public => "pub",
        bluetooth_driver::driver::AddressKind::Random => "rnd",
    };
    let flag = |set: bool, ch: char, color| {
        if set {
            Span::styled(ch.to_string(), Style::default().fg(color))
        } else {
            Span::styled("·", Style::default().fg(theme::TEXT_DIM))
        }
    };
    let rssi = device
        .rssi()
        .map(|r| format!("{:>4}", r.0))
        .unwrap_or_else(|| "   ·".to_owned());

    let line = Line::from(vec![
        Span::raw(format!(" {name:<24.24} ")),
        Span::styled(format!("{kind:<4}"), Style::default().fg(theme::TEXT_SECONDARY)),
        flag(device.is_paired(), 'P', theme::AMBER),
        Span::raw(" "),
        flag(device.is_bonded(), 'B', theme::AMBER),
        Span::raw(" "),
        flag(device.is_connected(), 'C', theme::AMBER),
        Span::raw(" "),
        flag(device.is_trusted(), 'T', theme::AMBER),
        Span::raw(" "),
        flag(device.is_blocked(), 'X', theme::ERROR_FG),
        Span::raw(format!(" {rssi} ")),
    ]);
    ListItem::new(line)
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::NONE);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(device) = app.selected_device() else {
        frame.render_widget(
            Paragraph::new("no device selected").style(Style::default().fg(theme::TEXT_MUTED)),
            inner,
        );
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    let title = device
        .alias()
        .or(device.name())
        .map(str::to_owned)
        .unwrap_or_else(|| "(unnamed)".to_owned());
    lines.push(Line::from(Span::styled(
        title.to_uppercase(),
        Style::default().fg(theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "a to edit alias",
        Style::default().fg(theme::TEXT_MUTED),
    )));
    lines.push(Line::default());

    lines.push(widgets::section_title("ADDRESS"));
    lines.push(widgets::field_line("address", widgets::value(device.address().to_string()), 12));
    let kind = match device.address_kind() {
        bluetooth_driver::driver::AddressKind::Public => "public — fixed, burned into hardware",
        bluetooth_driver::driver::AddressKind::Random => "random — rotates for privacy",
    };
    lines.push(widgets::field_line("kind", widgets::value(kind), 12));
    if let Some(class) = device.class() {
        let cod = bluetooth_driver::device_class::decode(class.0);
        let mut label = cod.major_device_class.to_owned();
        if let Some(minor) = cod.minor_device_class {
            label.push_str(": ");
            label.push_str(minor);
        }
        if let Some(sub) = cod.minor_device_subclass {
            label.push_str(" / ");
            label.push_str(sub);
        }
        lines.push(widgets::field_line("class", widgets::value(label), 12));
    }
    if let Some(appearance) = device.appearance() {
        let label = match bluetooth_driver::gap_appearance::name(appearance) {
            Some(name) => name.to_owned(),
            None => format!("0x{appearance:04X}"),
        };
        lines.push(widgets::field_line("appearance", widgets::value(label), 12));
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("VENDOR"));
    match crate::tui::app::quick_vendor_label(device) {
        Some(v) => lines.push(widgets::field_line("guess", widgets::amber_value(v), 12)),
        None => lines.push(widgets::field_line("guess", widgets::muted_value("unresolved"), 12)),
    }
    lines.push(widgets::field_line("chain", widgets::muted_value("v to inspect full chain"), 12));
    lines.push(Line::default());

    lines.push(widgets::section_title("STATE — EACH AXIS SEPARATE"));
    for (label, set) in [
        ("paired", device.is_paired()),
        ("bonded", device.is_bonded()),
        ("connected", device.is_connected()),
        ("trusted", device.is_trusted()),
        ("blocked", device.is_blocked()),
    ] {
        let v = if set {
            widgets::amber_value("yes")
        } else {
            widgets::muted_value("no")
        };
        lines.push(widgets::field_line(label, v, 12));
    }
    lines.push(Line::default());

    let uuids = device.service_uuids();
    let services_title = format!("SERVICES — {}", uuids.len());
    lines.push(widgets::section_title(&services_title));
    if uuids.is_empty() {
        lines.push(Line::from(widgets::muted_value("none advertised")));
    } else {
        const SHOWN: usize = 8;
        for uuid in uuids.iter().take(SHOWN) {
            let name = bluetooth_driver::gatt_uuid::service_name(*uuid)
                .map(str::to_owned)
                .unwrap_or_else(|| uuid.to_string());
            lines.push(Line::from(widgets::value(name)));
        }
        if uuids.len() > SHOWN {
            lines.push(Line::from(widgets::muted_value(format!(
                "…{} more — f for full list",
                uuids.len() - SHOWN
            ))));
        }
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("LINK"));
    if let Some(rssi) = device.rssi() {
        lines.push(widgets::field_line("rssi", widgets::value(format!("{} dBm", rssi.0)), 12));
    }
    if device.is_connected() {
        match app.battery {
            Some(percent) => {
                lines.push(widgets::field_line("battery", widgets::value(format!("{percent}%")), 12));
            }
            None => lines.push(widgets::field_line("battery", widgets::muted_value("—"), 12)),
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(theme::ON_AMBER).bg(theme::AMBER)
    } else {
        Style::default().fg(theme::TEXT_PRIMARY).bg(theme::BG_SELECTED_DIM)
    }
}

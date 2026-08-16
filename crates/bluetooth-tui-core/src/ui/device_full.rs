use bluetooth_driver::driver::{Adapter, AddressKind, BluetoothDriver, Device, VendorIdSource};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::theme;

use super::widgets;

const MAX_UUIDS_SHOWN: usize = 20;

/// The fullscreen raw-property dump (`f` in the Shell screen): every
/// BlueZ `Device1` property we track, plus advertising data and GATT
/// Device Information, split into two columns.
pub fn draw<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(area);

    let Some(device) = app.selected_device() else {
        frame.render_widget(
            Paragraph::new("no device selected").style(Style::default().fg(theme::TEXT_MUTED)),
            rows[1],
        );
        return;
    };

    draw_sub_header(frame, app, device, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    draw_left(frame, app, device, cols[0]);
    draw_right(frame, app, device, cols[1]);
}

fn draw_sub_header<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, device: &impl Device, area: Rect) {
    frame.render_widget(Block::default().style(Style::default().bg(theme::BG_BAR)), area);

    let name = device
        .alias()
        .or(device.name())
        .unwrap_or("UNKNOWN")
        .to_uppercase();
    let adapter_id = app
        .current_adapter()
        .map(|a| a.id().to_string())
        .unwrap_or_default();
    let left = format!(" DEVICE · {name}");
    let mid = format!("{adapter_id} / {}", device.address());

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            left,
            Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD),
        ))),
        area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(widgets::muted_value(mid))).alignment(Alignment::Center),
        area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(widgets::muted_value("esc back")))
            .alignment(Alignment::Right),
        area,
    );
}

fn yes_no(set: bool) -> Span<'static> {
    if set {
        widgets::amber_value("true")
    } else {
        widgets::muted_value("false")
    }
}

// Built incrementally with real conditionals threaded throughout, so a
// `vec![]` literal (clippy's usual suggestion here) doesn't fit.
#[allow(clippy::vec_init_then_push)]
fn draw_left<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, device: &impl Device, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(widgets::section_title("IDENTITY"));
    lines.push(widgets::field_line(
        "Name",
        widgets::value(device.name().unwrap_or("—").to_owned()),
        18,
    ));
    lines.push(widgets::field_line(
        "Alias",
        widgets::value(device.alias().unwrap_or("—").to_owned()),
        18,
    ));
    lines.push(widgets::field_line("Address", widgets::value(device.address().to_string()), 18));
    let kind = match device.address_kind() {
        AddressKind::Public => "public",
        AddressKind::Random => "random",
    };
    lines.push(widgets::field_line("AddressType", widgets::value(kind), 18));
    match device.class() {
        Some(class) => {
            let cod = bluetooth_driver::device_class::decode(class.0);
            let mut label = format!("0x{:06x} ", class.0);
            label.push_str(cod.major_device_class);
            if let Some(minor) = cod.minor_device_class {
                label.push_str(" / ");
                label.push_str(minor);
            }
            lines.push(widgets::field_line("Class", widgets::value(label), 18));
        }
        None => lines.push(widgets::field_line("Class", widgets::muted_value("—"), 18)),
    }
    match device.appearance() {
        Some(appearance) => {
            let label = match bluetooth_driver::gap_appearance::name(appearance) {
                Some(name) => name.to_owned(),
                None => format!("0x{appearance:04X}"),
            };
            lines.push(widgets::field_line("Appearance", widgets::value(label), 18));
        }
        None => lines.push(widgets::field_line("Appearance", widgets::muted_value("—"), 18)),
    }
    lines.push(widgets::field_line(
        "Icon",
        widgets::value(device.icon().unwrap_or("—").to_owned()),
        18,
    ));
    if let Some(adapter) = app.current_adapter() {
        lines.push(widgets::field_line("Adapter", widgets::value(adapter.id().to_string()), 18));
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("STATE"));
    lines.push(widgets::field_line("Paired", yes_no(device.is_paired()), 18));
    lines.push(widgets::field_line("Bonded", yes_no(device.is_bonded()), 18));
    lines.push(widgets::field_line("Connected", yes_no(device.is_connected()), 18));
    lines.push(widgets::field_line("Trusted", yes_no(device.is_trusted()), 18));
    lines.push(widgets::field_line("Blocked", yes_no(device.is_blocked()), 18));
    lines.push(widgets::field_line("LegacyPairing", yes_no(device.is_legacy_pairing()), 18));
    lines.push(widgets::field_line(
        "ServicesResolved",
        yes_no(device.are_services_resolved()),
        18,
    ));
    lines.push(widgets::field_line("WakeAllowed", yes_no(device.is_wake_allowed()), 18));
    lines.push(Line::default());

    lines.push(widgets::section_title("RADIO"));
    match device.rssi() {
        Some(rssi) => lines.push(widgets::field_line("RSSI", widgets::value(format!("{} dBm", rssi.0)), 18)),
        None => lines.push(widgets::field_line("RSSI", widgets::muted_value("—"), 18)),
    }
    match device.tx_power() {
        Some(tx) => lines.push(widgets::field_line("TxPower", widgets::value(format!("{tx} dBm")), 18)),
        None => lines.push(widgets::field_line("TxPower", widgets::muted_value("—"), 18)),
    }
    match app.last_seen.get(&device.address()) {
        Some(t) => {
            lines.push(widgets::field_line(
                "Last seen",
                widgets::value(format!("{}s ago", t.elapsed().as_secs())),
                18,
            ));
        }
        None => lines.push(widgets::field_line(
            "Last seen",
            widgets::muted_value("not observed this session"),
            18,
        )),
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_right<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, device: &impl Device, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    let uuids = device.service_uuids();
    let uuids_title = format!("UUIDS — {}", uuids.len());
    lines.push(widgets::section_title(&uuids_title));
    if uuids.is_empty() {
        lines.push(Line::from(widgets::muted_value("none advertised")));
    } else {
        for uuid in uuids.iter().take(MAX_UUIDS_SHOWN) {
            let name = bluetooth_driver::gatt_uuid::service_name(*uuid)
                .map(str::to_owned)
                .unwrap_or_else(|| uuid.to_string());
            lines.push(Line::from(widgets::value(name)));
        }
        if uuids.len() > MAX_UUIDS_SHOWN {
            lines.push(Line::from(widgets::muted_value(format!(
                "…{} more",
                uuids.len() - MAX_UUIDS_SHOWN
            ))));
        }
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("MANUFACTURER DATA"));
    let manufacturer_data = device.manufacturer_data();
    if manufacturer_data.is_empty() {
        lines.push(Line::from(widgets::muted_value("none")));
    } else {
        for (id, bytes) in manufacturer_data {
            let vendor = bluetooth_driver::company_id::vendor(*id)
                .map(|n| format!("{n} (SIG company ID)"))
                .unwrap_or_else(|| "unrecognized company ID".to_owned());
            lines.push(widgets::field_line(&format!("0x{id:04x}"), widgets::value(vendor), 10));
            for row in hex_dump(bytes) {
                lines.push(Line::from(widgets::muted_value(row)));
            }
        }
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("SERVICE DATA"));
    let service_data = device.service_data();
    if service_data.is_empty() {
        lines.push(Line::from(widgets::muted_value("none")));
    } else {
        for (uuid, bytes) in service_data {
            let name = bluetooth_driver::gatt_uuid::service_name(*uuid)
                .map(str::to_owned)
                .unwrap_or_else(|| uuid.to_string());
            lines.push(Line::from(widgets::value(name)));
            for row in hex_dump(bytes) {
                lines.push(Line::from(widgets::muted_value(row)));
            }
        }
    }
    lines.push(Line::default());

    lines.push(widgets::section_title("GATT — DEVICE INFORMATION"));
    match &app.full_info {
        Some(info) => {
            lines.push(widgets::field_line(
                "Manufacturer",
                widgets::value(info.device_info.manufacturer.clone().unwrap_or_else(|| "—".to_owned())),
                16,
            ));
            lines.push(widgets::field_line(
                "Model",
                widgets::value(info.device_info.model.clone().unwrap_or_else(|| "—".to_owned())),
                16,
            ));
            lines.push(widgets::field_line(
                "Firmware",
                widgets::value(info.device_info.firmware.clone().unwrap_or_else(|| "—".to_owned())),
                16,
            ));
            match &info.pnp_id {
                Some(pnp) => {
                    let source = match pnp.vendor_id_source {
                        VendorIdSource::BluetoothSig => "BT-SIG",
                        VendorIdSource::Usb => "USB",
                    };
                    let label = format!(
                        "src={source} vid=0x{:04x} pid=0x{:04x}",
                        pnp.vendor_id, pnp.product_id
                    );
                    lines.push(widgets::field_line("PnP ID", widgets::value(label), 16));
                }
                None => lines.push(widgets::field_line("PnP ID", widgets::muted_value("—"), 16)),
            }
        }
        None if device.is_connected() => {
            lines.push(Line::from(widgets::muted_value("reading…")));
        }
        None => {
            lines.push(Line::from(widgets::muted_value("connect to read")));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn hex_dump(bytes: &[u8]) -> Vec<String> {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
            format!("{:04x}  {}", i * 16, hex.join(" "))
        })
        .collect()
}

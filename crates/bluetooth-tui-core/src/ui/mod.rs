mod adapter_control;
mod device_full;
mod discovery;
mod event_log;
mod help;
mod overlays;
mod shell;
mod widgets;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use bluetooth_driver::driver::BluetoothDriver;

use crate::app::{App, Overlay, Screen};
use crate::theme;

/// Renders the whole UI for the current frame: the active screen, then
/// any open overlay/modal, then a status banner if one's showing. Call
/// this once per redraw - both frontends' run loops do.
pub fn draw<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(theme::BG_BASE)), area);

    match app.screen {
        Screen::Shell => shell::draw(frame, app, area),
        Screen::Discovery => discovery::draw(frame, app, area),
        Screen::EventLog => event_log::draw(frame, app, area),
        Screen::AdapterControl => adapter_control::draw(frame, app, area),
    }

    match app.overlay {
        Overlay::None => {}
        Overlay::Vendor => overlays::draw_vendor(frame, app, area),
        Overlay::AliasEdit => overlays::draw_alias_edit(frame, app, area),
        Overlay::Help => help::draw(frame, area),
        Overlay::Palette => overlays::draw_palette(frame, app, area),
        Overlay::ConfirmForget => overlays::draw_confirm_forget(frame, app, area),
    }
}

/// Centers a fixed-size rect within `area` - the standard placement for
/// modals over the shell.
pub(crate) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

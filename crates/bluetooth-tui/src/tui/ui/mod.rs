mod adapter_control;
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

use crate::tui::app::{App, Overlay, Screen};
use crate::tui::theme;

pub fn draw(frame: &mut Frame, app: &App) {
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

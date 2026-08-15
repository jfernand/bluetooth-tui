use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

use crate::tui::app::App;
use crate::tui::theme;

use super::widgets;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    widgets::header(
        frame,
        app,
        rows[0],
        "EVENTS",
        Line::from(widgets::muted_value(format!("{} total", app.events.len()))),
    );

    let items: Vec<ListItem> = app
        .events
        .iter()
        .rev()
        .take(rows[1].height as usize)
        .map(|logged| {
            let secs = logged.at.elapsed().as_secs();
            let is_error = matches!(
                logged.event,
                bluetooth_driver::driver::DriverEvent::AdapterRemoved(_)
            );
            let color = if is_error { theme::ERROR_FG } else { theme::TEXT_SECONDARY };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{secs:>4}s ago  "), Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(logged.event.to_string(), Style::default().fg(color)),
            ]))
        })
        .collect();
    frame.render_widget(List::new(items), rows[1]);

    let hints: &[(&str, &str)] = &[("esc", "back")];
    widgets::footer(frame, rows[2], hints);
}

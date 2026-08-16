use bluetooth_driver::driver::{BluetoothDriver, Device};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::theme;

use super::widgets;

pub fn draw_vendor<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let modal = super::centered_rect(76, 22, area);
    frame.render_widget(Clear, modal);

    let Some(info) = &app.vendor_info else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::AMBER))
            .style(Style::default().bg(theme::BG_MODAL))
            .title(" VENDOR ATTRIBUTION ");
        frame.render_widget(Paragraph::new("resolving…").block(block), modal);
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::AMBER))
        .style(Style::default().bg(theme::BG_MODAL))
        .title(" VENDOR ATTRIBUTION ");
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let mut lines = Vec::new();
    match &info.resolved {
        Some(name) => lines.push(Line::from(vec![
            Span::styled(
                name.clone(),
                Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            widgets::confidence_dots(3, 3),
        ])),
        None => lines.push(Line::from(Span::styled(
            "Unresolved",
            Style::default().fg(theme::TEXT_VALUE).add_modifier(Modifier::BOLD),
        ))),
    }
    lines.push(Line::default());

    let agree = info.tiers.iter().filter(|t| t.result.is_some()).count();
    for tier in &info.tiers {
        lines.push(Line::from(vec![
            widgets::confidence_dots(tier.confidence, 3),
            Span::raw("  "),
            Span::styled(tier.label, Style::default().fg(theme::TEXT_PRIMARY)),
        ]));
        // A tier that found *something* but disagrees with the resolved
        // answer (e.g. a Microsoft Swift Pair beacon on a non-Microsoft
        // device) is exactly the case worth flagging, not quietly
        // averaging away.
        let conflicts = matches!((&tier.result, &info.resolved), (Some(r), Some(resolved)) if r != resolved);
        match &tier.result {
            Some(r) => lines.push(Line::from(widgets::value(r.clone()))),
            None => lines.push(Line::from(widgets::muted_value("unavailable"))),
        }
        let caveat_color = if conflicts { theme::AMBER_WARN } else { theme::TEXT_LABEL };
        lines.push(Line::from(Span::styled(tier.caveat, Style::default().fg(caveat_color))));
        lines.push(Line::default());
    }
    lines.push(Line::from(format!("{agree} of {} tiers agree", info.tiers.len())));
    lines.push(Line::from(Span::styled(
        "c connect to upgrade   esc close",
        Style::default().fg(theme::TEXT_MUTED),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_alias_edit<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let modal = super::centered_rect(66, 8, area);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::AMBER))
        .style(Style::default().bg(theme::BG_MODAL))
        .title(" EDIT ALIAS ");
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let device_name = app
        .selected_device()
        .and_then(Device::name)
        .unwrap_or("(unknown)");

    let lines = vec![
        Line::from(vec![
            Span::styled("alias ▸ ", Style::default().fg(theme::AMBER)),
            Span::styled(
                format!("{}█", app.alias_buffer),
                Style::default().fg(theme::TEXT_PRIMARY),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            format!("Name reported by device: {device_name}"),
            Style::default().fg(theme::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "empty value resets to device name",
            Style::default().fg(theme::TEXT_MUTED),
        )),
        Line::default(),
        Line::from(Span::styled(
            "↵ save   esc cancel",
            Style::default().fg(theme::TEXT_SECONDARY),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_confirm_forget<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let modal = super::centered_rect(60, 6, area);
    frame.render_widget(Clear, modal);

    let name = app
        .selected_device()
        .map(|d| {
            d.alias()
                .or(d.name())
                .map(str::to_owned)
                .unwrap_or_else(|| d.address().to_string())
        })
        .unwrap_or_else(|| "(no device)".to_owned());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ERROR_BORDER))
        .style(Style::default().bg(theme::ERROR_BG))
        .title(" FORGET DEVICE ");
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let lines = vec![
        Line::from(Span::styled(
            format!("Forget {name}? This drops bonding keys."),
            Style::default().fg(theme::TEXT_VALUE),
        )),
        Line::from(Span::styled(
            "y confirm   any other key cancels",
            Style::default().fg(theme::ERROR_FG),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_palette<D: BluetoothDriver>(frame: &mut Frame, app: &App<D>, area: Rect) {
    let matches = crate::app::PaletteCommand::filtered(&app.palette_buffer);

    // Content is one input line plus one line per match (or one "no
    // matching command" line), plus two rows for the top/bottom border.
    let content_lines = 1 + matches.len().max(1) as u16;
    let modal = super::centered_rect(66, (content_lines + 2).min(16), area);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::AMBER))
        .style(Style::default().bg(theme::BG_MODAL));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let mut lines = vec![Line::from(vec![
        Span::styled(": ", Style::default().fg(theme::AMBER)),
        Span::styled(
            format!("{}█", app.palette_buffer),
            Style::default().fg(theme::TEXT_PRIMARY),
        ),
        Span::raw("   "),
        widgets::muted_value(format!("{} matches", matches.len())),
    ])];

    for (i, entry) in matches.iter().enumerate() {
        let name_style = if i == 0 {
            Style::default().fg(theme::ON_AMBER).bg(theme::AMBER)
        } else {
            Style::default().fg(theme::TEXT_PRIMARY)
        };
        let mut spans = vec![
            Span::styled(format!("{:<16}", entry.1), name_style),
            Span::raw(" "),
            Span::styled(entry.2, Style::default().fg(theme::TEXT_SECONDARY)),
        ];
        if let Some(hint) = entry.3 {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(hint, Style::default().fg(theme::TEXT_MUTED)));
        }
        lines.push(Line::from(spans));
    }
    if matches.is_empty() {
        lines.push(Line::from(widgets::muted_value("no matching command")));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

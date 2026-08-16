use std::time::Duration;

use bluetooth_driver::bluez::BluezDriver;
use bluetooth_driver::driver::{BluetoothDriver, EventStream};
use bluetooth_tui_core::{App, Key, ui};
use crossterm::event::{Event as CrosstermEvent, EventStream as CrosstermEventStream, KeyEventKind};
use futures_util::StreamExt;

pub async fn run() -> anyhow::Result<()> {
    let driver = BluezDriver::system().await?;
    let mut app = App::new(driver).await?;
    let mut driver_events = app.driver.events().await?;

    let mut terminal = ratatui::init();
    let mut crossterm_events = CrosstermEventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(250));

    let result = loop {
        if let Err(e) = terminal.draw(|frame| ui::draw(frame, &app)) {
            break Err(e.into());
        }

        tokio::select! {
            maybe_event = crossterm_events.next() => {
                match maybe_event {
                    Some(Ok(CrosstermEvent::Key(key))) if key.kind == KeyEventKind::Press => {
                        if let Some(key) = key_from_crossterm(key.code) {
                            app.handle_key(key).await;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => break Err(e.into()),
                    None => break Ok(()),
                }
            }
            driver_event = driver_events.next() => {
                if let Some(event) = driver_event {
                    app.handle_driver_event(event).await;
                }
            }
            _ = tick.tick() => {
                app.on_tick().await;
            }
        }

        if app.should_quit {
            break Ok(());
        }
    };

    ratatui::restore();
    result
}

/// Translates crossterm's key codes into `bluetooth-tui-core`'s
/// backend-agnostic `Key` - that crate is also built for a wasm32
/// frontend, and crossterm doesn't compile there at all, so it can't
/// depend on crossterm's type directly. Keys `App` never matches on
/// (function keys, media keys, ...) map to `None` and are dropped.
fn key_from_crossterm(code: crossterm::event::KeyCode) -> Option<Key> {
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}

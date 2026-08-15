mod app;
mod theme;
mod ui;

use std::time::Duration;

use bluetooth_driver::bluez::BluezDriver;
use bluetooth_driver::driver::{BluetoothDriver, EventStream};
use crossterm::event::{Event as CrosstermEvent, EventStream as CrosstermEventStream, KeyEventKind};
use futures_util::StreamExt;

use app::App;

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
                        app.handle_key(key.code).await;
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

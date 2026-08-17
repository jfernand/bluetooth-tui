use std::time::Duration;

use bluetooth_driver_web::WebBluetoothDriver;
use bluetooth_tui_core::{App, Key, ui};
use futures_util::StreamExt;
use ratzilla::event::KeyCode as RatzillaKeyCode;
use ratzilla::ratatui::Terminal;
use ratzilla::{DomBackend, WebRenderer};

/// Ratzilla apps are invoked like any other wasm32 `bin` crate's
/// `main()` (Trunk's generated glue calls it on load) - but everything
/// interesting here is async (connecting to `navigator.bluetooth`,
/// awaiting driver calls), so this just hands off to a spawned local
/// task immediately. There's no tokio runtime in a browser;
/// `wasm_bindgen_futures::spawn_local` is the wasm-native equivalent.
fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run().await {
            // Nothing else is on screen yet if setup itself failed -
            // the browser console is the only place left to say why.
            web_sys::console::error_1(&format!("bluetooth-tui-web failed to start: {e:#}").into());
        }
    });
}

enum WebEvent {
    Key(Key),
    Tick,
}

async fn run() -> anyhow::Result<()> {
    let driver = WebBluetoothDriver::new()?;
    let mut app = App::new(driver).await?;

    let backend = DomBackend::new()?;
    let mut terminal = Terminal::new(backend)?;

    // Both the key handler and the tick timer just push an event onto
    // this channel rather than touching `app` directly. That funnels
    // every state change through the one loop below, exactly one at a
    // time - the same property the native frontend's tokio::select!
    // loop gets from being a single task. Without it, two independent
    // spawned tasks (e.g. a keypress arriving mid-tick, or two
    // keypresses in a row before the first's driver call resolves)
    // could each try to mutate `app` concurrently.
    let (tx, mut rx) = futures_channel::mpsc::unbounded::<WebEvent>();

    {
        let tx = tx.clone();
        terminal.on_key_event(move |key_event| {
            let Some(key) = key_from_ratzilla(key_event.code) else {
                return;
            };
            // unbounded_send only fails if the receiver's gone, i.e.
            // the app loop below has already exited - nothing to do
            // about a keypress at that point.
            let _ = tx.unbounded_send(WebEvent::Key(key));
        })?;
    }

    {
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                futures_timer::Delay::new(Duration::from_millis(250)).await;
                if tx.unbounded_send(WebEvent::Tick).is_err() {
                    return;
                }
            }
        });
    }

    // Redraw after every processed event rather than continuously via
    // requestAnimationFrame (ratzilla's usual draw_web) - this TUI only
    // ever changes in response to a key or a tick, so there's nothing
    // to gain from redrawing 60 times a second while idle, and it lets
    // `app`/`terminal` stay plainly owned here instead of shared
    // through an Rc<RefCell<_>> with a separate render callback.
    terminal.draw(|frame| ui::draw(frame, &app))?;
    while let Some(event) = rx.next().await {
        match event {
            // 's' natively means "start scanning" - there's no
            // equivalent here (no background scan, see
            // WebAdapter::start_discovery's doc comment), so it's
            // repurposed for the nearest analogous action: opening the
            // browser's native device chooser. Intercepted here rather
            // than in the shared App::handle_key because
            // request_new_device() is a WebAdapter-only inherent
            // method, not part of the Adapter trait - App<D> only
            // knows D::Adapter generically, and it wouldn't make sense
            // on that trait anyway (see the doc comment: it's a one-
            // shot, user-gesture-gated action, nothing like
            // start_discovery/stop_discovery's shape).
            WebEvent::Key(Key::Char('s')) => add_device(&mut app).await,
            WebEvent::Key(key) => app.handle_key(key).await,
            WebEvent::Tick => app.on_tick().await,
        }
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Opens the browser's device chooser and adds whatever device the
/// user picks to the list. Must run directly from a key event handler
/// (a user gesture) or the browser rejects it outright.
async fn add_device(app: &mut App<WebBluetoothDriver>) {
    let result = match app.current_adapter() {
        Some(adapter) => Some(adapter.request_new_device().await),
        None => None,
    };
    match result {
        Some(Ok(device)) => {
            app.devices.push(device);
        }
        Some(Err(e)) => app.set_status_err("ADD DEVICE FAILED", &e),
        None => {}
    }
}

/// Translates ratzilla's key codes into `bluetooth-tui-core`'s
/// backend-agnostic `Key` - the same shape crossterm's KeyCode has, by
/// design (ratzilla wraps `web_sys::KeyboardEvent` itself rather than
/// depending on crossterm, which doesn't compile on wasm32 at all).
/// Keys `App` never matches on (function keys, Delete, Home/End, ...)
/// map to `None` and are dropped, same as the native frontend.
fn key_from_ratzilla(code: RatzillaKeyCode) -> Option<Key> {
    match code {
        RatzillaKeyCode::Up => Some(Key::Up),
        RatzillaKeyCode::Down => Some(Key::Down),
        RatzillaKeyCode::Left => Some(Key::Left),
        RatzillaKeyCode::Right => Some(Key::Right),
        RatzillaKeyCode::Enter => Some(Key::Enter),
        RatzillaKeyCode::Esc => Some(Key::Esc),
        RatzillaKeyCode::Tab => Some(Key::Tab),
        RatzillaKeyCode::Backspace => Some(Key::Backspace),
        RatzillaKeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}

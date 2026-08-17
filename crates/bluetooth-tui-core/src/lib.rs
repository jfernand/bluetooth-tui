#![warn(missing_docs)]
//! Shared app state and rendering, generic over `bluetooth_driver`'s
//! `BluetoothDriver` trait - reused as-is by the native (crossterm) and
//! web (ratzilla) frontends. Each frontend owns its own run loop and
//! translates its own key event type into this crate's [`Key`]; only
//! state and rendering live here.

/// [`App`], the shared state machine every frontend drives.
pub mod app;
pub mod key;
pub mod theme;
/// [`ui::draw`], the top-level render entry point, plus the screen/overlay/widget modules it dispatches to.
pub mod ui;

pub use app::App;
pub use key::Key;

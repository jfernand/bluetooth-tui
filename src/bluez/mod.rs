//! BlueZ-over-D-Bus implementation of the backend-agnostic `driver` traits.
//!
//! Talks to `bluetoothd` via its `org.bluez` D-Bus API (the same one
//! `bluetoothctl` itself uses), rather than a raw HCI socket, so it shares
//! state with the system's Bluetooth daemon instead of fighting it for the
//! controller.

mod error;
mod path;
mod properties;
mod proxy;

pub mod adapter;
pub mod device;
pub mod driver;
pub mod events;

pub use adapter::BluezAdapter;
pub use device::BluezDevice;
pub use driver::BluezDriver;
pub use events::BluezEvents;

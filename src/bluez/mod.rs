//! BlueZ-over-D-Bus implementation of the backend-agnostic `driver` traits.
//!
//! Talks to `bluetoothd` via its `org.bluez` D-Bus API (the same one
//! `bluetoothctl` itself uses), rather than a raw HCI socket, so it shares
//! state with the system's Bluetooth daemon instead of fighting it for the
//! controller.

// Not wired into `lib.rs` yet, so nothing here is used yet; drop this once
// the backend is assembled and exposed.
#![allow(dead_code)]

mod error;
mod path;
mod properties;
mod proxy;

pub mod device;

pub use device::BluezDevice;

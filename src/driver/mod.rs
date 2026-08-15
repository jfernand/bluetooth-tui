//! Backend-agnostic Bluetooth driver-layer traits.
//!
//! These mirror the operations `bluetoothctl` exposes (adapter power/
//! discoverable/pairable state, scanning, device pairing/connecting/
//! trusting) without committing to how they're carried out — a BlueZ
//! D-Bus client, a raw HCI socket, and a mock for tests can all
//! implement [`BluetoothDriver`]/[`Adapter`]/[`Device`].

pub mod adapter;
pub mod backend;
pub mod device;
pub mod error;
pub mod types;

pub use adapter::Adapter;
pub use backend::{BluetoothDriver, DriverEvent, EventStream};
pub use device::Device;
pub use error::DriverError;
pub use types::{
    AdapterId, Address, AddressKind, AddressParseError, DeviceClass, Rssi, Uuid, UuidParseError,
};

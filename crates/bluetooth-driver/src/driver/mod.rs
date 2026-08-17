//! Backend-agnostic Bluetooth driver-layer traits.
//!
//! These mirror the operations `bluetoothctl` exposes (adapter power/
//! discoverable/pairable state, scanning, device pairing/connecting/
//! trusting) without committing to how they're carried out — a BlueZ
//! D-Bus client, a raw HCI socket, and a mock for tests can all
//! implement [`BluetoothDriver`]/[`Adapter`]/[`Device`].

/// [`Adapter`], the local-controller trait.
pub mod adapter;
/// [`BluetoothDriver`], the entry point trait, plus [`DriverEvent`]/[`EventStream`].
pub mod backend;
/// [`Device`], the remote-device trait.
pub mod device;
/// [`DriverError`], the shared error type every backend maps into.
pub mod error;
/// Plain value types ([`Address`], [`AdapterId`], [`Uuid`], ...) shared
/// by the trait signatures, free of any backend-specific representation.
pub mod types;

pub use adapter::Adapter;
pub use backend::{BluetoothDriver, DriverEvent, EventStream};
pub use device::Device;
pub use error::DriverError;
pub use types::{
    AdapterId, Address, AddressKind, AddressParseError, DeviceClass, DeviceInfo, PnpId, Rssi,
    Uuid, UuidParseError, VendorIdSource,
};

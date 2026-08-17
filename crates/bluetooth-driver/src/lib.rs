#![warn(missing_docs)]
//! Backend-agnostic Bluetooth traits mirroring what `bluetoothctl`
//! exposes ([`driver`]), plus a BlueZ D-Bus implementation of them
//! ([`bluez`], Linux-only) and a handful of standalone lookup tables
//! for making raw Bluetooth/USB identifiers human-readable (vendor
//! names from hardware addresses and manufacturer IDs, GATT UUIDs,
//! GAP appearance values, device class bitfields).
//!
//! Start at [`driver::BluetoothDriver`] - it's the entry point every
//! backend implements, and its docs describe the overall trait shape
//! ([`driver::Adapter`] / [`driver::Device`]) that the rest of this
//! crate, and any other backend (a Web Bluetooth one lives in the
//! sibling `bluetooth-driver-web` crate), builds on.

#[cfg(target_os = "linux")]
pub mod bluez;
pub mod company_id;
pub mod device_class;
pub mod driver;
pub mod gap_appearance;
pub mod gatt_uuid;
pub mod oui;
pub mod usb_vendor;

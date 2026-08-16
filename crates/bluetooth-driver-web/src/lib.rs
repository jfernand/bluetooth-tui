//! A `bluetooth_driver::driver::BluetoothDriver` backend built on the
//! Web Bluetooth API (`navigator.bluetooth`) via `web-sys`, for a wasm32
//! frontend. Only builds for `wasm32-unknown-unknown` - `web-sys`'s
//! Bluetooth bindings don't exist off that target, and there'd be no
//! `navigator` to call into natively regardless.
//!
//! Web Bluetooth's capabilities are a strict subset of what BlueZ
//! exposes: no adapter enumeration or power/discoverable control, no
//! background scanning (only a one-shot, user-gesture-gated device
//! chooser via `WebAdapter::request_new_device`), no visibility into
//! pairing/bonding/trust state, and device identity is an opaque
//! per-origin token rather than a real BD_ADDR (see
//! `bluetooth_driver::driver::Address::Opaque`). Everything without a
//! Web Bluetooth equivalent answers with `DriverError::Unsupported`, a
//! constant, or an empty value rather than pretending to support it -
//! see each type's doc comments for specifics.

mod adapter;
mod device;
mod driver;
mod error;
mod events;

pub use adapter::WebAdapter;
pub use device::WebDevice;
pub use driver::WebBluetoothDriver;
pub use events::WebEvents;

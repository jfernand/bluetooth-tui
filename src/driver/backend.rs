use std::future::Future;

use crate::driver::adapter::Adapter;
use crate::driver::error::DriverError;
use crate::driver::types::{AdapterId, Address};

/// Entry point into the underlying Bluetooth stack. One implementation
/// might talk to BlueZ over D-Bus, another to a raw HCI socket; the TUI
/// only ever depends on this trait plus `Adapter` and `Device`.
pub trait BluetoothDriver {
    type Adapter: Adapter;
    type Events: EventStream;

    fn adapters(&self) -> impl Future<Output = Result<Vec<Self::Adapter>, DriverError>> + Send;

    fn adapter(
        &self,
        id: &AdapterId,
    ) -> impl Future<Output = Result<Option<Self::Adapter>, DriverError>> + Send;

    /// The controller `bluetoothctl` would select by default on startup.
    fn default_adapter(&self) -> impl Future<Output = Result<Option<Self::Adapter>, DriverError>> + Send;

    /// Subscribe to live driver events — the feed behind `bluetoothctl`'s
    /// scan output and property-change notifications.
    fn events(&self) -> impl Future<Output = Result<Self::Events, DriverError>> + Send;
}

/// A pull-based event stream. Kept as a plain async trait rather than
/// `futures::Stream` so this crate carries no async-runtime dependency
/// of its own.
pub trait EventStream {
    fn next(&mut self) -> impl Future<Output = Option<DriverEvent>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    AdapterAdded(AdapterId),
    AdapterRemoved(AdapterId),
    DeviceFound { adapter: AdapterId, address: Address },
    DeviceUpdated { adapter: AdapterId, address: Address },
    DeviceRemoved { adapter: AdapterId, address: Address },
    DiscoveringChanged { adapter: AdapterId, discovering: bool },
    PoweredChanged { adapter: AdapterId, powered: bool },
}

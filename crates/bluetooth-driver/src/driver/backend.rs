use std::fmt;
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

    fn adapters(&self) -> impl Future<Output = Result<Vec<Self::Adapter>, DriverError>>;

    fn adapter(
        &self,
        id: &AdapterId,
    ) -> impl Future<Output = Result<Option<Self::Adapter>, DriverError>>;

    /// Subscribe to live driver events — the feed behind `bluetoothctl`'s
    /// scan output and property-change notifications.
    fn events(&self) -> impl Future<Output = Result<Self::Events, DriverError>>;
}

/// A pull-based event stream. Kept as a plain async trait rather than
/// `futures::Stream` so this crate carries no async-runtime dependency
/// of its own.
pub trait EventStream {
    fn next(&mut self) -> impl Future<Output = Option<DriverEvent>>;
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

impl fmt::Display for DriverEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterAdded(id) => write!(f, "adapter {id} added"),
            Self::AdapterRemoved(id) => write!(f, "adapter {id} removed"),
            Self::DeviceFound { adapter, address } => {
                write!(f, "[{adapter}] device {address} found")
            }
            Self::DeviceUpdated { adapter, address } => {
                write!(f, "[{adapter}] device {address} updated")
            }
            Self::DeviceRemoved { adapter, address } => {
                write!(f, "[{adapter}] device {address} removed")
            }
            Self::DiscoveringChanged {
                adapter,
                discovering,
            } => {
                let state = if *discovering { "started" } else { "stopped" };
                write!(f, "[{adapter}] discovery {state}")
            }
            Self::PoweredChanged { adapter, powered } => {
                let state = if *powered { "powered on" } else { "powered off" };
                write!(f, "[{adapter}] {state}")
            }
        }
    }
}

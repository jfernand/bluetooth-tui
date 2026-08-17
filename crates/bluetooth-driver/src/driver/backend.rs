use std::fmt;
use std::future::Future;

use crate::driver::adapter::Adapter;
use crate::driver::error::DriverError;
use crate::driver::types::{AdapterId, Address};

/// Entry point into the underlying Bluetooth stack. One implementation
/// might talk to BlueZ over D-Bus, another to a raw HCI socket; the TUI
/// only ever depends on this trait plus `Adapter` and `Device`.
pub trait BluetoothDriver {
    /// The local-controller type this driver's `adapters()`/`adapter()` hand out.
    type Adapter: Adapter;
    /// The event stream type returned by `events()`.
    type Events: EventStream;

    /// All local controllers currently present, equivalent to
    /// `bluetoothctl list`.
    fn adapters(&self) -> impl Future<Output = Result<Vec<Self::Adapter>, DriverError>>;

    /// Looks up a single adapter by id, if one by that id is present.
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
    /// Waits for the next event; `None` once the stream is exhausted
    /// (the underlying connection closed).
    fn next(&mut self) -> impl Future<Output = Option<DriverEvent>>;
}

/// A single live change reported through `EventStream` - adapters/devices
/// appearing or disappearing, and the property changes the TUI cares
/// about reflecting immediately rather than on the next poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    /// A local controller was plugged in / became available.
    AdapterAdded(AdapterId),
    /// A local controller was unplugged / became unavailable.
    AdapterRemoved(AdapterId),
    /// A previously-unseen device appeared (typically during a scan).
    DeviceFound {
        /// Which adapter saw it.
        adapter: AdapterId,
        /// The device's address.
        address: Address,
    },
    /// A known device's properties changed (RSSI, name, connection state, ...).
    DeviceUpdated {
        /// Which adapter reported the change.
        adapter: AdapterId,
        /// The device's address.
        address: Address,
    },
    /// A known device was removed/forgotten.
    DeviceRemoved {
        /// Which adapter it was removed from.
        adapter: AdapterId,
        /// The device's address.
        address: Address,
    },
    /// An adapter's discovery (scanning) state changed.
    DiscoveringChanged {
        /// Which adapter.
        adapter: AdapterId,
        /// The new state.
        discovering: bool,
    },
    /// An adapter's radio power state changed.
    PoweredChanged {
        /// Which adapter.
        adapter: AdapterId,
        /// The new state.
        powered: bool,
    },
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

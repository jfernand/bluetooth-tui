use std::future::Future;

use crate::driver::device::Device;
use crate::driver::error::DriverError;
use crate::driver::types::{AdapterId, Address};

/// A local Bluetooth controller (`hci0`, `hci1`, ...) — the driver-layer
/// counterpart of what `bluetoothctl`'s `list` / `select` / `show`
/// commands operate on.
pub trait Adapter {
    /// The remote-device type this adapter's `devices()`/`device()` hand out.
    type Device: Device;

    /// Stable local identifier, e.g. `hci0`.
    fn id(&self) -> &AdapterId;

    /// This controller's own hardware address.
    fn address(&self) -> Address;

    /// The adapter's friendly name (alias, falling back to its system name).
    fn name(&self) -> &str;

    /// Whether the radio is powered on.
    fn is_powered(&self) -> bool;
    /// Powers the radio on or off.
    fn set_powered(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    /// Whether this adapter is currently advertising itself to nearby devices.
    fn is_discoverable(&self) -> bool;
    /// Sets whether this adapter advertises itself to nearby devices.
    fn set_discoverable(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    /// Whether this adapter currently accepts incoming pairing requests.
    fn is_pairable(&self) -> bool;
    /// Sets whether this adapter accepts incoming pairing requests.
    fn set_pairable(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    /// Start an inquiry (classic) / passive-or-active scan (LE).
    /// Discovered and updated devices surface through
    /// `BluetoothDriver::events`, mirroring how `bluetoothctl`'s
    /// `scan on` streams device lines as they arrive rather than
    /// returning them all at once.
    fn start_discovery(&mut self) -> impl Future<Output = Result<(), DriverError>>;

    /// Stops an in-progress `start_discovery()`.
    fn stop_discovery(&mut self) -> impl Future<Output = Result<(), DriverError>>;

    /// Whether a `start_discovery()` scan is currently running.
    fn is_discovering(&self) -> bool;

    /// Devices currently known to this adapter, equivalent to
    /// `bluetoothctl devices`.
    fn devices(&self) -> impl Future<Output = Result<Vec<Self::Device>, DriverError>>;

    /// Devices with an existing bond, equivalent to `bluetoothctl
    /// paired-devices`.
    fn paired_devices(&self) -> impl Future<Output = Result<Vec<Self::Device>, DriverError>>;

    /// Looks up a single known device by address, if this adapter knows
    /// of one at that address.
    fn device(
        &self,
        address: Address,
    ) -> impl Future<Output = Result<Option<Self::Device>, DriverError>>;

    /// Re-read live properties (powered, discoverable, ...) from the driver.
    fn refresh(&mut self) -> impl Future<Output = Result<(), DriverError>>;
}

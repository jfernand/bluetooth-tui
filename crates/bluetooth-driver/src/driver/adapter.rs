use std::future::Future;

use crate::driver::device::Device;
use crate::driver::error::DriverError;
use crate::driver::types::{AdapterId, Address};

/// A local Bluetooth controller (`hci0`, `hci1`, ...) — the driver-layer
/// counterpart of what `bluetoothctl`'s `list` / `select` / `show`
/// commands operate on.
pub trait Adapter {
    type Device: Device;

    fn id(&self) -> &AdapterId;

    fn address(&self) -> Address;

    fn name(&self) -> &str;

    fn is_powered(&self) -> bool;
    fn set_powered(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    fn is_discoverable(&self) -> bool;
    fn set_discoverable(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    fn is_pairable(&self) -> bool;
    fn set_pairable(&mut self, on: bool) -> impl Future<Output = Result<(), DriverError>>;

    /// Start an inquiry (classic) / passive-or-active scan (LE).
    /// Discovered and updated devices surface through
    /// `BluetoothDriver::events`, mirroring how `bluetoothctl`'s
    /// `scan on` streams device lines as they arrive rather than
    /// returning them all at once.
    fn start_discovery(&mut self) -> impl Future<Output = Result<(), DriverError>>;

    fn stop_discovery(&mut self) -> impl Future<Output = Result<(), DriverError>>;

    fn is_discovering(&self) -> bool;

    /// Devices currently known to this adapter, equivalent to
    /// `bluetoothctl devices`.
    fn devices(&self) -> impl Future<Output = Result<Vec<Self::Device>, DriverError>>;

    /// Devices with an existing bond, equivalent to `bluetoothctl
    /// paired-devices`.
    fn paired_devices(&self) -> impl Future<Output = Result<Vec<Self::Device>, DriverError>>;

    fn device(
        &self,
        address: Address,
    ) -> impl Future<Output = Result<Option<Self::Device>, DriverError>>;

    /// Re-read live properties (powered, discoverable, ...) from the driver.
    fn refresh(&mut self) -> impl Future<Output = Result<(), DriverError>>;
}

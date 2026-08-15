use std::future::Future;

use crate::driver::error::DriverError;
use crate::driver::types::{Address, AddressKind, DeviceClass, Rssi, Uuid};

/// A remote Bluetooth device as seen through a specific local adapter —
/// the driver-layer counterpart of what `bluetoothctl` prints per line
/// under `devices` / `info <address>`.
pub trait Device {
    /// Stable hardware address; the handle's identity.
    fn address(&self) -> Address;

    fn address_kind(&self) -> AddressKind;

    /// Name as advertised or read from the device itself.
    fn name(&self) -> Option<&str>;

    /// User- or backend-assigned alias (`bluetoothctl`'s "Alias").
    fn alias(&self) -> Option<&str>;

    fn class(&self) -> Option<DeviceClass>;

    /// Last-seen signal strength; present while the device is in range
    /// during an active scan.
    fn rssi(&self) -> Option<Rssi>;

    fn is_paired(&self) -> bool;
    fn is_bonded(&self) -> bool;
    fn is_connected(&self) -> bool;
    fn is_trusted(&self) -> bool;
    fn is_blocked(&self) -> bool;

    /// Service UUIDs advertised (LE) or discovered via SDP (classic).
    fn service_uuids(&self) -> &[Uuid];

    /// Initiate pairing/bonding. Resolves once bonding succeeds, fails,
    /// or is rejected by whatever pairing agent is registered.
    fn pair(&mut self) -> impl Future<Output = Result<(), DriverError>> + Send;

    /// Open an ACL/LE connection.
    fn connect(&mut self) -> impl Future<Output = Result<(), DriverError>> + Send;

    fn disconnect(&mut self) -> impl Future<Output = Result<(), DriverError>> + Send;

    /// Forget the device: drop bonding keys and any cached state.
    fn remove(&mut self) -> impl Future<Output = Result<(), DriverError>> + Send;

    fn set_trusted(&mut self, trusted: bool) -> impl Future<Output = Result<(), DriverError>> + Send;

    fn set_blocked(&mut self, blocked: bool) -> impl Future<Output = Result<(), DriverError>> + Send;

    /// Re-read live properties (RSSI, connected, ...) from the driver.
    fn refresh(&mut self) -> impl Future<Output = Result<(), DriverError>> + Send;
}

use std::future::Future;

use crate::driver::error::DriverError;
use crate::driver::types::{Address, AddressKind, DeviceClass, PnpId, Rssi, Uuid};

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

    /// Sets the alias. An empty string resets it back to the device's
    /// advertised/read name.
    fn set_alias(&mut self, alias: &str) -> impl Future<Output = Result<(), DriverError>> + Send;

    fn class(&self) -> Option<DeviceClass>;

    /// GAP Appearance value (icon/category hint, e.g. "mouse" or
    /// "keyboard") - see the `gap_appearance` module to resolve it to a
    /// name.
    fn appearance(&self) -> Option<u16>;

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

    /// Bluetooth SIG Company Identifiers seen in the device's advertising
    /// Manufacturer Specific Data. Note this identifies whose *data
    /// format* an advertisement uses, not necessarily who made the
    /// device - many third-party accessories include e.g. a Microsoft
    /// Swift Pair beacon alongside their own identity.
    fn manufacturer_ids(&self) -> &[u16];

    /// Reads the GATT Device Information Service's PnP ID characteristic
    /// (0x2A50) - the actual USB-VID equivalent, since its Vendor ID
    /// Source can point at a real USB-IF-assigned VID rather than just a
    /// Bluetooth SIG company ID. Requires an active, service-resolved
    /// connection; returns `Ok(None)` if the device doesn't expose it.
    fn pnp_id(&self) -> impl Future<Output = Result<Option<PnpId>, DriverError>> + Send;

    /// Reads the GATT Battery Service's Battery Level characteristic
    /// (0x2A19) as a 0-100 percentage. Requires an active,
    /// service-resolved connection; returns `Ok(None)` if the device
    /// doesn't expose it.
    fn battery_percent(&self) -> impl Future<Output = Result<Option<u8>, DriverError>> + Send;

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

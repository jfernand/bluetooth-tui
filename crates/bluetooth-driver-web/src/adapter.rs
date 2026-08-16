use bluetooth_driver::driver::{Adapter, AdapterId, Address, Device, DriverError};
use wasm_bindgen_futures::JsFuture;
use web_sys::RequestDeviceOptions;

use crate::device::WebDevice;
use crate::error::map_js_error;

/// The single synthetic adapter this backend ever hands out - Web
/// Bluetooth has no concept of multiple local controllers, adapter
/// power state, or discoverability; it's just "the browser's Bluetooth
/// access", scoped per-origin.
pub struct WebAdapter {
    id: AdapterId,
    bluetooth: web_sys::Bluetooth,
}

impl WebAdapter {
    pub(crate) fn new(bluetooth: web_sys::Bluetooth) -> Self {
        Self {
            id: AdapterId::new("web"),
            bluetooth,
        }
    }

    /// Opens the browser's native device chooser and returns whatever
    /// device the user picked. This is a required user gesture - call
    /// it only from directly inside a click/key handler, never
    /// speculatively on a timer or at startup, or the browser will
    /// reject it outright.
    ///
    /// Deliberately not part of the `Adapter` trait: Web Bluetooth has
    /// no "start discovery, stream found devices" equivalent, just "ask
    /// the user to pick one device to grant this origin access to" -
    /// a fundamentally different shape of operation, one-shot and
    /// modal rather than continuous.
    pub async fn request_new_device(&self) -> Result<WebDevice, DriverError> {
        let options = RequestDeviceOptions::new();
        options.set_accept_all_devices(true);
        let device = JsFuture::from(self.bluetooth.request_device(&options))
            .await
            .map_err(map_js_error)?;
        Ok(WebDevice::new(device))
    }
}

impl Adapter for WebAdapter {
    type Device = WebDevice;

    fn id(&self) -> &AdapterId {
        &self.id
    }

    /// The browser never exposes the local radio's own address to a
    /// page - there's nothing to put here but a placeholder.
    fn address(&self) -> Address {
        Address::opaque("web")
    }

    fn name(&self) -> &str {
        "Web Bluetooth"
    }

    /// `navigator.bluetooth.getAvailability()` is the closest thing to
    /// a power signal, but it's async and this needs to be a
    /// synchronous property read - it's also "is Bluetooth usable at
    /// all on this machine", not "is it currently on", so reporting
    /// unconditionally powered is the least misleading option: this
    /// adapter can never be turned off from here anyway.
    fn is_powered(&self) -> bool {
        true
    }

    async fn set_powered(&mut self, _on: bool) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth pages can't control radio power"))
    }

    fn is_discoverable(&self) -> bool {
        false
    }

    async fn set_discoverable(&mut self, _on: bool) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(
            "Web Bluetooth pages only ever act as a GATT central, never discoverable",
        ))
    }

    fn is_pairable(&self) -> bool {
        false
    }

    async fn set_pairable(&mut self, _on: bool) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth has no separate pairable state"))
    }

    /// There's no background scan to start: `request_new_device()` is
    /// the one-shot equivalent, gated behind a user gesture.
    async fn start_discovery(&mut self) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(
            "Web Bluetooth has no background scan - use request_new_device() instead",
        ))
    }

    async fn stop_discovery(&mut self) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth has no background scan to stop"))
    }

    fn is_discovering(&self) -> bool {
        false
    }

    /// Devices this origin has previously been granted permission to
    /// access - the closest Web Bluetooth equivalent of
    /// `bluetoothctl devices`, since there's no ambient discovery.
    async fn devices(&self) -> Result<Vec<WebDevice>, DriverError> {
        let devices = JsFuture::from(self.bluetooth.get_devices())
            .await
            .map_err(map_js_error)?;
        let mut devices: Vec<WebDevice> = devices.iter().map(WebDevice::new).collect();
        // AGENTS.md: predictable ordering rather than whatever order
        // getDevices() happened to return.
        devices.sort_by_key(|d| d.address().to_string());
        Ok(devices)
    }

    /// Being in `getDevices()`'s result at all is the closest Web
    /// Bluetooth equivalent of "paired" (see `Device::is_paired`), so
    /// every known device is also a "paired" one here.
    async fn paired_devices(&self) -> Result<Vec<WebDevice>, DriverError> {
        self.devices().await
    }

    async fn device(&self, address: Address) -> Result<Option<WebDevice>, DriverError> {
        let devices = self.devices().await?;
        Ok(devices.into_iter().find(|d| d.address() == address))
    }

    /// Nothing cached needs re-syncing - every property this adapter
    /// reports is either a hardcoded constant or, like `devices()`,
    /// read live from the browser on every call.
    async fn refresh(&mut self) -> Result<(), DriverError> {
        Ok(())
    }
}

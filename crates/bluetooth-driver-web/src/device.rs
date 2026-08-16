use bluetooth_driver::driver::{
    Address, AddressKind, Device, DeviceClass, DeviceInfo, DriverError, PnpId, Rssi, Uuid,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{BluetoothRemoteGattCharacteristic, BluetoothRemoteGattServer, BluetoothRemoteGattService};

use crate::error::map_js_error;

/// GATT Device Information Service and its characteristics of interest.
const DEVICE_INFORMATION_SERVICE_UUID: &str = "0000180a-0000-1000-8000-00805f9b34fb";
const PNP_ID_UUID: &str = "00002a50-0000-1000-8000-00805f9b34fb";
const MANUFACTURER_NAME_UUID: &str = "00002a29-0000-1000-8000-00805f9b34fb";
const MODEL_NUMBER_UUID: &str = "00002a24-0000-1000-8000-00805f9b34fb";
const FIRMWARE_REVISION_UUID: &str = "00002a26-0000-1000-8000-00805f9b34fb";

/// GATT Battery Service and its Battery Level characteristic.
const BATTERY_SERVICE_UUID: &str = "0000180f-0000-1000-8000-00805f9b34fb";
const BATTERY_LEVEL_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

/// A remote device reached through `navigator.bluetooth`.
///
/// Most of `Device`'s surface has no Web Bluetooth equivalent at all -
/// there's no visibility into pairing/bonding/trust state, no class-of-
/// device or GAP appearance, no RSSI or manufacturer/service advertising
/// data outside the experimental, rarely-supported `watchAdvertisements`
/// API. Those come back as `None`/`false`/empty rather than errors: the
/// frontend is expected to simply not offer UI for them against this
/// backend, per the reduced Web Bluetooth screen design, rather than
/// treat every read as a failure.
pub struct WebDevice {
    pub(crate) inner: web_sys::BluetoothDevice,
    address: Address,
    /// `BluetoothDevice::name()` returns an owned `String`, not a
    /// borrow, so it's cached here once at construction rather than on
    /// every `Device::name()` call, which needs to hand back `&str`.
    name: Option<String>,
}

impl WebDevice {
    pub(crate) fn new(inner: web_sys::BluetoothDevice) -> Self {
        let address = Address::opaque(inner.id());
        let name = inner.name();
        Self { inner, address, name }
    }

    fn gatt(&self) -> Result<BluetoothRemoteGattServer, DriverError> {
        self.inner
            .gatt()
            .ok_or(DriverError::Unsupported("this device has no GATT server"))
    }
}

impl Device for WebDevice {
    fn address(&self) -> Address {
        self.address.clone()
    }

    /// Web Bluetooth never exposes whether a device's address is public
    /// or random - it hands out an opaque per-origin token instead of
    /// the real BD_ADDR in the first place.
    fn address_kind(&self) -> AddressKind {
        AddressKind::Public
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    async fn set_alias(&mut self, _alias: &str) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth has no alias concept"))
    }

    fn class(&self) -> Option<DeviceClass> {
        None
    }

    fn appearance(&self) -> Option<u16> {
        None
    }

    fn rssi(&self) -> Option<Rssi> {
        None
    }

    fn tx_power(&self) -> Option<i16> {
        None
    }

    fn icon(&self) -> Option<&str> {
        None
    }

    /// Being in `getDevices()`'s result at all means the user already
    /// granted this origin permission to access the device - the
    /// closest Web Bluetooth equivalent of "paired".
    fn is_paired(&self) -> bool {
        true
    }

    fn is_bonded(&self) -> bool {
        true
    }

    fn is_connected(&self) -> bool {
        self.gatt().map(|g| g.connected()).unwrap_or(false)
    }

    fn is_trusted(&self) -> bool {
        true
    }

    fn is_blocked(&self) -> bool {
        false
    }

    fn is_legacy_pairing(&self) -> bool {
        false
    }

    fn are_services_resolved(&self) -> bool {
        self.is_connected()
    }

    fn is_wake_allowed(&self) -> bool {
        false
    }

    fn service_uuids(&self) -> &[Uuid] {
        &[]
    }

    fn manufacturer_data(&self) -> &[(u16, Vec<u8>)] {
        &[]
    }

    fn service_data(&self) -> &[(Uuid, Vec<u8>)] {
        &[]
    }

    async fn pnp_id(&self) -> Result<Option<PnpId>, DriverError> {
        let bytes = read_characteristic(&self.gatt()?, DEVICE_INFORMATION_SERVICE_UUID, PNP_ID_UUID).await?;
        Ok(bytes.and_then(|b| PnpId::parse(&b)))
    }

    async fn battery_percent(&self) -> Result<Option<u8>, DriverError> {
        let bytes = read_characteristic(&self.gatt()?, BATTERY_SERVICE_UUID, BATTERY_LEVEL_UUID).await?;
        Ok(bytes.and_then(|b| b.first().copied()))
    }

    async fn device_information(&self) -> Result<DeviceInfo, DriverError> {
        let gatt = self.gatt()?;
        let manufacturer = read_text_characteristic(&gatt, MANUFACTURER_NAME_UUID).await?;
        let model = read_text_characteristic(&gatt, MODEL_NUMBER_UUID).await?;
        let firmware = read_text_characteristic(&gatt, FIRMWARE_REVISION_UUID).await?;
        Ok(DeviceInfo {
            manufacturer,
            model,
            firmware,
        })
    }

    /// Web Bluetooth has no separate pairing step exposed to the page -
    /// accessing an encrypted characteristic triggers the OS's own
    /// pairing UI implicitly, outside the page's visibility or control.
    /// `connect()` is the closest equivalent action there is.
    async fn pair(&mut self) -> Result<(), DriverError> {
        self.connect().await
    }

    async fn connect(&mut self) -> Result<(), DriverError> {
        JsFuture::from(self.gatt()?.connect())
            .await
            .map_err(map_js_error)?;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), DriverError> {
        self.gatt()?.disconnect();
        Ok(())
    }

    /// `BluetoothDevice.forget()` (revoking this origin's permission
    /// grant) is newer and far less widely supported than the rest of
    /// this API, and isn't in the `web-sys` version pinned here.
    async fn remove(&mut self) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(
            "revoking this device's permission grant isn't supported yet",
        ))
    }

    async fn set_trusted(&mut self, _trusted: bool) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth has no trust concept"))
    }

    async fn set_blocked(&mut self, _blocked: bool) -> Result<(), DriverError> {
        Err(DriverError::Unsupported("Web Bluetooth has no block concept"))
    }

    /// Nothing cached needs re-fetching: `is_connected()` reads
    /// `gatt().connected` live on every call rather than from a
    /// snapshot, unlike BlueZ's D-Bus properties.
    async fn refresh(&mut self) -> Result<(), DriverError> {
        Ok(())
    }
}

async fn read_text_characteristic(
    gatt: &BluetoothRemoteGattServer,
    characteristic_uuid: &str,
) -> Result<Option<String>, DriverError> {
    let bytes = read_characteristic(gatt, DEVICE_INFORMATION_SERVICE_UUID, characteristic_uuid).await?;
    Ok(bytes.and_then(|b| String::from_utf8(b).ok()))
}

/// Reads a single characteristic's raw bytes, or `None` if the device
/// doesn't expose that service/characteristic at all - Web Bluetooth
/// rejects with a `NotFoundError` `DOMException` in that case, which
/// this treats as absence rather than a hard failure, matching the
/// BlueZ backend's `Ok(None)` for the same situation.
async fn read_characteristic(
    gatt: &BluetoothRemoteGattServer,
    service_uuid: &str,
    characteristic_uuid: &str,
) -> Result<Option<Vec<u8>>, DriverError> {
    // `JsFuture<T>::await` yields `Result<T, JsValue>` typed directly -
    // web-sys's Promise-returning methods are generic over their
    // resolved type, so no `dyn_into` casting is needed on the Ok side.
    let service: BluetoothRemoteGattService =
        match JsFuture::from(gatt.get_primary_service_with_str(service_uuid)).await {
            Ok(service) => service,
            Err(e) if is_not_found(&e) => return Ok(None),
            Err(e) => return Err(map_js_error(e)),
        };

    let characteristic: BluetoothRemoteGattCharacteristic =
        match JsFuture::from(service.get_characteristic_with_str(characteristic_uuid)).await {
            Ok(characteristic) => characteristic,
            Err(e) if is_not_found(&e) => return Ok(None),
            Err(e) => return Err(map_js_error(e)),
        };

    let view: js_sys::DataView = JsFuture::from(characteristic.read_value())
        .await
        .map_err(map_js_error)?;
    Ok(Some(data_view_to_vec(&view)))
}

fn is_not_found(err: &wasm_bindgen::JsValue) -> bool {
    err.dyn_ref::<web_sys::DomException>()
        .is_some_and(|e| e.name() == "NotFoundError")
}

fn data_view_to_vec(view: &js_sys::DataView) -> Vec<u8> {
    (0..view.byte_length()).map(|i| view.get_uint8(i)).collect()
}

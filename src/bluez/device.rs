use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

use crate::driver::{Address, AddressKind, DeviceClass, DriverError, PnpId, Rssi, Uuid};

use super::error::map_zbus_error;
use super::gatt;
use super::path;
use super::properties::{self, PropertyMap};
use super::proxy;

const INTERFACE: &str = "org.bluez.Device1";

/// A remote device, backed by BlueZ's `org.bluez.Device1` object at
/// `/org/bluez/<adapter>/dev_<address>`.
pub struct BluezDevice {
    connection: zbus::Connection,
    proxy: proxy::Device1Proxy<'static>,
    object_path: String,
    adapter_path: String,
    address: Address,
    address_kind: AddressKind,
    name: Option<String>,
    alias: Option<String>,
    class: Option<DeviceClass>,
    appearance: Option<u16>,
    rssi: Option<Rssi>,
    paired: bool,
    bonded: bool,
    connected: bool,
    trusted: bool,
    blocked: bool,
    service_uuids: Vec<Uuid>,
    manufacturer_ids: Vec<u16>,
}

struct Snapshot {
    address_kind: AddressKind,
    name: Option<String>,
    alias: Option<String>,
    class: Option<DeviceClass>,
    appearance: Option<u16>,
    rssi: Option<Rssi>,
    paired: bool,
    bonded: bool,
    connected: bool,
    trusted: bool,
    blocked: bool,
    service_uuids: Vec<Uuid>,
    manufacturer_ids: Vec<u16>,
}

fn snapshot(mut props: PropertyMap) -> Snapshot {
    let paired = properties::take_or_default(&mut props, "Paired");
    Snapshot {
        address_kind: path::parse_address_kind(properties::take(&mut props, "AddressType")),
        name: properties::take(&mut props, "Name"),
        alias: properties::take(&mut props, "Alias"),
        class: properties::take::<u32>(&mut props, "Class").map(DeviceClass),
        appearance: properties::take(&mut props, "Appearance"),
        rssi: properties::take::<i16>(&mut props, "RSSI").map(Rssi),
        paired,
        // BlueZ < 5.65 has no separate `Bonded` property; `Paired` was the
        // bonded state on those versions, so fall back to it.
        bonded: properties::take(&mut props, "Bonded").unwrap_or(paired),
        connected: properties::take_or_default(&mut props, "Connected"),
        trusted: properties::take_or_default(&mut props, "Trusted"),
        blocked: properties::take_or_default(&mut props, "Blocked"),
        service_uuids: properties::take::<Vec<String>>(&mut props, "UUIDs")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| s.parse().ok())
            .collect(),
        manufacturer_ids: {
            let mut ids: Vec<u16> = properties::take::<HashMap<u16, OwnedValue>>(
                &mut props,
                "ManufacturerData",
            )
            .map(|m| m.into_keys().collect())
            .unwrap_or_default();
            ids.sort_unstable();
            ids
        },
    }
}

impl BluezDevice {
    pub(crate) async fn new(
        connection: zbus::Connection,
        adapter_id: &crate::driver::AdapterId,
        address: Address,
    ) -> Result<Self, DriverError> {
        let object_path = path::device_path(adapter_id, address);
        let props = properties::get_all(&connection, &object_path, INTERFACE).await?;
        Self::build(connection, adapter_id, address, props).await
    }

    pub(crate) async fn from_managed(
        connection: zbus::Connection,
        adapter_id: &crate::driver::AdapterId,
        address: Address,
        props: PropertyMap,
    ) -> Result<Self, DriverError> {
        Self::build(connection, adapter_id, address, props).await
    }

    async fn build(
        connection: zbus::Connection,
        adapter_id: &crate::driver::AdapterId,
        address: Address,
        props: PropertyMap,
    ) -> Result<Self, DriverError> {
        let object_path = path::device_path(adapter_id, address);
        let adapter_path = path::adapter_path(adapter_id);
        let proxy = proxy::Device1Proxy::new(&connection, object_path.clone())
            .await
            .map_err(map_zbus_error)?;
        let snap = snapshot(props);
        Ok(Self {
            connection,
            proxy,
            object_path,
            adapter_path,
            address,
            address_kind: snap.address_kind,
            name: snap.name,
            alias: snap.alias,
            class: snap.class,
            appearance: snap.appearance,
            rssi: snap.rssi,
            paired: snap.paired,
            bonded: snap.bonded,
            connected: snap.connected,
            trusted: snap.trusted,
            blocked: snap.blocked,
            service_uuids: snap.service_uuids,
            manufacturer_ids: snap.manufacturer_ids,
        })
    }
}

impl crate::driver::Device for BluezDevice {
    fn address(&self) -> Address {
        self.address
    }

    fn address_kind(&self) -> AddressKind {
        self.address_kind
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    async fn set_alias(&mut self, alias: &str) -> Result<(), DriverError> {
        self.proxy.set_alias(alias).await.map_err(map_zbus_error)?;
        self.refresh().await
    }

    fn class(&self) -> Option<DeviceClass> {
        self.class
    }

    fn appearance(&self) -> Option<u16> {
        self.appearance
    }

    fn rssi(&self) -> Option<Rssi> {
        self.rssi
    }

    fn is_paired(&self) -> bool {
        self.paired
    }

    fn is_bonded(&self) -> bool {
        self.bonded
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn is_trusted(&self) -> bool {
        self.trusted
    }

    fn is_blocked(&self) -> bool {
        self.blocked
    }

    fn service_uuids(&self) -> &[Uuid] {
        &self.service_uuids
    }

    fn manufacturer_ids(&self) -> &[u16] {
        &self.manufacturer_ids
    }

    async fn pnp_id(&self) -> Result<Option<PnpId>, DriverError> {
        gatt::read_pnp_id(&self.connection, &self.object_path).await
    }

    async fn battery_percent(&self) -> Result<Option<u8>, DriverError> {
        gatt::read_battery_level(&self.connection, &self.object_path).await
    }

    async fn pair(&mut self) -> Result<(), DriverError> {
        self.proxy.pair().await.map_err(map_zbus_error)?;
        self.refresh().await
    }

    async fn connect(&mut self) -> Result<(), DriverError> {
        self.proxy.connect().await.map_err(map_zbus_error)?;
        self.refresh().await
    }

    async fn disconnect(&mut self) -> Result<(), DriverError> {
        self.proxy.disconnect().await.map_err(map_zbus_error)?;
        self.refresh().await
    }

    async fn remove(&mut self) -> Result<(), DriverError> {
        let adapter = proxy::Adapter1Proxy::new(&self.connection, self.adapter_path.clone())
            .await
            .map_err(map_zbus_error)?;
        let object_path = zbus::zvariant::ObjectPath::try_from(self.object_path.as_str())
            .map_err(|e| DriverError::Backend(e.to_string()))?;
        adapter
            .remove_device(&object_path)
            .await
            .map_err(map_zbus_error)
    }

    async fn set_trusted(&mut self, trusted: bool) -> Result<(), DriverError> {
        self.proxy
            .set_trusted(trusted)
            .await
            .map_err(map_zbus_error)?;
        self.trusted = trusted;
        Ok(())
    }

    async fn set_blocked(&mut self, blocked: bool) -> Result<(), DriverError> {
        self.proxy
            .set_blocked(blocked)
            .await
            .map_err(map_zbus_error)?;
        self.blocked = blocked;
        Ok(())
    }

    async fn refresh(&mut self) -> Result<(), DriverError> {
        let props = properties::get_all(&self.connection, &self.object_path, INTERFACE).await?;
        let snap = snapshot(props);
        self.address_kind = snap.address_kind;
        self.name = snap.name;
        self.alias = snap.alias;
        self.class = snap.class;
        self.appearance = snap.appearance;
        self.rssi = snap.rssi;
        self.paired = snap.paired;
        self.bonded = snap.bonded;
        self.connected = snap.connected;
        self.trusted = snap.trusted;
        self.blocked = snap.blocked;
        self.service_uuids = snap.service_uuids;
        self.manufacturer_ids = snap.manufacturer_ids;
        Ok(())
    }
}

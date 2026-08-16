use crate::driver::{AdapterId, Address, DriverError};

use super::device::BluezDevice;
use super::error::{map_fdo_error, map_zbus_error};
use super::path;
use super::properties::{self, PropertyMap};
use super::proxy;

const INTERFACE: &str = "org.bluez.Adapter1";

/// A local controller, backed by BlueZ's `org.bluez.Adapter1` object at
/// `/org/bluez/<id>` (e.g. `/org/bluez/hci0`).
pub struct BluezAdapter {
    connection: zbus::Connection,
    proxy: proxy::Adapter1Proxy<'static>,
    object_path: String,
    id: AdapterId,
    address: Address,
    name: String,
    powered: bool,
    discoverable: bool,
    pairable: bool,
    discovering: bool,
}

fn snapshot(mut props: PropertyMap) -> (Address, String, bool, bool, bool, bool) {
    let address = properties::take::<String>(&mut props, "Address")
        .and_then(|s| s.parse().ok())
        .unwrap_or(Address::new([0; 6]));
    let name = properties::take(&mut props, "Alias")
        .or_else(|| properties::take(&mut props, "Name"))
        .unwrap_or_default();
    (
        address,
        name,
        properties::take_or_default(&mut props, "Powered"),
        properties::take_or_default(&mut props, "Discoverable"),
        properties::take_or_default(&mut props, "Pairable"),
        properties::take_or_default(&mut props, "Discovering"),
    )
}

impl BluezAdapter {
    pub(crate) async fn new(connection: zbus::Connection, id: AdapterId) -> Result<Self, DriverError> {
        let object_path = path::adapter_path(&id);
        let props = properties::get_all(&connection, &object_path, INTERFACE).await?;
        let proxy = proxy::Adapter1Proxy::new(&connection, object_path.clone())
            .await
            .map_err(map_zbus_error)?;
        let (address, name, powered, discoverable, pairable, discovering) = snapshot(props);
        Ok(Self {
            connection,
            proxy,
            object_path,
            id,
            address,
            name,
            powered,
            discoverable,
            pairable,
            discovering,
        })
    }
}

async fn list_devices(
    connection: &zbus::Connection,
    adapter_id: &AdapterId,
    only_paired: bool,
) -> Result<Vec<BluezDevice>, DriverError> {
    let object_manager = zbus::fdo::ObjectManagerProxy::new(connection, "org.bluez", "/")
        .await
        .map_err(map_zbus_error)?;
    let objects = object_manager
        .get_managed_objects()
        .await
        .map_err(map_fdo_error)?;

    let mut devices = Vec::new();
    for (object_path, interfaces) in objects {
        let Some((path_adapter, address)) = path::device_from_path(object_path.as_str()) else {
            continue;
        };
        if path_adapter != *adapter_id {
            continue;
        }
        let props = interfaces
            .into_iter()
            .find(|(name, _)| name.as_str() == "org.bluez.Device1")
            .map(|(_, props)| props);
        let Some(props) = props else { continue };

        let device =
            BluezDevice::from_managed(connection.clone(), adapter_id, address, props).await?;
        if !only_paired || crate::driver::Device::is_paired(&device) {
            devices.push(device);
        }
    }
    devices.sort_by_key(crate::driver::Device::address);
    Ok(devices)
}

impl crate::driver::Adapter for BluezAdapter {
    type Device = BluezDevice;

    fn id(&self) -> &AdapterId {
        &self.id
    }

    fn address(&self) -> Address {
        self.address.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_powered(&self) -> bool {
        self.powered
    }

    async fn set_powered(&mut self, on: bool) -> Result<(), DriverError> {
        self.proxy.set_powered(on).await.map_err(map_zbus_error)?;
        self.powered = on;
        Ok(())
    }

    fn is_discoverable(&self) -> bool {
        self.discoverable
    }

    async fn set_discoverable(&mut self, on: bool) -> Result<(), DriverError> {
        self.proxy
            .set_discoverable(on)
            .await
            .map_err(map_zbus_error)?;
        self.discoverable = on;
        Ok(())
    }

    fn is_pairable(&self) -> bool {
        self.pairable
    }

    async fn set_pairable(&mut self, on: bool) -> Result<(), DriverError> {
        self.proxy
            .set_pairable(on)
            .await
            .map_err(map_zbus_error)?;
        self.pairable = on;
        Ok(())
    }

    async fn start_discovery(&mut self) -> Result<(), DriverError> {
        self.proxy.start_discovery().await.map_err(map_zbus_error)?;
        self.discovering = true;
        Ok(())
    }

    async fn stop_discovery(&mut self) -> Result<(), DriverError> {
        self.proxy.stop_discovery().await.map_err(map_zbus_error)?;
        self.discovering = false;
        Ok(())
    }

    fn is_discovering(&self) -> bool {
        self.discovering
    }

    async fn devices(&self) -> Result<Vec<BluezDevice>, DriverError> {
        list_devices(&self.connection, &self.id, false).await
    }

    async fn paired_devices(&self) -> Result<Vec<BluezDevice>, DriverError> {
        list_devices(&self.connection, &self.id, true).await
    }

    async fn device(&self, address: Address) -> Result<Option<BluezDevice>, DriverError> {
        match BluezDevice::new(self.connection.clone(), &self.id, address).await {
            Ok(device) => Ok(Some(device)),
            Err(DriverError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn refresh(&mut self) -> Result<(), DriverError> {
        let props = properties::get_all(&self.connection, &self.object_path, INTERFACE).await?;
        let (address, name, powered, discoverable, pairable, discovering) = snapshot(props);
        self.address = address;
        self.name = name;
        self.powered = powered;
        self.discoverable = discoverable;
        self.pairable = pairable;
        self.discovering = discovering;
        Ok(())
    }
}

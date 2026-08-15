//! Minimal GATT client support: just enough to find and read a single
//! well-known characteristic by UUID under a device's object subtree.

use std::collections::HashMap;

use crate::driver::{DriverError, PnpId};

use super::error::{map_fdo_error, map_zbus_error};
use super::properties;
use super::proxy;

/// GATT Device Information Service's PnP ID characteristic UUID.
const PNP_ID_UUID: &str = "00002a50-0000-1000-8000-00805f9b34fb";

/// GATT Battery Service's Battery Level characteristic UUID.
const BATTERY_LEVEL_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

pub(crate) async fn read_pnp_id(
    connection: &zbus::Connection,
    device_path: &str,
) -> Result<Option<PnpId>, DriverError> {
    let value = read_characteristic(connection, device_path, PNP_ID_UUID).await?;
    Ok(value.and_then(|bytes| PnpId::parse(&bytes)))
}

/// Battery Level is a single unsigned octet: 0-100, a percentage.
pub(crate) async fn read_battery_level(
    connection: &zbus::Connection,
    device_path: &str,
) -> Result<Option<u8>, DriverError> {
    let value = read_characteristic(connection, device_path, BATTERY_LEVEL_UUID).await?;
    Ok(value.and_then(|bytes| bytes.first().copied()))
}

async fn read_characteristic(
    connection: &zbus::Connection,
    device_path: &str,
    uuid: &str,
) -> Result<Option<Vec<u8>>, DriverError> {
    let Some(characteristic_path) = find_characteristic(connection, device_path, uuid).await?
    else {
        return Ok(None);
    };

    let characteristic = proxy::GattCharacteristic1Proxy::new(connection, characteristic_path)
        .await
        .map_err(map_zbus_error)?;
    let value = characteristic
        .read_value(HashMap::new())
        .await
        .map_err(map_zbus_error)?;
    Ok(Some(value))
}

async fn find_characteristic(
    connection: &zbus::Connection,
    device_path: &str,
    uuid: &str,
) -> Result<Option<String>, DriverError> {
    let object_manager = zbus::fdo::ObjectManagerProxy::new(connection, "org.bluez", "/")
        .await
        .map_err(map_zbus_error)?;
    let objects = object_manager
        .get_managed_objects()
        .await
        .map_err(map_fdo_error)?;

    let prefix = format!("{device_path}/");
    for (object_path, interfaces) in objects {
        let path = object_path.as_str();
        if !path.starts_with(&prefix) {
            continue;
        }
        let Some(mut props) = interfaces
            .into_iter()
            .find(|(name, _)| name.as_str() == "org.bluez.GattCharacteristic1")
            .map(|(_, props)| props)
        else {
            continue;
        };
        let Some(char_uuid) = properties::take::<String>(&mut props, "UUID") else {
            continue;
        };
        if char_uuid.eq_ignore_ascii_case(uuid) {
            return Ok(Some(path.to_owned()));
        }
    }
    Ok(None)
}

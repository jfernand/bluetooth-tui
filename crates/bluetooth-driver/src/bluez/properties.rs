//! Helpers for reading `org.freedesktop.DBus.Properties.GetAll` results
//! (a `HashMap<String, OwnedValue>`) into our own typed snapshot fields.

use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

use crate::driver::DriverError;

use super::error::{map_fdo_error, map_zbus_error};

pub(crate) type PropertyMap = HashMap<String, OwnedValue>;

pub(crate) async fn get_all(
    connection: &zbus::Connection,
    path: &str,
    interface: &str,
) -> Result<PropertyMap, DriverError> {
    let props = zbus::fdo::PropertiesProxy::new(connection, "org.bluez", path.to_owned())
        .await
        .map_err(map_zbus_error)?;
    let interface = zbus::names::InterfaceName::try_from(interface)
        .map_err(|e| DriverError::Backend(e.to_string()))?;
    props.get_all(interface).await.map_err(map_fdo_error)
}

/// Take and convert a property, dropping it whether or not the conversion
/// succeeds — BlueZ omits some properties entirely rather than sending a
/// null, so a missing key is a normal "unknown" rather than an error.
pub(crate) fn take<T>(props: &mut PropertyMap, key: &str) -> Option<T>
where
    T: TryFrom<OwnedValue>,
{
    props.remove(key).and_then(|v| T::try_from(v).ok())
}

pub(crate) fn take_or_default<T>(props: &mut PropertyMap, key: &str) -> T
where
    T: Default + TryFrom<OwnedValue>,
{
    take(props, key).unwrap_or_default()
}

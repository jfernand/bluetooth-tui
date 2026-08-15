use crate::driver::{AdapterId, DriverError};

use super::adapter::BluezAdapter;
use super::error::{map_fdo_error, map_zbus_error};
use super::events::BluezEvents;
use super::path;

/// Entry point into BlueZ over the system D-Bus bus.
pub struct BluezDriver {
    connection: zbus::Connection,
}

impl BluezDriver {
    /// Connect to `bluetoothd` over the system bus, the same one
    /// `bluetoothctl` talks to.
    pub async fn system() -> Result<Self, DriverError> {
        let connection = zbus::Connection::system().await.map_err(map_zbus_error)?;
        Ok(Self { connection })
    }
}

impl crate::driver::BluetoothDriver for BluezDriver {
    type Adapter = BluezAdapter;
    type Events = BluezEvents;

    async fn adapters(&self) -> Result<Vec<BluezAdapter>, DriverError> {
        let object_manager =
            zbus::fdo::ObjectManagerProxy::new(&self.connection, "org.bluez", "/")
                .await
                .map_err(map_zbus_error)?;
        let objects = object_manager
            .get_managed_objects()
            .await
            .map_err(map_fdo_error)?;

        let mut ids: Vec<AdapterId> = objects
            .into_iter()
            .filter_map(|(object_path, interfaces)| {
                let id = path::adapter_id_from_path(object_path.as_str())?;
                interfaces
                    .keys()
                    .any(|name| name.as_str() == "org.bluez.Adapter1")
                    .then_some(id)
            })
            .collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        let mut adapters = Vec::with_capacity(ids.len());
        for id in ids {
            adapters.push(BluezAdapter::new(self.connection.clone(), id).await?);
        }
        Ok(adapters)
    }

    async fn adapter(&self, id: &AdapterId) -> Result<Option<BluezAdapter>, DriverError> {
        match BluezAdapter::new(self.connection.clone(), id.clone()).await {
            Ok(adapter) => Ok(Some(adapter)),
            Err(DriverError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// BlueZ has no explicit "default adapter" D-Bus property; like
    /// `bluetoothctl`, we just take the first controller found.
    async fn default_adapter(&self) -> Result<Option<BluezAdapter>, DriverError> {
        Ok(self.adapters().await?.into_iter().next())
    }

    async fn events(&self) -> Result<BluezEvents, DriverError> {
        BluezEvents::new(self.connection.clone()).await
    }
}

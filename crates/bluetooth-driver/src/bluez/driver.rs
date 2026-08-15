use std::time::Duration;

use crate::driver::{AdapterId, DriverError};

use super::adapter::BluezAdapter;
use super::error::{map_fdo_error, map_zbus_error};
use super::events::BluezEvents;
use super::path;

/// Bound every D-Bus method call to this long rather than none at all.
/// `zbus::Connection::system()` applies no timeout by default, so a
/// bluetoothd restart (or any other moment it stops answering) would
/// otherwise hang whatever call is in flight forever - and since this
/// app's event loop runs one call to completion before it can process
/// its next key press or redraw, that's a fully frozen UI, not just a
/// slow one. 30s errs on the generous side deliberately: it's long
/// enough that a real Pair() needing interactive confirmation elsewhere
/// on the system won't get cut off, while still turning "hangs forever"
/// into "recovers within half a minute at worst".
const METHOD_TIMEOUT: Duration = Duration::from_secs(30);

/// Entry point into BlueZ over the system D-Bus bus.
pub struct BluezDriver {
    connection: zbus::Connection,
}

impl BluezDriver {
    /// Connect to `bluetoothd` over the system bus, the same one
    /// `bluetoothctl` talks to.
    pub async fn system() -> Result<Self, DriverError> {
        let connection = zbus::connection::Builder::system()
            .map_err(map_zbus_error)?
            .method_timeout(METHOD_TIMEOUT)
            .build()
            .await
            .map_err(map_zbus_error)?;
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
        // Sort by the trailing number ("hci2" before "hci10"), not the raw
        // string ("hci10" < "hci2" lexicographically).
        ids.sort_by_key(|id| (numeric_suffix(id), id.as_str().to_owned()));

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

    async fn events(&self) -> Result<BluezEvents, DriverError> {
        BluezEvents::new(self.connection.clone()).await
    }
}

/// The number after BlueZ's "hci" prefix, e.g. `hci10` -> `Some(10)`.
fn numeric_suffix(id: &AdapterId) -> Option<u32> {
    let s = id.as_str();
    let digits_start = s.len() - s.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    s[digits_start..].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_adapter_ids_numerically_not_lexicographically() {
        let mut ids = [
            AdapterId::new("hci10"),
            AdapterId::new("hci2"),
            AdapterId::new("hci0"),
            AdapterId::new("hci1"),
        ];
        ids.sort_by_key(|id| (numeric_suffix(id), id.as_str().to_owned()));

        let sorted: Vec<&str> = ids.iter().map(AdapterId::as_str).collect();
        assert_eq!(sorted, ["hci0", "hci1", "hci2", "hci10"]);
    }
}

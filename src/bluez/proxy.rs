//! Typed D-Bus proxies for the `org.bluez` method calls and writable
//! properties we need. Read-only property snapshots go through
//! `properties::get_all` instead, since BlueZ omits absent properties
//! rather than nulling them and `Properties.GetAll` makes that easy to
//! handle uniformly.

use std::collections::HashMap;

use zbus::zvariant::{ObjectPath, Value};

#[zbus::proxy(interface = "org.bluez.Adapter1", default_service = "org.bluez")]
pub(crate) trait Adapter1 {
    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
    fn remove_device(&self, device: &ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn set_discoverable(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn set_pairable(&self, value: bool) -> zbus::Result<()>;
}

#[zbus::proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
pub(crate) trait Device1 {
    fn pair(&self) -> zbus::Result<()>;
    fn cancel_pairing(&self) -> zbus::Result<()>;
    fn connect(&self) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_trusted(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn set_blocked(&self, value: bool) -> zbus::Result<()>;
}

#[zbus::proxy(interface = "org.bluez.GattCharacteristic1", default_service = "org.bluez")]
pub(crate) trait GattCharacteristic1 {
    fn read_value(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<Vec<u8>>;
}

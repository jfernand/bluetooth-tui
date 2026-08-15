use std::collections::HashMap;

use futures_util::StreamExt;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, MatchRule, Message, MessageStream};

use crate::driver::backend::{DriverEvent, EventStream};
use crate::driver::DriverError;

use super::error::map_zbus_error;
use super::path;
use super::properties::{self, PropertyMap};

/// Live BlueZ notifications, backed by three raw D-Bus signal
/// subscriptions rather than per-object property streams: devices come
/// and go constantly during a scan, and subscribing to each one
/// individually would mean tracking a stream per discovered device.
pub struct BluezEvents {
    interfaces_added: MessageStream,
    interfaces_removed: MessageStream,
    properties_changed: MessageStream,
}

impl BluezEvents {
    pub(crate) async fn new(connection: Connection) -> Result<Self, DriverError> {
        Ok(Self {
            interfaces_added: object_manager_stream(&connection, "InterfacesAdded").await?,
            interfaces_removed: object_manager_stream(&connection, "InterfacesRemoved").await?,
            properties_changed: properties_changed_stream(&connection).await?,
        })
    }
}

async fn object_manager_stream(
    connection: &Connection,
    member: &str,
) -> Result<MessageStream, DriverError> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.ObjectManager")
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .member(member)
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .path("/")
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .build();
    MessageStream::for_match_rule(rule, connection, None)
        .await
        .map_err(map_zbus_error)
}

async fn properties_changed_stream(connection: &Connection) -> Result<MessageStream, DriverError> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .member("PropertiesChanged")
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .path_namespace("/org/bluez")
        .map_err(|e| DriverError::Backend(e.to_string()))?
        .build();
    MessageStream::for_match_rule(rule, connection, None)
        .await
        .map_err(map_zbus_error)
}

impl EventStream for BluezEvents {
    async fn next(&mut self) -> Option<DriverEvent> {
        loop {
            let event = tokio::select! {
                Some(Ok(msg)) = self.interfaces_added.next() => interfaces_added_event(&msg),
                Some(Ok(msg)) = self.interfaces_removed.next() => interfaces_removed_event(&msg),
                Some(Ok(msg)) = self.properties_changed.next() => properties_changed_event(&msg),
                else => return None,
            };
            if event.is_some() {
                return event;
            }
        }
    }
}

fn interfaces_added_event(msg: &Message) -> Option<DriverEvent> {
    let (object_path, interfaces): (OwnedObjectPath, HashMap<String, PropertyMap>) =
        msg.body().deserialize().ok()?;
    let path_str = object_path.as_str();
    if interfaces.contains_key("org.bluez.Adapter1") {
        return path::adapter_id_from_path(path_str).map(DriverEvent::AdapterAdded);
    }
    if interfaces.contains_key("org.bluez.Device1") {
        let (adapter, address) = path::device_from_path(path_str)?;
        return Some(DriverEvent::DeviceFound { adapter, address });
    }
    None
}

fn interfaces_removed_event(msg: &Message) -> Option<DriverEvent> {
    let (object_path, interfaces): (OwnedObjectPath, Vec<String>) =
        msg.body().deserialize().ok()?;
    let path_str = object_path.as_str();
    if interfaces.iter().any(|i| i == "org.bluez.Adapter1") {
        return path::adapter_id_from_path(path_str).map(DriverEvent::AdapterRemoved);
    }
    if interfaces.iter().any(|i| i == "org.bluez.Device1") {
        let (adapter, address) = path::device_from_path(path_str)?;
        return Some(DriverEvent::DeviceRemoved { adapter, address });
    }
    None
}

fn properties_changed_event(msg: &Message) -> Option<DriverEvent> {
    let path = msg.header().path()?.to_string();
    let (interface, mut changed, _invalidated): (String, PropertyMap, Vec<String>) =
        msg.body().deserialize().ok()?;

    match interface.as_str() {
        "org.bluez.Adapter1" => {
            let id = path::adapter_id_from_path(&path)?;
            if let Some(powered) = properties::take::<bool>(&mut changed, "Powered") {
                return Some(DriverEvent::PoweredChanged { adapter: id, powered });
            }
            if let Some(discovering) = properties::take::<bool>(&mut changed, "Discovering") {
                return Some(DriverEvent::DiscoveringChanged {
                    adapter: id,
                    discovering,
                });
            }
            None
        }
        "org.bluez.Device1" => {
            let (adapter, address) = path::device_from_path(&path)?;
            Some(DriverEvent::DeviceUpdated { adapter, address })
        }
        _ => None,
    }
}

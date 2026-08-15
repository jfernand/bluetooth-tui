//! Conversions between BlueZ D-Bus object paths and our backend-agnostic
//! `AdapterId`/`Address`. BlueZ places adapters at `/org/bluez/hciN` and
//! devices at `/org/bluez/hciN/dev_AA_BB_CC_DD_EE_FF`.

use crate::driver::{Address, AdapterId};

pub(crate) fn adapter_path(id: &AdapterId) -> String {
    format!("/org/bluez/{}", id.as_str())
}

pub(crate) fn device_path(adapter: &AdapterId, address: Address) -> String {
    format!(
        "/org/bluez/{}/dev_{}",
        adapter.as_str(),
        address.to_string().replace(':', "_")
    )
}

pub(crate) fn adapter_id_from_path(path: &str) -> Option<AdapterId> {
    let rest = path.strip_prefix("/org/bluez/")?;
    (!rest.is_empty() && !rest.contains('/')).then(|| AdapterId::new(rest))
}

pub(crate) fn device_from_path(path: &str) -> Option<(AdapterId, Address)> {
    let rest = path.strip_prefix("/org/bluez/")?;
    let (adapter, dev) = rest.split_once('/')?;
    let hex = dev.strip_prefix("dev_")?;
    if hex.is_empty() || hex.contains('/') {
        // A GATT service/characteristic/descriptor path nested under a
        // device, not the device object itself.
        return None;
    }
    let address: Address = hex.replace('_', ":").parse().ok()?;
    Some((AdapterId::new(adapter), address))
}

pub(crate) fn parse_address_kind(value: Option<String>) -> crate::driver::AddressKind {
    match value.as_deref() {
        Some("random") => crate::driver::AddressKind::Random,
        _ => crate::driver::AddressKind::Public,
    }
}

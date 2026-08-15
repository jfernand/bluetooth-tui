//! GATT service and characteristic UUID name lookup.
//!
//! Covers both Bluetooth SIG 16-bit assigned numbers (expanded to their
//! full 128-bit Bluetooth Base UUID form, e.g. `0x180F` ->
//! `0000180f-0000-1000-8000-00805f9b34fb` = "Battery Service") and
//! vendor-specific full 128-bit UUIDs from well-known ecosystems (Apple,
//! Philips Hue, micro:bit, ...) that Nordic's database also tracks - so
//! this resolves more than just SIG-assigned numbers.
//!
//! Tables: Nordic Semiconductor's mirror of the SIG's assigned numbers
//! (<https://github.com/NordicSemiconductor/bluetooth-numbers-database>),
//! embedded at compile time.

use std::sync::OnceLock;

use crate::driver::Uuid;

const SERVICE_DATA: &str = include_str!("../data/gatt_service_data.tsv");
const CHARACTERISTIC_DATA: &str = include_str!("../data/gatt_characteristic_data.tsv");

fn parse(data: &'static str) -> Vec<(u128, &'static str)> {
    let mut entries: Vec<(u128, &'static str)> = data
        .lines()
        .filter_map(|line| {
            let (uuid, name) = line.split_once('\t')?;
            let uuid = u128::from_str_radix(uuid, 16).ok()?;
            Some((uuid, name))
        })
        .collect();
    entries.sort_unstable_by_key(|&(uuid, _)| uuid);
    entries
}

fn lookup(table: &[(u128, &'static str)], uuid: Uuid) -> Option<&'static str> {
    table
        .binary_search_by_key(&uuid.0, |&(key, _)| key)
        .ok()
        .map(|i| table[i].1)
}

/// The GATT service this UUID identifies, e.g. "Battery Service".
pub fn service_name(uuid: Uuid) -> Option<&'static str> {
    static TABLE: OnceLock<Vec<(u128, &'static str)>> = OnceLock::new();
    lookup(TABLE.get_or_init(|| parse(SERVICE_DATA)), uuid)
}

/// The GATT characteristic this UUID identifies, e.g. "Battery Level".
pub fn characteristic_name(uuid: Uuid) -> Option<&'static str> {
    static TABLE: OnceLock<Vec<(u128, &'static str)>> = OnceLock::new();
    lookup(TABLE.get_or_init(|| parse(CHARACTERISTIC_DATA)), uuid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn resolves_a_sig_short_uuid() {
        let battery_service = Uuid::from_str("0000180f-0000-1000-8000-00805f9b34fb").unwrap();
        assert_eq!(service_name(battery_service), Some("Battery Service"));

        let pnp_id = Uuid::from_str("00002a50-0000-1000-8000-00805f9b34fb").unwrap();
        assert_eq!(characteristic_name(pnp_id), Some("PnP ID"));
    }

    #[test]
    fn resolves_a_vendor_specific_full_uuid() {
        let apple_ancs = Uuid::from_str("7905f431-b5ce-4e99-a40f-4b1e122d00d0").unwrap();
        assert_eq!(
            service_name(apple_ancs),
            Some("Apple Notification Center Service")
        );
    }

    #[test]
    fn returns_none_for_an_unrecognized_uuid() {
        let random = Uuid::from_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(service_name(random), None);
        assert_eq!(characteristic_name(random), None);
    }
}

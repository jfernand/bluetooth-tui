//! USB-IF Vendor ID lookup.
//!
//! The GATT PnP ID characteristic's `Vendor ID Source` can point at a
//! genuine USB-IF-assigned VID rather than a Bluetooth SIG company ID
//! (many BLE HID peripherals reuse their USB VID here even though they
//! never plug into a USB port). This resolves that VID to the
//! registered vendor name.
//!
//! Table: the canonical USB ID registry (<http://www.linux-usb.org/usb.ids>),
//! trimmed to vendor-level entries and embedded at compile time.

use std::sync::OnceLock;

const DATA: &str = include_str!("usb_vendor_data.tsv");

fn table() -> &'static [(u16, &'static str)] {
    static TABLE: OnceLock<Vec<(u16, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut entries: Vec<(u16, &'static str)> = DATA
            .lines()
            .filter_map(|line| {
                let (id, name) = line.split_once('\t')?;
                let id = u16::from_str_radix(id, 16).ok()?;
                Some((id, name))
            })
            .collect();
        entries.sort_unstable_by_key(|&(id, _)| id);
        entries
    })
}

/// The organization USB-IF registered this vendor ID to.
pub fn vendor(id: u16) -> Option<&'static str> {
    let table = table();
    table
        .binary_search_by_key(&id, |&(key, _)| key)
        .ok()
        .map(|i| table[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_a_known_vendor() {
        // The VID this machine's Logitech peripherals report via their
        // GATT PnP ID characteristic.
        assert_eq!(vendor(0x046D), Some("Logitech, Inc."));
    }

    #[test]
    fn returns_none_for_an_unassigned_id() {
        assert_eq!(vendor(0xFFFF), None, "test assumes 0xFFFF is unassigned");
    }
}

//! Bluetooth SIG Company Identifier lookup - the closest Bluetooth
//! equivalent to a USB Vendor ID. It's a 16-bit number the SIG assigns to
//! each member company, and it shows up in two places: advertising
//! Manufacturer Specific Data (any company may format their own data this
//! way, so seeing e.g. Microsoft's ID just means "this AD structure
//! follows Microsoft's format", not "Microsoft made this device" - many
//! third-party accessories include a Microsoft Swift Pair beacon), and as
//! one of the two `Vendor ID Source`s in the GATT PnP ID characteristic
//! (the other being an actual USB-IF-assigned VID) - the latter is the
//! genuine USB-VID equivalent.
//!
//! Table: Nordic Semiconductor's actively maintained mirror of the SIG's
//! assigned numbers
//! (<https://github.com/NordicSemiconductor/bluetooth-numbers-database>),
//! embedded at compile time.

use std::sync::OnceLock;

const DATA: &str = include_str!("../data/company_id_data.tsv");

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

/// The company the Bluetooth SIG assigned this identifier to.
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
    fn looks_up_known_companies() {
        assert_eq!(vendor(0x0006), Some("Microsoft"));
        assert_eq!(vendor(0x01DA), Some("Logitech International SA"));
    }

    #[test]
    fn returns_none_for_an_unassigned_id() {
        assert_eq!(vendor(0xFFFE), None, "test assumes 0xFFFE is unassigned");
    }
}

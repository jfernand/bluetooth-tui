//! IEEE OUI (Organizationally Unique Identifier) vendor lookup.
//!
//! Maps a hardware address's first three octets to the organization IEEE
//! registered them to, e.g. `AA:BB:CC:11:22:33` -> `Some Vendor:11:22:33`.
//! The table is the IEEE MA-L registry
//! (<https://standards-oui.ieee.org/oui/oui.csv>), trimmed to
//! assignment/organization and embedded at compile time - no network
//! access needed at runtime.

use std::sync::OnceLock;

use crate::driver::Address;

const DATA: &str = include_str!("../data/oui_data.tsv");

fn table() -> &'static [(u32, &'static str)] {
    static TABLE: OnceLock<Vec<(u32, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut entries: Vec<(u32, &'static str)> = DATA
            .lines()
            .filter_map(|line| {
                let (oui, vendor) = line.split_once('\t')?;
                let oui = u32::from_str_radix(oui, 16).ok()?;
                Some((oui, vendor))
            })
            .collect();
        entries.sort_unstable_by_key(|&(oui, _)| oui);
        entries
    })
}

fn oui_of(address: &Address) -> Option<u32> {
    let [a, b, c, ..] = address.octets()?;
    Some(u32::from_be_bytes([0, a, b, c]))
}

/// Whether the address's "locally administered" bit (the second-least-
/// significant bit of the first octet) is set - i.e. it was assigned by
/// software rather than burned into hardware by a vendor, so no IEEE OUI
/// lookup can ever succeed for it. Common for BLE random addresses,
/// which is why so many discovered-but-unpaired devices show up with no
/// vendor. Always `false` for a backend-opaque [`Address::Opaque`] -
/// there's no OUI concept to speak of, random or otherwise.
pub fn is_locally_administered(address: &Address) -> bool {
    address.octets().is_some_and(|o| o[0] & 0x02 != 0)
}

/// The organization IEEE registered this address's OUI to, if any.
/// Always `None` for a backend-opaque [`Address::Opaque`].
pub fn vendor(address: &Address) -> Option<&'static str> {
    let table = table();
    let oui = oui_of(address)?;
    table
        .binary_search_by_key(&oui, |&(key, _)| key)
        .ok()
        .map(|i| table[i].1)
}

/// Formats an address as `VENDOR:xx:xx:xx` (last three octets) when the
/// vendor is known, `(random):xx:xx:xx` for locally administered
/// addresses, or the plain address otherwise (including the opaque
/// backend-assigned string for an [`Address::Opaque`], which has no
/// octets to format as a suffix in the first place).
pub fn humanize(address: &Address) -> String {
    let Some([.., d, e, f]) = address.octets() else {
        return address.to_string();
    };
    let suffix = format!("{d:02X}:{e:02X}:{f:02X}");
    match vendor(address) {
        Some(vendor) => format!("{vendor}:{suffix}"),
        None if is_locally_administered(address) => format!("(random):{suffix}"),
        None => address.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_a_known_vendor() {
        // Logitech's registered OUI for the "LIFT" mouse family seen on
        // this machine's paired-device list.
        let address: Address = "38:F0:C8:11:22:33".parse().unwrap();
        assert_eq!(vendor(&address), Some("Logitech"));
        assert_eq!(humanize(&address), "Logitech:11:22:33");
    }

    #[test]
    fn flags_locally_administered_addresses_as_random() {
        // Bit 0x02 of the first octet set -> no OUI will ever match.
        let address: Address = "D2:45:FF:1D:B9:82".parse().unwrap();
        assert!(is_locally_administered(&address));
        assert_eq!(vendor(&address), None);
        assert_eq!(humanize(&address), "(random):1D:B9:82");
    }

    #[test]
    fn falls_back_to_the_plain_address_when_unknown_and_not_random() {
        // F4:00:00 has neither IEEE bit set and isn't in the table.
        let address: Address = "F4:00:00:00:00:00".parse().unwrap();
        assert!(!is_locally_administered(&address));
        assert_eq!(vendor(&address), None, "test assumes F4:00:00 is unassigned");
        assert_eq!(humanize(&address), address.to_string());
    }

    #[test]
    fn opaque_addresses_have_no_oui_and_format_as_themselves() {
        let address = Address::opaque("web-token-abc123");
        assert!(!is_locally_administered(&address));
        assert_eq!(vendor(&address), None);
        assert_eq!(humanize(&address), "web-token-abc123");
    }
}

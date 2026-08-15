//! GAP Appearance value lookup.
//!
//! The GAP `Appearance` characteristic/property is a 16-bit value: a
//! 10-bit category (`value >> 6`) plus a 6-bit subcategory
//! (`value & 0x3F`), e.g. `0x03C2` = Human Interface Device / Mouse.
//! Flattened here into a single category+subcategory -> name table, so a
//! lookup is a plain binary search like the other embedded tables.
//!
//! Table: Nordic Semiconductor's mirror of the SIG's assigned numbers
//! (<https://github.com/NordicSemiconductor/bluetooth-numbers-database>),
//! embedded at compile time.

use std::sync::OnceLock;

const DATA: &str = include_str!("gap_appearance_data.tsv");

fn table() -> &'static [(u16, &'static str)] {
    static TABLE: OnceLock<Vec<(u16, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut entries: Vec<(u16, &'static str)> = DATA
            .lines()
            .filter_map(|line| {
                let (value, name) = line.split_once('\t')?;
                let value = u16::from_str_radix(value, 16).ok()?;
                Some((value, name))
            })
            .collect();
        entries.sort_unstable_by_key(|&(value, _)| value);
        entries
    })
}

/// The category (and, where assigned, subcategory) name for a GAP
/// Appearance value, e.g. `"Human Interface Device: Mouse"`.
pub fn name(value: u16) -> Option<&'static str> {
    let table = table();
    table
        .binary_search_by_key(&value, |&(key, _)| key)
        .ok()
        .map(|i| table[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_category_with_subcategory() {
        // The value this machine's actual LIFT mouse reports.
        assert_eq!(name(0x03C2), Some("Human Interface Device: Mouse"));
    }

    #[test]
    fn resolves_a_bare_category_with_no_subcategory() {
        assert_eq!(name(0x0040), Some("Phone"));
    }
}

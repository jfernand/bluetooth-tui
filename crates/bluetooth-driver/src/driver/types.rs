//! Value types shared by the driver-layer traits, kept free of any
//! backend-specific representation (D-Bus variants, HCI wire formats, ...).

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// A Bluetooth device's identity, as seen through a specific backend.
///
/// BlueZ/HCI backends deal in a real 48-bit BD_ADDR. Not every backend
/// can promise that, though: the Web Bluetooth API deliberately hands a
/// page an opaque, per-origin token instead of the hardware address
/// (`BluetoothDevice.id`), specifically so a page can't use it to track
/// a device across sites. `Opaque` carries that kind of identifier -
/// stable enough to key a `HashMap` or compare for equality, but with
/// no OUI/vendor-prefix structure to read out of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Address {
    Mac([u8; 6]),
    Opaque(String),
}

impl Address {
    pub const fn new(octets: [u8; 6]) -> Self {
        Self::Mac(octets)
    }

    pub fn opaque(id: impl Into<String>) -> Self {
        Self::Opaque(id.into())
    }

    /// The raw 48-bit address, if this identifies a real BD_ADDR rather
    /// than an opaque backend-assigned token.
    pub const fn octets(&self) -> Option<[u8; 6]> {
        match self {
            Self::Mac(octets) => Some(*octets),
            Self::Opaque(_) => None,
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mac([a, b, c, d, e, g]) => {
                write!(f, "{a:02X}:{b:02X}:{c:02X}:{d:02X}:{e:02X}:{g:02X}")
            }
            Self::Opaque(id) => f.write_str(id),
        }
    }
}

/// Parses a colon-separated hex BD_ADDR (`AA:BB:CC:DD:EE:FF`) into
/// [`Address::Mac`]. There's no textual form to parse an [`Address::Opaque`]
/// from - backends that hand out opaque identifiers construct one
/// directly via [`Address::opaque`].
impl FromStr for Address {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut octets = [0u8; 6];
        let mut parts = s.split(':');
        for octet in octets.iter_mut() {
            let part = parts.next().ok_or(AddressParseError)?;
            *octet = u8::from_str_radix(part, 16).map_err(|_| AddressParseError)?;
        }
        if parts.next().is_some() {
            return Err(AddressParseError);
        }
        Ok(Self::Mac(octets))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid Bluetooth address, expected AA:BB:CC:DD:EE:FF")]
pub struct AddressParseError;

/// Classic BR/EDR addresses are always public; LE addresses may be a
/// resolvable/non-resolvable private address instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressKind {
    Public,
    Random,
}

/// Identifies a local controller (e.g. `hci0`) independent of backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdapterId(String);

impl AdapterId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Received signal strength, in dBm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rssi(pub i16);

/// Raw 24-bit class-of-device bitfield. Left undecoded here since
/// major/minor/service-class decoding is a presentation-layer concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceClass(pub u32);

/// The GATT Device Information Service's PnP ID characteristic (0x2A50)
/// — the Bluetooth analogue of a USB device descriptor's vendor/product
/// IDs. `vendor_id_source` says whether `vendor_id` is a Bluetooth SIG
/// company identifier or an actual USB-IF-assigned VID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PnpId {
    pub vendor_id_source: VendorIdSource,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_version: u16,
}

impl PnpId {
    /// Parses a PnP ID characteristic's raw value: vendor ID source (1
    /// octet), vendor ID, product ID, product version (2 octets each,
    /// little-endian) — 7 octets total, per the GATT Device Information
    /// Service specification. Returns `None` for a malformed value
    /// (wrong length, or an unrecognized vendor ID source).
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 7] = bytes.try_into().ok()?;
        let vendor_id_source = match bytes[0] {
            1 => VendorIdSource::BluetoothSig,
            2 => VendorIdSource::Usb,
            _ => return None,
        };
        Some(Self {
            vendor_id_source,
            vendor_id: u16::from_le_bytes([bytes[1], bytes[2]]),
            product_id: u16::from_le_bytes([bytes[3], bytes[4]]),
            product_version: u16::from_le_bytes([bytes[5], bytes[6]]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorIdSource {
    BluetoothSig,
    Usb,
}

/// A handful of the GATT Device Information Service's other
/// characteristics — plain UTF-8 text fields describing the device
/// itself, separate from the more structured `PnpId`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
}

/// A 128-bit Bluetooth UUID (service/characteristic/profile identifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uuid(pub u128);

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0.to_be_bytes();
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
             {:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12],
            b[13], b[14], b[15]
        )
    }
}

impl FromStr for Uuid {
    type Err = UuidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex: String = s.chars().filter(|c| *c != '-').collect();
        if hex.len() != 32 {
            return Err(UuidParseError);
        }
        u128::from_str_radix(&hex, 16)
            .map(Uuid)
            .map_err(|_| UuidParseError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid Bluetooth UUID")]
pub struct UuidParseError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_pnp_id_characteristic_value() {
        // source=SIG(1), vendor=0x01DA (Logitech, LE), product=0x1234,
        // version=0x0100.
        let bytes = [0x01, 0xDA, 0x01, 0x34, 0x12, 0x00, 0x01];
        let pnp_id = PnpId::parse(&bytes).unwrap();
        assert_eq!(pnp_id.vendor_id_source, VendorIdSource::BluetoothSig);
        assert_eq!(pnp_id.vendor_id, 0x01DA);
        assert_eq!(pnp_id.product_id, 0x1234);
        assert_eq!(pnp_id.product_version, 0x0100);
    }

    #[test]
    fn rejects_the_wrong_length_or_an_unknown_vendor_id_source() {
        assert!(PnpId::parse(&[1, 2, 3]).is_none());
        assert!(PnpId::parse(&[0, 0, 0, 0, 0, 0, 0]).is_none());
    }
}

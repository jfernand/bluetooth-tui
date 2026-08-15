//! Value types shared by the driver-layer traits, kept free of any
//! backend-specific representation (D-Bus variants, HCI wire formats, ...).

use std::fmt;
use std::str::FromStr;

/// A 48-bit Bluetooth device address (BD_ADDR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address([u8; 6]);

impl Address {
    pub const fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    pub const fn octets(&self) -> [u8; 6] {
        self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a:02X}:{b:02X}:{c:02X}:{d:02X}:{e:02X}:{g:02X}")
    }
}

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
        Ok(Self(octets))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressParseError;

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid Bluetooth address, expected AA:BB:CC:DD:EE:FF")
    }
}

impl std::error::Error for AddressParseError {}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UuidParseError;

impl fmt::Display for UuidParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid Bluetooth UUID")
    }
}

impl std::error::Error for UuidParseError {}

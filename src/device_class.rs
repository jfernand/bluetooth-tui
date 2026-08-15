//! Class of Device (CoD) decoding.
//!
//! Classic Bluetooth's `Class` property packs three things into a 24-bit
//! bitfield:
//!
//! ```text
//! bits  0- 1: format type (always 0b00 today)
//! bits  2- 7: minor device class (6 bits, meaning depends on the major)
//! bits  8-12: major device class (5 bits)
//! bits 13-23: major service class (11 independent flag bits)
//! ```
//!
//! Most modern peripherals are LE-only and don't report a Class of
//! Device at all (BlueZ's `Device1.Class` property is simply absent for
//! them) — this only decodes something for devices with classic
//! Bluetooth support.
//!
//! Data transcribed directly from the Bluetooth SIG's own
//! `assigned_numbers/core/class_of_device.yaml`
//! (<https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/core/class_of_device.yaml>) —
//! small and stable enough to not need embedding as a data file, unlike
//! the other lookup tables in this crate.

/// A decoded Class of Device bitfield.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassOfDevice {
    pub major_device_class: &'static str,
    /// Meaning depends on the major class; `None` for majors with no
    /// assigned minor-class table (Miscellaneous, Imaging, Uncategorized).
    pub minor_device_class: Option<&'static str>,
    /// Only a few majors (LAN/Network Access Point, Peripheral) split
    /// their minor field into two parts; `None` for everything else.
    pub minor_device_subclass: Option<&'static str>,
    /// Independent capability flags, e.g. "Audio", "Telephony" - a
    /// device can have any combination set.
    pub major_service_classes: Vec<&'static str>,
}

/// Decodes a raw `Class` bitfield (BlueZ's `Device1.Class` property).
pub fn decode(raw: u32) -> ClassOfDevice {
    let minor_field = ((raw >> 2) & 0b11_1111) as u8;
    let major_field = ((raw >> 8) & 0b1_1111) as u8;
    let service_bits = (raw >> 13) & 0b111_1111_1111;

    let (minor_device_class, minor_device_subclass) = minor_names(major_field, minor_field);

    ClassOfDevice {
        major_device_class: major_name(major_field),
        minor_device_class,
        minor_device_subclass,
        major_service_classes: service_flag_names(service_bits),
    }
}

fn major_name(major: u8) -> &'static str {
    match major {
        0 => "Miscellaneous",
        1 => "Computer",
        2 => "Phone",
        3 => "LAN/Network Access Point",
        4 => "Audio/Video",
        5 => "Peripheral",
        6 => "Imaging",
        7 => "Wearable",
        8 => "Toy",
        9 => "Health",
        31 => "Uncategorized",
        _ => "Reserved for Future Use",
    }
}

fn minor_names(major: u8, minor: u8) -> (Option<&'static str>, Option<&'static str>) {
    match major {
        1 => (computer_minor(minor), None),
        2 => (phone_minor(minor), None),
        3 => {
            // Upper 3 bits: link utilization. Lower 3 bits: subminor,
            // only value 0 ("Uncategorized") is assigned.
            let (util, sub) = (minor >> 3, minor & 0b111);
            (lan_minor(util), (sub == 0).then_some("Uncategorized"))
        }
        4 => (audio_video_minor(minor), None),
        5 => {
            // Upper 2 bits: device type (keyboard/pointing/combo). Lower
            // 4 bits: peripheral type (joystick, gamepad, ...).
            let (device_type, peripheral_type) = (minor >> 4, minor & 0b1111);
            (
                peripheral_minor(device_type),
                peripheral_subminor(peripheral_type),
            )
        }
        7 => (wearable_minor(minor), None),
        8 => (toy_minor(minor), None),
        9 => (health_minor(minor), None),
        // Miscellaneous, Imaging, and Uncategorized/Reserved have no
        // reliably decodable minor-class table.
        _ => (None, None),
    }
}

fn computer_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Uncategorized",
        1 => "Desktop Workstation",
        2 => "Server-class Computer",
        3 => "Laptop",
        4 => "Handheld PC/PDA (clamshell)",
        5 => "Palm-size PC/PDA",
        6 => "Wearable Computer (watch size)",
        7 => "Tablet",
        _ => return None,
    })
}

fn phone_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Uncategorized",
        1 => "Cellular",
        2 => "Cordless",
        3 => "Smartphone",
        4 => "Wired Modem or Voice Gateway",
        5 => "Common ISDN Access",
        _ => return None,
    })
}

fn lan_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Fully available",
        1 => "1% to 17% utilized",
        2 => "17% to 33% utilized",
        3 => "33% to 50% utilized",
        4 => "50% to 67% utilized",
        5 => "67% to 83% utilized",
        6 => "83% to 99% utilized",
        7 => "No service available",
        _ => return None,
    })
}

fn audio_video_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Uncategorized",
        1 => "Wearable Headset Device",
        2 => "Hands-free Device",
        3 => "Reserved for Future Use",
        4 => "Microphone",
        5 => "Loudspeaker",
        6 => "Headphones",
        7 => "Portable Audio",
        8 => "Car Audio",
        9 => "Set-top Box",
        10 => "HiFi Audio Device",
        11 => "VCR",
        12 => "Video Camera",
        13 => "Camcorder",
        14 => "Video Monitor",
        15 => "Video Display and Loudspeaker",
        16 => "Video Conferencing",
        17 => "Reserved for Future Use",
        18 => "Gaming/Toy",
        19 => "Hearing Aid",
        20 => "Glasses",
        _ => return None,
    })
}

fn peripheral_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Uncategorized",
        1 => "Keyboard",
        2 => "Pointing Device",
        3 => "Combo Keyboard/Pointing Device",
        _ => return None,
    })
}

fn peripheral_subminor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Uncategorized",
        1 => "Joystick",
        2 => "Gamepad",
        3 => "Remote Control",
        4 => "Sensing Device",
        5 => "Digitizer Tablet",
        6 => "Card Reader",
        7 => "Digital Pen",
        8 => "Handheld Scanner",
        9 => "Handheld Gestural Input Device",
        _ => return None,
    })
}

fn wearable_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        1 => "Wristwatch",
        2 => "Pager",
        3 => "Jacket",
        4 => "Helmet",
        5 => "Glasses",
        6 => "Pin",
        _ => return None,
    })
}

fn toy_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        1 => "Robot",
        2 => "Vehicle",
        3 => "Doll/Action Figure",
        4 => "Controller",
        5 => "Game",
        _ => return None,
    })
}

fn health_minor(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "Undefined",
        1 => "Blood Pressure Monitor",
        2 => "Thermometer",
        3 => "Weighing Scale",
        4 => "Glucose Meter",
        5 => "Pulse Oximeter",
        6 => "Heart/Pulse Rate Monitor",
        7 => "Health Data Display",
        8 => "Step Counter",
        9 => "Body Composition Analyzer",
        10 => "Peak Flow Monitor",
        11 => "Medication Monitor",
        12 => "Knee Prosthesis",
        13 => "Ankle Prosthesis",
        14 => "Generic Health Manager",
        15 => "Personal Mobility Device",
        _ => return None,
    })
}

/// `(bit index within the 11-bit service field, name)`. Bit 2 (CoD bit
/// 15, "Reserved for Future Use") is deliberately absent - a reserved
/// bit isn't a capability worth surfacing even if a device happens to
/// have it set.
const SERVICE_FLAGS: &[(u32, &str)] = &[
    (0, "Limited Discoverable Mode"), // CoD bit 13
    (1, "LE audio"),                  // CoD bit 14
    (3, "Positioning"),               // CoD bit 16
    (4, "Networking"),                // CoD bit 17
    (5, "Rendering"),                 // CoD bit 18
    (6, "Capturing"),                 // CoD bit 19
    (7, "Object Transfer"),           // CoD bit 20
    (8, "Audio"),                     // CoD bit 21
    (9, "Telephony"),                 // CoD bit 22
    (10, "Information"),              // CoD bit 23
];

fn service_flag_names(service_bits: u32) -> Vec<&'static str> {
    SERVICE_FLAGS
        .iter()
        .filter(|&&(bit, _)| service_bits & (1 << bit) != 0)
        .map(|&(_, name)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_flat_minor_class() {
        // Major 1 (Computer), minor 3 (Laptop).
        let raw = (1 << 8) | (3 << 2);
        let cod = decode(raw);
        assert_eq!(cod.major_device_class, "Computer");
        assert_eq!(cod.minor_device_class, Some("Laptop"));
        assert_eq!(cod.minor_device_subclass, None);
        assert!(cod.major_service_classes.is_empty());
    }

    #[test]
    fn decodes_a_split_minor_class_peripheral() {
        // Major 5 (Peripheral), device type 2 (Pointing Device),
        // peripheral type 0 (Uncategorized) - a plain mouse.
        let minor_field = 2 << 4;
        let raw = (5 << 8) | (minor_field << 2);
        let cod = decode(raw);
        assert_eq!(cod.major_device_class, "Peripheral");
        assert_eq!(cod.minor_device_class, Some("Pointing Device"));
        assert_eq!(cod.minor_device_subclass, Some("Uncategorized"));
    }

    #[test]
    fn decodes_service_class_flags_and_skips_the_reserved_bit() {
        // Bits 21 (Audio) and 18 (Rendering) set, plus the reserved bit
        // 15, which must not show up in the output.
        let raw = (1 << 21) | (1 << 18) | (1 << 15);
        let cod = decode(raw);
        assert_eq!(cod.major_service_classes, vec!["Rendering", "Audio"]);
    }
}

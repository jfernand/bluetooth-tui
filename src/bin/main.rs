use bluetooth_tui::bluez::BluezDriver;
use bluetooth_tui::driver::{self, Adapter, BluetoothDriver, DriverEvent, EventStream, VendorIdSource};
use bluetooth_tui::{company_id, oui, usb_vendor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let driver = BluezDriver::system().await?;

    let adapter = driver
        .adapters()
        .await?
        .into_iter()
        .next()
        .expect("no adapter found");
    println!("adapter: {} [{}]", adapter.id(), adapter.address());

    println!("paired devices:");
    for device in adapter.paired_devices().await? {
        println!("  {}", describe_device(&device).await);
    }

    println!("listening for events for 60s...");
    let mut events = driver.events().await?;
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        while let Some(event) = events.next().await {
            println!("  {event}");
            if let DriverEvent::DeviceFound { address, .. }
            | DriverEvent::DeviceUpdated { address, .. } = event
            {
                match adapter.device(address).await {
                    Ok(Some(device)) => println!("    {}", describe_device(&device).await),
                    Ok(None) => println!("    (no longer available)"),
                    Err(e) => println!("    (failed to read device: {e})"),
                }
            }
        }
    })
    .await
    .ok();

    Ok(())
}

/// Best-effort vendor label, most to least authoritative:
///
/// 1. IEEE OUI match on the hardware address - only works for addresses
///    that aren't randomized, which most BLE peripherals are.
/// 2. GATT PnP ID (0x2A50) - the real USB-VID equivalent, but only
///    readable while connected with services resolved.
/// 3. A company ID from advertising Manufacturer Specific Data - tells
///    you whose *beacon format* is present, not necessarily who made the
///    device (e.g. a third-party mouse carrying a Microsoft Swift Pair
///    beacon), so it's labeled "beacon:" rather than presented as fact.
async fn vendor_label(device: &impl driver::Device) -> String {
    let address = device.address();
    if let Some(vendor) = oui::vendor(address) {
        return vendor.to_owned();
    }

    if let Ok(Some(pnp_id)) = device.pnp_id().await {
        let vendor = match pnp_id.vendor_id_source {
            VendorIdSource::BluetoothSig => company_id::vendor(pnp_id.vendor_id),
            VendorIdSource::Usb => usb_vendor::vendor(pnp_id.vendor_id),
        };
        let source = match pnp_id.vendor_id_source {
            VendorIdSource::BluetoothSig => "BT-SIG",
            VendorIdSource::Usb => "USB",
        };
        return match vendor {
            Some(name) => format!("{name} (PnP {source} 0x{:04X})", pnp_id.vendor_id),
            None => format!("PnP {source} 0x{:04X}", pnp_id.vendor_id),
        };
    }

    if let Some(&id) = device.manufacturer_ids().first() {
        return match company_id::vendor(id) {
            Some(name) => format!("beacon:{name}"),
            None => format!("beacon:0x{id:04X}"),
        };
    }

    if oui::is_locally_administered(address) {
        "(random)".to_owned()
    } else {
        "(unknown)".to_owned()
    }
}

/// A single human-readable line with every detail the driver layer
/// exposes for a device: address, vendor, name/alias, class, signal
/// strength, pairing/connection state, and advertised service UUIDs.
async fn describe_device(device: &impl driver::Device) -> String {
    let address = device.address();
    let mut parts = vec![format!("{address} ({})", vendor_label(device).await)];

    if let Some(name) = device.name() {
        parts.push(format!("name={name:?}"));
    }
    if let Some(alias) = device.alias() {
        parts.push(format!("alias={alias:?}"));
    }
    if let Some(class) = device.class() {
        parts.push(format!("class=0x{:06x}", class.0));
    }
    if let Some(rssi) = device.rssi() {
        parts.push(format!("rssi={}dBm", rssi.0));
    }

    let mut state = Vec::new();
    if device.is_paired() {
        state.push("paired");
    }
    if device.is_bonded() {
        state.push("bonded");
    }
    if device.is_connected() {
        state.push("connected");
    }
    if device.is_trusted() {
        state.push("trusted");
    }
    if device.is_blocked() {
        state.push("blocked");
    }
    parts.push(format!("[{}]", state.join(" ")));

    let uuids = device.service_uuids();
    if uuids.is_empty() {
        parts.push("uuids=none".to_owned());
    } else {
        let uuids: Vec<String> = uuids.iter().map(ToString::to_string).collect();
        parts.push(format!("uuids=[{}]", uuids.join(", ")));
    }

    parts.join(" ")
}

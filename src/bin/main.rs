use bluetooth_tui::bluez::BluezDriver;
use bluetooth_tui::driver::{self, Adapter, BluetoothDriver, DriverEvent, EventStream};
use bluetooth_tui::oui;

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
        println!("  {}", describe_device(&device));
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
                    Ok(Some(device)) => println!("    {}", describe_device(&device)),
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

/// A single human-readable line with every detail the driver layer
/// exposes for a device: address, name/alias, class, signal strength,
/// pairing/connection state, and advertised service UUIDs.
fn describe_device(device: &impl driver::Device) -> String {
    let address = device.address();
    let mut parts = vec![format!("{address} ({})", oui::humanize(address))];

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

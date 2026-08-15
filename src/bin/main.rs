use bluetooth_tui::bluez::BluezDriver;
use bluetooth_tui::driver::{Adapter, BluetoothDriver, Device, EventStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let driver = BluezDriver::system().await?;

    let adapter = driver
        .default_adapter()
        .await?
        .expect("no adapter found");
    println!("default adapter: {} [{}]", adapter.id(), adapter.address());

    println!("paired devices:");
    for device in adapter.paired_devices().await? {
        println!("  {} alias={:?}", device.address(), device.alias());
    }

    println!("listening for events for 5s...");
    let mut events = driver.events().await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(event) = events.next().await {
            println!("  event: {event:?}");
        }
    })
    .await
    .ok();

    Ok(())
}

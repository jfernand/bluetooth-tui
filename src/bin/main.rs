use bluetooth_tui::bluez::BluezDriver;
use bluetooth_tui::driver::{Adapter, BluetoothDriver, Device, EventStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        println!("  {} alias={:?}", device.address(), device.alias());
    }

    println!("listening for events for 60s...");
    let mut events = driver.events().await?;
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        while let Some(event) = events.next().await {
            println!("  event: {event:?}");
        }
    })
    .await
    .ok();

    Ok(())
}

use bluetooth_driver::driver::{DriverEvent, EventStream};

/// Web Bluetooth has essentially no ambient event stream to speak of:
/// no adapter power/discoverable signals (there's no adapter to change),
/// no passive device-found stream (there's no background scan), and
/// per-device `gattserverdisconnected` events require already holding a
/// `BluetoothDevice` handle to subscribe through in the first place -
/// there's nothing to listen to before the frontend has one.
///
/// A real wiring of that per-device signal is future work for whichever
/// frontend actually tracks connected devices; until then this simply
/// never yields anything, which is a valid, honest `EventStream` - the
/// frontend's periodic polling (`Adapter::devices()`, `Device::refresh()`)
/// carries the weight BlueZ's D-Bus signals otherwise would.
pub struct WebEvents;

impl EventStream for WebEvents {
    async fn next(&mut self) -> Option<DriverEvent> {
        std::future::pending().await
    }
}

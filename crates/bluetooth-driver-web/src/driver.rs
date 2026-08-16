use bluetooth_driver::driver::{AdapterId, BluetoothDriver, DriverError};

use crate::adapter::WebAdapter;
use crate::events::WebEvents;

/// Entry point into `navigator.bluetooth`. Must be constructed from
/// within a browser context - there's no equivalent to `BluezDriver`'s
/// "connect to a system service" step, since `navigator.bluetooth` is
/// already just sitting there on the global `Window`.
pub struct WebBluetoothDriver {
    bluetooth: web_sys::Bluetooth,
}

impl WebBluetoothDriver {
    /// Grabs `navigator.bluetooth` from the current page. Fails if
    /// there's no `Window` (not running in a browser tab) or the
    /// browser doesn't implement Web Bluetooth at all (Safari, Firefox,
    /// and any non-Chromium browser as of this writing).
    pub fn new() -> Result<Self, DriverError> {
        let window = web_sys::window()
            .ok_or(DriverError::Unsupported("not running in a browser window"))?;
        let bluetooth = window
            .navigator()
            .bluetooth()
            .ok_or(DriverError::Unsupported("this browser has no Web Bluetooth support"))?;
        Ok(Self { bluetooth })
    }
}

impl BluetoothDriver for WebBluetoothDriver {
    type Adapter = WebAdapter;
    type Events = WebEvents;

    /// Always exactly the one synthetic adapter - see `WebAdapter`.
    async fn adapters(&self) -> Result<Vec<WebAdapter>, DriverError> {
        Ok(vec![WebAdapter::new(self.bluetooth.clone())])
    }

    async fn adapter(&self, id: &AdapterId) -> Result<Option<WebAdapter>, DriverError> {
        if id.as_str() == "web" {
            Ok(Some(WebAdapter::new(self.bluetooth.clone())))
        } else {
            Ok(None)
        }
    }

    async fn events(&self) -> Result<WebEvents, DriverError> {
        Ok(WebEvents)
    }
}

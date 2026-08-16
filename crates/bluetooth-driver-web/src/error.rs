use bluetooth_driver::driver::DriverError;
use wasm_bindgen::JsCast;

/// Maps a rejected JS promise onto `DriverError`. Web Bluetooth mostly
/// rejects with a `DOMException` (name + message) rather than a plain
/// `Error`, so check that shape first; fall back to a debug print of
/// whatever else came back rather than losing the failure entirely.
pub(crate) fn map_js_error(err: wasm_bindgen::JsValue) -> DriverError {
    if let Some(exception) = err.dyn_ref::<web_sys::DomException>() {
        return match exception.name().as_str() {
            "NotFoundError" => DriverError::NotFound,
            "SecurityError" | "NotAllowedError" => DriverError::PermissionDenied,
            "NetworkError" => DriverError::NotReady,
            _ => DriverError::Backend(format!("{}: {}", exception.name(), exception.message())),
        };
    }
    if let Some(error) = err.dyn_ref::<js_sys::Error>() {
        return DriverError::Backend(String::from(error.message()));
    }
    DriverError::Backend(format!("{err:?}"))
}

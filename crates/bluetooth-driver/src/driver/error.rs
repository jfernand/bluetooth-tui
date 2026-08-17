use thiserror::Error;

/// Failure modes common to every backend (D-Bus/BlueZ, raw HCI, mock).
#[derive(Debug, Error)]
pub enum DriverError {
    /// The requested adapter or device doesn't exist.
    #[error("device or adapter not found")]
    NotFound,
    /// The backend refused the operation for lack of authorization.
    #[error("permission denied")]
    PermissionDenied,
    /// The adapter exists but isn't in a state that can service the request.
    #[error("adapter not ready")]
    NotReady,
    /// The operation didn't complete within the backend's own bound.
    #[error("operation timed out")]
    Timeout,
    /// This backend has no equivalent for the requested operation at
    /// all (e.g. Web Bluetooth has no adapter power control) - the
    /// string names what was asked for.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// The backend (or its pairing agent) declined the request.
    #[error("rejected: {0}")]
    Rejected(String),
    /// Any other backend-specific failure, carrying its own message.
    #[error("backend error: {0}")]
    Backend(String),
}

use thiserror::Error;

/// Failure modes common to every backend (D-Bus/BlueZ, raw HCI, mock).
#[derive(Debug, Error)]
pub enum DriverError {
    #[error("device or adapter not found")]
    NotFound,
    #[error("permission denied")]
    PermissionDenied,
    #[error("adapter not ready")]
    NotReady,
    #[error("operation timed out")]
    Timeout,
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("rejected: {0}")]
    Rejected(String),
    #[error("backend error: {0}")]
    Backend(String),
}

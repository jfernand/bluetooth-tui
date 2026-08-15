use std::fmt;

/// Failure modes common to every backend (D-Bus/BlueZ, raw HCI, mock).
#[derive(Debug)]
pub enum DriverError {
    NotFound,
    PermissionDenied,
    NotReady,
    Timeout,
    Unsupported(&'static str),
    Rejected(String),
    Backend(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "device or adapter not found"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::NotReady => write!(f, "adapter not ready"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
            Self::Rejected(reason) => write!(f, "rejected: {reason}"),
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
        }
    }
}

impl std::error::Error for DriverError {}

//! Error types for the media scanner

use std::path::PathBuf;
use thiserror::Error;

/// Error kinds that can occur during scanning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanErrorKind {
    /// Permission denied when accessing a file or directory
    PermissionDenied,
    /// File or directory not found
    NotFound,
    /// I/O error during file operations
    IoError,
    /// Database operation failed
    DatabaseError,
    /// Hash computation failed
    HashError,
    /// Invalid path encoding
    InvalidPath,
    /// Unknown error
    Unknown,
}

/// Represents an error that occurred during scanning
#[derive(Debug, Error)]
#[error("{kind:?}: {message} (path: {path:?})")]
pub struct ScanError {
    /// The kind of error
    pub kind: ScanErrorKind,
    /// The path where the error occurred
    pub path: Option<PathBuf>,
    /// Human-readable error message
    pub message: String,
}

impl ScanError {
    /// Create a new scan error
    pub fn new(kind: ScanErrorKind, path: Option<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            kind,
            path,
            message: message.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(path: PathBuf) -> Self {
        Self::new(
            ScanErrorKind::PermissionDenied,
            Some(path.clone()),
            format!("Permission denied: {:?}", path),
        )
    }

    /// Create a not found error
    pub fn not_found(path: PathBuf) -> Self {
        Self::new(
            ScanErrorKind::NotFound,
            Some(path.clone()),
            format!("Not found: {:?}", path),
        )
    }

    /// Create an I/O error
    pub fn io_error(path: Option<PathBuf>, message: impl Into<String>) -> Self {
        Self::new(ScanErrorKind::IoError, path, message)
    }

    /// Create a database error
    pub fn database_error(message: impl Into<String>) -> Self {
        Self::new(ScanErrorKind::DatabaseError, None, message)
    }

    /// Create a hash computation error
    pub fn hash_error(path: PathBuf, message: impl Into<String>) -> Self {
        Self::new(ScanErrorKind::HashError, Some(path), message)
    }
}

impl From<std::io::Error> for ScanError {
    fn from(err: std::io::Error) -> Self {
        let kind = match err.kind() {
            std::io::ErrorKind::PermissionDenied => ScanErrorKind::PermissionDenied,
            std::io::ErrorKind::NotFound => ScanErrorKind::NotFound,
            _ => ScanErrorKind::IoError,
        };
        Self::new(kind, None, err.to_string())
    }
}

impl From<rusqlite::Error> for ScanError {
    fn from(err: rusqlite::Error) -> Self {
        Self::database_error(err.to_string())
    }
}

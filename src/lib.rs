//! High-performance media file scanner with parallel directory traversal
//!
//! This library provides efficient scanning and indexing of media files
//! using parallel processing with rayon and walkdir.

#![allow(dead_code)]

pub mod config;
pub mod error;
pub mod models;
pub mod scanner;

pub use config::ScanConfig;
pub use error::{ScanError, ScanErrorKind};
pub use models::{MediaType, ScanProgress, ScanResult, ScannedFile};
pub use scanner::scan_full;

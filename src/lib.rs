//! High-performance media file scanner with parallel directory traversal
//!
//! This library provides efficient scanning and indexing of media files
//! using parallel processing with rayon and walkdir.

#![allow(dead_code)]

pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod scanner;

pub use config::ScanConfig;
pub use db::ScanDatabase;
pub use error::{ScanError, ScanErrorKind};
pub use models::{CompactFile, MediaType, ScanProgress, ScanResult, ScannedDirectory, ScannedFile};
pub use scanner::{scan_full, scan_incremental};

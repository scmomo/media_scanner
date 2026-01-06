//! Core data models for the media scanner

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::ScanError;

/// File status in incremental scan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    /// New file (not in previous scan)
    #[default]
    New,
    /// Modified file (size or mtime changed)
    Modified,
    /// Unchanged file (same as previous scan)
    Unchanged,
    /// Deleted file (was in previous scan, now missing)
    Deleted,
}

impl FileStatus {
    /// Get short code for compact output
    pub fn as_char(&self) -> char {
        match self {
            FileStatus::New => 'n',
            FileStatus::Modified => 'm',
            FileStatus::Unchanged => 'u',
            FileStatus::Deleted => 'd',
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FileStatus::New => "new",
            FileStatus::Modified => "modified",
            FileStatus::Unchanged => "unchanged",
            FileStatus::Deleted => "deleted",
        }
    }
}

/// Media type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Video files (mp4, mkv, avi, etc.)
    Video,
    /// Image files (jpg, png, webp, etc.)
    Image,
    /// Audio files (mp3, flac, wav, etc.)
    Audio,
    /// Unknown or unsupported media type
    Unknown,
}

impl MediaType {
    /// Infer media type from file extension
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        match ext_lower.as_str() {
            // Video extensions
            "mp4" | "mkv" | "avi" | "wmv" | "flv" | "mov" | "webm" | "m4v" | "ts" | "rmvb" => {
                MediaType::Video
            }
            // Image extensions
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" => MediaType::Image,
            // Audio extensions
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "wma" | "m4a" => MediaType::Audio,
            // Unknown
            _ => MediaType::Unknown,
        }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Video => "video",
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a scanned file with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    /// Full path to the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// File name without path
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Modification time as Unix timestamp
    pub mtime: i64,
    /// Creation time as Unix timestamp
    pub ctime: i64,
    /// File extension (lowercase, without dot)
    pub extension: String,
    /// Inferred media type
    pub media_type: MediaType,
    /// File hash (MD5 or partial hash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Whether the hash is a partial hash (for large files)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_partial_hash: bool,
    /// File status (new/modified/unchanged/deleted)
    #[serde(skip_serializing_if = "is_default_status")]
    pub status: FileStatus,
}

fn is_default_status(status: &FileStatus) -> bool {
    *status == FileStatus::New
}

impl ScannedFile {
    /// Create a new ScannedFile with basic metadata
    pub fn new(
        path: PathBuf,
        name: String,
        size: u64,
        mtime: i64,
        ctime: i64,
        extension: String,
    ) -> Self {
        let media_type = MediaType::from_extension(&extension);
        Self {
            path: Some(path),
            name,
            size,
            mtime,
            ctime,
            extension,
            media_type,
            hash: None,
            is_partial_hash: false,
            status: FileStatus::New,
        }
    }

    /// Set the file hash
    pub fn with_hash(mut self, hash: String, is_partial: bool) -> Self {
        self.hash = Some(hash);
        self.is_partial_hash = is_partial;
        self
    }

    /// Set the file status
    pub fn with_status(mut self, status: FileStatus) -> Self {
        self.status = status;
        self
    }

    /// Get full path (for internal use)
    pub fn full_path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}

/// Represents a directory with its files (compact format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedDirectory {
    /// Directory path
    pub path: String,
    /// Files in this directory
    pub files: Vec<CompactFile>,
}

/// Compact file representation (without directory path)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactFile {
    /// File name only
    #[serde(rename = "n")]
    pub name: String,
    /// File size in bytes
    #[serde(rename = "s")]
    pub size: u64,
    /// Modification time as Unix timestamp
    #[serde(rename = "m")]
    pub mtime: i64,
    /// Media type (v=video, i=image, a=audio, u=unknown)
    #[serde(rename = "t")]
    pub media_type: char,
    /// File status (n=new, m=modified, u=unchanged)
    #[serde(rename = "st", skip_serializing_if = "is_new_status_char")]
    pub status: char,
    /// File hash (optional)
    #[serde(rename = "h", skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

fn is_new_status_char(c: &char) -> bool {
    *c == 'n'
}

impl CompactFile {
    /// Create from ScannedFile
    pub fn from_scanned(file: &ScannedFile) -> Self {
        Self {
            name: file.name.clone(),
            size: file.size,
            mtime: file.mtime,
            media_type: match file.media_type {
                MediaType::Video => 'v',
                MediaType::Image => 'i',
                MediaType::Audio => 'a',
                MediaType::Unknown => 'u',
            },
            status: file.status.as_char(),
            hash: file.hash.clone(),
        }
    }
}

/// Result of a scan operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanResult {
    /// Total number of files found
    pub total_files: u64,
    /// Total number of directories traversed
    pub total_dirs: u64,
    /// Number of new files (for incremental scans)
    pub new_files: u64,
    /// Number of modified files (for incremental scans)
    pub modified_files: u64,
    /// Number of unchanged files (for incremental scans)
    pub unchanged_files: u64,
    /// Number of deleted files (for incremental scans)
    pub deleted_files: u64,
    /// List of scanned files with metadata (new + modified only in incremental mode)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<ScannedFile>,
    /// List of deleted file paths (incremental mode only)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deleted_paths: Vec<String>,
    /// Errors encountered during scanning
    #[serde(skip)]
    pub errors: Vec<ScanError>,
    /// Total scan duration in milliseconds
    pub duration_ms: u64,
}

impl ScanResult {
    /// Create a new empty scan result
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Check if the scan completed without errors
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Progress information during a scan
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Number of directories scanned so far
    pub scanned_dirs: u64,
    /// Number of files scanned so far
    pub scanned_files: u64,
    /// Current path being scanned
    pub current_path: String,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
}

impl ScanProgress {
    /// Create a new progress instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the current path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.current_path = path.into();
        self
    }

    /// Calculate estimated remaining time based on total expected items
    pub fn estimated_remaining_ms(&self, total_expected: u64) -> Option<u64> {
        if self.scanned_files == 0 || self.elapsed_ms == 0 {
            return None;
        }
        let rate = self.scanned_files as f64 / self.elapsed_ms as f64;
        let remaining = total_expected.saturating_sub(self.scanned_files);
        Some((remaining as f64 / rate) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_from_extension() {
        // Video extensions
        assert_eq!(MediaType::from_extension("mp4"), MediaType::Video);
        assert_eq!(MediaType::from_extension("MKV"), MediaType::Video);
        assert_eq!(MediaType::from_extension("avi"), MediaType::Video);
        assert_eq!(MediaType::from_extension("webm"), MediaType::Video);

        // Image extensions
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Image);
        assert_eq!(MediaType::from_extension("JPEG"), MediaType::Image);
        assert_eq!(MediaType::from_extension("png"), MediaType::Image);
        assert_eq!(MediaType::from_extension("webp"), MediaType::Image);

        // Audio extensions
        assert_eq!(MediaType::from_extension("mp3"), MediaType::Audio);
        assert_eq!(MediaType::from_extension("FLAC"), MediaType::Audio);

        // Unknown
        assert_eq!(MediaType::from_extension("txt"), MediaType::Unknown);
        assert_eq!(MediaType::from_extension("exe"), MediaType::Unknown);
    }

    #[test]
    fn test_scanned_file_creation() {
        let file = ScannedFile::new(
            PathBuf::from("/test/video.mp4"),
            "video.mp4".to_string(),
            1024,
            1234567890,
            1234567800,
            "mp4".to_string(),
        );

        assert_eq!(file.name, "video.mp4");
        assert_eq!(file.size, 1024);
        assert_eq!(file.media_type, MediaType::Video);
        assert!(file.hash.is_none());
        assert!(!file.is_partial_hash);
    }

    #[test]
    fn test_scanned_file_with_hash() {
        let file = ScannedFile::new(
            PathBuf::from("/test/image.jpg"),
            "image.jpg".to_string(),
            512,
            1234567890,
            1234567800,
            "jpg".to_string(),
        )
        .with_hash("abc123".to_string(), false);

        assert_eq!(file.hash, Some("abc123".to_string()));
        assert!(!file.is_partial_hash);
    }

    #[test]
    fn test_scan_progress_estimated_remaining() {
        let progress = ScanProgress {
            scanned_dirs: 10,
            scanned_files: 100,
            current_path: "/test".to_string(),
            elapsed_ms: 1000,
        };

        // 100 files in 1000ms = 0.1 files/ms
        // 900 remaining files / 0.1 = 9000ms
        let remaining = progress.estimated_remaining_ms(1000);
        assert_eq!(remaining, Some(9000));
    }

    #[test]
    fn test_scan_result_default() {
        let result = ScanResult::new();
        assert_eq!(result.total_files, 0);
        assert_eq!(result.error_count(), 0);
        assert!(result.is_success());
    }
}

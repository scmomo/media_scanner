//! Scanner module - implements the actual file scanning logic

use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

use crate::config::ScanConfig;
use crate::error::{ScanError, ScanErrorKind};
use crate::models::{MediaType, ScanResult, ScannedFile};

/// Perform a full scan of the configured directories
pub fn scan_full(config: &ScanConfig) -> ScanResult {
    let start = Instant::now();
    let total_files = AtomicU64::new(0);
    let total_dirs = AtomicU64::new(0);
    let mut errors = Vec::new();
    let mut files = Vec::new();

    for root in &config.roots {
        if !root.exists() {
            errors.push(ScanError::not_found(root.clone()));
            continue;
        }

        let walker = WalkDir::new(root)
            .max_depth(config.effective_max_depth())
            .follow_links(false)
            .into_iter();

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // Check if directory should be ignored
                    if entry.file_type().is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if config.should_ignore_dir(name) {
                                continue;
                            }
                        }
                        total_dirs.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }

                    // Process files
                    if entry.file_type().is_file() {
                        if let Some(scanned) = process_file(path, config) {
                            files.push(scanned);
                            total_files.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(e) => {
                    let path = e.path().map(|p| p.to_path_buf());
                    let kind = if e.io_error().map(|e| e.kind())
                        == Some(std::io::ErrorKind::PermissionDenied)
                    {
                        ScanErrorKind::PermissionDenied
                    } else {
                        ScanErrorKind::IoError
                    };
                    errors.push(ScanError::new(kind, path, e.to_string()));
                }
            }
        }
    }

    let duration = start.elapsed();

    ScanResult {
        total_files: total_files.load(Ordering::Relaxed),
        total_dirs: total_dirs.load(Ordering::Relaxed),
        new_files: total_files.load(Ordering::Relaxed),
        modified_files: 0,
        deleted_files: 0,
        errors,
        duration_ms: duration.as_millis() as u64,
    }
}

/// Process a single file and return ScannedFile if it matches the filter
fn process_file(path: &Path, config: &ScanConfig) -> Option<ScannedFile> {
    // Get file extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // Check if extension is in whitelist
    if !config.should_include_extension(&extension) {
        return None;
    }

    // Get file metadata
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return None,
    };

    // Get file name
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Get timestamps
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let ctime = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(mtime);

    let mut scanned = ScannedFile::new(
        path.to_path_buf(),
        name,
        metadata.len(),
        mtime,
        ctime,
        extension,
    );

    // Compute hash if enabled
    if config.compute_hash {
        if let Some((hash, is_partial)) = compute_file_hash(path, config.large_file_threshold) {
            scanned = scanned.with_hash(hash, is_partial);
        }
    }

    Some(scanned)
}

/// Compute file hash (MD5)
/// For large files, compute partial hash (first 1MB + last 1MB)
fn compute_file_hash(path: &Path, large_file_threshold: u64) -> Option<(String, bool)> {
    use md5::{Digest, Md5};
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let file_size = metadata.len();

    let mut hasher = Md5::new();

    if file_size <= large_file_threshold {
        // Full hash for small files
        let mut file = file;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        hasher.update(&buffer);
        let result = hasher.finalize();
        Some((format!("{:x}", result), false))
    } else {
        // Partial hash for large files (first 1MB + last 1MB)
        let chunk_size = 1024 * 1024; // 1MB
        let mut file = file;
        let mut buffer = vec![0u8; chunk_size];

        // Read first 1MB
        let bytes_read = file.read(&mut buffer).ok()?;
        hasher.update(&buffer[..bytes_read]);

        // Read last 1MB
        if file_size > chunk_size as u64 {
            file.seek(SeekFrom::End(-(chunk_size as i64))).ok()?;
            let bytes_read = file.read(&mut buffer).ok()?;
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Some((format!("{:x}", result), true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_media_type_detection() {
        assert_eq!(MediaType::from_extension("mp4"), MediaType::Video);
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Image);
        assert_eq!(MediaType::from_extension("mp3"), MediaType::Audio);
        assert_eq!(MediaType::from_extension("txt"), MediaType::Unknown);
    }
}

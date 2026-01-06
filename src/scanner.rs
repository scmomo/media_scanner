//! Scanner module - implements the actual file scanning logic

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

use crate::config::ScanConfig;
use crate::db::{FileRecord, ScanDatabase};
use crate::error::{ScanError, ScanErrorKind};
use crate::models::{MediaType, ScanResult, ScannedFile};

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(&ScanProgress) + Send + Sync>;

/// Scan progress information
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    /// Total files scanned so far
    pub scanned_files: u64,
    /// Total directories scanned so far
    pub scanned_dirs: u64,
    /// Video files count
    pub video_count: u64,
    /// Image files count
    pub image_count: u64,
    /// Audio files count
    pub audio_count: u64,
    /// Current directory being scanned
    pub current_dir: String,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
}

impl ScanProgress {
    /// Output progress to stderr as JSON
    pub fn print_to_stderr(&self) {
        let json = serde_json::json!({
            "_t": "p",
            "f": self.scanned_files,
            "d": self.scanned_dirs,
            "v": self.video_count,
            "i": self.image_count,
            "a": self.audio_count,
            "dir": self.current_dir,
            "ms": self.elapsed_ms
        });
        eprintln!("{}", json);
        std::io::stderr().flush().ok();
    }
}

/// Perform a full scan of the configured directories
pub fn scan_full(config: &ScanConfig) -> ScanResult {
    scan_internal(config, None, config.show_progress)
}

/// Perform a full scan with progress callback
pub fn scan_full_with_progress(config: &ScanConfig, show_progress: bool) -> ScanResult {
    scan_internal(config, None, show_progress)
}

/// Perform an incremental scan using database for comparison
pub fn scan_incremental(config: &ScanConfig, db: &mut ScanDatabase) -> ScanResult {
    // Load existing file index from database
    let file_index = match db.load_file_index() {
        Ok(index) => index,
        Err(e) => {
            log::error!("Failed to load file index: {}", e);
            return scan_full(config);
        }
    };

    let result = scan_internal(config, Some(&file_index), config.show_progress);

    // Update database with changes
    if !result.files.is_empty() {
        if let Err(e) = db.upsert_files(&result.files) {
            log::error!("Failed to update database: {}", e);
        }
    }

    // Delete removed files from database
    if !result.deleted_paths.is_empty() {
        if let Err(e) = db.delete_files(&result.deleted_paths) {
            log::error!("Failed to delete files from database: {}", e);
        }
    }

    result
}

/// Internal scan implementation
fn scan_internal(
    config: &ScanConfig,
    file_index: Option<&std::collections::HashMap<String, FileRecord>>,
    show_progress: bool,
) -> ScanResult {
    let start = Instant::now();
    let total_files = AtomicU64::new(0);
    let total_dirs = AtomicU64::new(0);
    let new_files = AtomicU64::new(0);
    let modified_files = AtomicU64::new(0);
    let video_count = AtomicU64::new(0);
    let image_count = AtomicU64::new(0);
    let audio_count = AtomicU64::new(0);
    let mut errors = Vec::new();
    let mut files = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    // Progress tracking
    let mut last_progress_time = Instant::now();
    let mut current_dir = String::new();
    let progress_interval_ms = 500; // Report every 500ms

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
                        current_dir = path.to_string_lossy().to_string();
                        continue;
                    }

                    // Process files
                    if entry.file_type().is_file() {
                        let path_str = path.to_string_lossy().to_string();
                        seen_paths.insert(path_str.clone());

                        // Check if file changed (incremental mode)
                        if let Some(index) = file_index {
                            if let Some(record) = index.get(&path_str) {
                                // Quick check: size + mtime
                                let metadata = match std::fs::metadata(path) {
                                    Ok(m) => m,
                                    Err(_) => continue,
                                };

                                let current_size = metadata.len();
                                let current_mtime = metadata
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);

                                // File unchanged - skip
                                if record.size == current_size && record.mtime == current_mtime {
                                    total_files.fetch_add(1, Ordering::Relaxed);
                                    continue;
                                }

                                // File modified - process and mark
                                if let Some(scanned) = process_file(path, config) {
                                    update_media_counts(&scanned.media_type, &video_count, &image_count, &audio_count);
                                    files.push(scanned);
                                    total_files.fetch_add(1, Ordering::Relaxed);
                                    modified_files.fetch_add(1, Ordering::Relaxed);
                                }
                                continue;
                            }
                        }

                        // New file or full scan mode
                        if let Some(scanned) = process_file(path, config) {
                            update_media_counts(&scanned.media_type, &video_count, &image_count, &audio_count);
                            files.push(scanned);
                            total_files.fetch_add(1, Ordering::Relaxed);
                            if file_index.is_some() {
                                new_files.fetch_add(1, Ordering::Relaxed);
                            }
                        }

                        // Report progress periodically
                        if show_progress && last_progress_time.elapsed().as_millis() >= progress_interval_ms {
                            let progress = ScanProgress {
                                scanned_files: total_files.load(Ordering::Relaxed),
                                scanned_dirs: total_dirs.load(Ordering::Relaxed),
                                video_count: video_count.load(Ordering::Relaxed),
                                image_count: image_count.load(Ordering::Relaxed),
                                audio_count: audio_count.load(Ordering::Relaxed),
                                current_dir: current_dir.clone(),
                                elapsed_ms: start.elapsed().as_millis() as u64,
                            };
                            progress.print_to_stderr();
                            last_progress_time = Instant::now();
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

    // Find deleted files (only in incremental mode)
    let mut deleted_paths = Vec::new();
    let deleted_count;
    if let Some(index) = file_index {
        for path in index.keys() {
            if !seen_paths.contains(path) {
                deleted_paths.push(path.clone());
            }
        }
        deleted_count = deleted_paths.len() as u64;
    } else {
        deleted_count = 0;
    }

    let duration = start.elapsed();

    let total = total_files.load(Ordering::Relaxed);
    let new_count = if file_index.is_some() {
        new_files.load(Ordering::Relaxed)
    } else {
        total
    };

    // Final progress report
    if show_progress {
        let progress = ScanProgress {
            scanned_files: total,
            scanned_dirs: total_dirs.load(Ordering::Relaxed),
            video_count: video_count.load(Ordering::Relaxed),
            image_count: image_count.load(Ordering::Relaxed),
            audio_count: audio_count.load(Ordering::Relaxed),
            current_dir: "完成".to_string(),
            elapsed_ms: duration.as_millis() as u64,
        };
        progress.print_to_stderr();
    }

    ScanResult {
        total_files: total,
        total_dirs: total_dirs.load(Ordering::Relaxed),
        new_files: new_count,
        modified_files: modified_files.load(Ordering::Relaxed),
        deleted_files: deleted_count,
        files,
        deleted_paths,
        errors,
        duration_ms: duration.as_millis() as u64,
    }
}

/// Update media type counters
fn update_media_counts(
    media_type: &MediaType,
    video_count: &AtomicU64,
    image_count: &AtomicU64,
    audio_count: &AtomicU64,
) {
    match media_type {
        MediaType::Video => video_count.fetch_add(1, Ordering::Relaxed),
        MediaType::Image => image_count.fetch_add(1, Ordering::Relaxed),
        MediaType::Audio => audio_count.fetch_add(1, Ordering::Relaxed),
        MediaType::Unknown => 0,
    };
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
    use crate::models::MediaType;

    #[test]
    fn test_media_type_detection() {
        assert_eq!(MediaType::from_extension("mp4"), MediaType::Video);
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Image);
        assert_eq!(MediaType::from_extension("mp3"), MediaType::Audio);
        assert_eq!(MediaType::from_extension("txt"), MediaType::Unknown);
    }
}

//! Configuration for the media scanner

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Default large file threshold (100 MB)
pub const DEFAULT_LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Default batch size for database writes
pub const DEFAULT_BATCH_SIZE: usize = 1000;

/// Default checkpoint interval (files processed)
pub const DEFAULT_CHECKPOINT_INTERVAL: u64 = 5000;

/// Default max depth for recursive scanning
pub const DEFAULT_MAX_DEPTH: usize = 3;

/// Configuration for the scanner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Root directories to scan
    pub roots: Vec<PathBuf>,

    /// File extensions to include (whitelist)
    /// If empty, all media extensions are included
    pub extensions: HashSet<String>,

    /// Directory names to ignore
    pub ignore_dirs: HashSet<String>,

    /// Whether to compute file hashes
    pub compute_hash: bool,

    /// Threshold for using partial hash (bytes)
    /// Files larger than this use partial hash (first 1MB + last 1MB)
    pub large_file_threshold: u64,

    /// Number of threads for parallel processing
    /// 0 means auto-detect (CPU cores × 2)
    pub num_threads: usize,

    /// Batch size for database writes
    pub batch_size: usize,

    /// Checkpoint interval (save progress every N files)
    pub checkpoint_interval: u64,

    /// Database path for storing results
    pub db_path: Option<PathBuf>,

    /// Whether to scan subdirectories recursively
    /// If false, only scan files in the root directories
    pub recursive: bool,

    /// Maximum depth for recursive scanning
    /// Default is 3 levels
    pub max_depth: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            extensions: Self::default_extensions(),
            ignore_dirs: Self::default_ignore_dirs(),
            compute_hash: true,
            large_file_threshold: DEFAULT_LARGE_FILE_THRESHOLD,
            num_threads: 0,
            batch_size: DEFAULT_BATCH_SIZE,
            checkpoint_interval: DEFAULT_CHECKPOINT_INTERVAL,
            db_path: None,
            recursive: true,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl ScanConfig {
    /// Create a new config with the given root directories
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            ..Default::default()
        }
    }

    /// Create a config builder
    pub fn builder() -> ScanConfigBuilder {
        ScanConfigBuilder::new()
    }

    /// Get the default video extensions
    pub fn default_video_extensions() -> HashSet<String> {
        [
            "mp4", "mkv", "avi", "wmv", "flv", "mov", "webm", "m4v", "ts", "rmvb",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Get the default image extensions
    pub fn default_image_extensions() -> HashSet<String> {
        ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the default audio extensions
    pub fn default_audio_extensions() -> HashSet<String> {
        ["mp3", "flac", "wav", "aac", "ogg", "wma", "m4a"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get all default media extensions
    pub fn default_extensions() -> HashSet<String> {
        let mut extensions = Self::default_video_extensions();
        extensions.extend(Self::default_image_extensions());
        extensions.extend(Self::default_audio_extensions());
        extensions
    }

    /// Get the default directories to ignore
    pub fn default_ignore_dirs() -> HashSet<String> {
        [
            "$RECYCLE.BIN",
            "System Volume Information",
            ".Trash",
            ".Trash-1000",
            "@eaDir",
            ".git",
            ".svn",
            "node_modules",
            "__pycache__",
            ".cache",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Check if an extension should be included
    pub fn should_include_extension(&self, ext: &str) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        self.extensions.contains(&ext.to_lowercase())
    }

    /// Check if a directory should be ignored
    pub fn should_ignore_dir(&self, name: &str) -> bool {
        // Ignore hidden directories (starting with .)
        if name.starts_with('.') {
            return true;
        }
        // Ignore configured directories
        self.ignore_dirs.contains(name)
    }

    /// Get the effective number of threads
    pub fn effective_threads(&self) -> usize {
        if self.num_threads == 0 {
            // Auto-detect: CPU cores × 2
            std::thread::available_parallelism()
                .map(|p| p.get() * 2)
                .unwrap_or(4)
        } else {
            self.num_threads
        }
    }

    /// Get the effective max depth for walkdir
    /// Returns the depth limit based on recursive and max_depth settings
    pub fn effective_max_depth(&self) -> usize {
        if !self.recursive {
            1 // Only scan immediate children (depth 1)
        } else {
            self.max_depth
        }
    }
}

/// Builder for ScanConfig
#[derive(Debug, Default)]
pub struct ScanConfigBuilder {
    config: ScanConfig,
}

impl ScanConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root directories
    pub fn roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.config.roots = roots;
        self
    }

    /// Add a root directory
    pub fn add_root(mut self, root: PathBuf) -> Self {
        self.config.roots.push(root);
        self
    }

    /// Set the extensions whitelist
    pub fn extensions(mut self, extensions: HashSet<String>) -> Self {
        self.config.extensions = extensions;
        self
    }

    /// Set only video extensions
    pub fn video_only(mut self) -> Self {
        self.config.extensions = ScanConfig::default_video_extensions();
        self
    }

    /// Set only image extensions
    pub fn image_only(mut self) -> Self {
        self.config.extensions = ScanConfig::default_image_extensions();
        self
    }

    /// Set the directories to ignore
    pub fn ignore_dirs(mut self, dirs: HashSet<String>) -> Self {
        self.config.ignore_dirs = dirs;
        self
    }

    /// Add a directory to ignore
    pub fn add_ignore_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.ignore_dirs.insert(dir.into());
        self
    }

    /// Enable or disable hash computation
    pub fn compute_hash(mut self, enabled: bool) -> Self {
        self.config.compute_hash = enabled;
        self
    }

    /// Set the large file threshold
    pub fn large_file_threshold(mut self, threshold: u64) -> Self {
        self.config.large_file_threshold = threshold;
        self
    }

    /// Set the number of threads
    pub fn num_threads(mut self, threads: usize) -> Self {
        self.config.num_threads = threads;
        self
    }

    /// Set the batch size
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    /// Set the checkpoint interval
    pub fn checkpoint_interval(mut self, interval: u64) -> Self {
        self.config.checkpoint_interval = interval;
        self
    }

    /// Set the database path
    pub fn db_path(mut self, path: PathBuf) -> Self {
        self.config.db_path = Some(path);
        self
    }

    /// Enable or disable recursive scanning
    pub fn recursive(mut self, enabled: bool) -> Self {
        self.config.recursive = enabled;
        self
    }

    /// Set the maximum depth for recursive scanning
    /// Default is 3 levels
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Build the config
    pub fn build(self) -> ScanConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScanConfig::default();
        assert!(config.roots.is_empty());
        assert!(config.compute_hash);
        assert_eq!(config.large_file_threshold, DEFAULT_LARGE_FILE_THRESHOLD);
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn test_default_extensions() {
        let extensions = ScanConfig::default_extensions();
        assert!(extensions.contains("mp4"));
        assert!(extensions.contains("jpg"));
        assert!(extensions.contains("mp3"));
        assert!(!extensions.contains("txt"));
    }

    #[test]
    fn test_should_include_extension() {
        let config = ScanConfig::default();
        assert!(config.should_include_extension("mp4"));
        assert!(config.should_include_extension("MP4"));
        assert!(config.should_include_extension("jpg"));
        assert!(!config.should_include_extension("txt"));
    }

    #[test]
    fn test_should_ignore_dir() {
        let config = ScanConfig::default();
        // Hidden directories
        assert!(config.should_ignore_dir(".git"));
        assert!(config.should_ignore_dir(".hidden"));
        // System directories
        assert!(config.should_ignore_dir("$RECYCLE.BIN"));
        assert!(config.should_ignore_dir("System Volume Information"));
        // Normal directories
        assert!(!config.should_ignore_dir("Videos"));
        assert!(!config.should_ignore_dir("Photos"));
    }

    #[test]
    fn test_config_builder() {
        let config = ScanConfig::builder()
            .add_root(PathBuf::from("/test"))
            .video_only()
            .compute_hash(false)
            .num_threads(4)
            .batch_size(500)
            .build();

        assert_eq!(config.roots.len(), 1);
        assert!(!config.compute_hash);
        assert_eq!(config.num_threads, 4);
        assert_eq!(config.batch_size, 500);
        assert!(config.extensions.contains("mp4"));
        assert!(!config.extensions.contains("jpg"));
    }

    #[test]
    fn test_effective_threads() {
        let config = ScanConfig::builder().num_threads(8).build();
        assert_eq!(config.effective_threads(), 8);

        let auto_config = ScanConfig::default();
        assert!(auto_config.effective_threads() > 0);
    }
}

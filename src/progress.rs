//! Progress reporting module for scan operations
//!
//! This module provides data structures and utilities for reporting
//! scan progress to external callers via stderr.

use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::config::ScanConfig;
use crate::error::ScanError;
use crate::models::ScanResult;
use crate::scanner::ScanProgress;

/// Scan phase indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanPhase {
    /// Scanning directories and discovering files
    #[default]
    Scan,
    /// Processing discovered files
    Process,
    /// Scan completed
    Done,
}

impl ScanPhase {
    /// Get string representation of the phase
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanPhase::Scan => "scan",
            ScanPhase::Process => "process",
            ScanPhase::Done => "done",
        }
    }
}

/// Start message sent when scan begins
#[derive(Debug, Clone, Serialize)]
pub struct StartMessage {
    /// Message type identifier
    #[serde(rename = "_t")]
    pub msg_type: &'static str,
    /// Sequence number
    pub seq: u64,
    /// Timestamp in milliseconds since reporter creation
    pub ts: u64,
    /// Scan root paths
    pub roots: Vec<String>,
    /// Whether recursive scanning is enabled
    pub recursive: bool,
    /// Maximum depth for recursive scanning
    pub max_depth: usize,
    /// Whether hash computation is enabled
    pub compute_hash: bool,
}

impl StartMessage {
    /// Create a new start message
    pub fn new(
        seq: u64,
        ts: u64,
        roots: Vec<String>,
        recursive: bool,
        max_depth: usize,
        compute_hash: bool,
    ) -> Self {
        Self {
            msg_type: "start",
            seq,
            ts,
            roots,
            recursive,
            max_depth,
            compute_hash,
        }
    }
}


/// Progress message sent during scan
#[derive(Debug, Clone, Serialize)]
pub struct ProgressMessage {
    /// Message type identifier ("p" for progress)
    #[serde(rename = "_t")]
    pub msg_type: &'static str,
    /// Sequence number
    pub seq: u64,
    /// Timestamp in milliseconds since reporter creation
    pub ts: u64,
    /// Current scan phase
    pub phase: ScanPhase,
    /// Number of files scanned
    #[serde(rename = "f")]
    pub files: u64,
    /// Number of directories scanned
    #[serde(rename = "d")]
    pub dirs: u64,
    /// Number of video files found
    #[serde(rename = "v")]
    pub video_count: u64,
    /// Number of image files found
    #[serde(rename = "i")]
    pub image_count: u64,
    /// Number of audio files found
    #[serde(rename = "a")]
    pub audio_count: u64,
    /// Current directory being scanned
    pub dir: String,
    /// Elapsed time in milliseconds
    pub ms: u64,
    /// Estimated remaining time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
}

impl ProgressMessage {
    /// Create a new progress message
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seq: u64,
        ts: u64,
        phase: ScanPhase,
        files: u64,
        dirs: u64,
        video_count: u64,
        image_count: u64,
        audio_count: u64,
        dir: String,
        ms: u64,
        eta_ms: Option<u64>,
    ) -> Self {
        Self {
            msg_type: "p",
            seq,
            ts,
            phase,
            files,
            dirs,
            video_count,
            image_count,
            audio_count,
            dir,
            ms,
            eta_ms,
        }
    }
}


/// Error message sent when an error occurs during scan
#[derive(Debug, Clone, Serialize)]
pub struct ErrorProgressMessage {
    /// Message type identifier ("err" for error)
    #[serde(rename = "_t")]
    pub msg_type: &'static str,
    /// Sequence number
    pub seq: u64,
    /// Timestamp in milliseconds since reporter creation
    pub ts: u64,
    /// Error type/category
    pub error_type: String,
    /// Error message description
    pub message: String,
    /// Path that caused the error (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ErrorProgressMessage {
    /// Create a new error progress message
    pub fn new(
        seq: u64,
        ts: u64,
        error_type: String,
        message: String,
        path: Option<String>,
    ) -> Self {
        Self {
            msg_type: "err",
            seq,
            ts,
            error_type,
            message,
            path,
        }
    }
}

/// Done message sent when scan completes
#[derive(Debug, Clone, Serialize)]
pub struct DoneMessage {
    /// Message type identifier ("done" for completion)
    #[serde(rename = "_t")]
    pub msg_type: &'static str,
    /// Sequence number
    pub seq: u64,
    /// Timestamp in milliseconds since reporter creation
    pub ts: u64,
    /// Total number of files scanned
    #[serde(rename = "tf")]
    pub total_files: u64,
    /// Total number of directories scanned
    #[serde(rename = "td")]
    pub total_dirs: u64,
    /// Number of new files found
    #[serde(rename = "nf")]
    pub new_files: u64,
    /// Number of modified files found
    #[serde(rename = "mf")]
    pub modified_files: u64,
    /// Number of deleted files found
    #[serde(rename = "df")]
    pub deleted_files: u64,
    /// Number of errors encountered
    #[serde(rename = "ec")]
    pub error_count: usize,
    /// Total scan duration in milliseconds
    pub ms: u64,
}

impl DoneMessage {
    /// Create a new done message
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seq: u64,
        ts: u64,
        total_files: u64,
        total_dirs: u64,
        new_files: u64,
        modified_files: u64,
        deleted_files: u64,
        error_count: usize,
        ms: u64,
    ) -> Self {
        Self {
            msg_type: "done",
            seq,
            ts,
            total_files,
            total_dirs,
            new_files,
            modified_files,
            deleted_files,
            error_count,
            ms,
        }
    }
}

/// Progress reporter for outputting scan progress to stderr
///
/// This component manages the output of progress messages during scan operations.
/// It handles timing, sequence numbers, and formatting of various message types.
pub struct ProgressReporter {
    /// Whether progress reporting is enabled
    enabled: bool,
    /// Reporting interval in milliseconds
    interval_ms: u64,
    /// Last report time
    last_report: std::cell::Cell<Instant>,
    /// Sequence number for messages (atomic for thread safety)
    seq: AtomicU64,
    /// Start time of the reporter
    start_time: Instant,
}

impl ProgressReporter {
    /// Create a new ProgressReporter
    ///
    /// # Arguments
    /// * `enabled` - Whether progress reporting is enabled
    /// * `interval_ms` - Minimum interval between progress messages in milliseconds
    pub fn new(enabled: bool, interval_ms: u64) -> Self {
        let now = Instant::now();
        Self {
            enabled,
            interval_ms,
            last_report: std::cell::Cell::new(now),
            seq: AtomicU64::new(0),
            start_time: now,
        }
    }

    /// Check if enough time has passed since the last report
    ///
    /// Returns true if the interval has elapsed and a new progress message should be sent.
    pub fn should_report(&self) -> bool {
        if !self.enabled {
            return false;
        }
        let elapsed = self.last_report.get().elapsed().as_millis() as u64;
        elapsed >= self.interval_ms
    }

    /// Get the next sequence number (monotonically increasing)
    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    /// Get the current timestamp in milliseconds since reporter creation
    pub fn current_timestamp(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Output a serializable message to stderr as JSON
    pub fn output_to_stderr<T: Serialize>(&self, msg: &T) {
        if let Ok(json) = serde_json::to_string(msg) {
            eprintln!("{}", json);
            std::io::stderr().flush().ok();
        }
    }

    /// Report scan start
    ///
    /// Outputs a StartMessage with scan configuration summary.
    pub fn report_start(&self, config: &ScanConfig) {
        if !self.enabled {
            return;
        }

        let roots: Vec<String> = config
            .roots
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let msg = StartMessage::new(
            self.next_seq(),
            self.current_timestamp(),
            roots,
            config.recursive,
            config.max_depth,
            config.compute_hash,
        );

        self.output_to_stderr(&msg);
    }

    /// Report scan progress
    ///
    /// Outputs a ProgressMessage with current scan statistics.
    /// Returns true if a message was actually sent (respects interval timing).
    pub fn report_progress(&self, progress: &ScanProgress) -> bool {
        if !self.enabled {
            return false;
        }

        if !self.should_report() {
            return false;
        }

        let msg = ProgressMessage::new(
            self.next_seq(),
            self.current_timestamp(),
            ScanPhase::Scan,
            progress.scanned_files,
            progress.scanned_dirs,
            progress.video_count,
            progress.image_count,
            progress.audio_count,
            progress.current_dir.clone(),
            progress.elapsed_ms,
            None, // eta_ms - can be calculated externally if needed
        );

        self.output_to_stderr(&msg);
        self.last_report.set(Instant::now());
        true
    }

    /// Report an error during scan
    ///
    /// Outputs an ErrorProgressMessage immediately (ignores interval timing).
    pub fn report_error(&self, error: &ScanError) {
        if !self.enabled {
            return;
        }

        let msg = ErrorProgressMessage::new(
            self.next_seq(),
            self.current_timestamp(),
            format!("{:?}", error.kind),
            error.message.clone(),
            error.path.as_ref().map(|p| p.to_string_lossy().to_string()),
        );

        self.output_to_stderr(&msg);
    }

    /// Report scan completion
    ///
    /// Outputs a DoneMessage with final scan statistics.
    pub fn report_done(&self, result: &ScanResult) {
        if !self.enabled {
            return;
        }

        let msg = DoneMessage::new(
            self.next_seq(),
            self.current_timestamp(),
            result.total_files,
            result.total_dirs,
            result.new_files,
            result.modified_files,
            result.deleted_files,
            result.error_count(),
            result.duration_ms,
        );

        self.output_to_stderr(&msg);
    }

    /// Check if the reporter is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_phase_serialization() {
        assert_eq!(
            serde_json::to_string(&ScanPhase::Scan).unwrap(),
            "\"scan\""
        );
        assert_eq!(
            serde_json::to_string(&ScanPhase::Process).unwrap(),
            "\"process\""
        );
        assert_eq!(
            serde_json::to_string(&ScanPhase::Done).unwrap(),
            "\"done\""
        );
    }

    #[test]
    fn test_scan_phase_as_str() {
        assert_eq!(ScanPhase::Scan.as_str(), "scan");
        assert_eq!(ScanPhase::Process.as_str(), "process");
        assert_eq!(ScanPhase::Done.as_str(), "done");
    }

    #[test]
    fn test_start_message_serialization() {
        let msg = StartMessage::new(
            1,
            100,
            vec!["/path/to/scan".to_string()],
            true,
            10,
            false,
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["_t"], "start");
        assert_eq!(parsed["seq"], 1);
        assert_eq!(parsed["ts"], 100);
        assert_eq!(parsed["roots"][0], "/path/to/scan");
        assert_eq!(parsed["recursive"], true);
        assert_eq!(parsed["max_depth"], 10);
        assert_eq!(parsed["compute_hash"], false);
    }

    #[test]
    fn test_progress_message_serialization() {
        let msg = ProgressMessage::new(
            2,
            200,
            ScanPhase::Scan,
            100,
            10,
            50,
            30,
            20,
            "/current/dir".to_string(),
            1500,
            Some(3000),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["_t"], "p");
        assert_eq!(parsed["seq"], 2);
        assert_eq!(parsed["ts"], 200);
        assert_eq!(parsed["phase"], "scan");
        assert_eq!(parsed["f"], 100);
        assert_eq!(parsed["d"], 10);
        assert_eq!(parsed["v"], 50);
        assert_eq!(parsed["i"], 30);
        assert_eq!(parsed["a"], 20);
        assert_eq!(parsed["dir"], "/current/dir");
        assert_eq!(parsed["ms"], 1500);
        assert_eq!(parsed["eta_ms"], 3000);
    }

    #[test]
    fn test_progress_message_without_eta() {
        let msg = ProgressMessage::new(
            1,
            100,
            ScanPhase::Scan,
            10,
            5,
            5,
            3,
            2,
            "/dir".to_string(),
            500,
            None,
        );
        let json = serde_json::to_string(&msg).unwrap();
        
        // eta_ms should not be present when None
        assert!(!json.contains("eta_ms"));
    }

    #[test]
    fn test_error_message_serialization() {
        let msg = ErrorProgressMessage::new(
            3,
            300,
            "PermissionDenied".to_string(),
            "Access denied to file".to_string(),
            Some("/path/to/file".to_string()),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["_t"], "err");
        assert_eq!(parsed["seq"], 3);
        assert_eq!(parsed["ts"], 300);
        assert_eq!(parsed["error_type"], "PermissionDenied");
        assert_eq!(parsed["message"], "Access denied to file");
        assert_eq!(parsed["path"], "/path/to/file");
    }

    #[test]
    fn test_error_message_without_path() {
        let msg = ErrorProgressMessage::new(
            1,
            100,
            "IoError".to_string(),
            "General IO error".to_string(),
            None,
        );
        let json = serde_json::to_string(&msg).unwrap();
        
        // path should not be present when None
        assert!(!json.contains("\"path\""));
    }

    #[test]
    fn test_done_message_serialization() {
        let msg = DoneMessage::new(
            10,
            5000,
            1000,
            100,
            500,
            200,
            50,
            5,
            4500,
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["_t"], "done");
        assert_eq!(parsed["seq"], 10);
        assert_eq!(parsed["ts"], 5000);
        assert_eq!(parsed["tf"], 1000);
        assert_eq!(parsed["td"], 100);
        assert_eq!(parsed["nf"], 500);
        assert_eq!(parsed["mf"], 200);
        assert_eq!(parsed["df"], 50);
        assert_eq!(parsed["ec"], 5);
        assert_eq!(parsed["ms"], 4500);
    }

    #[test]
    fn test_progress_reporter_new() {
        let reporter = ProgressReporter::new(true, 200);
        assert!(reporter.is_enabled());
        
        let reporter_disabled = ProgressReporter::new(false, 200);
        assert!(!reporter_disabled.is_enabled());
    }

    #[test]
    fn test_progress_reporter_sequence_numbers() {
        let reporter = ProgressReporter::new(true, 200);
        
        // Sequence numbers should be monotonically increasing
        let seq1 = reporter.next_seq();
        let seq2 = reporter.next_seq();
        let seq3 = reporter.next_seq();
        
        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 2);
        assert!(seq1 < seq2);
        assert!(seq2 < seq3);
    }

    #[test]
    fn test_progress_reporter_timestamp() {
        let reporter = ProgressReporter::new(true, 200);
        
        let ts1 = reporter.current_timestamp();
        // Small delay
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = reporter.current_timestamp();
        
        // Timestamp should increase over time
        assert!(ts2 >= ts1);
    }

    #[test]
    fn test_progress_reporter_should_report_disabled() {
        let reporter = ProgressReporter::new(false, 200);
        
        // Should never report when disabled
        assert!(!reporter.should_report());
    }

    #[test]
    fn test_progress_reporter_should_report_timing() {
        // Use a very short interval for testing
        let reporter = ProgressReporter::new(true, 10);
        
        // Update last_report to now
        reporter.last_report.set(std::time::Instant::now());
        
        // Should not report immediately after
        assert!(!reporter.should_report());
        
        // Wait for interval to pass (with some buffer)
        std::thread::sleep(std::time::Duration::from_millis(20));
        
        // Should report after interval
        assert!(reporter.should_report());
    }

    #[test]
    fn test_progress_reporter_report_start_disabled() {
        use crate::config::ScanConfig;
        
        let reporter = ProgressReporter::new(false, 200);
        let config = ScanConfig::default();
        
        // Should not panic when disabled
        reporter.report_start(&config);
        
        // Sequence should not increment when disabled
        assert_eq!(reporter.next_seq(), 0);
    }

    #[test]
    fn test_progress_reporter_report_progress_disabled() {
        use crate::scanner::ScanProgress;
        
        let reporter = ProgressReporter::new(false, 200);
        let progress = ScanProgress::default();
        
        // Should return false when disabled
        let result = reporter.report_progress(&progress);
        assert!(!result);
    }

    #[test]
    fn test_progress_reporter_report_error_disabled() {
        use crate::error::{ScanError, ScanErrorKind};
        
        let reporter = ProgressReporter::new(false, 200);
        let error = ScanError::new(
            ScanErrorKind::IoError,
            None,
            "Test error".to_string(),
        );
        
        // Should not panic when disabled
        reporter.report_error(&error);
    }

    #[test]
    fn test_progress_reporter_report_done_disabled() {
        use crate::models::ScanResult;
        
        let reporter = ProgressReporter::new(false, 200);
        let result = ScanResult::default();
        
        // Should not panic when disabled
        reporter.report_done(&result);
    }
}

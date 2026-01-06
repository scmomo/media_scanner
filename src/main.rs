//! Media Scanner CLI
//!
//! High-performance media file scanner with parallel directory traversal.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use std::path::PathBuf;

use media_scanner::{ScanConfig, ScanResult};

/// High-performance media file scanner
#[derive(Parser)]
#[command(name = "media-scanner")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan directories for media files
    Scan {
        /// Root directories to scan
        #[arg(short, long, required = true)]
        roots: Vec<PathBuf>,

        /// Number of threads (0 = auto)
        #[arg(short, long, default_value = "0")]
        threads: usize,

        /// Batch size for database writes
        #[arg(short, long, default_value = "1000")]
        batch_size: usize,

        /// Database file path
        #[arg(short, long)]
        db: Option<PathBuf>,

        /// Perform incremental scan
        #[arg(short, long)]
        incremental: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Skip hash computation
        #[arg(long)]
        no_hash: bool,

        /// Disable recursive scanning (only scan root directories)
        #[arg(long)]
        no_recursive: bool,

        /// Maximum depth for recursive scanning (default: 3)
        #[arg(long, default_value = "3")]
        max_depth: usize,
    },
}

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            roots,
            threads,
            batch_size,
            db,
            incremental,
            json,
            no_hash,
            no_recursive,
            max_depth,
        } => {
            info!("Starting media scan...");
            info!("Roots: {:?}", roots);
            info!("Threads: {}", if threads == 0 { "auto".to_string() } else { threads.to_string() });
            info!("Batch size: {}", batch_size);
            info!("Incremental: {}", incremental);
            info!("Recursive: {}", !no_recursive);
            info!("Max depth: {}", max_depth);

            let config = ScanConfig::builder()
                .roots(roots)
                .num_threads(threads)
                .batch_size(batch_size)
                .compute_hash(!no_hash)
                .recursive(!no_recursive)
                .max_depth(max_depth)
                .db_path(db.unwrap_or_else(|| PathBuf::from("media_scanner.db")))
                .build();

            // TODO: Implement actual scanning in later tasks
            // For now, just log the config to avoid unused variable warning
            info!("Config: {:?}", config);
            let result = ScanResult::new();

            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                println!("Scan completed:");
                println!("  Total files: {}", result.total_files);
                println!("  Total dirs: {}", result.total_dirs);
                println!("  New files: {}", result.new_files);
                println!("  Modified files: {}", result.modified_files);
                println!("  Deleted files: {}", result.deleted_files);
                println!("  Errors: {}", result.error_count());
                println!("  Duration: {}ms", result.duration_ms);
            }
        }
    }
}

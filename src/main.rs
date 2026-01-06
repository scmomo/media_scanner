//! Media Scanner CLI
//!
//! High-performance media file scanner with parallel directory traversal.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use std::path::PathBuf;

use media_scanner::{scan_full, ScanConfig};

const ABOUT: &str = r#"
Media Scanner - 高性能媒体文件扫描器

使用示例:
  media_scanner scan -r /path/to/media              扫描单个目录
  media_scanner scan -r /videos -r /photos          扫描多个目录
  media_scanner scan -r /media --max-depth 5        扫描5层深度
  media_scanner scan -r /media --no-recursive       只扫描根目录
  media_scanner scan -r /media --json               JSON格式输出
  media_scanner scan -r /media -d output.db         指定数据库文件

更多信息请查看: https://github.com/your-repo/media-scanner
"#;

/// High-performance media file scanner
#[derive(Parser)]
#[command(name = "media_scanner")]
#[command(author, version, about = ABOUT, long_about = None)]
#[command(help_template = "\
{before-help}{name} {version}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 扫描目录中的媒体文件
    #[command(about = "扫描目录中的媒体文件")]
    Scan {
        /// 扫描的根目录（可指定多个）
        #[arg(short = 'r', long, required = true)]
        roots: Vec<PathBuf>,

        /// 并行线程数（0 = 自动检测）
        #[arg(short = 't', long, default_value = "0")]
        threads: usize,

        /// 数据库批量写入大小
        #[arg(short = 'b', long, default_value = "1000")]
        batch_size: usize,

        /// 数据库文件路径
        #[arg(short = 'd', long)]
        db: Option<PathBuf>,

        /// 执行增量扫描
        #[arg(short = 'i', long)]
        incremental: bool,

        /// 以 JSON 格式输出结果
        #[arg(long)]
        json: bool,

        /// 跳过文件哈希计算
        #[arg(long)]
        no_hash: bool,

        /// 禁用递归扫描（只扫描根目录）
        #[arg(long)]
        no_recursive: bool,

        /// 最大扫描深度
        #[arg(long, default_value = "3")]
        max_depth: usize,
    },
}

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Scan {
            roots,
            threads,
            batch_size,
            db,
            incremental,
            json,
            no_hash,
            no_recursive,
            max_depth,
        }) => {
            info!("Starting media scan...");
            info!("Roots: {:?}", roots);
            info!(
                "Threads: {}",
                if threads == 0 {
                    "auto".to_string()
                } else {
                    threads.to_string()
                }
            );
            info!("Batch size: {}", batch_size);
            info!("Incremental: {}", incremental);
            info!("Recursive: {}", !no_recursive);
            info!("Max depth: {}", max_depth);

            let _db_path = db.unwrap_or_else(|| PathBuf::from("media_scanner.db"));

            let config = ScanConfig::builder()
                .roots(roots)
                .num_threads(threads)
                .batch_size(batch_size)
                .compute_hash(!no_hash)
                .recursive(!no_recursive)
                .max_depth(max_depth)
                .build();

            info!("Config: {:?}", config);

            // Perform the actual scan
            let result = scan_full(&config);

            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                println!("扫描完成:");
                println!("  媒体文件数: {}", result.total_files);
                println!("  目录数: {}", result.total_dirs);
                println!("  新文件: {}", result.new_files);
                println!("  修改文件: {}", result.modified_files);
                println!("  删除文件: {}", result.deleted_files);
                println!("  错误数: {}", result.error_count());
                println!("  耗时: {}ms", result.duration_ms);
            }
        }
        None => {
            println!("{}", ABOUT);
            println!("使用 'media_scanner scan -h' 查看扫描命令的详细帮助");
            println!("使用 'media_scanner --help' 查看完整帮助信息");
        }
    }
}

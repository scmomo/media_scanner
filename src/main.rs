//! Media Scanner CLI
//!
//! High-performance media file scanner with parallel directory traversal.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use std::path::PathBuf;

use media_scanner::{ScanConfig, ScanResult};

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

    /// 显示帮助信息
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, global = true)]
    help: Option<bool>,
}

#[derive(Subcommand)]
enum Commands {
    /// 扫描目录中的媒体文件
    #[command(about = "扫描目录中的媒体文件")]
    Scan {
        /// 扫描的根目录（可指定多个）
        #[arg(short = 'r', long, required = true, help = "扫描的根目录，可多次指定")]
        roots: Vec<PathBuf>,

        /// 并行线程数（0 = 自动检测）
        #[arg(short = 't', long, default_value = "0", help = "并行线程数，0表示自动检测")]
        threads: usize,

        /// 数据库批量写入大小
        #[arg(short = 'b', long, default_value = "1000", help = "批量写入数据库的记录数")]
        batch_size: usize,

        /// 数据库文件路径
        #[arg(short = 'd', long, help = "SQLite数据库文件路径")]
        db: Option<PathBuf>,

        /// 执行增量扫描
        #[arg(short = 'i', long, help = "只扫描新增或修改的文件")]
        incremental: bool,

        /// 以 JSON 格式输出结果
        #[arg(long, help = "输出JSON格式的扫描结果")]
        json: bool,

        /// 跳过文件哈希计算
        #[arg(long, help = "不计算文件哈希值（加快扫描速度）")]
        no_hash: bool,

        /// 禁用递归扫描（只扫描根目录）
        #[arg(long, help = "不递归扫描子目录")]
        no_recursive: bool,

        /// 最大扫描深度
        #[arg(long, default_value = "3", help = "递归扫描的最大深度")]
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
        None => {
            // 没有子命令时显示帮助
            println!("{}", ABOUT);
            println!("使用 'media_scanner scan -h' 查看扫描命令的详细帮助");
            println!("使用 'media_scanner --help' 查看完整帮助信息");
        }
    }
}

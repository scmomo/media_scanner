//! Media Scanner CLI
//!
//! High-performance media file scanner with parallel directory traversal.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use media_scanner::{
    scan_full, scan_incremental, CompactFile, ScanConfig, ScanDatabase, ScannedDirectory,
};

const ABOUT: &str = r#"
Media Scanner - 高性能媒体文件扫描器

使用示例:
  media_scanner scan -r /path/to/media              扫描单个目录
  media_scanner scan -r /videos -r /photos          扫描多个目录
  media_scanner scan -r /media --max-depth 5        扫描5层深度
  media_scanner scan -r /media --no-recursive       只扫描根目录
  media_scanner scan -r /media --json               JSON格式输出（完整）
  media_scanner scan -r /media --ndjson             NDJSON流式输出（每行一个文件）
  media_scanner scan -r /media --compact            紧凑格式（按目录分组，推荐）
  media_scanner scan -r /media -o result.json       输出到文件
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

        /// 以 JSON 格式输出结果（完整JSON，适合小量文件）
        #[arg(long)]
        json: bool,

        /// 以 NDJSON 格式输出（每行一个文件，适合大量文件）
        #[arg(long)]
        ndjson: bool,

        /// 紧凑格式输出（按目录分组，字段缩写，推荐大量文件）
        #[arg(long)]
        compact: bool,

        /// 输出结果到文件（避免stdout缓冲问题）
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// 跳过文件哈希计算
        #[arg(long)]
        no_hash: bool,

        /// 禁用递归扫描（只扫描根目录）
        #[arg(long)]
        no_recursive: bool,

        /// 最大扫描深度
        #[arg(long, default_value = "3")]
        max_depth: usize,

        /// 显示扫描进度（输出到stderr，不影响JSON输出）
        #[arg(short = 'p', long)]
        progress: bool,
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
            ndjson,
            compact,
            output,
            no_hash,
            no_recursive,
            max_depth,
            progress,
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
            info!("Progress: {}", progress);

            let db_path = db.unwrap_or_else(|| PathBuf::from("media_scanner.db"));

            let config = ScanConfig::builder()
                .roots(roots)
                .num_threads(threads)
                .batch_size(batch_size)
                .compute_hash(!no_hash)
                .recursive(!no_recursive)
                .max_depth(max_depth)
                .show_progress(progress)
                .build();

            info!("Config: {:?}", config);

            // Perform scan (incremental or full)
            let result = if incremental {
                info!("Opening database: {:?}", db_path);
                match ScanDatabase::open(&db_path) {
                    Ok(mut scan_db) => {
                        let r = scan_incremental(&config, &mut scan_db);
                        info!(
                            "Incremental scan: {} new, {} modified, {} deleted",
                            r.new_files, r.modified_files, r.deleted_files
                        );
                        r
                    }
                    Err(e) => {
                        eprintln!("无法打开数据库 {:?}: {}", db_path, e);
                        eprintln!("将执行完整扫描...");
                        let r = scan_full(&config);
                        // Save to database for next incremental scan
                        if let Ok(mut scan_db) = ScanDatabase::open(&db_path) {
                            if let Err(e) = scan_db.upsert_files(&r.files) {
                                eprintln!("保存到数据库失败: {}", e);
                            }
                        }
                        r
                    }
                }
            } else {
                let r = scan_full(&config);
                // Save to database for future incremental scans
                if let Ok(mut scan_db) = ScanDatabase::open(&db_path) {
                    info!("Saving {} files to database", r.files.len());
                    if let Err(e) = scan_db.upsert_files(&r.files) {
                        eprintln!("保存到数据库失败: {}", e);
                    }
                }
                r
            };

            // Determine output destination
            let mut writer: Box<dyn Write> = if let Some(ref path) = output {
                match File::create(path) {
                    Ok(f) => Box::new(BufWriter::new(f)),
                    Err(e) => {
                        eprintln!("无法创建输出文件 {:?}: {}", path, e);
                        return;
                    }
                }
            } else {
                Box::new(BufWriter::new(std::io::stdout()))
            };

            if compact {
                // 紧凑格式：按目录分组，字段使用缩写
                // 第一行输出统计信息
                let stats = serde_json::json!({
                    "_t": "s",
                    "tf": result.total_files,
                    "td": result.total_dirs,
                    "nf": result.new_files,
                    "mf": result.modified_files,
                    "uf": result.unchanged_files,
                    "df": result.deleted_files,
                    "ec": result.error_count(),
                    "ms": result.duration_ms
                });
                writeln!(writer, "{}", stats).ok();

                // 按目录分组 (新增+修改的文件)
                let mut dirs: HashMap<String, Vec<CompactFile>> = HashMap::new();
                for file in &result.files {
                    if let Some(path) = file.full_path() {
                        let dir = path
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        dirs.entry(dir)
                            .or_default()
                            .push(CompactFile::from_scanned(file));
                    }
                }

                // 每个目录一行
                for (path, files) in dirs {
                    let dir = ScannedDirectory { path, files };
                    if let Ok(line) = serde_json::to_string(&dir) {
                        writeln!(writer, "{}", line).ok();
                    }
                }

                // 输出删除的文件（如果有）
                if !result.deleted_paths.is_empty() {
                    let deleted = serde_json::json!({
                        "_t": "d",
                        "paths": result.deleted_paths
                    });
                    writeln!(writer, "{}", deleted).ok();
                }
            } else if ndjson {
                // NDJSON 格式：每行一个文件，适合大量文件流式处理
                // 第一行输出统计信息
                let stats = serde_json::json!({
                    "_type": "summary",
                    "total_files": result.total_files,
                    "total_dirs": result.total_dirs,
                    "new_files": result.new_files,
                    "modified_files": result.modified_files,
                    "unchanged_files": result.unchanged_files,
                    "deleted_files": result.deleted_files,
                    "error_count": result.error_count(),
                    "duration_ms": result.duration_ms
                });
                writeln!(writer, "{}", stats).ok();

                // 每个文件一行 (新增+修改)
                for file in &result.files {
                    if let Ok(line) = serde_json::to_string(file) {
                        writeln!(writer, "{}", line).ok();
                    }
                }

                // 输出删除的文件
                for path in &result.deleted_paths {
                    let deleted = serde_json::json!({
                        "_type": "deleted",
                        "path": path
                    });
                    writeln!(writer, "{}", deleted).ok();
                }
            } else if json {
                // JSON 格式：目录树结构，适合完整输出
                // 按目录分组
                let mut dirs: HashMap<String, Vec<CompactFile>> = HashMap::new();
                for file in &result.files {
                    if let Some(path) = file.full_path() {
                        let dir = path
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        dirs.entry(dir)
                            .or_default()
                            .push(CompactFile::from_scanned(file));
                    }
                }

                let directories: Vec<ScannedDirectory> = dirs
                    .into_iter()
                    .map(|(path, files)| ScannedDirectory { path, files })
                    .collect();

                let output_json = serde_json::json!({
                    "summary": {
                        "total_files": result.total_files,
                        "total_dirs": result.total_dirs,
                        "new_files": result.new_files,
                        "modified_files": result.modified_files,
                        "unchanged_files": result.unchanged_files,
                        "deleted_files": result.deleted_files,
                        "error_count": result.error_count(),
                        "duration_ms": result.duration_ms
                    },
                    "directories": directories,
                    "deleted": result.deleted_paths
                });

                writeln!(
                    writer,
                    "{}",
                    serde_json::to_string_pretty(&output_json).unwrap()
                )
                .ok();
            } else {
                // 人类可读格式
                writeln!(writer, "扫描完成:").ok();
                writeln!(writer, "  媒体文件数: {}", result.total_files).ok();
                writeln!(writer, "  目录数: {}", result.total_dirs).ok();
                writeln!(writer, "  新文件: {}", result.new_files).ok();
                writeln!(writer, "  修改文件: {}", result.modified_files).ok();
                writeln!(writer, "  未更改: {}", result.unchanged_files).ok();
                writeln!(writer, "  删除文件: {}", result.deleted_files).ok();
                writeln!(writer, "  错误数: {}", result.error_count()).ok();
                writeln!(writer, "  耗时: {}ms", result.duration_ms).ok();
            }

            writer.flush().ok();

            if let Some(path) = output {
                println!("结果已保存到: {:?}", path);
            }
        }
        None => {
            println!("{}", ABOUT);
            println!("使用 'media_scanner scan -h' 查看扫描命令的详细帮助");
            println!("使用 'media_scanner --help' 查看完整帮助信息");
        }
    }
}

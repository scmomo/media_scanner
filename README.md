# Media Scanner

高性能媒体文件扫描器，使用 Rust 编写，支持并行目录遍历和批量数据库写入。

## 功能特性

- 🚀 **高性能并行扫描** - 使用 rayon 实现多线程并行处理
- 📁 **灵活的目录配置** - 支持多个根目录、递归深度控制
- 🎬 **媒体文件过滤** - 自动识别视频、图片、音频文件
- 🔍 **文件哈希计算** - 支持完整 MD5 和大文件部分哈希
- 💾 **SQLite 存储** - 批量写入，支持增量扫描
- 📊 **JSON 输出** - 支持 JSON 格式输出扫描结果

## 安装

### 从 Release 下载

前往 [Releases](https://github.com/your-repo/media-scanner/releases) 下载对应平台的二进制文件：

- `media-scanner-linux-x86_64` - Linux x86_64
- `media-scanner-windows-x86_64.exe` - Windows x86_64
- `media-scanner-macos-arm64` - macOS Apple Silicon

### 从源码编译

```bash
git clone https://github.com/your-repo/media-scanner.git
cd media-scanner
cargo build --release
```

编译后的二进制文件位于 `target/release/media_scanner`

## 使用方法

### 基本命令

```bash
media_scanner scan --roots <目录路径>
```

### 命令行参数

| 参数 | 短参数 | 说明 | 默认值 |
|------|--------|------|--------|
| `--roots` | `-r` | 扫描的根目录（必需，可指定多个） | - |
| `--threads` | `-t` | 并行线程数（0 = 自动检测） | 0 |
| `--batch-size` | `-b` | 数据库批量写入大小 | 1000 |
| `--db` | `-d` | 数据库文件路径 | media_scanner.db |
| `--incremental` | `-i` | 执行增量扫描 | false |
| `--json` | - | 以 JSON 格式输出结果 | false |
| `--no-hash` | - | 跳过文件哈希计算 | false |
| `--no-recursive` | - | 禁用递归扫描（只扫描根目录） | false |
| `--max-depth` | - | 最大扫描深度 | 3 |

### 使用示例

#### 1. 扫描单个目录

```bash
media_scanner scan --roots /path/to/media
```

#### 2. 扫描多个目录

```bash
media_scanner scan --roots /path/to/videos --roots /path/to/photos
```

#### 3. 控制扫描深度

```bash
# 只扫描根目录下的文件（不递归）
media_scanner scan --roots /path/to/media --no-recursive

# 扫描 5 层深度
media_scanner scan --roots /path/to/media --max-depth 5

# 只扫描第一层子目录
media_scanner scan --roots /path/to/media --max-depth 1
```

#### 4. 指定数据库文件

```bash
media_scanner scan --roots /path/to/media --db /path/to/output.db
```

#### 5. 增量扫描（只处理变化的文件）

```bash
media_scanner scan --roots /path/to/media --incremental
```

#### 6. JSON 格式输出

```bash
media_scanner scan --roots /path/to/media --json
```

#### 7. 高性能扫描配置

```bash
# 使用 8 线程，批量写入 2000 条
media_scanner scan --roots /path/to/media --threads 8 --batch-size 2000
```

#### 8. 快速扫描（跳过哈希计算）

```bash
media_scanner scan --roots /path/to/media --no-hash
```

### 完整示例

```bash
# 扫描多个目录，深度 5 层，8 线程，输出到指定数据库
media_scanner scan \
  --roots /mnt/nas/videos \
  --roots /mnt/nas/photos \
  --max-depth 5 \
  --threads 8 \
  --batch-size 2000 \
  --db /data/media_index.db
```

## 支持的媒体格式

### 视频
mp4, mkv, avi, wmv, flv, mov, webm, m4v, ts, rmvb

### 图片
jpg, jpeg, png, gif, webp, bmp, tiff, tif

### 音频
mp3, flac, wav, aac, ogg, wma, m4a

## 输出格式

### 控制台输出

```
Scan completed:
  Total files: 12345
  Total dirs: 678
  New files: 100
  Modified files: 50
  Deleted files: 10
  Errors: 2
  Duration: 5432ms
```

### JSON 输出

```json
{
  "total_files": 12345,
  "total_dirs": 678,
  "new_files": 100,
  "modified_files": 50,
  "deleted_files": 10,
  "duration_ms": 5432
}
```

## 数据库结构

扫描结果存储在 SQLite 数据库中，主要表结构：

### scanned_files 表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 主键 |
| path | TEXT | 文件完整路径 |
| name | TEXT | 文件名 |
| size | INTEGER | 文件大小（字节） |
| mtime | INTEGER | 修改时间（Unix 时间戳） |
| ctime | INTEGER | 创建时间（Unix 时间戳） |
| extension | TEXT | 文件扩展名 |
| media_type | TEXT | 媒体类型（video/image/audio） |
| hash | TEXT | 文件哈希值 |
| is_partial_hash | INTEGER | 是否为部分哈希 |

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `RUST_LOG` | 日志级别 | info |

```bash
# 启用调试日志
RUST_LOG=debug media_scanner scan --roots /path/to/media
```

## 许可证

MIT License

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
| `--output` | `-o` | 输出结果到文件 | - |
| `--incremental` | `-i` | 执行增量扫描 | false |
| `--json` | - | 以 JSON 格式输出结果（完整） | false |
| `--ndjson` | - | 以 NDJSON 格式输出（每行一个文件） | false |
| `--compact` | - | 紧凑格式（按目录分组，字段缩写，推荐大量文件） | false |
| `--progress` | `-p` | 显示扫描进度（输出到stderr） | false |
| `--progress-interval` | - | 进度报告间隔（毫秒） | 200 |
| `--hash` | - | 启用文件哈希计算（默认不计算） | false |
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
# 首次扫描：记录所有文件到数据库
media_scanner scan --roots /path/to/media

# 后续扫描：只输出变化的文件（新增/修改/删除）
media_scanner scan --roots /path/to/media --incremental --compact
```

增量扫描工作原理：
1. 首次扫描时，所有文件信息保存到 SQLite 数据库
2. 后续扫描时，通过 size + mtime 快速判断文件是否变化
3. 只有变化的文件才重新计算 hash
4. 输出只包含：新增文件、修改文件、删除文件

#### 6. JSON 格式输出

```bash
# 完整 JSON（适合小量文件）
media_scanner scan --roots /path/to/media --json

# NDJSON 流式输出（每行一个文件）
media_scanner scan --roots /path/to/media --ndjson

# 紧凑格式（按目录分组，推荐大量文件 10万+）
media_scanner scan --roots /path/to/media --compact

# 输出到文件（避免 stdout 缓冲问题）
media_scanner scan --roots /path/to/media --compact -o result.ndjson

# 显示扫描进度（进度输出到stderr，不影响JSON）
media_scanner scan --roots /path/to/media --compact --progress

# 自定义进度报告间隔（毫秒）
media_scanner scan --roots /path/to/media --compact --progress --progress-interval 500
```

#### 7. 高性能扫描配置

```bash
# 使用 8 线程，批量写入 2000 条
media_scanner scan --roots /path/to/media --threads 8 --batch-size 2000
```

#### 8. 启用哈希计算

```bash
# 默认不计算哈希，使用 --hash 启用
media_scanner scan --roots /path/to/media --hash
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
扫描完成:
  媒体文件数: 12345
  目录数: 678
  新文件: 100
  修改文件: 50
  删除文件: 10
  错误数: 2
  耗时: 5432ms
```

### JSON 输出 (`--json`)

目录树结构的完整 JSON，支持 `-o` 输出到文件：

```bash
# 输出到控制台
media_scanner scan --roots /path/to/media --json

# 输出到文件
media_scanner scan --roots /path/to/media --json -o result.json
```

```json
{
  "summary": {
    "total_files": 12345,
    "total_dirs": 678,
    "new_files": 100,
    "modified_files": 50,
    "deleted_files": 10,
    "error_count": 2,
    "duration_ms": 5432
  },
  "directories": [
    {
      "path": "/media/videos/2024",
      "files": [
        {"n": "movie1.mp4", "s": 1234567890, "m": 1704067200, "t": "v", "h": "abc123"},
        {"n": "movie2.mkv", "s": 987654321, "m": 1704067300, "t": "v"}
      ]
    },
    {
      "path": "/media/photos",
      "files": [
        {"n": "photo1.jpg", "s": 2048000, "m": 1704067400, "t": "i"}
      ]
    }
  ],
  "deleted": ["/media/old/deleted.mp4"]
}
```

### NDJSON 输出 (`--ndjson`)

每行一个 JSON 对象，适合流式处理：

```
{"_type":"summary","total_files":12345,"total_dirs":678,"new_files":100,"modified_files":50,"deleted_files":10,"error_count":2,"duration_ms":5432}
{"name":"video1.mp4","size":1234567890,"mtime":1704067200,"ctime":1704067200,"extension":"mp4","media_type":"video","hash":"abc123..."}
{"name":"video2.mkv","size":987654321,"mtime":1704067300,"ctime":1704067300,"extension":"mkv","media_type":"video","hash":"def456..."}
```

### 紧凑格式输出 (`--compact`) - 推荐大量文件

按目录分组，字段使用缩写，大幅减少数据量：

```
{"_t":"s","tf":12345,"td":678,"nf":100,"ec":2,"ms":5432}
{"path":"/media/videos/2024","files":[{"n":"movie1.mp4","s":1234567890,"m":1704067200,"t":"v","h":"abc123"},{"n":"movie2.mkv","s":987654321,"m":1704067300,"t":"v"}]}
{"path":"/media/photos","files":[{"n":"photo1.jpg","s":2048000,"m":1704067400,"t":"i"},{"n":"photo2.png","s":1024000,"m":1704067500,"t":"i"}]}
```

**字段缩写说明：**
| 缩写 | 完整名称 | 说明 |
|------|----------|------|
| `_t` | type | 类型标识 (s=summary, d=deleted) |
| `tf` | total_files | 总文件数 |
| `td` | total_dirs | 总目录数 |
| `nf` | new_files | 新文件数 |
| `mf` | modified_files | 修改文件数 |
| `df` | deleted_files | 删除文件数 |
| `ec` | error_count | 错误数 |
| `ms` | duration_ms | 耗时(毫秒) |
| `n` | name | 文件名 |
| `s` | size | 文件大小 |
| `m` | mtime | 修改时间 |
| `t` | type | 媒体类型 (v/i/a/u) |
| `h` | hash | 文件哈希 |

**媒体类型缩写：** v=video, i=image, a=audio, u=unknown

**进度输出示例（stderr）：**
```
{"_t":"start","seq":0,"ts":1704067200000,"roots":["/media/videos","/media/photos"],"recursive":true,"max_depth":3,"compute_hash":false}
{"_t":"p","seq":1,"ts":1704067202500,"phase":"scan","f":1000,"d":50,"v":800,"i":150,"a":50,"dir":"/media/videos/2024","ms":2500}
{"_t":"err","seq":2,"ts":1704067203000,"error_type":"PermissionDenied","message":"Permission denied","path":"/media/private/secret.mp4"}
{"_t":"p","seq":3,"ts":1704067205000,"phase":"scan","f":2000,"d":100,"v":1600,"i":300,"a":100,"dir":"/media/photos","ms":5000}
{"_t":"p","seq":4,"ts":1704067207500,"phase":"process","f":3000,"d":150,"v":2400,"i":450,"a":150,"dir":"完成","ms":7500}
{"_t":"done","seq":5,"ts":1704067207600,"tf":3000,"td":150,"nf":100,"mf":50,"df":10,"ec":1,"ms":7600}
```

**进度消息类型说明：**
| 类型 | `_t` 值 | 说明 |
|------|---------|------|
| 开始消息 | `start` | 扫描开始时发送，包含配置信息 |
| 进度消息 | `p` | 定期发送的进度更新 |
| 错误消息 | `err` | 遇到错误时立即发送 |
| 完成消息 | `done` | 扫描完成时发送，包含最终统计 |

**开始消息字段说明 (`_t: "start"`)：**
| 字段 | 说明 |
|------|------|
| `seq` | 序列号（从0开始） |
| `ts` | 时间戳（毫秒） |
| `roots` | 扫描根目录列表 |
| `recursive` | 是否递归扫描 |
| `max_depth` | 最大扫描深度 |
| `compute_hash` | 是否计算哈希 |

**进度消息字段说明 (`_t: "p"`)：**
| 字段 | 说明 |
|------|------|
| `seq` | 序列号（单调递增） |
| `ts` | 时间戳（毫秒） |
| `phase` | 扫描阶段 (scan/process/done) |
| `f` | 已扫描文件数 |
| `d` | 已扫描目录数 |
| `v` | 视频文件数 |
| `i` | 图片文件数 |
| `a` | 音频文件数 |
| `dir` | 当前扫描目录 |
| `ms` | 已用时间(毫秒) |
| `eta_ms` | 预计剩余时间(毫秒，可选) |

**错误消息字段说明 (`_t: "err"`)：**
| 字段 | 说明 |
|------|------|
| `seq` | 序列号 |
| `ts` | 时间戳（毫秒） |
| `error_type` | 错误类型 |
| `message` | 错误描述 |
| `path` | 相关文件路径（可选） |

**完成消息字段说明 (`_t: "done"`)：**
| 字段 | 说明 |
|------|------|
| `seq` | 序列号 |
| `ts` | 时间戳（毫秒） |
| `tf` | 总文件数 |
| `td` | 总目录数 |
| `nf` | 新文件数 |
| `mf` | 修改文件数 |
| `df` | 删除文件数 |
| `ec` | 错误数 |
| `ms` | 总耗时(毫秒) |

**增量扫描输出示例：**
```
{"_t":"s","tf":12345,"td":678,"nf":5,"mf":3,"df":2,"ec":0,"ms":150}
{"path":"/media/videos/2024","files":[{"n":"new_movie.mp4","s":1234567890,"m":1704067200,"t":"v","h":"abc123"}]}
{"_t":"d","paths":["/media/videos/old/deleted1.mp4","/media/videos/old/deleted2.mkv"]}
```

### Python 读取示例

```python
import subprocess
import json
import os
import sys
import threading

def read_progress(process):
    """读取进度信息（从stderr）"""
    for line in process.stderr:
        try:
            data = json.loads(line.strip())
            msg_type = data.get('_t')
            
            if msg_type == 'start':
                roots = ', '.join(data['roots'])
                print(f"开始扫描: {roots} (递归={data['recursive']}, 深度={data['max_depth']}, 哈希={data['compute_hash']})", file=sys.stderr)
            
            elif msg_type == 'p':
                phase = data.get('phase', 'scan')
                eta = f", 预计剩余{data['eta_ms']}ms" if data.get('eta_ms') else ""
                print(f"\r[{phase}] {data['f']}文件, {data['v']}视频, {data['i']}图片, {data['a']}音频 - {data['dir'][:50]}{eta}", end='', file=sys.stderr)
            
            elif msg_type == 'err':
                path_info = f" ({data['path']})" if data.get('path') else ""
                print(f"\n错误: {data['error_type']} - {data['message']}{path_info}", file=sys.stderr)
            
            elif msg_type == 'done':
                print(f"\n扫描完成: 共{data['tf']}文件, {data['td']}目录, 耗时{data['ms']}ms, 错误{data['ec']}个", file=sys.stderr)
        except:
            pass

# 方式1: 带进度显示的扫描
process = subprocess.Popen(
    ['media_scanner', 'scan', '-r', '/path/to/media', '--compact', '--progress'],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

# 启动进度读取线程
progress_thread = threading.Thread(target=read_progress, args=(process,))
progress_thread.start()

# 读取结果（从stdout）
added_files = []
deleted_files = []

for line in process.stdout:
    data = json.loads(line.strip())
    
    if data.get('_t') == 's':
        print(f"\n扫描完成: 新增{data['nf']}, 修改{data['mf']}, 删除{data['df']}")
    elif data.get('_t') == 'd':
        deleted_files.extend(data['paths'])
    elif 'path' in data:
        dir_path = data['path']
        for f in data['files']:
            added_files.append(os.path.join(dir_path, f['n']))

progress_thread.join()
process.wait()

# 方式2: 简单增量扫描（无进度）
result = subprocess.run(
    ['media_scanner', 'scan', '-r', '/path/to/media', '--incremental', '--compact'],
    capture_output=True, text=True
)

for line in result.stdout.strip().split('\n'):
    data = json.loads(line)
    if data.get('_t') == 's':
        print(f"新增: {data['nf']}, 修改: {data['mf']}, 删除: {data['df']}")

# 方式3: 自定义进度间隔
process = subprocess.Popen(
    ['media_scanner', 'scan', '-r', '/path/to/media', '--compact', '--progress', '--progress-interval', '500'],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)
# ... 同方式1处理
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

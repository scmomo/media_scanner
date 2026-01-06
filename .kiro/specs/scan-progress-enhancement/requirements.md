# Requirements Document

## Introduction

本文档定义了媒体扫描器（Media Scanner）的扫描进度增强功能需求。该功能旨在：
1. 默认跳过文件哈希计算，仅获取文件基本信息，提升扫描速度
2. 增强扫描进度反馈机制，为外部调用程序提供更详细、更及时的进度信息，防止用户在扫描大量文件时误以为程序假死或崩溃

## Glossary

- **Scanner**: 媒体文件扫描器核心模块，负责遍历目录并收集文件信息
- **Progress_Reporter**: 进度报告组件，负责将扫描进度信息输出到 stderr
- **External_Caller**: 调用本 CLI 程序的外部程序（如 Python 脚本、GUI 应用等）
- **Progress_Message**: JSON 格式的进度消息，输出到 stderr 供外部程序解析
- **Heartbeat**: 定期发送的心跳消息，表明程序仍在正常运行

## Requirements

### Requirement 1: 默认跳过哈希计算

**User Story:** 作为用户，我希望扫描程序默认不计算文件哈希，以便更快地完成扫描任务。

#### Acceptance Criteria

1. THE Scanner SHALL default to skipping hash computation when scanning files
2. WHEN the `--hash` flag is provided, THE Scanner SHALL compute file hashes
3. THE Scanner SHALL remove the `--no-hash` flag as it becomes the default behavior
4. WHEN hash computation is disabled, THE ScannedFile SHALL have hash field set to None

### Requirement 2: 增强进度报告频率

**User Story:** 作为外部调用程序的开发者，我希望获得更频繁的进度更新，以便及时向用户展示扫描状态。

#### Acceptance Criteria

1. WHEN progress reporting is enabled, THE Progress_Reporter SHALL output progress messages at configurable intervals
2. THE Scanner SHALL support a `--progress-interval` parameter to configure progress reporting interval in milliseconds
3. WHEN `--progress-interval` is not specified, THE Progress_Reporter SHALL default to 200ms interval
4. THE Progress_Reporter SHALL output progress to stderr to avoid interfering with stdout JSON output

### Requirement 3: 心跳机制

**User Story:** 作为外部调用程序的开发者，我希望即使在处理单个大目录时也能收到定期更新，以便确认程序仍在运行。

#### Acceptance Criteria

1. WHEN progress reporting is enabled, THE Progress_Reporter SHALL send heartbeat messages even when no new files are found
2. THE Progress_Message SHALL include a timestamp field to indicate message freshness
3. WHEN scanning a directory with many files, THE Progress_Reporter SHALL report progress after processing each batch of files
4. THE Progress_Reporter SHALL include current operation description in heartbeat messages

### Requirement 4: 详细进度信息

**User Story:** 作为外部调用程序的开发者，我希望进度消息包含更详细的信息，以便准确展示扫描状态。

#### Acceptance Criteria

1. THE Progress_Message SHALL include total files scanned count
2. THE Progress_Message SHALL include total directories scanned count
3. THE Progress_Message SHALL include current directory path being scanned
4. THE Progress_Message SHALL include elapsed time in milliseconds
5. THE Progress_Message SHALL include estimated remaining time when possible
6. THE Progress_Message SHALL include scan phase indicator (scanning/processing/completing)
7. THE Progress_Message SHALL include media type breakdown (video/image/audio counts)

### Requirement 5: 进度消息格式

**User Story:** 作为外部调用程序的开发者，我希望进度消息格式清晰且易于解析，以便在各种编程语言中处理。

#### Acceptance Criteria

1. THE Progress_Message SHALL be formatted as valid JSON
2. THE Progress_Message SHALL use the type identifier `_t: "p"` for progress messages
3. THE Progress_Message SHALL include a monotonically increasing sequence number
4. WHEN serializing Progress_Message, THE Progress_Reporter SHALL output one complete JSON object per line
5. THE Progress_Message SHALL include a `phase` field with values: "scan", "process", or "done"

### Requirement 6: 扫描开始和结束通知

**User Story:** 作为外部调用程序的开发者，我希望明确知道扫描何时开始和结束，以便正确管理程序生命周期。

#### Acceptance Criteria

1. WHEN scan starts, THE Progress_Reporter SHALL output a start message with type `_t: "start"`
2. THE start message SHALL include scan configuration summary (roots, recursive, max_depth)
3. WHEN scan completes, THE Progress_Reporter SHALL output a completion message with type `_t: "done"`
4. THE completion message SHALL include final statistics (total files, dirs, duration, errors)
5. IF an error causes scan to abort, THE Progress_Reporter SHALL output an error message with type `_t: "error"`

### Requirement 7: 错误报告增强

**User Story:** 作为外部调用程序的开发者，我希望在扫描过程中及时获知错误信息，以便向用户展示问题。

#### Acceptance Criteria

1. WHEN a scan error occurs, THE Progress_Reporter SHALL immediately output an error progress message
2. THE error Progress_Message SHALL include error type and description
3. THE error Progress_Message SHALL include the path that caused the error when available
4. THE Progress_Reporter SHALL continue scanning after non-fatal errors and report them in progress

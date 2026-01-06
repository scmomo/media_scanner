# Implementation Plan: Scan Progress Enhancement

## Overview

本实现计划将扫描进度增强功能分解为可执行的编码任务。按照配置修改 → 进度组件 → 扫描器集成 → CLI 更新的顺序实现。

## Tasks

- [x] 1. 修改 ScanConfig 默认配置
  - [x] 1.1 修改 compute_hash 默认值为 false
    - 在 `src/config.rs` 中修改 `Default` 实现
    - 将 `compute_hash: true` 改为 `compute_hash: false`
    - _Requirements: 1.1_
  - [x] 1.2 添加 progress_interval_ms 配置字段
    - 添加 `progress_interval_ms: u64` 字段，默认值 200
    - 添加常量 `DEFAULT_PROGRESS_INTERVAL_MS: u64 = 200`
    - 在 builder 中添加 `progress_interval_ms()` 方法
    - _Requirements: 2.2, 2.3_
  - [x] 1.3 编写配置默认值属性测试
    - **Property 1: Default hash computation disabled**
    - **Validates: Requirements 1.1**

- [x] 2. 创建进度消息数据结构
  - [x] 2.1 创建 src/progress.rs 模块
    - 定义 `ScanPhase` 枚举 (Scan, Process, Done)
    - 定义 `StartMessage` 结构体
    - 定义 `ProgressMessage` 结构体
    - 定义 `ErrorProgressMessage` 结构体
    - 定义 `DoneMessage` 结构体
    - 实现 Serialize trait
    - _Requirements: 5.1, 5.2, 5.5_
  - [ ] 2.2 编写消息结构属性测试
    - **Property 3: Progress message structure validity**
    - **Property 6: Start message structure**
    - **Property 7: Done message structure**
    - **Property 8: Error message structure**
    - **Validates: Requirements 3.2, 3.4, 4.1-4.7, 5.1, 5.2, 5.5, 6.2, 6.4, 7.2, 7.3**

- [x] 3. 实现 ProgressReporter 组件
  - [x] 3.1 实现 ProgressReporter 核心功能
    - 实现 `new()` 构造函数
    - 实现 `should_report()` 时间间隔检查
    - 实现 `next_seq()` 序列号生成
    - 实现 `current_timestamp()` 时间戳获取
    - 实现 `output_to_stderr()` 输出方法
    - _Requirements: 2.1, 2.4, 5.3_
  - [x] 3.2 实现消息报告方法
    - 实现 `report_start()` 方法
    - 实现 `report_progress()` 方法
    - 实现 `report_error()` 方法
    - 实现 `report_done()` 方法
    - _Requirements: 6.1, 6.3, 7.1_
  - [ ]* 3.3 编写序列号单调性属性测试
    - **Property 5: Sequence number monotonicity**
    - **Validates: Requirements 5.3**

- [x] 4. Checkpoint - 确保进度组件测试通过
  - 运行 `cargo test` 确保所有测试通过
  - 如有问题请询问用户

- [x] 5. 集成 ProgressReporter 到 Scanner
  - [x] 5.1 修改 scan_internal 函数
    - 创建 ProgressReporter 实例
    - 在扫描开始时调用 report_start()
    - 在处理文件时调用 report_progress()
    - 在遇到错误时调用 report_error()
    - 在扫描结束时调用 report_done()
    - _Requirements: 3.1, 3.3, 6.1, 6.3, 7.1, 7.4_
  - [x] 5.2 修改 ScanProgress 结构
    - 添加 `phase: ScanPhase` 字段
    - 添加 `estimated_total: Option<u64>` 字段
    - 实现 `estimated_remaining_ms()` 方法
    - _Requirements: 4.5, 4.6_
  - [ ]* 5.3 编写哈希禁用属性测试
    - **Property 2: Hash field None when disabled**
    - **Validates: Requirements 1.4**

- [x] 6. 更新 CLI 参数
  - [x] 6.1 修改 main.rs 命令行参数
    - 移除 `--no-hash` 参数
    - 添加 `--hash` 参数启用哈希计算
    - 添加 `--progress-interval` 参数
    - 更新帮助文档
    - _Requirements: 1.2, 1.3, 2.2_
  - [x] 6.2 更新配置构建逻辑
    - 使用 `--hash` 参数设置 `compute_hash`
    - 使用 `--progress-interval` 参数设置 `progress_interval_ms`
    - _Requirements: 1.2, 2.2_

- [x] 7. 更新 lib.rs 导出
  - 导出 progress 模块
  - 导出 ProgressReporter, ScanPhase 等类型
  - _Requirements: N/A (代码组织)_

- [x] 8. 更新 README 文档
  - [x] 8.1 更新命令行参数说明
    - 移除 `--no-hash` 说明
    - 添加 `--hash` 说明
    - 添加 `--progress-interval` 说明
    - _Requirements: N/A (文档)_
  - [x] 8.2 更新进度输出格式说明
    - 添加新消息类型说明 (start, err, done)
    - 更新字段说明表格
    - 更新 Python 示例代码
    - _Requirements: N/A (文档)_

- [x] 9. Final Checkpoint - 确保所有测试通过
  - 运行 `cargo test` 确保所有测试通过
  - 运行 `cargo clippy` 检查代码质量
  - 如有问题请询问用户

## Notes

- 任务标记 `*` 的为可选测试任务，可跳过以加快 MVP 开发
- 每个任务引用具体的需求条款以便追溯
- Checkpoint 任务用于验证阶段性成果
- 属性测试使用 proptest 库，配置最少 100 次迭代

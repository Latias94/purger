# Rust Project Purger

一个用于清理Rust项目构建目录的工具，支持CLI和GUI两种界面。

## 功能特性

- 🔍 **智能扫描**: 递归扫描目录下的所有Rust项目
- 🧹 **多种清理策略**: 支持`cargo clean`和直接删除两种方式
- 📊 **大小统计**: 显示每个项目的target目录大小
- ✅ **选择性清理**: 可以选择要清理的特定项目
- 🖥️ **双界面**: 提供命令行和图形界面两种使用方式
- 🚀 **并行处理**: 支持并行扫描和清理以提高效率
- ⏰ **时间过滤**: 保留最近编译的项目，避免清理正在使用的项目
- 📏 **大小过滤**: 保留小于指定大小的target目录
- 📁 **路径忽略**: 支持忽略特定目录，避免误删重要项目
- 💾 **可执行文件备份**: 在清理前自动备份编译好的可执行文件

## 项目结构

```
purger/
├── purger-core/     # 核心逻辑库
├── purger-cli/      # 命令行界面
├── purger-gui/      # 图形界面
└── README.md
```

## 安装

[![Crates.io](https://img.shields.io/crates/v/purger-cli.svg)](https://crates.io/crates/purger-cli)
[![Crates.io](https://img.shields.io/crates/v/purger-gui.svg)](https://crates.io/crates/purger-gui)
[![Crates.io](https://img.shields.io/crates/v/purger-core.svg)](https://crates.io/crates/purger-core)

### 从 crates.io 安装 (推荐)

```bash
# 安装命令行版本
cargo install purger-cli

# 安装图形界面版本
cargo install purger-gui

# 或者安装完整包
cargo install purger
```

### 从源码编译

```bash
# 安装 CLI 版本
cargo install purger

# 安装包含 GUI 的完整版本
cargo install purger --features gui
```

安装后可以直接使用：
```bash
# CLI 版本
purger scan
purger clean --all

# GUI 版本 (如果安装了 gui 功能)
purger-gui
```

### 从源码构建

```bash
git clone <repository-url>
cd purger
cargo build --release
```

构建完成后，可执行文件位于：
- CLI版本: `target/release/purger`
- GUI版本: `target/release/purger-gui`

## 使用方法

### 命令行界面 (CLI)

#### 扫描项目
```bash
# 扫描当前目录
purger scan

# 扫描指定目录
purger scan /path/to/projects

# 只显示有target目录的项目
purger scan --target-only

# 按大小排序
purger scan --sort-by-size

# 限制扫描深度
purger scan --max-depth 5

# 使用过滤选项
purger scan --keep-days 7 --keep-size 10MB --ignore ~/Downloads
```

#### 清理项目
```bash
# 清理当前目录下的所有项目
purger clean

# 预览清理操作（不实际删除）
purger clean --dry-run

# 使用直接删除策略
purger clean --strategy delete

# 跳过确认提示
purger clean --yes

# 清理指定目录
purger clean /path/to/projects

# 使用过滤和备份选项
purger clean --keep-days 7 --keep-size 10MB --keep-executable --ignore ~/Downloads
```

#### 新增命令行选项

**扫描过滤选项:**
- `--keep-days <DAYS>`: 保留最近N天编译的项目
- `--keep-size <SIZE>`: 保留target目录小于指定大小的项目 (如: 10MB, 1GB)
- `--ignore <PATH>`: 忽略特定目录 (可多次使用)

**清理选项:**
- `--keep-executable`: 保留可执行文件 (自动备份)
- `--executable-backup-dir <DIR>`: 指定可执行文件备份目录

**通用选项:**
- `--verbose, -v`: 显示详细日志
- `--debug, -d`: 显示调试日志
- `--help, -h`: 显示帮助信息

### 图形界面 (GUI)

运行GUI版本：
```bash
purger-gui
```

GUI功能：
- 设置扫描路径和深度
- 实时显示扫描进度
- 可视化项目列表和大小
- 选择性清理项目
- 显示清理结果统计
- 过滤选项配置 (时间、大小、忽略路径)
- 可执行文件备份设置
- 多语言支持

## 清理策略

### Cargo Clean (推荐)
- 使用`cargo clean`命令清理
- 更安全，只删除Cargo生成的构建产物
- 保留用户手动放入target的文件
- 正确处理工作空间依赖

### 直接删除
- 直接删除整个target目录
- 速度更快，但可能删除非Cargo文件
- 适用于确定target目录只包含构建产物的情况

## 配置

### 扫描配置
- `max_depth`: 最大扫描深度（默认10层）
- `follow_links`: 是否跟随符号链接（默认false）
- `respect_gitignore`: 是否遵循.gitignore规则（默认true）
- `ignore_hidden`: 是否忽略隐藏文件（默认true）
- `keep_days`: 保留最近N天编译的项目（可选）
- `keep_size`: 保留小于指定大小的target目录（可选）
- `ignore_paths`: 忽略的路径列表（可选）

### 清理配置
- `strategy`: 清理策略（cargo/delete）
- `dry_run`: 预览模式（默认false）
- `parallel`: 并行处理（默认true）
- `keep_executable`: 是否保留可执行文件（默认false）
- `executable_backup_dir`: 可执行文件备份目录（可选）

## 开发

### 运行测试
```bash
cargo test
```

### 运行CLI版本
```bash
cargo run --bin purger -- scan
```

### 运行GUI版本
```bash
cargo run --bin purger-gui
```

### 代码结构

#### purger-core
核心功能库，包含：
- `ProjectScanner`: 项目扫描器
- `ProjectCleaner`: 项目清理器
- `ProjectFilter`: 项目过滤器
- `RustProject`: 项目信息结构
- 工具函数和类型定义

#### purger-cli
命令行界面，使用clap进行参数解析。

#### purger-gui
图形界面，使用egui框架构建。

## 依赖

主要依赖：
- `walkdir`: 目录遍历
- `ignore`: .gitignore支持
- `clap`: 命令行参数解析
- `egui/eframe`: GUI框架
- `tokio`: 异步运行时
- `rayon`: 并行处理
- `anyhow`: 错误处理

## 许可证

MIT OR Apache-2.0

## 贡献

欢迎提交Issue和Pull Request！

## 注意事项

- 清理操作不可逆，请谨慎使用
- 建议先使用`--dry-run`预览清理结果
- 大型项目清理可能需要较长时间
- 确保有足够的权限访问目标目录

use criterion::{Criterion, criterion_group, criterion_main};
use purger_core::{
    ProjectCleaner, ProjectScanner,
    cleaner::{CleanConfig, CleanStrategy},
    scanner::ScanConfig,
};
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;

/// 创建一个测试用的Rust项目
fn create_test_project(
    base_path: &std::path::Path,
    name: &str,
    with_target: bool,
) -> anyhow::Result<()> {
    let project_path = base_path.join(name);
    fs::create_dir_all(&project_path)?;

    // 创建Cargo.toml
    let cargo_toml = format!(
        r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#
    );
    fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

    // 创建src/main.rs
    let src_dir = project_path.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(
        src_dir.join("main.rs"),
        r#"use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    value: i32,
}

fn main() {
    println!("Hello, world!");
    let config = Config {
        name: "test".to_string(),
        value: 42,
    };
    println!("{:?}", config);
}"#,
    )?;

    // 如果需要，创建target目录
    if with_target {
        let target_dir = project_path.join("target");
        fs::create_dir_all(&target_dir)?;

        // 创建一些假的编译产物
        let debug_dir = target_dir.join("debug");
        fs::create_dir_all(&debug_dir)?;

        // 创建可执行文件
        fs::write(debug_dir.join(name), "fake executable")?;

        // 创建依赖文件
        let deps_dir = debug_dir.join("deps");
        fs::create_dir_all(&deps_dir)?;

        // 创建多个依赖文件来模拟真实的编译产物
        for i in 0..10 {
            let content = "x".repeat(1024 * (i + 1)); // 1KB到10KB的文件
            fs::write(deps_dir.join(format!("lib{i}.rlib")), &content)?;
        }

        // 创建增量编译缓存
        let incremental_dir = debug_dir.join("incremental");
        fs::create_dir_all(&incremental_dir)?;
        for i in 0..5 {
            let cache_content = "cache".repeat(512); // 2KB缓存文件
            fs::write(
                incremental_dir.join(format!("cache_{i}.bin")),
                &cache_content,
            )?;
        }
    }

    Ok(())
}

/// 创建多个测试项目
fn create_multiple_projects(base_path: &std::path::Path, count: usize) -> anyhow::Result<()> {
    for i in 0..count {
        create_test_project(base_path, &format!("project_{i:03}"), true)?;
    }
    Ok(())
}

/// 基准测试：扫描小量项目（10个）
fn bench_scan_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 10).unwrap();

    let scanner = ProjectScanner::default();

    c.bench_function("scan_10_projects", |b| {
        b.iter(|| {
            let projects = scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });
}

/// 基准测试：扫描中等数量项目（50个）
fn bench_scan_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 50).unwrap();

    let scanner = ProjectScanner::default();

    c.bench_function("scan_50_projects", |b| {
        b.iter(|| {
            let projects = scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });
}

/// 基准测试：扫描大量项目（100个）
fn bench_scan_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 100).unwrap();

    let scanner = ProjectScanner::default();

    c.bench_function("scan_100_projects", |b| {
        b.iter(|| {
            let projects = scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });
}

/// 基准测试：并行vs串行扫描
fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 50).unwrap();

    let parallel_config = ScanConfig {
        parallel: true,
        ..Default::default()
    };
    let sequential_config = ScanConfig {
        parallel: false,
        ..Default::default()
    };

    let parallel_scanner = ProjectScanner::new(parallel_config);
    let sequential_scanner = ProjectScanner::new(sequential_config);

    let mut group = c.benchmark_group("scan_parallel_vs_sequential");

    group.bench_function("parallel", |b| {
        b.iter(|| {
            let projects = parallel_scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });

    group.bench_function("sequential", |b| {
        b.iter(|| {
            let projects = sequential_scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });

    group.finish();
}

/// 基准测试：清理性能（dry run）
fn bench_clean_dry_run(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 20).unwrap();

    let scanner = ProjectScanner::default();
    let projects = scanner.scan(temp_dir.path()).unwrap();
    let projects_with_target = ProjectScanner::filter_with_target(projects);

    let clean_config = CleanConfig {
        strategy: CleanStrategy::DirectDelete,
        dry_run: true,
        ..Default::default()
    };
    let cleaner = ProjectCleaner::new(clean_config);

    c.bench_function("clean_20_projects_dry_run", |b| {
        b.iter(|| {
            let result = cleaner.clean_projects(black_box(&projects_with_target));
            black_box(result);
        })
    });
}

/// 基准测试：过滤器性能
fn bench_filter_performance(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_multiple_projects(temp_dir.path(), 100).unwrap();

    let config = ScanConfig {
        keep_days: Some(7),
        keep_size: Some(1024 * 1024), // 1MB
        ..Default::default()
    };

    let scanner = ProjectScanner::new(config.clone());
    let projects = scanner.scan(temp_dir.path()).unwrap();

    let filter = purger_core::ProjectFilter::new(config);

    c.bench_function("filter_100_projects", |b| {
        b.iter(|| {
            let filtered = filter.filter_projects(black_box(projects.clone()));
            black_box(filtered);
        })
    });
}

/// 基准测试：深度扫描性能
fn bench_deep_scan(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut current_path = temp_dir.path().to_path_buf();

    // 创建深层嵌套结构
    for i in 0..10 {
        current_path = current_path.join(format!("level_{i}"));
        fs::create_dir_all(&current_path).unwrap();

        // 在每一层创建一个项目
        create_test_project(&current_path, &format!("project_level_{i}"), true).unwrap();
    }

    let config = ScanConfig {
        max_depth: Some(15),
        ..Default::default()
    };
    let scanner = ProjectScanner::new(config);

    c.bench_function("scan_deep_nested_projects", |b| {
        b.iter(|| {
            let projects = scanner.scan(black_box(temp_dir.path())).unwrap();
            black_box(projects);
        })
    });
}

criterion_group!(
    benches,
    bench_scan_small,
    bench_scan_medium,
    bench_scan_large,
    bench_parallel_vs_sequential,
    bench_clean_dry_run,
    bench_filter_performance,
    bench_deep_scan
);
criterion_main!(benches);

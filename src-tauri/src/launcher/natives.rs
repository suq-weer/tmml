//! native 依赖定位与解压。
//!
//! 原生库在 `<version>.json` 中可能有两种形态：
//! - 老格式：库对象里携带 `downloads.classifiers` 字典（键如 natives-linux / natives-linux-arm64）；
//! - 新格式：原生库本身是独立的库条目，名称以 `:natives-<os>[<-arch>]` 结尾。
//!
//! 两者都依据当前平台 + 架构（架构优先）挑选，随后把选中的原生 jar 解压到
//! versions/<dir>/natives 供 -Djava.library.path 使用。解压过程会返回可读日志行，
//! 由调用方写入「Island」日志视图。

use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use zip::ZipArchive;

use crate::{
    downloader::deserializer::VersionContent,
    platform::{features_default, native_classifier_candidates, rules_allow},
};

/// 需要解压的单个原生 jar
#[derive(Debug)]
pub struct NativeTask {
    pub jar_path: PathBuf,
    /// 用于日志展示的来源描述（maven 相对路径）
    pub source: String,
}

/// 依据 <version>.json 收集应当解压的原生 jar（按架构优先级挑选候选键）
pub fn resolve_native_tasks(content: &VersionContent, libraries_root: &Path) -> Vec<NativeTask> {
    let candidates = native_classifier_candidates();
    let mut seen: HashSet<String> = HashSet::new();
    let mut tasks = Vec::new();

    for lib in &content.libraries {
        // 与下载器一致：rules 不命中当前平台的原生库不下/不解压
        if !rules_allow(lib.rules.as_deref(), &features_default(), true) {
            continue;
        }

        let picked: Option<&str> = if let Some(classifiers) = &lib.classifiers {
            // 老格式：在分类器字典中按“架构优先”的顺序挑键
            candidates
                .iter()
                .find_map(|key| classifiers.get(key).map(|a| a.path.as_str()))
        } else {
            // 新格式：独立原生库条目（name 形如 group:artifact:ver:natives-linux）
            let token = lib.name.rsplit(':').next().unwrap_or("");
            if candidates.iter().any(|c| c == token) {
                Some(lib.downloads.artifact.path.as_str())
            } else {
                None
            }
        };

        if let Some(path) = picked {
            if seen.insert(path.to_string()) {
                tasks.push(NativeTask {
                    jar_path: libraries_root.join(path),
                    source: path.to_string(),
                });
            }
        }
    }
    tasks
}

/// 把所有原生 jar 解压到 natives_dir，返回（解压文件数，日志行）
pub fn extract_natives(tasks: &[NativeTask], natives_dir: &Path) -> Result<(usize, Vec<String>)> {
    let mut logs = Vec::new();
    let mut total = 0usize;

    if natives_dir.exists() {
        logs.push(format!("原生库目录已就绪：{}", natives_dir.display()));
        return Ok((0, logs));
    }
    std::fs::create_dir_all(natives_dir)
        .with_context(|| format!("创建原生库目录失败：{}", natives_dir.display()))?;

    for task in tasks {
        if !task.jar_path.exists() {
            logs.push(format!("跳过缺失的原生 jar：{}", task.source));
            continue;
        }
        logs.push(format!("正在解压 {} → natives/", task.source));
        let file = File::open(&task.jar_path)
            .with_context(|| format!("打开原生 jar 失败：{}", task.jar_path.display()))?;
        let mut archive =
            ZipArchive::new(file).with_context(|| format!("解析 zip 失败：{}", task.source))?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .with_context(|| format!("读取 zip 条目失败：{}", task.source))?;
            if entry.is_dir() {
                continue;
            }
            let base = entry
                .name()
                .rsplit('/')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let upper = base.to_ascii_uppercase();
            if base.is_empty()
                || base.starts_with('.')
                || upper.starts_with("META-INF")
                || upper.starts_with("__MACOSX")
            {
                continue;
            }
            let lower = base.to_ascii_lowercase();
            if lower.ends_with(".sf") || lower.ends_with(".rsa") || lower.ends_with(".dsa") {
                // 跳过 jar 签名文件
                continue;
            }
            let dest = natives_dir.join(&base);
            let mut out = File::create(&dest)
                .with_context(|| format!("创建文件失败：{}", dest.display()))?;
            std::io::copy(&mut entry, &mut out)
                .with_context(|| format!("解压文件失败：{}", dest.display()))?;
            total += 1;
            logs.push(format!("  释放 {}（来自 {}）", base, task.source));
        }
    }

    logs.push(format!(
        "原生库解压完成：共 {} 个文件 → {}",
        total,
        natives_dir.display()
    ));
    Ok((total, logs))
}

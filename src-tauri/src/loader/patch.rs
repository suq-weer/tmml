//! 加载器 Patch 数据与合并规则。
//!
//! `VersionPatch` 描述「在纯净原版之上要替换/追加什么」。合并时只改
//! mainClass / libraries / arguments / javaVersion，不动 assets、downloads、
//! logging 等资源元数据，确保启动器其它读取路径无需感知加载器存在。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::downloader::deserializer::{Arguments, OnceLibraries, StringOrArgument, VersionContent};

/// 由加载器安装产物生成的启动补丁
#[derive(Debug, Clone)]
pub struct VersionPatch {
    pub main_class: String,
    pub libraries: Vec<OnceLibraries>,
    pub jvm_args: Vec<StringOrArgument>,
    pub game_args: Vec<StringOrArgument>,
    /// 加载器要求的 Java 主版本（可选）
    pub java_major: Option<u64>,
}

/// 合并两段参数表：**只追加不去重**。
///
/// 参数是「标志 + 值」的成对序列，不能按值去重——例如两条 `--add-opens` 各自带一个值，
/// 若把重复的标志去掉，剩下的值会失去所属标志，被 JVM 误当成主类。
fn merge_args(base: &[StringOrArgument], extra: &[StringOrArgument]) -> Vec<StringOrArgument> {
    base.iter().chain(extra.iter()).cloned().collect()
}

/// 合并 libraries：按 name 去重，base 的条目优先（其携带更完整的原生库信息）
fn merge_libraries(base: &[OnceLibraries], extra: &[OnceLibraries]) -> Vec<OnceLibraries> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for lib in base.iter().chain(extra.iter()) {
        if seen.insert(lib.name.clone()) {
            out.push(lib.clone());
        }
    }
    out
}

/// 将 patch 应用到原版 version JSON，返回新的 VersionContent（未落盘）
pub fn merge(base: &VersionContent, patch: &VersionPatch) -> VersionContent {
    let mut merged = base.clone();
    merged.main_class = patch.main_class.clone();
    merged.libraries = merge_libraries(&base.libraries, &patch.libraries);
    merged.arguments = Arguments {
        game: merge_args(&base.arguments.game, &patch.game_args),
        jvm: merge_args(&base.arguments.jvm, &patch.jvm_args),
    };
    if let Some(major) = patch.java_major {
        if major > merged.java_version.major_version {
            merged.java_version.major_version = major;
        }
    }
    merged
}

/// 先备份原版，再把合并体写回 `game_dir/<version_id>.json`。
/// 返回 (合并体路径, 原版备份路径)。
pub fn persist(
    game_dir: &Path,
    version_id: &str,
    base: &VersionContent,
    merged: &VersionContent,
) -> Result<(PathBuf, PathBuf)> {
    let main_path = game_dir.join(format!("{}.json", version_id));
    let backup_path = game_dir.join(format!("{}.vanilla.json", version_id));

    // 仅在不存在时备份，避免多次安装把“合并后的内容”当成原版
    if !backup_path.exists() {
        let data = serde_json::to_string_pretty(base).context("序列化原版 version JSON 失败")?;
        std::fs::write(&backup_path, data).context("写原版备份失败")?;
    }

    let data = serde_json::to_string_pretty(merged).context("序列化合并后 version JSON 失败")?;
    std::fs::write(&main_path, data).context("写合并后 version JSON 失败")?;
    Ok((main_path, backup_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloader::deserializer::{
        Artifact, DownloadsFile, JavaVersion, LibrariesDownloads,
    };

    fn dummy_base() -> VersionContent {
        serde_json::from_str(
            r#"{
              "arguments": { "game": ["--username", "${auth_player_name}"], "jvm": ["-Xmx1G", "-cp", "${classpath}"] },
              "assetIndex": { "id": "1", "sha1": "a", "size": 1, "totalSize": 1, "url": "u" },
              "assets": "1",
              "complianceLevel": 1,
              "downloads": { "client": { "sha1": "s", "size": 1, "url": "u" } },
              "id": "1.21.1",
              "javaVersion": { "component": "java-runtime-alpha", "majorVersion": 21 },
              "libraries": [{ "name": "a:b:1", "downloads": { "artifact": { "path": "a/b/1/b-1.jar", "sha1": "", "size": 0, "url": "" } } }],
              "logging": { "client": { "argument": "-x", "file": { "id": "i", "sha1": "s", "size": 1, "url": "u" }, "type": "log4j2" } },
              "mainClass": "net.minecraft.client.main.Main",
              "minimumLauncherVersion": 21,
              "releaseTime": "t",
              "time": "t",
              "type": "release"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn merge_preserves_repeated_option_flags() {
        // 模拟 base 带一个 --add-opens、extra 带两个 --add-opens 的真实场景
        let jvm = |flags: Vec<StringOrArgument>| flags;
        let base_args = jvm(vec![
            StringOrArgument::String("--add-opens".into()),
            StringOrArgument::String("java.base/java.util.jar=mod".into()),
        ]);
        let extra_args = jvm(vec![
            StringOrArgument::String("--add-opens".into()),
            StringOrArgument::String("java.base/java.util.jar=mod".into()),
            StringOrArgument::String("--add-opens".into()),
            StringOrArgument::String("java.base/java.lang.invoke=mod".into()),
        ]);
        let merged = merge_args(&base_args, &extra_args);
        // 每个值必须紧跟在属于自己的标志之后，不允许丢标志
        let flags: Vec<&StringOrArgument> = merged
            .iter()
            .filter(|u| matches!(u, StringOrArgument::String(s) if s == "--add-opens"))
            .collect();
        assert_eq!(flags.len(), 3);
        // 三条标志后都必须跟一个“以 java. 开头的值”
        for (idx, unit) in merged.iter().enumerate() {
            if matches!(unit, StringOrArgument::String(s) if s == "--add-opens") {
                let value = &merged[idx + 1];
                assert!(matches!(
                    value,
                    StringOrArgument::String(v) if v.starts_with("java.")
                ));
            }
        }
    }

    #[test]
    fn merge_overrides_main_and_keeps_base() {
        let base = dummy_base();
        let lib = OnceLibraries {
            name: "net.fabricmc:fabric-loader:0.16.14".into(),
            rules: None,
            downloads: LibrariesDownloads {
                artifact: Artifact {
                    path: "net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar".into(),
                    sha1: String::new(),
                    size: 0,
                    url: String::new(),
                },
            },
            classifiers: None,
        };
        let patch = VersionPatch {
            main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".into(),
            libraries: vec![lib, base.libraries[0].clone()],
            jvm_args: vec![],
            game_args: vec![StringOrArgument::String("--tweakClass".into())],
            java_major: None,
        };
        let merged = merge(&base, &patch);
        assert_eq!(
            merged.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        // base 库 + loader 库，重复的 a:b:1 只保留一份
        assert_eq!(merged.libraries.len(), 2);
        assert_eq!(merged.libraries[0].name, "a:b:1");
        // 游戏参数被追加
        assert!(merged
            .arguments
            .game
            .iter()
            .any(|u| matches!(u, StringOrArgument::String(s) if s == "--tweakClass")));
        // 原版下载字段不变
        assert!(merged.downloads.client.is_some());
        let _ = DownloadsFile {
            sha1: "s".into(),
            size: 1,
            url: "u".into(),
        };
        let _ = JavaVersion {
            component: "x".into(),
            major_version: 1,
        };
    }
}

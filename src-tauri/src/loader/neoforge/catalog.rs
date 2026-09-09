//! NeoForge 版本目录：从官方 Maven 元数据按 Minecraft 版本筛选。

use anyhow::{Context, Result};

use crate::loader::LoaderVersion;

/// 解析正式版号（仅数字+点，如 `1.21.1`、`26.2`）为 (major, minor, patch)。
/// 快照（24w14a / 1.21.5-pre1）等含字母的 ID 一律返回 None。
pub fn parse_minecraft(mc: &str) -> Option<(u64, u64, u64)> {
    if mc.is_empty() || !mc.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let nums: Vec<u64> = mc
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().ok())
        .collect::<Option<Vec<u64>>>()?;
    let major = *nums.first()?;
    let minor = nums.get(1).copied().unwrap_or(0);
    let patch = nums.get(2).copied().unwrap_or(0);
    Some((major, minor, patch))
}

/// NeoForge 支持 Minecraft 1.20.1 及以上的正式版（含 26.x 新版本线）
pub fn mc_supports_neoforge(mc: &str) -> bool {
    matches!(parse_minecraft(mc), Some(v) if v >= (1, 20, 1))
}

/// NeoForge 现代制品的版本前缀：
/// - MC 主版本为 1 时去掉“1.”：mc 1.21.1 -> 21.1、1.21.11 -> 21.11、1.21 -> 21.0
/// - 更高主版本（26.x）直接沿用：mc 26.1 -> 26.1、26.2 -> 26.2
fn modern_prefix(mc: &str) -> Option<String> {
    let nums: Vec<u64> = mc
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().ok())
        .collect::<Option<Vec<u64>>>()?;
    let first = *nums.first()?;
    if first == 1 {
        let minor = *nums.get(1)?;
        if minor < 20 {
            return None;
        }
        Some(format!("{}.{}", minor, nums.get(2).copied().unwrap_or(0)))
    } else {
        Some(format!("{}.{}", first, nums.get(1).copied().unwrap_or(0)))
    }
}

/// 取版本串末尾三段数字用于倒序排序（忽略 mc 前缀等干扰段）
fn numeric_tuple(raw: &str) -> (u64, u64, u64) {
    let mut nums: Vec<u64> = Vec::new();
    for chunk in raw.split(|c: char| !c.is_ascii_digit()) {
        if !chunk.is_empty() {
            if let Ok(v) = chunk.parse::<u64>() {
                nums.push(v);
            }
        }
    }
    let take = |n: usize| nums.iter().rev().nth(n).copied().unwrap_or(0);
    (take(2), take(1), take(0))
}

pub(crate) fn is_legacy_1201(mc: &str) -> bool {
    parse_minecraft(mc) == Some((1, 20, 1))
}

/// NeoForge 版本是否为稳定版：带 -beta / -alpha / -rc / -pre / -snapshot 后缀的都不是
fn is_stable_release(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    !["-beta", "-alpha", "-rc", "-pre", "-snapshot", "-cr"]
        .iter()
        .any(|s| lower.contains(s))
}

/// 拉取某 MC 版本的 NeoForge 构建列表（1.20.1 走旧 forge 制品线）
pub async fn list_versions(minecraft_version: &str) -> Result<Vec<LoaderVersion>> {
    if !mc_supports_neoforge(minecraft_version) {
        return Ok(Vec::new());
    }
    if is_legacy_1201(minecraft_version) {
        return list_legacy(minecraft_version).await;
    }
    let Some(prefix) = modern_prefix(minecraft_version) else {
        return Ok(Vec::new());
    };
    let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
    let bytes = crate::loader::download::fetch_bytes(url).await?;
    let body: MavenVersions =
        serde_json::from_slice(&bytes).context("解析 NeoForge 版本列表失败")?;
    let mut versions: Vec<LoaderVersion> = body
        .versions
        .iter()
        .filter(|v| v.starts_with(&format!("{}.", prefix)))
        .map(|v| LoaderVersion {
            version: v.clone(),
            stable: is_stable_release(v),
        })
        .collect();
    versions.sort_by_key(|a| std::cmp::Reverse(numeric_tuple(&a.version)));
    Ok(versions)
}

/// 1.20.1：net.neoforged:forge（NeoForge 前身制品线）
async fn list_legacy(_mc: &str) -> Result<Vec<LoaderVersion>> {
    let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge";
    let bytes = crate::loader::download::fetch_bytes(url).await?;
    let body: MavenVersions =
        serde_json::from_slice(&bytes).context("解析 NeoForge(Forge) 版本列表失败")?;
    let mut versions: Vec<LoaderVersion> = body
        .versions
        .iter()
        .filter(|v| v.starts_with("1.20.1-"))
        .map(|v| LoaderVersion {
            version: v.clone(),
            stable: is_stable_release(v),
        })
        .collect();
    versions.sort_by_key(|a| std::cmp::Reverse(numeric_tuple(&a.version)));
    Ok(versions)
}

#[derive(serde::Deserialize)]
struct MavenVersions {
    versions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_gate() {
        assert!(!mc_supports_neoforge("1.19.4"));
        assert!(!mc_supports_neoforge("1.20"));
        assert!(mc_supports_neoforge("1.20.1"));
        assert!(mc_supports_neoforge("1.21.1"));
        // 26.x 新版本线（主版本不再是 1）
        assert!(mc_supports_neoforge("26.1"));
        assert!(mc_supports_neoforge("26.2"));
        assert!(!mc_supports_neoforge("24w14a"));
        assert!(!mc_supports_neoforge("1.21.5-pre1"));
    }

    #[test]
    fn prefix() {
        assert_eq!(modern_prefix("1.21.1").as_deref(), Some("21.1"));
        assert_eq!(modern_prefix("1.21").as_deref(), Some("21.0"));
        assert_eq!(modern_prefix("1.20.6").as_deref(), Some("20.6"));
        assert_eq!(modern_prefix("1.21.11").as_deref(), Some("21.11"));
        // 新版本线：直接沿用 major.minor
        assert_eq!(modern_prefix("26.1").as_deref(), Some("26.1"));
        assert_eq!(modern_prefix("26.2").as_deref(), Some("26.2"));
        assert_eq!(modern_prefix("1.12.2"), None);
        assert_eq!(modern_prefix("24w14a"), None);
    }

    #[test]
    fn parse_26_line() {
        assert_eq!(parse_minecraft("26.2"), Some((26, 2, 0)));
        assert_eq!(parse_minecraft("26.1"), Some((26, 1, 0)));
        assert_eq!(parse_minecraft("1.21.11"), Some((1, 21, 11)));
    }

    #[test]
    fn stable_suffix_judgement() {
        assert!(is_stable_release("21.1.250"));
        assert!(is_stable_release("26.2.0.83"));
        assert!(is_stable_release("1.20.1-47.1.106"));
        assert!(!is_stable_release("21.11.39-beta"));
        assert!(!is_stable_release("26.2.0.80-beta"));
        assert!(!is_stable_release("21.1.200-rc"));
    }

    #[test]
    fn legacy_sort() {
        assert!(numeric_tuple("1.20.1-47.1.106") > numeric_tuple("1.20.1-47.1.82"));
        assert!(numeric_tuple("21.1.250") > numeric_tuple("21.1.200"));
    }
}

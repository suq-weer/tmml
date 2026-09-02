//! 与「下载」和「启动」共用的平台/规则判定，避免两处各自实现导致漂移。

use crate::downloader::deserializer::Rule;

/// Mojang 语义的操作系统名
pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

/// Mojang 语义的架构名（用于 rules 的 os.arch 判定）
pub fn os_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x86",
        "aarch64" | "arm64" => "arm",
        "x86" | "i386" | "i686" => "x86",
        _ => "unknown",
    }
}

/// 实际架构后缀（用于 native 分类器键：amd64/arm64/x86），未知时返回 None
pub fn arch_suffix() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" | "amd64" => Some("amd64"),
        "aarch64" | "arm64" => Some("arm64"),
        "x86" | "i386" | "i686" => Some("x86"),
        _ => None,
    }
}

/// native 分类器的候选键。**架构优先**：先试「平台+实际架构」，再试平台通用键，
/// 避免 ARM 机器按「natives-linux」这类 x86/amd64 键误下载/误解压错误的原生依赖；
/// macOS 额外兼容旧命名 natives-osx。
pub fn native_classifier_candidates() -> Vec<String> {
    native_classifier_candidates_for(os_name(), arch_suffix())
}

/// 针对给定平台与架构后缀计算候选键（便于测试与跨平台定制）
pub fn native_classifier_candidates_for(
    platform_os: &str,
    arch: Option<&str>,
) -> Vec<String> {
    let bases: &[&str] = match platform_os {
        "osx" => &["natives-macos", "natives-osx"],
        "windows" => &["natives-windows"],
        _ => &["natives-linux"],
    };
    let mut candidates = Vec::new();
    for base in bases {
        if let Some(arch) = arch {
            candidates.push(format!("{}-{}", base, arch));
        }
        candidates.push(base.to_string());
    }
    candidates
}

/// 类路径分隔符
pub fn path_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

/// 当前运行特征（用于 arguments 的 features 门控）
#[derive(Clone, Copy, Debug, Default)]
pub struct FeatureState {
    pub is_demo_user: bool,
    pub has_custom_resolution: bool,
    pub has_quick_plays_support: bool,
    pub is_quick_play_singleplayer: bool,
    pub is_quick_play_multiplayer: bool,
    pub is_quick_play_realms: bool,
}

impl FeatureState {
    /// 按是否使用自定义分辨率构造（其余特征默认 false）
    pub fn with_custom_resolution(custom_resolution: bool) -> Self {
        Self {
            has_custom_resolution: custom_resolution,
            ..Self::default()
        }
    }
}

/// 单条规则的 OS 部分是否命中当前平台
fn os_part_matches(rule: &Rule) -> bool {
    match &rule.os {
        Some(os) => {
            let name_ok = match &os.name {
                Some(name) => name == os_name(),
                None => true,
            };
            let arch_ok = match &os.arch {
                Some(arch) => arch == os_arch(),
                None => true,
            };
            name_ok && arch_ok
        }
        None => true,
    }
}

/// 单条规则的 features 部分是否命中当前特征（rule 未列出该项视为命中）
fn feature_part_matches(rule: &Rule, features: &FeatureState) -> bool {
    let Some(f) = &rule.features else {
        return true;
    };
    let mut ok = true;
    if let Some(v) = f.is_demo_user {
        ok &= v == features.is_demo_user;
    }
    if let Some(v) = f.has_custom_resolution {
        ok &= v == features.has_custom_resolution;
    }
    if let Some(v) = f.has_quick_plays_support {
        ok &= v == features.has_quick_plays_support;
    }
    if let Some(v) = f.is_quick_play_singleplayer {
        ok &= v == features.is_quick_play_singleplayer;
    }
    if let Some(v) = f.is_quick_play_multiplayer {
        ok &= v == features.is_quick_play_multiplayer;
    }
    if let Some(v) = f.is_quick_play_realms {
        ok &= v == features.is_quick_play_realms;
    }
    ok
}

fn rule_applies(rule: &Rule, features: &FeatureState) -> bool {
    os_part_matches(rule) && feature_part_matches(rule, features)
}

/// 依次应用一组 rules 得出最终 allow/disallow。
///
/// `default_allowed` 决定「存在 rules 但没有任何一条命中」时的初值：
/// - 库过滤/类路径：`true`（仅当命中 disallow 才排除，兼容官方多数库仅用单条 allow/disallow 的写法）
/// - arguments 门控：`false`（仅当命中 allow 才纳入，避免 -XstartOnFirstThread 这类参数漏到其它平台）
pub fn rules_allow(
    rules: Option<&[Rule]>,
    features: &FeatureState,
    default_allowed: bool,
) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    if rules.is_empty() {
        return true;
    }
    let mut allowed = default_allowed;
    for rule in rules {
        if rule_applies(rule, features) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

/// 便捷：FeatureState 空/全 false 时不改变判定结果的占位值，避免误用
pub fn features_default() -> FeatureState {
    FeatureState::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloader::deserializer::{FeaturesFlag, Rule};

    #[test]
    fn arm_linux_orders_arch_specific_first() {
        let candidates = native_classifier_candidates_for("linux", Some("arm64"));
        assert_eq!(candidates[0], "natives-linux-arm64");
        assert!(candidates.contains(&"natives-linux".to_string()));
    }

    #[test]
    fn x64_linux_falls_back_to_generic() {
        let candidates = native_classifier_candidates_for("linux", Some("amd64"));
        assert_eq!(candidates[0], "natives-linux-amd64");
        // 通用键始终保留，保证找不到 amd64 专属键时能回退
        assert!(candidates.contains(&"natives-linux".to_string()));
    }

    #[test]
    fn macos_includes_legacy_naming() {
        let candidates = native_classifier_candidates_for("osx", Some("arm64"));
        assert_eq!(candidates[0], "natives-macos-arm64");
        assert!(candidates.iter().any(|c| c == "natives-osx"));
    }

    #[test]
    fn no_rules_always_allowed() {
        assert!(rules_allow(None, &FeatureState::default(), false));
        assert!(rules_allow(Some(&[]), &FeatureState::default(), false));
    }

    #[test]
    fn argument_feature_gating_requires_matching_allow() {
        // 仅一条 allow + features(has_custom_resolution: true)
        let rule = Rule {
            action: "allow".into(),
            features: Some(FeaturesFlag {
                is_demo_user: None,
                has_custom_resolution: Some(true),
                has_quick_plays_support: None,
                is_quick_play_singleplayer: None,
                is_quick_play_multiplayer: None,
                is_quick_play_realms: None,
            }),
            os: None,
        };
        let rules = [rule];

        // 未启用自定义分辨率 → 不纳入（default=false 生效）
        let no = FeatureState::with_custom_resolution(false);
        assert!(!rules_allow(Some(&rules), &no, false));
        // 启用 → 纳入
        let yes = FeatureState::with_custom_resolution(true);
        assert!(rules_allow(Some(&rules), &yes, false));
    }
}



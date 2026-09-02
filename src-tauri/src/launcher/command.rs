//! Minecraft 启动命令生成器。
//!
//! 先由 `<version>.json`（arguments.jvm / arguments.game 的规则与令牌）生成一段
//! 带参数的完整命令，再套用实例配置里的命令前缀 / 后缀、自定义 JVM/游戏参数、
//! 分辨率与 javaPath，输出最终用于启动 Minecraft 的命令。

use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Result};

use crate::{
    downloader::deserializer::{self, ArgumentValue, StringOrArgument},
    instance::InstanceConfig,
    platform::{features_default, path_separator, rules_allow, FeatureState},
    profile::GameProfile,
};

/// 本次启动涉及的文件系统路径（绝对路径）
#[derive(Debug, Clone)]
pub struct LaunchPaths {
    /// 实例目录（游戏目录），即 versions/<dir>
    pub game_dir: PathBuf,
    /// versions/<dir>/natives
    pub natives_dir: PathBuf,
    /// .minecraft/libraries
    pub libraries_root: PathBuf,
    /// .minecraft/assets
    pub assets_root: PathBuf,
    /// log4j 配置 assets/log_configs/<id>
    pub log_config_path: PathBuf,
    /// versions/<dir>/<version>.jar
    pub client_jar: PathBuf,
}

pub struct LaunchContext<'a> {
    pub content: &'a deserializer::VersionContent,
    pub paths: &'a LaunchPaths,
    pub config: &'a InstanceConfig,
    pub profile: &'a GameProfile,
    /// java 可执行程序（可为绝对路径，或仅 "java"）
    pub java_bin: &'a str,
    pub launcher_name: &'a str,
    pub launcher_version: &'a str,
}

/// 生成结果：可直接 spawn 的 argv 与便于阅读/复制的完整命令
#[derive(Debug)]
pub struct LaunchCommand {
    pub argv: Vec<String>,
    pub display: String,
    /// 生成过程中的提示（缺失但非致命的文件等）
    pub warnings: Vec<String>,
}

/// 构造最终启动命令。
pub fn build_final_command(ctx: &LaunchContext<'_>) -> Result<LaunchCommand> {
    let mut warnings = Vec::new();

    let classpath = build_classpath(ctx)?;
    let missing: Vec<String> = classpath
        .iter()
        .filter(|p| !p.exists())
        .map(|p| p.display().to_string())
        .collect();
    if !missing.is_empty() {
        bail!(
            "缺少 {} 个依赖文件，请先在实例列表完成该版本的下载或重新下载：\n  - {}",
            missing.len(),
            missing.join("\n  - ")
        );
    }

    let mut tokens = HashMap::new();
    tokens.insert("classpath".to_string(), join_paths(&classpath));
    tokens.insert("classpath_separator".to_string(), path_separator().to_string());
    tokens.insert("natives_directory".to_string(), ctx.paths.natives_dir.display().to_string());
    tokens.insert("launcher_name".to_string(), ctx.launcher_name.to_string());
    tokens.insert("launcher_version".to_string(), ctx.launcher_version.to_string());
    tokens.insert("game_directory".to_string(), ctx.paths.game_dir.display().to_string());
    tokens.insert("assets_root".to_string(), ctx.paths.assets_root.display().to_string());
    tokens.insert("assets_index_name".to_string(), ctx.content.assets_index.id.clone());
    tokens.insert("library_directory".to_string(), ctx.paths.libraries_root.display().to_string());
    tokens.insert("client_jar".to_string(), ctx.paths.client_jar.display().to_string());
    tokens.insert("version_name".to_string(), ctx.content.id.clone());
    tokens.insert("version_type".to_string(), ctx.content.version_type.clone());
    tokens.insert("auth_player_name".to_string(), player_name(ctx));
    tokens.insert("auth_uuid".to_string(), ctx.profile.id.clone());
    tokens.insert("auth_access_token".to_string(), "0".to_string());
    tokens.insert("clientid".to_string(), uuid::Uuid::new_v4().to_string());
    tokens.insert("auth_xuid".to_string(), "0".to_string());
    tokens.insert("user_type".to_string(), "legacy".to_string());
    if let (Some(w), Some(h)) = (ctx.config.width, ctx.config.height) {
        tokens.insert("resolution_width".to_string(), w.to_string());
        tokens.insert("resolution_height".to_string(), h.to_string());
    }

    let custom_resolution = ctx.config.width.is_some() && ctx.config.height.is_some();
    let features = FeatureState::with_custom_resolution(custom_resolution);

    let mut jvm = expand_arguments(&ctx.content.arguments.jvm, &tokens, &features);
    let game = expand_arguments(&ctx.content.arguments.game, &tokens, &features);

    // 用户自定义 JVM 参数（追加在版本自带参数之后，位于 -cp 之后，Java 不关心顺序）
    jvm.extend(ctx.config.jvm_args.iter().cloned());

    // log4j 配置参数：-Dlog4j.configurationFile=<path>
    let log_arg = ctx.content.logging.client.argument.replace("${path}", &ctx.paths.log_config_path.display().to_string());
    if !log_arg.trim().is_empty() {
        if !ctx.paths.log_config_path.exists() {
            warnings.push(format!(
                "日志配置文件不存在（{}），将跳过 -Dlog4j.configurationFile",
                ctx.paths.log_config_path.display()
            ));
        } else {
            jvm.push(log_arg);
        }
    }

    // 组装最终命令：前缀 + java + JVM 参数 + 主类 + 游戏参数 + 后缀
    let mut argv = ctx.config.launch_command_prefix.clone();
    argv.push(ctx.java_bin.to_string());
    argv.extend(jvm);
    argv.push(ctx.content.main_class.clone());
    argv.extend(game);
    argv.extend(ctx.config.launch_command_suffix.iter().cloned());

    let display = shell_join(&argv);

    Ok(LaunchCommand { argv, display, warnings })
}

/// 收集类路径：所有被 rules 放行的依赖库 + 客户端 jar
fn build_classpath(ctx: &LaunchContext<'_>) -> Result<Vec<PathBuf>> {
    let mut cp = Vec::new();
    for lib in &ctx.content.libraries {
        if !rules_allow(lib.rules.as_deref(), &features_default(), true) {
            continue;
        }
        cp.push(ctx.paths.libraries_root.join(&lib.downloads.artifact.path));
    }
    cp.push(ctx.paths.client_jar.clone());
    Ok(cp)
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(path_separator())
}

fn player_name(ctx: &LaunchContext<'_>) -> String {
    ctx.profile
        .username
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| ctx.profile.name.clone())
}

/// 依次解析 arguments（普通字符串或带 rules 的条目），按平台与 features 门控
fn expand_arguments(
    units: &[StringOrArgument],
    tokens: &HashMap<String, String>,
    features: &FeatureState,
) -> Vec<String> {
    let mut out = Vec::new();
    for unit in units {
        match unit {
            StringOrArgument::String(s) => {
                if !s.trim().is_empty() {
                    out.push(expand(s, tokens));
                }
            }
            StringOrArgument::Argument(arg) => {
                if !rules_allow(Some(&arg.rules), features, false) {
                    continue;
                }
                match &arg.value {
                    ArgumentValue::String(s) => {
                        if !s.trim().is_empty() {
                            out.push(expand(s, tokens));
                        }
                    }
                    ArgumentValue::Vec(vals) => {
                        for v in vals {
                            if !v.trim().is_empty() {
                                out.push(expand(v, tokens));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// 令牌替换：识别 ${key}，已登记的替换为值，未登记的替换为空串（避免残留非法参数）
fn expand(template: &str, tokens: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                if let Some(v) = tokens.get(key) {
                    out.push_str(v);
                }
                rest = &after[end + 1..];
            }
            None => {
                // 未闭合的 ${，按字面保留
                out.push_str("${");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// 将 argv 组装成便于展示/复制的终端命令字符串
fn shell_join(argv: &[String]) -> String {
    argv.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ")
}

fn quote(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    #[cfg(windows)]
    {
        if arg.chars().any(|c| c.is_whitespace() || c == '"') {
            format!("\"{}\"", arg.replace('"', "\\\""))
        } else {
            arg.to_string()
        }
    }
    #[cfg(not(windows))]
    {
        if arg
            .chars()
            .all(|c| c.is_alphanumeric() || "-_./:=+@%^,~".contains(c))
        {
            arg.to_string()
        } else {
            format!("'{}'", arg.replace('\'', "'\\''"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_replaces_known_and_clears_unknown() {
        let mut tokens = HashMap::new();
        tokens.insert("natives_directory".to_string(), "/n".to_string());
        tokens.insert("launcher_name".to_string(), "tmml".to_string());
        let out = expand("-Djava.library.path=${natives_directory} ${launcher_name} ${unknown_token}", &tokens);
        assert_eq!(out, "-Djava.library.path=/n tmml ");
    }

    #[test]
    fn expand_keeps_unclosed_brace_literal() {
        let tokens = HashMap::new();
        assert_eq!(expand("${oops", &tokens), "${oops");
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_join_quotes_spaces_on_posix() {
        let joined = shell_join(&["java".to_string(), "-Xmx 2G".to_string(), "net.minecraft".to_string()]);
        assert!(joined.starts_with("java "));
        assert!(joined.contains("'-Xmx 2G'"));
    }
}

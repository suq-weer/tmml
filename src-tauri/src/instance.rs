use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use time::OffsetDateTime;

use crate::appfile::dirs;

/// 实例注册表条目（轻量索引），持久化于 .minecraft/tmml_instances.json
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MinecraftInstance {
    pub id: String,
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// 实例配置
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    /// 启动命令前缀，对接未来启动模块（插在 java 之前）
    #[serde(default)]
    pub launch_command_prefix: Vec<String>,
    /// 启动命令后缀（追加在游戏参数之后）
    #[serde(default)]
    pub launch_command_suffix: Vec<String>,
    #[serde(default)]
    pub jvm_args: Vec<String>,
    #[serde(default)]
    pub game_args: Vec<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// 实例完整信息，持久化于 versions/<version>/tmml_instance.json
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    pub id: String,
    pub version_id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub config: InstanceConfig,
}

/// 将实例名称清洗为安全的目录名（剔除非法路径字符）；清洗后为空返回空串
pub fn sanitize_dir_name(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' | '\n' | '\r' | '\t'))
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() || s == "." || s == ".." {
        String::new()
    } else {
        s
    }
}

fn registry_file() -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?.join("tmml_instances.json"))
}

fn instance_config_file(dir_name: &str) -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?
        .join("versions")
        .join(dir_name)
        .join("tmml_instance.json"))
}

/// 解析实例所在的目录名：优先取注册表里记录的 path（可能为实例名目录），无则回退版本号
fn resolve_dir(version_id: &str) -> String {
    load_registry()
        .ok()
        .and_then(|registry| {
            registry
                .into_iter()
                .find(|i| i.id == version_id)
                .map(|i| i.path)
        })
        .and_then(|path| path.strip_prefix("versions/").map(|s| s.to_string()))
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| version_id.to_string())
}

fn load_registry() -> Result<Vec<MinecraftInstance>> {
    let path = registry_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).context("解析 tmml_instances.json 失败")
}

fn save_registry(instances: &[MinecraftInstance]) -> Result<()> {
    let path = registry_file()?;
    let data = serde_json::to_string_pretty(instances)?;
    fs::write(&path, data)?;
    Ok(())
}

/// 下载安装完成后创建/更新实例（upsert）：写实例配置文件 + 更新注册表
/// 实例配置与全局默认配置合并（实例非空则用实例，否则用全局默认托底）
/// `dir_name` 为实例文件所在目录名（通常为实例名或版本号）
pub fn create(
    version_id: &str,
    name: Option<String>,
    config: Option<InstanceConfig>,
    defaults: &InstanceConfig,
    dir_name: &str,
) -> Result<InstanceInfo> {
    let mut registry = load_registry()?;
    let now = OffsetDateTime::now_utc().to_string();

    let instance_name = name.unwrap_or_else(|| version_id.to_string());
    let config = merge_with_defaults(config.unwrap_or_default(), defaults);
    let info = InstanceInfo {
        id: version_id.to_string(),
        version_id: version_id.to_string(),
        name: instance_name.clone(),
        path: format!("versions/{}", dir_name),
        created_at: now.clone(),
        config,
    };

    // 写入实例配置文件（与 <version>.jar 同级）
    let config_path = instance_config_file(dir_name)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_path, serde_json::to_string_pretty(&info)?)?;

    // 更新注册表
    let entry = MinecraftInstance {
        id: version_id.to_string(),
        version_id: version_id.to_string(),
        name: instance_name,
        path: format!("versions/{}", dir_name),
        created_at: now,
    };
    match registry.iter_mut().find(|i| i.id == version_id) {
        Some(existing) => *existing = entry,
        None => registry.push(entry),
    }
    save_registry(&registry)?;

    tracing::info!(version_id, dir_name, "已创建/更新 Minecraft 实例");
    Ok(info)
}

/// 读取实例完整信息（含配置），不存在返回 None
pub fn get(version_id: &str) -> Result<Option<InstanceInfo>> {
    let dir = resolve_dir(version_id);
    let path = instance_config_file(&dir)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data)
        .context("解析 tmml_instance.json 失败")
        .map(Some)
}

/// 更新实例名称与配置，并同步到配置文件与注册表（配置同样用全局默认托底）
pub fn update(
    version_id: &str,
    name: Option<String>,
    config: Option<InstanceConfig>,
    defaults: &InstanceConfig,
) -> Result<InstanceInfo> {
    let mut info = get(version_id)?.ok_or_else(|| anyhow!("实例 {} 不存在", version_id))?;
    if let Some(name) = name {
        if !name.trim().is_empty() {
            info.name = name.trim().to_string();
        }
    }
    if let Some(config) = config {
        info.config = merge_with_defaults(config, defaults);
    }

    let dir = resolve_dir(version_id);
    let config_path = instance_config_file(&dir)?;
    fs::write(&config_path, serde_json::to_string_pretty(&info)?)?;

    let mut registry = load_registry()?;
    if let Some(entry) = registry.iter_mut().find(|i| i.id == version_id) {
        entry.name = info.name.clone();
    }
    save_registry(&registry)?;
    Ok(info)
}

/// 列出全部实例（注册表条目）
pub fn list() -> Result<Vec<MinecraftInstance>> {
    load_registry()
}

/// 读取实例目录下的自定义图标 versions/<dir>/tmml_instance_icon.png，转为 base64 data URL；不存在返回 None
pub fn get_instance_icon(dir_name: &str) -> Result<Option<String>> {
    // 防止路径穿越
    if dir_name.is_empty()
        || dir_name.contains('/')
        || dir_name.contains('\\')
        || dir_name.contains("..")
    {
        return Ok(None);
    }
    let path = dirs::dot_minecraft()?
        .join("versions")
        .join(dir_name)
        .join("tmml_instance_icon.png");
    if !path.exists() {
        return Ok(None);
    }
    use base64::{engine::general_purpose, Engine as _};
    let data = fs::read(&path)?;
    Ok(Some(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(data)
    )))
}

/// 实例配置与全局默认托底合并：数组字段实例非空则用实例，否则用全局默认；分辨率为实例 or 全局默认
fn merge_with_defaults(config: InstanceConfig, defaults: &InstanceConfig) -> InstanceConfig {
    let pick = |instance: Vec<String>, default: &Vec<String>| {
        if instance.is_empty() { default.clone() } else { instance }
    };
    InstanceConfig {
        launch_command_prefix: pick(config.launch_command_prefix, &defaults.launch_command_prefix),
        launch_command_suffix: pick(config.launch_command_suffix, &defaults.launch_command_suffix),
        jvm_args: pick(config.jvm_args, &defaults.jvm_args),
        game_args: pick(config.game_args, &defaults.game_args),
        width: config.width.or(defaults.width),
        height: config.height.or(defaults.height),
    }
}

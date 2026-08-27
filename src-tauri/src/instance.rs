use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use time::OffsetDateTime;

use crate::appfile::dirs;

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

fn instances_file() -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?.join("instances.json"))
}

fn load() -> Result<Vec<MinecraftInstance>> {
    let path = instances_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).context("解析 instances.json 失败")
}

fn save(instances: &[MinecraftInstance]) -> Result<()> {
    let path = instances_file()?;
    let data = serde_json::to_string_pretty(instances)?;
    fs::write(&path, data)?;
    Ok(())
}

/// 为一个下载完成的版本创建实例（已存在则跳过，幂等）
pub fn create(version_id: &str) -> Result<MinecraftInstance> {
    let mut instances = load()?;
    if let Some(existing) = instances.iter().find(|i| i.id == version_id) {
        return Ok(existing.clone());
    }
    let now = OffsetDateTime::now_utc();
    let instance = MinecraftInstance {
        id: version_id.to_string(),
        version_id: version_id.to_string(),
        name: version_id.to_string(),
        path: format!("versions/{}", version_id),
        created_at: now.to_string(),
    };
    instances.push(instance.clone());
    save(&instances)?;
    tracing::info!(version_id, "已创建 Minecraft 实例");
    Ok(instance)
}

/// 列出全部实例
pub fn list() -> Result<Vec<MinecraftInstance>> {
    load()
}

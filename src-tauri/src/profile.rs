use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::lock::Mutex;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{appfile::dirs, config::MainConfig};

/// 游戏档案登录方式（离线已实现；其余为未来预留入口）
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Offline,
    Microsoft,
    AuthlibInjector,
}

impl AuthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthType::Offline => "offline",
            AuthType::Microsoft => "microsoft",
            AuthType::AuthlibInjector => "authlib-injector",
        }
    }
}

/// 游戏档案：对应一个准备在游戏里启动的玩家账号
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameProfile {
    pub id: String,
    /// 显示名（离线登录下即玩家名）
    pub name: String,
    pub auth_type: AuthType,
    /// 离线登录的玩家名；其它登录方式下的账号标识
    #[serde(default)]
    pub username: Option<String>,
    /// 其它登录方式的扩展数据（如正版 XSTS 令牌、皮肤站 base url 等，供未来使用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_data: Option<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// 当前激活的游戏档案（启动时自动切换到默认档案）
static CURRENT_PROFILE: LazyLock<Mutex<Option<GameProfile>>> =
    LazyLock::new(|| Mutex::new(None));

fn profiles_file() -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?.join("tmml_profiles.json"))
}

fn load() -> Result<Vec<GameProfile>> {
    let path = profiles_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).context("解析 tmml_profiles.json 失败")
}

fn save(profiles: &[GameProfile]) -> Result<()> {
    let path = profiles_file()?;
    let data = serde_json::to_string_pretty(profiles)?;
    fs::write(&path, data)?;
    Ok(())
}

fn now_iso() -> String {
    OffsetDateTime::now_utc().to_string()
}

/// 创建游戏档案。目前仅实现离线登录；其它登录方式留入口
pub fn create(auth_type: &str, username: Option<&str>) -> Result<GameProfile> {
    let parsed = parse_auth_type(auth_type)?;
    match parsed {
        AuthType::Offline => {
            let name = username
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("离线登录需要填写玩家名"))?;
            let profile = GameProfile {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                auth_type: AuthType::Offline,
                username: Some(name.to_string()),
                auth_data: None,
                created_at: now_iso(),
            };
            push_profile(&profile)?;
            tracing::info!(id = %profile.id, name = %profile.name, "已创建离线游戏档案");
            Ok(profile)
        }
        AuthType::Microsoft | AuthType::AuthlibInjector => {
            bail!("登录方式 {} 尚未实现", parsed.as_str())
        }
    }
}

/// 删除游戏档案；若其为默认/当前档案则同步清空
pub async fn delete(id: &str) -> Result<()> {
    let mut profiles = load()?;
    let before = profiles.len();
    profiles.retain(|p| p.id != id);
    if profiles.len() == before {
        bail!("游戏档案 {} 不存在", id);
    }
    save(&profiles)?;
    // 同步清空默认/当前
    let _ = MainConfig::update(|config| {
        if config.default_profile_id.as_deref() == Some(id) {
            config.default_profile_id = None;
        }
    })
    .await;
    let mut current = CURRENT_PROFILE.lock().await;
    if current.as_ref().map(|p| p.id.as_str()) == Some(id) {
        *current = None;
    }
    tracing::info!(id, "已删除游戏档案");
    Ok(())
}

fn push_profile(profile: &GameProfile) -> Result<()> {
    let mut profiles = load()?;
    profiles.push(profile.clone());
    save(&profiles)
}

/// 列出全部游戏档案
pub fn list() -> Result<Vec<GameProfile>> {
    load()
}

/// 获取单个游戏档案，不存在返回 None
pub fn get(id: &str) -> Result<Option<GameProfile>> {
    Ok(load()?.into_iter().find(|p| p.id == id))
}

/// 启动时调用：根据全局默认档案自动切换当前档案（无默认则切换到第一个档案）
pub async fn init_current() -> Result<()> {
    let config = MainConfig::get().await;
    let profiles = list()?;
    let mut current = CURRENT_PROFILE.lock().await;
    *current = match config.default_profile_id.as_deref() {
        Some(id) => get(id)?.or_else(|| profiles.first().cloned()),
        None => profiles.first().cloned(),
    };
    Ok(())
}

/// 获取当前激活的游戏档案
pub async fn get_current() -> Option<GameProfile> {
    CURRENT_PROFILE.lock().await.clone()
}

/// 设置当前激活的游戏档案
pub async fn set_current(profile: Option<GameProfile>) {
    *CURRENT_PROFILE.lock().await = profile;
}

fn parse_auth_type(s: &str) -> Result<AuthType> {
    match s.to_lowercase().as_str() {
        "offline" => Ok(AuthType::Offline),
        "microsoft" => Ok(AuthType::Microsoft),
        "authlib-injector" | "authlib_injector" => Ok(AuthType::AuthlibInjector),
        _ => bail!("未知的登录方式: {}", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_type_known() {
        assert_eq!(parse_auth_type("offline").unwrap(), AuthType::Offline);
        assert_eq!(parse_auth_type("Microsoft").unwrap(), AuthType::Microsoft);
        assert_eq!(parse_auth_type("authlib-injector").unwrap(), AuthType::AuthlibInjector);
        assert_eq!(parse_auth_type("AUTHLIB_INJECTOR").unwrap(), AuthType::AuthlibInjector);
        assert!(parse_auth_type("foo").is_err());
    }

    #[test]
    fn auth_type_serde_roundtrip() {
        assert_eq!(serde_json::to_string(&AuthType::Offline).unwrap(), "\"offline\"");
        assert_eq!(
            serde_json::from_str::<AuthType>("\"microsoft\"").unwrap(),
            AuthType::Microsoft
        );
    }
}

//! Fabric Meta API 结构定义。

use serde::Deserialize;

/// `GET /v2/versions/loader/{game_version}` 的每个条目我们只关心 loader 部分
#[derive(Deserialize, Debug, Clone)]
pub struct FabricLoaderListEntry {
    pub loader: FabricLoaderInfo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FabricLoaderInfo {
    pub maven: String,
    pub version: String,
    #[serde(default)]
    pub stable: bool,
}

/// `GET /v2/versions/loader/{game}/{loader}` 返回的完整启动元数据
#[derive(Deserialize, Debug, Clone)]
pub struct FabricLaunchMeta {
    pub loader: FabricLoaderInfo,
    pub intermediary: FabricIntermediaryInfo,
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: FabricLauncherMeta,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FabricIntermediaryInfo {
    pub maven: String,
    pub version: String,
    #[serde(default)]
    pub stable: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FabricLauncherMeta {
    #[serde(rename = "mainClass")]
    pub main_class: FabricMainClass,
    pub libraries: FabricLibraries,
    #[serde(default)]
    pub launchwrapper: Option<FabricLaunchWrapper>,
}

/// mainClass 兼容字符串与双端对象两种写法
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum FabricMainClass {
    String(String),
    Sides {
        client: String,
        server: Option<String>,
    },
}

impl FabricMainClass {
    pub fn client(&self) -> Option<&str> {
        match self {
            FabricMainClass::String(v) => Some(v),
            FabricMainClass::Sides { client, .. } => Some(client),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct FabricLibraries {
    #[serde(default)]
    pub common: Vec<FabricLibrary>,
    #[serde(default)]
    pub client: Vec<FabricLibrary>,
    #[serde(default)]
    pub server: Vec<FabricLibrary>,
}

/// 每条库：name 为坐标，url 为仓库根地址，附带可选 sha1/size
#[derive(Deserialize, Debug, Clone)]
pub struct FabricLibrary {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FabricLaunchWrapper {
    pub tweakers: Option<FabricTweakers>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FabricTweakers {
    #[serde(default)]
    pub client: Vec<String>,
    #[serde(default)]
    pub server: Vec<String>,
}

impl FabricLaunchWrapper {
    pub fn client_tweaker(&self) -> Option<&str> {
        self.tweakers
            .as_ref()
            .and_then(|t| t.client.first())
            .map(|s| s.as_str())
    }
}

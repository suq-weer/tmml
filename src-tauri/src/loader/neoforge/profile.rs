//! NeoForge/Forge installer 内 install_profile.json 的结构。

use std::collections::HashMap;

use serde::Deserialize;

/// install_profile.json
#[derive(Deserialize, Debug, Clone)]
pub struct InstallProfile {
    /// 该构建对应的 Minecraft 版本（如 1.21.1）
    #[serde(default)]
    pub minecraft: Option<String>,
    /// installer 内的版本 JSON 路径（如 /version.json）
    #[serde(default)]
    pub json: Option<String>,
    /// 安装期依赖（processor classpath/工具与运行时通用制品）
    #[serde(default)]
    pub libraries: Vec<ProfileLibrary>,
    #[serde(default)]
    pub processors: Vec<Processor>,
    /// 数据变量（client 侧按客户端取值）
    #[serde(default)]
    pub data: HashMap<String, DataEntry>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProfileLibrary {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub downloads: Option<ProfileDownloads>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProfileDownloads {
    #[serde(default)]
    pub artifact: Option<ProfileArtifact>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProfileArtifact {
    pub path: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub url: Option<String>,
}

impl ProfileLibrary {
    /// 若有 downloads.artifact 则给出其信息
    pub fn artifact(&self) -> Option<&ProfileArtifact> {
        self.downloads.as_ref().and_then(|d| d.artifact.as_ref())
    }
}

/// data 条目：client/server 各自的值（可为坐标、字面量或 installer 内文件）
#[derive(Deserialize, Debug, Clone, Default)]
pub struct DataEntry {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Processor {
    #[serde(default)]
    pub sides: Option<Vec<String>>,
    pub jar: String,
    #[serde(default)]
    pub classpath: Option<Vec<String>>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub outputs: Option<HashMap<String, String>>,
}

impl Processor {
    /// 是否应在客户端执行：sides 缺失或包含 client
    pub fn for_client(&self) -> bool {
        match &self.sides {
            None => true,
            Some(sides) => sides.iter().any(|s| s == "client"),
        }
    }
}

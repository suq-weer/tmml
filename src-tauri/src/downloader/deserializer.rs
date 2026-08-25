use serde::{Deserialize, Serialize};

// | version_manifest.json 解析 |

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<SingleVersion>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SingleVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

// 硬编码的愚人节版本列表
pub const FOOL_VERSIONS: &[&str] = &[
    // "2.0",
    "15w14a",
    "1.RV-Pre1",
    "3D Shareware v1.34",
    "20w14infinite",
    "22w13oneblockatatime",
    "23w13a_or_b",
    "24w14potato",
    "25w14craftmine",
    "26w14a", // ...
];

// | <version>.json 解析 |

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionContent {
    pub arguments: Arguments,
    #[serde(rename = "assetIndex")]
    pub assets_index: AssetsIndex,
    pub assets: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: u64,
    pub downloads: Downloads,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: JavaVersion,
    pub libraries: Vec<OnceLibraries>,
    pub logging: Logging,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: u64,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub version_type: String,
}

// - arguments

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Arguments {
    pub game: Vec<StringOrArgument>,
    pub jvm: Vec<StringOrArgument>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Argument {
    pub rules: Vec<Rule>,
    pub value: ArgumentValue,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub features: Option<FeaturesFlag>,
    #[serde(default)]
    pub os: Option<OS>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OS {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FeaturesFlag {
    #[serde(default)]
    pub is_demo_user: Option<bool>,
    #[serde(default)]
    pub has_custom_resolution: Option<bool>,
    #[serde(default)]
    pub has_quick_plays_support: Option<bool>,
    #[serde(default)]
    pub is_quick_play_singleplayer: Option<bool>,
    #[serde(default)]
    pub is_quick_play_multiplayer: Option<bool>,
    #[serde(default)]
    pub is_quick_play_realms: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum StringOrArgument {
    String(String),
    Argument(Argument),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Vec(Vec<String>),
}

// - assetsIndex

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AssetsIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

// - downloads

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Downloads {
    pub client: DownloadsFile,
    pub client_mappings: DownloadsFile,
    pub server: DownloadsFile,
    pub server_mappings: DownloadsFile,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DownloadsFile {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

// - javaVersion

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u64,
}

// - libraries

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OnceLibraries {
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    pub downloads: LibrariesDownloads,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LibrariesDownloads {
    pub artifact: Artifact,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

// - logging

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingClientFile,
    #[serde(rename = "type")]
    pub file_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<SingleVersion>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
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

//! Maven 坐标解析与本地库路径计算。
//!
//! 兼容格式：
//! ```text
//! group:artifact:version
//! group:artifact:version:classifier
//! group:artifact:version@extension
//! group:artifact:version:classifier@extension
//! ```
//! 默认扩展名为 jar。坐标解析出后，可以换算成标准 Maven 布局的相对路径。

use std::path::PathBuf;

/// 解析失败（非法坐标）
#[derive(Debug, Clone)]
pub struct CoordinateError(pub String);

/// 拆分 Maven 坐标，返回 (group, artifact, version, classifier, extension)
pub fn split_coordinate(raw: &str) -> Result<MavenCoordinate, CoordinateError> {
    let (no_ext, ext) = match raw.split_once('@') {
        Some((head, e)) => (head, e.to_string()),
        None => (raw, "jar".to_string()),
    };
    let parts: Vec<&str> = no_ext.split(':').collect();
    let (group, artifact, version, classifier) = match parts.as_slice() {
        [g, a, v] => (g.to_string(), a.to_string(), v.to_string(), None),
        [g, a, v, c] => (
            g.to_string(),
            a.to_string(),
            v.to_string(),
            Some(c.to_string()),
        ),
        _ => return Err(CoordinateError(format!("无法解析 Maven 坐标「{}」", raw))),
    };
    if group.is_empty() || artifact.is_empty() || version.is_empty() {
        return Err(CoordinateError(format!("无法解析 Maven 坐标「{}」", raw)));
    }
    Ok(MavenCoordinate {
        group,
        artifact,
        version,
        classifier,
        extension: ext,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenCoordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl MavenCoordinate {
    /// 生成标准 Maven 目录下的相对路径，例如
    /// `net/neoforged/neoforge/21.1.200/neoforge-21.1.200.jar`
    pub fn relative_path(&self) -> PathBuf {
        let mut file = format!("{}-{}", self.artifact, self.version);
        if let Some(classifier) = &self.classifier {
            file.push('-');
            file.push_str(classifier);
        }
        file.push('.');
        file.push_str(&self.extension);
        PathBuf::from(&self.group.replace('.', "/"))
            .join(&self.artifact)
            .join(&self.version)
            .join(file)
    }
}

/// 由坐标字符串直接推导本地相对路径（坐标非法时返回 None）
pub fn relative_path_of(raw: &str) -> Option<PathBuf> {
    split_coordinate(raw).ok().map(|c| c.relative_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain() {
        let c = split_coordinate("net.neoforged:neoforge:21.1.200").unwrap();
        assert_eq!(c.classifier, None);
        assert_eq!(c.extension, "jar");
        let rel = c.relative_path().to_string_lossy().replace('\\', "/");
        assert_eq!(rel, "net/neoforged/neoforge/21.1.200/neoforge-21.1.200.jar");
    }

    #[test]
    fn parse_classifier() {
        let c = split_coordinate("net.minecraft:client:1.21.1:mappings@txt").unwrap();
        assert_eq!(c.classifier.as_deref(), Some("mappings"));
        assert_eq!(c.extension, "txt");
        let rel = c.relative_path().to_string_lossy().replace('\\', "/");
        assert_eq!(
            rel,
            "net/minecraft/client/1.21.1/client-1.21.1-mappings.txt"
        );
    }

    #[test]
    fn parse_classifier_plain_ext() {
        let c = split_coordinate("net.neoforged:AutoRenamingTool:2.0.3:all").unwrap();
        assert_eq!(c.classifier.as_deref(), Some("all"));
        assert_eq!(c.extension, "jar");
        let rel = c.relative_path().to_string_lossy().replace('\\', "/");
        assert_eq!(
            rel,
            "net/neoforged/AutoRenamingTool/2.0.3/AutoRenamingTool-2.0.3-all.jar"
        );
    }

    #[test]
    fn reject_bad() {
        assert!(split_coordinate("not-a-coordinate").is_err());
    }
}

//! 加载器安装通用下载原语。
//!
//! 所有落盘均遵循：写临时文件 -> 校验 SHA-1 -> 原子改名到最终位置，
//! 中途失败不残留半成品。文件已存在（且大小匹配时直接复用）不再重复下载。

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use crate::downloader::minecraft::PhaseProgress;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .expect("构建 HTTP 客户端失败")
});

/// 计算单个文件的 SHA-1（小文件走内存，避免多余读写）
pub fn sha1_hex(data: &[u8]) -> String {
    format!("{:x}", Sha1::digest(data))
}

pub fn sha1_hex_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path).with_context(|| format!("读取 {} 失败", path.display()))?;
    Ok(sha1_hex(&data))
}

/// 整块下载并返回字节（适用于 JSON/小型文件）
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .with_context(|| format!("请求失败: {}", url))?;
    if !response.status().is_success() {
        bail!("HTTP {} {}", response.status(), url);
    }
    Ok(response.bytes().await?.to_vec())
}

/// 下载到目标文件：临时文件 + 可选 sha1 校验 + 原子改名；失败自动重试
///
/// `progress` 非空时，会向前端推送该文件的字节级下载进度。
pub(crate) async fn download_file(
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    progress: Option<&PhaseProgress>,
) -> Result<()> {
    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 0..3 {
        match download_once(url, dest, expected_sha1, progress).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("下载 {} 失败(第 {} 次): {}", url, attempt + 1, e);
                last_error = Some(e);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("下载失败: {}", url)))
}

async fn download_once(
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    progress: Option<&PhaseProgress>,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = dest.with_extension("tmp");
    let response = HTTP_CLIENT.get(url).send().await?;
    if !response.status().is_success() {
        bail!("HTTP {} {}", response.status(), url);
    }

    let file_size = response.content_length().unwrap_or(0);
    let name = dest.display().to_string();

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut stream = response.bytes_stream();
    let mut hasher = Sha1::new();
    let mut written: u64 = 0;
    let mut last_emit = Instant::now();
    let mut bytes_since_last: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        bytes_since_last += chunk.len() as u64;

        if let Some(p) = progress {
            if last_emit.elapsed() >= Duration::from_millis(150) {
                let speed =
                    (bytes_since_last as f64 / last_emit.elapsed().as_secs_f64()).round() as u64;
                p.emit_throttled(name.clone(), written, file_size, speed, false, false);
                last_emit = Instant::now();
                bytes_since_last = 0;
            }
        }
    }
    file.flush().await?;

    if let Some(expected) = expected_sha1 {
        let digest = format!("{:x}", hasher.finalize());
        if !digest.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("sha1 校验失败: {} (期望 {} 实际 {})", url, expected, digest);
        }
    }

    tokio::fs::rename(&tmp, dest).await?;

    if let Some(p) = progress {
        p.add_done(written);
        p.emit(name, written, file_size.max(written), 0, true, false);
    }
    Ok(())
}

/// 准备下载一个 Maven 制品到库目录；文件已就绪则直接返回。
///
/// `exact_url` 优先；否则按 `bases` 依序尝试「base + 相对路径」。
pub struct RepoArtifact {
    pub rel_path: PathBuf,
    pub bases: Vec<String>,
    pub exact_url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub(crate) async fn ensure_artifact(
    a: &RepoArtifact,
    libs_dir: &Path,
    progress: Option<&PhaseProgress>,
) -> Result<PathBuf> {
    let dest = libs_dir.join(&a.rel_path);

    // 已存在：大小已知则按大小判定复用，否则只要有文件即视为可用
    if dest.exists() {
        if let Some(size) = a.size {
            let actual = tokio::fs::metadata(&dest).await.map(|m| m.len()).ok();
            if actual == Some(size) {
                if let Some(p) = progress {
                    p.add_reused(size);
                    p.emit(dest.display().to_string(), size, size, 0, true, true);
                }
                return Ok(dest);
            }
        } else {
            let actual = tokio::fs::metadata(&dest)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            if let Some(p) = progress {
                p.add_reused(actual);
                p.emit(dest.display().to_string(), actual, actual, 0, true, true);
            }
            return Ok(dest);
        }
    }

    let urls: Vec<String> = match &a.exact_url {
        Some(url) => vec![url.clone()],
        None => a
            .bases
            .iter()
            .map(|base| {
                format!(
                    "{}/{}",
                    base.trim_end_matches('/'),
                    a.rel_path.to_string_lossy().replace('\\', "/")
                )
            })
            .collect(),
    };

    let mut last_error: Option<anyhow::Error> = None;
    for url in &urls {
        match download_file(url, &dest, a.sha1.as_deref(), progress).await {
            Ok(()) => return Ok(dest),
            Err(e) => {
                tracing::debug!("下载 {} 失败: {}", url, e);
                last_error = Some(e);
                let _ = tokio::fs::remove_file(&dest.with_extension("tmp")).await;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("没有可用下载源: {}", dest.display())))
}

#[cfg(test)]
mod tests {
    use super::sha1_hex;

    #[test]
    fn sha1_known_vector() {
        assert_eq!(
            sha1_hex(b"hello world"),
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
        );
    }
}

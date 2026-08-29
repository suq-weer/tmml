use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use futures_util::{stream, StreamExt};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter};
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    appfile::dirs,
    config::MainConfig,
    downloader::{
        deserializer::{AssetsIndexContent, Rule, VersionContent, VersionManifest},
        net::fetch_and_parse_json,
        provider::VER_ALL,
        urls::{RESOURCES_API, VERSION_MANIFEST},
    },
};

pub const DOWNLOAD_PROGRESS_EVENT: &str = "minecraft-download-progress";
pub const DOWNLOAD_FINISHED_EVENT: &str = "minecraft-download-finished";

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum DownloadPhase {
    VersionJson,
    ClientJar,
    Libraries,
    AssetsIndex,
    Logging,
    Assets,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub version_id: String,
    pub phase: DownloadPhase,
    pub name: String,
    pub index: u64,
    pub count: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub speed: u64,
    pub finished: bool,
    pub file_bytes_done: u64,
    pub file_size: u64,
    pub reused: bool,
    pub reused_count: u64,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFinished {
    pub version_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskKind {
    VersionJson,
    ClientJar,
    Library,
    Native,
    AssetsIndex,
    Logging,
    Asset,
}

struct DownloadTask {
    kind: TaskKind,
    url: String,
    dest: PathBuf,
    sha1: Option<String>,
    size: u64,
    legacy: Option<PathBuf>,
}

// | 镜像 URL 重写 |

/// 去掉 URL 的 scheme 与主机名，改用镜像前缀重写路径
fn rewrite_path(official: &str, prefix: &str) -> String {
    match official.split_once("://") {
        Some((_, rest)) => match rest.split_once('/') {
            Some((_, path)) => format!("{}/{}", prefix.trim_end_matches('/'), path),
            None => prefix.trim_end_matches('/').to_string(),
        },
        None => official.to_string(),
    }
}

/// 根据镜像配置将官方 URL 重写为镜像 URL；未配置镜像时原样返回
fn resolve_url(mirror: Option<&str>, kind: TaskKind, official: &str, version_id: &str) -> String {
    let Some(mirror) = mirror.filter(|m| !m.is_empty()) else {
        return official.to_string();
    };
    let mirror = mirror.trim_end_matches('/');
    match kind {
        TaskKind::VersionJson => format!("{}/version/{}/json", mirror, version_id),
        TaskKind::ClientJar => format!("{}/version/{}/client", mirror, version_id),
        TaskKind::Library | TaskKind::Native => {
            rewrite_path(official, &format!("{}/maven", mirror))
        }
        TaskKind::AssetsIndex => rewrite_path(official, mirror),
        TaskKind::Logging => official.to_string(),
        TaskKind::Asset => rewrite_path(official, &format!("{}/assets", mirror)),
    }
}

// | 平台判定 |

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

fn os_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x86",
        "aarch64" | "arm64" => "arm",
        "x86" | "i386" | "i686" => "x86",
        _ => "unknown",
    }
}

/// 判断依赖库的 rules 是否允许当前平台使用（无 rules 时默认允许）
fn rules_allowed(rules: Option<&[Rule]>) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    let mut allowed: bool = true;
    for rule in rules {
        let os_matches = match &rule.os {
            Some(os) => {
                let name_ok: bool = match &os.name {
                    Some(name) => name == os_name(),
                    None => true,
                };
                let arch_ok: bool = match &os.arch {
                    Some(arch) => arch == os_arch(),
                    None => true,
                };
                name_ok && arch_ok
            }
            None => true,
        };
        if os_matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

/// natives 分类器的候选键（按平台 + 架构）
fn native_classifier_candidates() -> Vec<String> {
    let mut candidates: Vec<String> = match std::env::consts::OS {
        "macos" => vec!["natives-macos".to_string(), "natives-osx".to_string()],
        "windows" => vec!["natives-windows".to_string()],
        _ => vec!["natives-linux".to_string()],
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" | "arm64" => Some("arm64"),
        "x86_64" | "amd64" => Some("amd64"),
        "x86" | "i386" | "i686" => Some("x86"),
        _ => None,
    };
    if let Some(arch) = arch {
        let base: String = candidates[0].clone();
        candidates.push(format!("{}-{}", base, arch));
    }
    candidates
}

// | 进度上报 |

#[derive(Clone)]
struct PhaseProgress {
    app: AppHandle,
    version_id: String,
    phase: DownloadPhase,
    count: u64,
    files_done: Arc<AtomicUsize>,
    bytes_done: Arc<AtomicU64>,
    bytes_total: u64,
    reused: Arc<AtomicUsize>,
    last_emit: Arc<AtomicU64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl PhaseProgress {
    fn emit(
        &self,
        name: String,
        file_bytes_done: u64,
        file_size: u64,
        speed: u64,
        finished: bool,
        reused: bool,
    ) {
        let index = (self.files_done.load(Ordering::Relaxed) + 1) as u64;
        let bytes_done = self.bytes_done.load(Ordering::Relaxed) + file_bytes_done;
        let payload = DownloadProgress {
            version_id: self.version_id.clone(),
            phase: self.phase.clone(),
            name,
            index,
            count: self.count,
            bytes_done,
            bytes_total: self.bytes_total,
            speed,
            finished,
            file_bytes_done,
            file_size,
            reused,
            reused_count: self.reused.load(Ordering::Relaxed) as u64,
        };
        if let Err(e) = self.app.emit(DOWNLOAD_PROGRESS_EVENT, &payload) {
            tracing::warn!("发送下载进度事件失败: {}", e);
        }
    }

    /// 节流 emit：同一阶段 150ms 内最多 emit 一次，避免复用/完成大量文件时的事件洪泛
    fn emit_throttled(
        &self,
        name: String,
        file_bytes_done: u64,
        file_size: u64,
        speed: u64,
        finished: bool,
        reused: bool,
    ) {
        let now = now_ms();
        if now.saturating_sub(self.last_emit.load(Ordering::Relaxed)) < 150 {
            return;
        }
        self.last_emit.store(now, Ordering::Relaxed);
        self.emit(name, file_bytes_done, file_size, speed, finished, reused);
    }
}

// | 路径工具 |

async fn versions_dir(dir_name: &str) -> Result<PathBuf> {
    let dir = dirs::dot_minecraft()?.join("versions").join(dir_name);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

fn libraries_dir() -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?.join("libraries"))
}

fn assets_dir() -> Result<PathBuf> {
    Ok(dirs::dot_minecraft()?.join("assets"))
}

// | 主下载器 |

pub struct MinecraftDownloader {
    app: AppHandle,
    config: MainConfig,
    cancel: Arc<AtomicBool>,
    /// 实例文件存放目录名（通常为实例名或版本号）
    dir_name: String,
}

impl MinecraftDownloader {
    pub fn new(app: AppHandle, config: MainConfig, cancel: Arc<AtomicBool>, dir_name: String) -> Self {
        Self {
            app,
            config,
            cancel,
            dir_name,
        }
    }

    fn mirror(&self) -> Option<&str> {
        self.config.mirror_url.as_deref()
    }

    fn concurrency(&self) -> usize {
        self.config.download_concurrency.max(1) as usize
    }

    fn check_cancel(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            bail!("下载已取消");
        }
        Ok(())
    }

    /// 完整流程：查找版本 json -> 下载 version.json -> 下载客户端/依赖库/资源等
    pub async fn download_version(&self, version_id: &str) -> Result<VersionContent> {
        let official_url: String = self.find_version_url(version_id).await?;
        let content: VersionContent = self
            .download_version_json(version_id, &official_url)
            .await?;
        let tasks: Vec<DownloadTask> = self.build_tasks(&content).await?;
        self.run_tasks(version_id, tasks).await?;
        self.download_assets(version_id, &content).await?;
        Ok(content)
    }

    /// 从 version_manifest.json 中查找指定版本的 <version>.json 官方 URL
    async fn find_version_url(&self, version_id: &str) -> Result<String> {
        let mut provider = VER_ALL.lock().await;
        if provider.ver.is_none() {
            let manifest: VersionManifest = fetch_and_parse_json(VERSION_MANIFEST)
                .await
                .context("拉取 version_manifest 失败")?;
            provider.ver = Some(manifest);
        }
        let manifest: &VersionManifest = provider.ver.as_ref().expect("version_manifest 已加载");
        match manifest.versions.iter().find(|v| v.id == version_id) {
            Some(version) => Ok(version.url.clone()),
            None => bail!("版本列表中未找到 {}", version_id),
        }
    }

    /// 下载 <version>.json 到 .minecraft/versions/<id>/<id>.json
    async fn download_version_json(
        &self,
        version_id: &str,
        official_url: &str,
    ) -> Result<VersionContent> {
        let url = resolve_url(
            self.mirror(),
            TaskKind::VersionJson,
            official_url,
            version_id,
        );
        let dest = versions_dir(&self.dir_name)
            .await?
            .join(format!("{}.json", version_id));
        let progress = PhaseProgress {
            app: self.app.clone(),
            version_id: version_id.to_string(),
            phase: DownloadPhase::VersionJson,
            count: 1,
            files_done: Arc::new(AtomicUsize::new(0)),
            bytes_done: Arc::new(AtomicU64::new(0)),
            bytes_total: 0,
            reused: Arc::new(AtomicUsize::new(0)),
            last_emit: Arc::new(AtomicU64::new(0)),
        };
        let task = DownloadTask {
            kind: TaskKind::VersionJson,
            url,
            dest: dest.clone(),
            sha1: None,
            size: 0,
            legacy: None,
        };
        self.download_one(&task, &progress, format!("{}.json", version_id))
            .await?;
        let bytes: Vec<u8> = fs::read(&dest).await?;
        let content: VersionContent =
            serde_json::from_slice(&bytes).context("解析 <version>.json 失败")?;
        Ok(content)
    }

    /// 依据 <version>.json 构建下载任务（客户端、依赖库、natives、资源索引、日志配置）
    async fn build_tasks(&self, content: &VersionContent) -> Result<Vec<DownloadTask>> {
        let version_id = &content.id;
        let mut tasks = Vec::new();

        let Some(client) = &content.downloads.client else {
            bail!("该版本缺少 downloads.client，无法下载客户端");
        };
        let jar_dest = versions_dir(&self.dir_name)
            .await?
            .join(format!("{}.jar", version_id));
        tasks.push(DownloadTask {
            kind: TaskKind::ClientJar,
            url: resolve_url(self.mirror(), TaskKind::ClientJar, &client.url, version_id),
            dest: jar_dest,
            sha1: Some(client.sha1.clone()),
            size: client.size,
            legacy: None,
        });

        for library in &content.libraries {
            if !rules_allowed(library.rules.as_deref()) {
                continue;
            }
            let artifact = &library.downloads.artifact;
            tasks.push(DownloadTask {
                kind: TaskKind::Library,
                url: resolve_url(self.mirror(), TaskKind::Library, &artifact.url, version_id),
                dest: libraries_dir()?.join(&artifact.path),
                sha1: Some(artifact.sha1.clone()),
                size: artifact.size,
                legacy: None,
            });
            if let Some(classifiers) = &library.classifiers {
                for key in native_classifier_candidates() {
                    if let Some(native) = classifiers.get(&key) {
                        tasks.push(DownloadTask {
                            kind: TaskKind::Native,
                            url: resolve_url(
                                self.mirror(),
                                TaskKind::Native,
                                &native.url,
                                version_id,
                            ),
                            dest: libraries_dir()?.join(&native.path),
                            sha1: Some(native.sha1.clone()),
                            size: native.size,
                            legacy: None,
                        });
                        break;
                    }
                }
            }
        }

        let index = &content.assets_index;
        let index_dest = assets_dir()?
            .join("indexes")
            .join(format!("{}.json", index.id));
        tasks.push(DownloadTask {
            kind: TaskKind::AssetsIndex,
            url: resolve_url(self.mirror(), TaskKind::AssetsIndex, &index.url, version_id),
            dest: index_dest,
            sha1: Some(index.sha1.clone()),
            size: index.size,
            legacy: None,
        });

        let logging = &content.logging.client;
        let logging_dest = assets_dir()?.join("log_configs").join(&logging.file.id);
        tasks.push(DownloadTask {
            kind: TaskKind::Logging,
            url: resolve_url(
                self.mirror(),
                TaskKind::Logging,
                &logging.file.url,
                version_id,
            ),
            dest: logging_dest,
            sha1: Some(logging.file.sha1.clone()),
            size: logging.file.size,
            legacy: None,
        });

        Ok(tasks)
    }

    /// 分阶段执行客户端、依赖库、资源索引与日志配置的下载
    async fn run_tasks(&self, version_id: &str, tasks: Vec<DownloadTask>) -> Result<()> {
        let mut clients = Vec::new();
        let mut libraries = Vec::new();
        let mut indexes = Vec::new();
        let mut loggings = Vec::new();
        for task in tasks {
            match task.kind {
                TaskKind::ClientJar => clients.push(task),
                TaskKind::Library | TaskKind::Native => libraries.push(task),
                TaskKind::AssetsIndex => indexes.push(task),
                TaskKind::Logging => loggings.push(task),
                _ => {}
            }
        }
        self.run_phase(version_id, DownloadPhase::ClientJar, clients)
            .await?;
        self.run_phase(version_id, DownloadPhase::Libraries, libraries)
            .await?;
        self.run_phase(version_id, DownloadPhase::AssetsIndex, indexes)
            .await?;
        self.run_phase(version_id, DownloadPhase::Logging, loggings)
            .await?;
        Ok(())
    }

    /// 有界并行的阶段下载
    async fn run_phase(
        &self,
        version_id: &str,
        phase: DownloadPhase,
        tasks: Vec<DownloadTask>,
    ) -> Result<()> {
        if tasks.is_empty() {
            return Ok(());
        }
        let bytes_total: u64 = tasks.iter().map(|t| t.size).sum();
        let progress = PhaseProgress {
            app: self.app.clone(),
            version_id: version_id.to_string(),
            phase,
            count: tasks.len() as u64,
            files_done: Arc::new(AtomicUsize::new(0)),
            bytes_done: Arc::new(AtomicU64::new(0)),
            bytes_total,
            reused: Arc::new(AtomicUsize::new(0)),
            last_emit: Arc::new(AtomicU64::new(0)),
        };
        let results: Vec<Result<()>> = stream::iter(tasks)
            .map(|task| {
                let progress = progress.clone();
                let name = task.dest.display().to_string();
                async move { self.download_one(&task, &progress, name).await }
            })
            .buffer_unordered(self.concurrency())
            .collect()
            .await;
        for result in results {
            result?;
        }
        // 阶段结束：无条件 emit 一次 finished=true，携带累计复用数，保证前端最终状态一致
        progress.emit(String::new(), 0, 0, 0, true, false);
        Ok(())
    }

    /// 下载资源索引后，解析并下载所有资源文件（含 virtual/legacy 副本）
    async fn download_assets(&self, version_id: &str, content: &VersionContent) -> Result<()> {
        let index = &content.assets_index;
        let index_path = assets_dir()?
            .join("indexes")
            .join(format!("{}.json", index.id));
        let data = fs::read(&index_path).await.context("读取资源索引失败")?;
        let assets_index: AssetsIndexContent =
            serde_json::from_slice(&data).context("解析资源索引失败")?;

        let mut tasks = Vec::new();
        let legacy_root = if assets_index.virtual_ {
            Some(assets_dir()?.join("virtual").join("legacy"))
        } else {
            None
        };
        for (legacy_path, object) in assets_index.objects {
            let hash = object.hash;
            let url = format!("{}{}/{}", RESOURCES_API, &hash[..2], hash);
            tasks.push(DownloadTask {
                kind: TaskKind::Asset,
                url,
                dest: assets_dir()?.join("objects").join(&hash[..2]).join(&hash),
                sha1: Some(hash),
                size: object.size,
                legacy: legacy_root.clone().map(|root| root.join(&legacy_path)),
            });
        }
        self.run_phase(version_id, DownloadPhase::Assets, tasks)
            .await?;
        Ok(())
    }

    /// 单个文件下载：跳过已存在且大小匹配的文件（复用），失败自动重试
    async fn download_one(
        &self,
        task: &DownloadTask,
        progress: &PhaseProgress,
        name: String,
    ) -> Result<()> {
        self.check_cancel()?;
        if file_valid(&task.dest, Some(task.size)).await {
            if let Some(legacy) = &task.legacy {
                copy_legacy(&task.dest, legacy).await;
            }
            progress.bytes_done.fetch_add(task.size, Ordering::Relaxed);
            progress.files_done.fetch_add(1, Ordering::Relaxed);
            progress.reused.fetch_add(1, Ordering::Relaxed);
            progress.emit_throttled(name, task.size, task.size, 0, true, true);
            return Ok(());
        }

        let mut last_error: Option<anyhow::Error> = None;
        for attempt in 0..3 {
            self.check_cancel()?;
            match self.download_once(task, progress, name.clone()).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("下载 {} 失败(第 {} 次): {}", name, attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("下载失败: {}", name)))
    }

    /// 单次下载：流式写入临时文件并实时计算 sha1
    async fn download_once(
        &self,
        task: &DownloadTask,
        progress: &PhaseProgress,
        name: String,
    ) -> Result<()> {
        self.check_cancel()?;
        if let Some(parent) = task.dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        let tmp = task.dest.with_extension("tmp");

        let client = reqwest::Client::new();
        let response = client.get(&task.url).send().await?;
        if !response.status().is_success() {
            bail!("HTTP {} {}", response.status(), task.url);
        }

        let mut file = fs::File::create(&tmp).await?;
        let mut stream = response.bytes_stream();
        let mut hasher = Sha1::new();
        let mut written: u64 = 0;
        let mut last_emit = Instant::now();
        let mut bytes_since_last: u64 = 0;

        while let Some(chunk) = stream.next().await {
            if self.cancel.load(Ordering::Relaxed) {
                let _ = fs::remove_file(&tmp).await;
                bail!("下载已取消");
            }
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            written += chunk.len() as u64;
            bytes_since_last += chunk.len() as u64;

            if last_emit.elapsed() >= Duration::from_millis(150) {
                let speed =
                    (bytes_since_last as f64 / last_emit.elapsed().as_secs_f64()).round() as u64;
                progress.emit(name.clone(), written, task.size, speed, false, false);
                last_emit = Instant::now();
                bytes_since_last = 0;
            }
        }
        file.flush().await?;

        if let Some(expected) = &task.sha1 {
            let digest = hex::encode(hasher.finalize());
            if !digest.eq_ignore_ascii_case(expected) {
                let _ = fs::remove_file(&tmp).await;
                bail!(
                    "sha1 校验失败: {} (期望 {} 实际 {})",
                    name,
                    expected,
                    digest
                );
            }
        }

        fs::rename(&tmp, &task.dest).await?;

        if let Some(legacy) = &task.legacy {
            copy_legacy(&task.dest, legacy).await;
        }
        progress.bytes_done.fetch_add(task.size, Ordering::Relaxed);
        progress.files_done.fetch_add(1, Ordering::Relaxed);
        progress.emit_throttled(name, written, task.size, 0, true, false);
        Ok(())
    }
}

/// 判断文件是否可复用：仅按“存在 + 大小匹配”判定，不再整文件读盘计算 sha1，
/// 以大幅减少 HDD 上的随机读（assets 按内容哈希寻址，路径即哈希，大小匹配即足够）
async fn file_valid(path: &Path, expected_size: Option<u64>) -> bool {
    if !path.exists() {
        return false;
    }
    match expected_size {
        Some(size) => match fs::metadata(path).await {
            Ok(meta) => meta.len() == size,
            Err(_) => false,
        },
        None => true,
    }
}

async fn copy_legacy(src: &Path, legacy: &Path) {
    if legacy.exists() {
        return;
    }
    if let Some(parent) = legacy.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    if let Err(e) = fs::copy(src, legacy).await {
        tracing::warn!(
            "复制资源副本失败 {} -> {}: {}",
            src.display(),
            legacy.display(),
            e
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloader::deserializer::{Rule, OS};

    const MIRROR: &str = "https://bmclapi2.bangbang93.com";

    #[test]
    fn resolve_without_mirror_returns_original() {
        let url = "https://libraries.minecraft.net/com/mojang/logging/1.2.7/logging-1.2.7.jar";
        assert_eq!(resolve_url(None, TaskKind::Library, url, "1.21.1"), url);
    }

    #[test]
    fn resolve_library_to_maven() {
        let url = "https://libraries.minecraft.net/com/mojang/logging/1.2.7/logging-1.2.7.jar";
        assert_eq!(
            resolve_url(Some(MIRROR), TaskKind::Library, url, "1.21.1"),
            "https://bmclapi2.bangbang93.com/maven/com/mojang/logging/1.2.7/logging-1.2.7.jar"
        );
    }

    #[test]
    fn resolve_asset_to_assets() {
        let url =
            "https://resources.download.minecraft.net/b6/b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356";
        assert_eq!(
            resolve_url(Some(MIRROR), TaskKind::Asset, url, "1.21.1"),
            "https://bmclapi2.bangbang93.com/assets/b6/b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356"
        );
    }

    #[test]
    fn resolve_version_json_and_client() {
        assert_eq!(
            resolve_url(
                Some(MIRROR),
                TaskKind::VersionJson,
                "https://piston-meta.mojang.com/v1/packages/xxx/1.21.1.json",
                "1.21.1"
            ),
            "https://bmclapi2.bangbang93.com/version/1.21.1/json"
        );
        assert_eq!(
            resolve_url(
                Some(MIRROR),
                TaskKind::ClientJar,
                "https://piston-data.mojang.com/v1/objects/yyy/client.jar",
                "1.21.1"
            ),
            "https://bmclapi2.bangbang93.com/version/1.21.1/client"
        );
    }

    #[test]
    fn rules_default_allow() {
        assert!(rules_allowed(None));
        assert!(rules_allowed(Some(&[])));
    }

    #[test]
    fn rules_current_os_control() {
        let os = Some(OS {
            name: Some(os_name().to_string()),
            arch: None,
        });
        let allow = vec![Rule {
            action: "allow".into(),
            features: None,
            os: os.clone(),
        }];
        let disallow = vec![Rule {
            action: "disallow".into(),
            features: None,
            os,
        }];
        assert!(rules_allowed(Some(&allow)));
        assert!(!rules_allowed(Some(&disallow)));
    }

    #[test]
    fn native_candidates_nonempty() {
        assert!(!native_classifier_candidates().is_empty());
    }

    /// 复现 26.2 下载失败：其 downloads 缺少 client_mappings/server_mappings，
    /// 真实文件必须能成功解析为 VersionContent
    #[tokio::test]
    async fn real_version_262_deserializes() {
        let manifest: VersionManifest = fetch_and_parse_json(VERSION_MANIFEST)
            .await
            .expect("拉取 version_manifest 失败");
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == "26.2")
            .expect("version_manifest 中未找到 26.2");
        let content: VersionContent = fetch_and_parse_json(&entry.url)
            .await
            .expect("解析 26.2.json 失败");
        assert_eq!(content.id, "26.2");
        assert!(
            content.downloads.client.is_some(),
            "26.2 应有 downloads.client"
        );
        assert!(
            content.downloads.client_mappings.is_none(),
            "26.2 不应有 client_mappings"
        );
    }
}

//! 加载器安装上下文：一次「原版已下载、待装加载器」的全部本地路径与信息。

use std::path::PathBuf;

use tauri::AppHandle;

use crate::downloader::{
    deserializer::VersionContent,
    minecraft::{DownloadPhase, PhaseProgress},
};

pub struct InstallContext {
    /// 实例目录 versions/<dir>
    pub game_dir: PathBuf,
    /// .minecraft/libraries
    pub libraries_dir: PathBuf,
    /// .minecraft（供处理器 {ROOT} 变量使用，{ROOT}/libraries 即库目录）
    pub dot_minecraft: PathBuf,
    /// 原版客户端 versions/<dir>/<version_id>.jar
    pub client_jar: PathBuf,
    /// java 可执行文件（绝对路径或仅 java）
    pub java_bin: String,
    /// 原版版本 JSON（作为合并基底）
    pub base: VersionContent,
    /// Tauri 应用句柄，用于向前端推送安装进度
    pub app: AppHandle,
    /// 进度事件使用的版本号（与下载 Toast 的 versionId 一致）
    pub version_id: String,
}

impl InstallContext {
    /// 处理器/下载用的独立临时目录（安装失败即清理）
    pub fn temp_dir(&self) -> PathBuf {
        self.game_dir.join(".loader-tmp")
    }

    pub fn minecraft_version(&self) -> &str {
        &self.base.id
    }

    /// 构造一个加载器安装阶段的进度上报器，复用原版下载的进度事件格式
    pub(crate) fn progress(
        &self,
        phase: DownloadPhase,
        count: u64,
        bytes_total: u64,
    ) -> PhaseProgress {
        PhaseProgress::new(
            self.app.clone(),
            self.version_id.clone(),
            phase,
            count,
            bytes_total,
        )
    }
}

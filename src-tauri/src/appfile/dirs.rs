use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok, Result};
use directories::ProjectDirs;

pub fn project() -> Result<ProjectDirs> {
    directories::ProjectDirs::from("top", "xiaosuoaa", "tmml")
        .context("无法确定系统项目目录，请检查环境变量")
}

pub fn config() -> Result<PathBuf> {
    let binding: ProjectDirs = project()?;
    let config: &Path = binding.config_dir();
    fs::create_dir_all(config)?;
    return Ok(config.to_path_buf());
}

pub fn log() -> Result<PathBuf> {
    let log = config()?.join("logs");
    fs::create_dir(&log)?;
    return Ok(log);
}

pub fn dot_minecraft() -> Result<PathBuf> {
    let dot = config()?.join(".minecraft");
    fs::create_dir_all(&dot)?;
    return Ok(dot);
}

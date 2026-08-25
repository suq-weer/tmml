pub mod dirs;
pub mod file;

use std::path::PathBuf;

use crate::config::MainConfig;

pub fn read_data_file(path: PathBuf) -> anyhow::Result<MainConfig> {
    file::read_and_parse_json::<MainConfig>(path)
}

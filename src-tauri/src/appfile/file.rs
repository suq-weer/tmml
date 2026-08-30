use anyhow::Result;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub fn read_and_parse_json<T: DeserializeOwned + Debug>(path: PathBuf) -> Result<T> {
    let file: File = File::open(&path)?;
    let reader: BufReader<File> = BufReader::new(file);
    let data: T = serde_json::from_reader(reader)?;

    tracing::debug!("成功解析本地 JSON: {:#?}", &path);
    Ok(data)
}

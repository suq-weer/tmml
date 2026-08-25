use anyhow::{Ok, Result};
use reqwest::{self, Client};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use tracing;

pub async fn fetch_and_parse_json<T: DeserializeOwned + Debug>(url: &str) -> Result<T> {
    let client = Client::new();

    // 直接利用 reqwest 的泛型 json() 方法，它内部会自动匹配传入的 T 类型
    let data: reqwest::Response = client.get(url).send().await?;
    let data_text = data.text().await?;

    match serde_json::from_str::<T>(&data_text) {
        std::result::Result::Ok(v) => {
            return Ok(v);
        },
        Err(e) => {
            tracing::error!("反序列化失败，第 {} 行第 {} 列: {}", e.line(), e.column(), e);
            tracing::error!("响应体开头: {}", &data_text.chars().take(200).collect::<String>());
            anyhow::bail!("解析 version.json 时遇到错误");
        }
    }
}

use anyhow::Result;
use reqwest::{self, Client};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use tracing;

pub async fn fetch_and_parse_json<T: DeserializeOwned + Debug>(url: &str) -> Result<T> {
    let client = Client::new();

    // 直接利用 reqwest 的泛型 json() 方法，它内部会自动匹配传入的 T 类型
    let data: T = client.get(url).send().await?.json().await?;

    tracing::debug!("成功解析 JSON: {:#?}", data);

    Ok(data)
}

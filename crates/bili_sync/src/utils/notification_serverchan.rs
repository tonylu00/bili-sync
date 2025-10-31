use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
struct ServerChanRequest {
    title: String,
    desp: String,
}

#[derive(Deserialize)]
struct ServerChanResponse {
    #[serde(deserialize_with = "deserialize_code")]
    code: i32,
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

pub(super) async fn send(client: &Client, key: &str, title: &str, desp: &str) -> Result<()> {
    let url = format!("https://sctapi.ftqq.com/{}.send", key);
    let request = ServerChanRequest {
        title: title.to_string(),
        desp: desp.to_string(),
    };

    let response = client.post(&url).json(&request).send().await?;
    let response_text = response.text().await?;
    let server_response: ServerChanResponse = serde_json::from_str(&response_text)
        .map_err(|e| anyhow!("解析响应失败: {}, 响应内容: {}", e, response_text))?;

    if server_response.code == 0 {
        Ok(())
    } else {
        Err(anyhow!("Server酱返回错误: {}", server_response.message))
    }
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| D::Error::custom("code is not a valid i32")),
        serde_json::Value::String(s) => s
            .parse::<i32>()
            .map_err(|_| D::Error::custom(format!("code string '{}' is not a valid i32", s))),
        _ => Err(D::Error::custom("code must be a number or string")),
    }
}

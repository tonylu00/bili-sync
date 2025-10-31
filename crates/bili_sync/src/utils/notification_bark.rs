use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Serialize;
use std::str::FromStr;

use crate::config::BarkDefaults;
use crate::utils::notification::NotificationMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarkLevel {
    Active,
    TimeSensitive,
    Passive,
    Critical,
}

impl BarkLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            BarkLevel::Active => "active",
            BarkLevel::TimeSensitive => "timeSensitive",
            BarkLevel::Passive => "passive",
            BarkLevel::Critical => "critical",
        }
    }
}

impl FromStr for BarkLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().replace('-', "").replace('_', "").as_str() {
            "active" => Ok(BarkLevel::Active),
            "timesensitive" => Ok(BarkLevel::TimeSensitive),
            "passive" => Ok(BarkLevel::Passive),
            "critical" => Ok(BarkLevel::Critical),
            other => Err(anyhow!("未知的 Bark level: {}", other)),
        }
    }
}

#[derive(Debug)]
pub struct DeviceKeySelection {
    pub device_key: Option<String>,
    pub device_keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BarkPayload {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_key: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub device_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "call")]
    pub call: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "autoCopy")]
    pub auto_copy: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isArchive")]
    pub is_archive: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<u8>,
}

impl BarkPayload {
    pub fn from_message(
        message: &NotificationMessage,
        defaults: &BarkDefaults,
        keys: DeviceKeySelection,
    ) -> Result<Self> {
        let level = if let Some(level) = message.level {
            Some(level.as_str().to_string())
        } else if let Some(default_level) = defaults.level.as_ref() {
            Some(BarkLevel::from_str(default_level)?.as_str().to_string())
        } else {
            None
        };

        let copy = message
            .copy
            .clone()
            .or_else(|| defaults.copy.as_ref().map(|value| value.to_string()));

        let sound = message
            .sound
            .clone()
            .or_else(|| defaults.sound.as_ref().map(|value| value.to_string()));

        let icon = message
            .icon
            .clone()
            .or_else(|| defaults.icon.as_ref().map(|value| value.to_string()));

        let group = message
            .group
            .clone()
            .or_else(|| defaults.group.as_ref().map(|value| value.to_string()));

        let ciphertext = message
            .ciphertext
            .clone()
            .or_else(|| defaults.ciphertext.as_ref().map(|value| value.to_string()));

        let url = message
            .url
            .clone()
            .or_else(|| defaults.url.as_ref().map(|value| value.to_string()));

        let action = message
            .action
            .clone()
            .or_else(|| defaults.action.as_ref().map(|value| value.to_string()));

        let id = message
            .id
            .clone()
            .or_else(|| defaults.id.as_ref().map(|value| value.to_string()));

        Ok(Self {
            title: message.title.clone(),
            subtitle: message
                .subtitle
                .clone()
                .or_else(|| defaults.subtitle.as_ref().map(|value| value.to_string())),
            body: message.body_plain.clone(),
            device_key: keys.device_key,
            device_keys: keys.device_keys,
            level,
            volume: message.volume.or(defaults.volume),
            badge: message.badge.or(defaults.badge),
            call: message.call.or(defaults.call).map(bool_to_flag),
            auto_copy: message.auto_copy.or(defaults.auto_copy).map(bool_to_flag),
            copy,
            sound,
            icon,
            group,
            ciphertext,
            is_archive: message.is_archive.or(defaults.is_archive).map(bool_to_flag),
            url,
            action,
            id,
            delete: message.delete.or(defaults.delete).map(bool_to_flag),
        })
    }
}

pub(super) async fn send(client: &Client, server: &str, payload: BarkPayload) -> Result<()> {
    let url = format!("{}/push", server.trim_end_matches('/'));
    let response = client.post(&url).json(&payload).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(anyhow!("Bark返回错误: {} {}", status, text))
    }
}

fn bool_to_flag(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

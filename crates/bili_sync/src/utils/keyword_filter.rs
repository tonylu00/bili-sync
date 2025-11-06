use anyhow::Result;
use tracing::warn;

fn sanitize_keywords_list(list: &[String]) -> Vec<String> {
    list.iter()
        .map(|kw| kw.trim())
        .filter(|kw| !kw.is_empty())
        .map(|kw| kw.to_string())
        .collect()
}

/// Serialize keyword list into JSON string, returning `None` when the list is empty after trimming.
pub fn serialize_keywords(keywords: &Option<Vec<String>>) -> Result<Option<String>> {
    if let Some(list) = keywords {
        let sanitized = sanitize_keywords_list(list);
        if sanitized.is_empty() {
            Ok(None)
        } else {
            Ok(Some(serde_json::to_string(&sanitized)?))
        }
    } else {
        Ok(None)
    }
}

/// Deserialize keyword JSON string and trim empty entries.
pub fn deserialize_keywords(raw: &Option<String>) -> Option<Vec<String>> {
    raw.as_ref()
        .and_then(|json| match serde_json::from_str::<Vec<String>>(json) {
            Ok(list) => {
                let sanitized = sanitize_keywords_list(&list);
                if sanitized.is_empty() {
                    None
                } else {
                    Some(sanitized)
                }
            }
            Err(err) => {
                warn!("解析关键词过滤配置失败: {}", err);
                None
            }
        })
}

/// Check whether a title passes include/exclude keyword filters. All comparisons are case-insensitive.
pub fn matches_keyword_filters(
    title: &str,
    include_keywords: Option<&[String]>,
    exclude_keywords: Option<&[String]>,
) -> bool {
    let title_lower = title.to_lowercase();

    if let Some(include) = include_keywords {
        if !include.is_empty() && !include.iter().any(|kw| title_lower.contains(kw)) {
            return false;
        }
    }

    if let Some(exclude) = exclude_keywords {
        if exclude.iter().any(|kw| title_lower.contains(kw)) {
            return false;
        }
    }

    true
}

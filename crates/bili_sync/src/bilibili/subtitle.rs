use std::fmt::Display;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct SubTitlesInfo {
    pub subtitles: Vec<SubTitleInfo>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleInfo {
    pub lan: String,
    pub subtitle_url: String,
    #[serde(default)]
    pub lan_doc: Option<String>,
}

pub struct SubTitle {
    pub lan: String,
    pub body: SubTitleBody,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleBody(pub Vec<SubTitleItem>);

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleItem {
    from: f64,
    to: f64,
    content: String,
}

impl SubTitleInfo {
    pub fn is_ai_sub(&self) -> bool {
        // ai： aisubtitle.hdslb.com/bfs/ai_subtitle/xxxx
        // 非 ai： aisubtitle.hdslb.com/bfs/subtitle/xxxx
        if self.lan.starts_with("ai-") {
            return true;
        }
        if let Some(doc) = &self.lan_doc {
            let doc = doc.trim();
            if doc.contains("自动") {
                return true;
            }
            let lower = doc.to_ascii_lowercase();
            if lower.contains("auto") || lower.contains("ai") {
                return true;
            }
        }
        self.subtitle_url.contains("ai_subtitle")
    }

    pub fn normalized_lan(&self) -> String {
        let trimmed = self.lan.trim();
        if let Some(rest) = trimmed.strip_prefix("ai-") {
            let rest = rest.trim();
            if rest.is_empty() {
                trimmed.to_string()
            } else {
                rest.to_string()
            }
        } else {
            trimmed.to_string()
        }
    }
}

impl Display for SubTitleBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, item) in self.0.iter().enumerate() {
            writeln!(f, "{}", idx + 1)?;
            writeln!(f, "{} --> {}", format_time(item.from), format_time(item.to))?;
            writeln!(f, "{}", item.content)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

fn format_time(time: f64) -> String {
    let (second, millisecond) = (time.trunc(), (time.fract() * 1e3) as u32);
    let (hour, minute, second) = (
        (second / 3600.0) as u32,
        ((second % 3600.0) / 60.0) as u32,
        (second % 60.0) as u32,
    );
    format!("{:02}:{:02}:{:02},{:03}", hour, minute, second, millisecond)
}

#[cfg(test)]
mod tests {
    use super::{SubTitleBody, SubTitleInfo, SubTitleItem};

    #[test]
    fn test_format_time() {
        // float 解析会有精度问题，但误差几毫秒应该不太关键
        // 想再健壮一点就得手写 serde_json 解析拆分秒和毫秒，然后分别处理了
        let testcases = [
            (0.0, "00:00:00,000"),
            (1.5, "00:00:01,500"),
            (206.45, "00:03:26,449"),
            (360001.23, "100:00:01,229"),
        ];
        for (time, expect) in testcases.iter() {
            assert_eq!(super::format_time(*time), *expect);
        }
    }

    #[test]
    fn subtitle_display_starts_from_one() {
        let body = SubTitleBody(vec![
            SubTitleItem {
                from: 0.0,
                to: 1.5,
                content: "Hello".to_string(),
            },
            SubTitleItem {
                from: 1.5,
                to: 3.0,
                content: "World".to_string(),
            },
        ]);

        let rendered = body.to_string();
        let expected = "1\n00:00:00,000 --> 00:00:01,500\nHello\n\n2\n00:00:01,500 --> 00:00:03,000\nWorld\n\n";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn detect_ai_subtitle_by_lan_doc() {
        let ai_info = SubTitleInfo {
            lan: "zh".to_string(),
            subtitle_url: "https://aisubtitle.hdslb.com/bfs/subtitle/test.json".to_string(),
            lan_doc: Some("中文（自动生成）".to_string()),
        };
        assert!(ai_info.is_ai_sub());

        let normal_info = SubTitleInfo {
            lan: "zh-CN".to_string(),
            subtitle_url: "https://aisubtitle.hdslb.com/bfs/subtitle/test.json".to_string(),
            lan_doc: Some("中文".to_string()),
        };
        assert!(!normal_info.is_ai_sub());
    }

    #[test]
    fn normalized_lan_strips_ai_prefix() {
        let info = SubTitleInfo {
            lan: "ai-zh-CN".to_string(),
            subtitle_url: "https://example.com/subtitle.json".to_string(),
            lan_doc: None,
        };
        assert_eq!(info.normalized_lan(), "zh-CN");

        let no_prefix = SubTitleInfo {
            lan: "en-US".to_string(),
            subtitle_url: "https://example.com/subtitle.json".to_string(),
            lan_doc: None,
        };
        assert_eq!(no_prefix.normalized_lan(), "en-US");
    }
}

use regex::Regex;

/// 番剧名称提取器，用于从完整的番剧标题中提取基础系列名称和季度信息
pub struct BangumiNameExtractor;

impl BangumiNameExtractor {
    /// 从番剧标题中提取基础系列名称和季度信息
    ///
    /// # 参数
    /// - `title`: 完整的番剧标题，例如 "灵笼 第二季"
    /// - `season_title`: 可选的季度标题，例如 "第二季"
    ///
    /// # 返回值
    /// 返回元组 (基础系列名称, 季度编号)
    /// 例如：("灵笼", 2)
    pub fn extract_series_name_and_season(title: &str, season_title: Option<&str>) -> (String, u32) {
        // 如果提供了 season_title，优先使用它来提取
        if let Some(season_part) = season_title {
            let base_name = title.replace(season_part, "").trim().to_string();
            let season_number = Self::extract_season_number(season_part).unwrap_or(1);
            return (base_name, season_number);
        }

        // 如果没有 season_title，尝试从 title 中识别季度信息
        Self::extract_from_title(title)
    }

    /// 从完整标题中提取系列名称和季度信息
    fn extract_from_title(title: &str) -> (String, u32) {
        // 常见的季度模式
        let patterns = [
            // 中文季度模式：第一季、第二季、第三季等
            r"(.+?)\s*第([一二三四五六七八九十\d]+)季\s*",
            // 英文季度模式：S1、S2、Season 1等
            r"(.+?)\s*S(\d+)\s*$",
            r"(.+?)\s*Season\s*(\d+)\s*$",
            // 日文季度模式
            r"(.+?)\s*第(\d+)期\s*",
            // 其他可能的模式
            r"(.+?)\s*(\d+)\s*$",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(title) {
                    let base_name = captures.get(1).map_or("", |m| m.as_str()).trim().to_string();
                    let season_str = captures.get(2).map_or("1", |m| m.as_str());
                    let season_number = Self::parse_season_number(season_str);

                    if !base_name.is_empty() {
                        return (base_name, season_number);
                    }
                }
            }
        }

        // 如果没有匹配到任何模式，返回原标题和季度1
        (title.trim().to_string(), 1)
    }

    /// 从季度字符串中提取季度数字
    fn extract_season_number(season_str: &str) -> Option<u32> {
        // 中文数字映射
        let chinese_numbers = [
            ("一", 1),
            ("二", 2),
            ("三", 3),
            ("四", 4),
            ("五", 5),
            ("六", 6),
            ("七", 7),
            ("八", 8),
            ("九", 9),
            ("十", 10),
        ];

        // 尝试直接解析数字
        if let Some(number) = Self::extract_number_from_string(season_str) {
            return Some(number);
        }

        // 尝试中文数字
        for (chinese, number) in &chinese_numbers {
            if season_str.contains(chinese) {
                return Some(*number);
            }
        }

        None
    }

    /// 解析季度数字（支持中文和阿拉伯数字）
    fn parse_season_number(season_str: &str) -> u32 {
        // 尝试直接解析阿拉伯数字
        if let Some(number) = Self::extract_number_from_string(season_str) {
            return number;
        }

        // 尝试中文数字
        let chinese_numbers = [
            ("一", 1),
            ("二", 2),
            ("三", 3),
            ("四", 4),
            ("五", 5),
            ("六", 6),
            ("七", 7),
            ("八", 8),
            ("九", 9),
            ("十", 10),
        ];

        for (chinese, number) in &chinese_numbers {
            if season_str.contains(chinese) {
                return *number;
            }
        }

        // 默认返回1
        1
    }

    /// 从字符串中提取数字
    fn extract_number_from_string(s: &str) -> Option<u32> {
        for part in s.split_whitespace() {
            if let Ok(number) = part.parse::<u32>() {
                return Some(number);
            }
        }

        // 尝试提取字符串中的连续数字
        let re = Regex::new(r"\d+").ok()?;
        if let Some(mat) = re.find(s) {
            return mat.as_str().parse().ok();
        }

        None
    }

    /// 生成标准的季度文件夹名称
    ///
    /// # 参数
    /// - `season_number`: 季度编号
    ///
    /// # 返回值
    /// 标准的季度文件夹名称，例如 "Season 01"、"Season 02"
    pub fn generate_season_folder_name(season_number: u32) -> String {
        format!("Season {:02}", season_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_with_season_title() {
        let (base_name, season) = BangumiNameExtractor::extract_series_name_and_season("灵笼 第二季", Some("第二季"));
        assert_eq!(base_name, "灵笼");
        assert_eq!(season, 2);
    }

    #[test]
    fn test_extract_chinese_season() {
        let (base_name, season) = BangumiNameExtractor::extract_series_name_and_season("进击的巨人 第三季", None);
        assert_eq!(base_name, "进击的巨人");
        assert_eq!(season, 3);
    }

    #[test]
    fn test_extract_english_season() {
        let (base_name, season) = BangumiNameExtractor::extract_series_name_and_season("Attack on Titan S2", None);
        assert_eq!(base_name, "Attack on Titan");
        assert_eq!(season, 2);
    }

    #[test]
    fn test_extract_season_folder_name() {
        assert_eq!(BangumiNameExtractor::generate_season_folder_name(1), "Season 01");
        assert_eq!(BangumiNameExtractor::generate_season_folder_name(12), "Season 12");
    }

    #[test]
    fn test_no_season_info() {
        let (base_name, season) = BangumiNameExtractor::extract_series_name_and_season("鬼灭之刃", None);
        assert_eq!(base_name, "鬼灭之刃");
        assert_eq!(season, 1);
    }
}

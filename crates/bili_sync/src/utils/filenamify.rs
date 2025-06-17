macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).expect("invalid regex"))
    }};
}

pub fn filenamify<S: AsRef<str>>(input: S) -> String {
    // Windows不允许的字符：< > : " / \ | ? *
    // Unicode控制字符：\u{0000}-\u{001F} \u{007F} \u{0080}-\u{009F}
    let reserved = regex!("[<>:\"/\\\\|?*\u{0000}-\u{001F}\u{007F}\u{0080}-\u{009F}]+");
    
    // Windows保留名称：CON, PRN, AUX, NUL, COM1-COM9, LPT1-LPT9（不区分大小写）
    let windows_reserved = regex!("^(con|prn|aux|nul|com\\d|lpt\\d)$");
    
    // 文件名开头和结尾不能是点号
    let outer_periods = regex!("^\\.+|\\.+$");
    
    // 全角字符映射
    let fullwidth_colon = regex!("："); // 全角冒号 → 半角冒号
    let fullwidth_space = regex!("　"); // 全角空格 → 半角空格
    let angle_brackets = regex!("[《》]"); // 角括号 → 方括号
    
    // 其他可能有问题的字符
    let problematic_chars = regex!("[★☆♪♫♬♩♭♮♯※〈〉〔〕【】『』〖〗‖§¶°±×÷≈≠≤≥∞∴∵∠⊥∥∧∨∩∪⊂⊃⊆⊇∈∉∃∀]");
    
    let replacement = "_";
    let space_replacement = " ";
    let bracket_replacement_left = "[";
    let bracket_replacement_right = "]";
    let paren_replacement_left = "(";
    let paren_replacement_right = ")";
    let colon_replacement = "-";

    let mut input = input.as_ref().to_string();
    
    // 1. 处理全角字符映射
    input = fullwidth_colon.replace_all(&input, colon_replacement).into_owned();
    input = fullwidth_space.replace_all(&input, space_replacement).into_owned();
    input = angle_brackets.replace_all(&input, replacement).into_owned();
    
    // 2. 处理全角括号
    input = input.replace('「', bracket_replacement_left);
    input = input.replace('」', bracket_replacement_right);
    input = input.replace('（', paren_replacement_left);
    input = input.replace('）', paren_replacement_right);
    
    // 3. 处理其他有问题的字符
    input = problematic_chars.replace_all(&input, replacement).into_owned();
    
    // 4. 处理Windows保留字符
    input = reserved.replace_all(&input, replacement).into_owned();
    
    // 5. 处理开头和结尾的点号
    input = outer_periods.replace_all(&input, replacement).into_owned();
    
    // 6. 检查Windows保留名称
    if windows_reserved.is_match(&input.to_lowercase()) {
        input.push_str(replacement);
    }
    
    // 7. 去除多余的连续下划线和空格
    let cleanup_underscores = regex!("_{2,}"); // 多个连续下划线 → 单个下划线
    let cleanup_spaces = regex!(" {2,}"); // 多个连续空格 → 单个空格
    let cleanup_mixed = regex!("[_ ]{3,}"); // 混合的空格和下划线 → 单个下划线
    
    input = cleanup_underscores.replace_all(&input, "_").into_owned();
    input = cleanup_spaces.replace_all(&input, " ").into_owned();
    input = cleanup_mixed.replace_all(&input, "_").into_owned();
    
    // 8. 去除开头和结尾的空格和下划线
    input = input.trim_matches(|c| c == ' ' || c == '_').to_string();
    
    // 9. 确保文件名不为空
    if input.is_empty() {
        input = "unnamed".to_string();
    }
    
    // 10. 限制文件名长度（Windows文件名最大255字符）
    if input.len() > 200 {
        input = input.chars().take(200).collect::<String>();
        // 确保不在多字节字符中间截断
        while !input.is_char_boundary(input.len()) {
            input.pop();
        }
        input = input.trim_matches(|c| c == ' ' || c == '_').to_string();
    }

    input
}

#[cfg(test)]
mod tests {
    use super::filenamify;

    #[test]
    fn test_filenamify() {
        assert_eq!(filenamify("foo/bar"), "foo_bar");
        assert_eq!(filenamify("foo//bar"), "foo_bar");
        assert_eq!(filenamify("//foo//bar//"), "_foo_bar_");
        assert_eq!(filenamify("foo\\bar"), "foo_bar");
        assert_eq!(filenamify("foo\\\\\\bar"), "foo_bar");
        assert_eq!(filenamify(r"foo\\bar"), "foo_bar");
        assert_eq!(filenamify(r"foo\\\\\\bar"), "foo_bar");
        assert_eq!(filenamify("////foo////bar////"), "_foo_bar_");
        assert_eq!(filenamify("foo\u{0000}bar"), "foo_bar");
        assert_eq!(filenamify("\"foo<>bar*"), "_foo_bar_");
        assert_eq!(filenamify("."), "_");
        assert_eq!(filenamify(".."), "_");
        assert_eq!(filenamify("./"), "__");
        assert_eq!(filenamify("../"), "__");
        assert_eq!(filenamify("../../foo/bar"), "__.._foo_bar");
        assert_eq!(filenamify("foo.bar."), "foo.bar_");
        assert_eq!(filenamify("foo.bar.."), "foo.bar_");
        assert_eq!(filenamify("foo.bar..."), "foo.bar_");
        assert_eq!(filenamify("con"), "con_");
        assert_eq!(filenamify("com1"), "com1_");
        assert_eq!(filenamify(":nul|"), "_nul_");
        assert_eq!(filenamify("foo/bar/nul"), "foo_bar_nul");
        assert_eq!(filenamify("file:///file.tar.gz"), "file_file.tar.gz");
        assert_eq!(filenamify("http://www.google.com"), "http_www.google.com");
        assert_eq!(
            filenamify("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            "https_www.youtube.com_watch_v=dQw4w9WgXcQ"
        );
    }
}

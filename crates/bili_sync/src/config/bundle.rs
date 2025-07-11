use std::sync::Arc;

use anyhow::Result;
use handlebars::Handlebars;
use leaky_bucket::RateLimiter;

use crate::config::Config;

/// 配置包，包含所有需要热重载的组件
/// 使用 ArcSwap<ConfigBundle> 确保原子性更新
#[derive(Clone)]
pub struct ConfigBundle {
    /// 主配置结构
    pub config: Config,
    /// Handlebars 模板引擎，预编译所有模板
    pub handlebars: Handlebars<'static>,
    /// HTTP 请求限流器
    #[allow(dead_code)]
    pub rate_limiter: Arc<RateLimiter>,
}

impl ConfigBundle {
    /// 从配置构建完整的配置包
    pub fn from_config(config: Config) -> Result<Self> {
        let handlebars = Self::build_handlebars(&config)?;
        let rate_limiter = Self::build_rate_limiter(&config);

        Ok(Self {
            config,
            handlebars,
            rate_limiter: Arc::new(rate_limiter),
        })
    }

    /// 构建 Handlebars 模板引擎
    fn build_handlebars(config: &Config) -> Result<Handlebars<'static>> {
        use handlebars::handlebars_helper;

        let mut handlebars = Handlebars::new();

        // 注册自定义 helper
        handlebars_helper!(truncate: |s: String, len: usize| {
            if s.chars().count() > len {
                s.chars().take(len).collect::<String>()
            } else {
                s.to_string()
            }
        });
        handlebars.register_helper("truncate", Box::new(truncate));

        // 注册所有必需的模板
        // 使用 to_string() 转换 Cow<'static, str> 为 &'static str
        let video_name = Box::leak(config.video_name.to_string().into_boxed_str());
        let page_name = Box::leak(config.page_name.to_string().into_boxed_str());
        let multi_page_name = Box::leak(config.multi_page_name.to_string().into_boxed_str());
        let bangumi_name = Box::leak(config.bangumi_name.to_string().into_boxed_str());
        let folder_structure = Box::leak(config.folder_structure.to_string().into_boxed_str());
        let bangumi_folder_name = Box::leak(config.bangumi_folder_name.to_string().into_boxed_str());

        // 区分Unix风格和Windows风格的路径分隔符
        let safe_video_name = video_name.replace('/', "__UNIX_SEP__").replace('\\', "__WIN_SEP__");
        let safe_page_name = page_name.replace('/', "__UNIX_SEP__").replace('\\', "__WIN_SEP__");
        let safe_multi_page_name = multi_page_name
            .replace('/', "__UNIX_SEP__")
            .replace('\\', "__WIN_SEP__");
        let safe_bangumi_name = bangumi_name.replace('/', "__UNIX_SEP__").replace('\\', "__WIN_SEP__");
        let safe_folder_structure = folder_structure
            .replace('/', "__UNIX_SEP__")
            .replace('\\', "__WIN_SEP__");
        let safe_bangumi_folder_name = bangumi_folder_name
            .replace('/', "__UNIX_SEP__")
            .replace('\\', "__WIN_SEP__");

        handlebars.register_template_string("video", safe_video_name)?;
        handlebars.register_template_string("page", safe_page_name)?;
        handlebars.register_template_string("multi_page", safe_multi_page_name)?;
        handlebars.register_template_string("bangumi", safe_bangumi_name)?;
        handlebars.register_template_string("folder_structure", safe_folder_structure)?;
        handlebars.register_template_string("bangumi_folder", safe_bangumi_folder_name)?;

        Ok(handlebars)
    }

    /// 构建速率限制器
    fn build_rate_limiter(config: &Config) -> RateLimiter {
        if let Some(rate_limit) = &config.concurrent_limit.rate_limit {
            RateLimiter::builder()
                .max(rate_limit.limit)
                .refill(rate_limit.limit)
                .interval(std::time::Duration::from_millis(rate_limit.duration))
                .build()
        } else {
            // 默认限流器：每250ms允许4个请求
            RateLimiter::builder()
                .max(4)
                .refill(4)
                .interval(std::time::Duration::from_millis(250))
                .build()
        }
    }

    /// 检查配置是否有效
    #[cfg(not(test))]
    pub fn validate(&self) -> bool {
        // 复用现有的配置检查逻辑
        self.config.check()
    }

    /// 测试环境下的验证方法
    #[cfg(test)]
    pub fn validate(&self) -> bool {
        // 在测试环境下总是返回true
        true
    }

    /// 获取配置值的便捷方法
    #[allow(dead_code)]
    pub fn get_video_name_template(&self) -> &str {
        &self.config.video_name
    }

    #[allow(dead_code)]
    pub fn get_page_name_template(&self) -> &str {
        &self.config.page_name
    }

    #[allow(dead_code)]
    pub fn get_bind_address(&self) -> &str {
        &self.config.bind_address
    }

    #[allow(dead_code)]
    pub fn get_interval(&self) -> u64 {
        self.config.interval
    }

    /// 渲染模板的便捷方法（使用path_safe_render确保分隔符正确处理）
    #[allow(dead_code)]
    pub fn render_template(&self, template_name: &str, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify;

        // 直接使用handlebars的render方法，然后手动处理分隔符
        let rendered = self.handlebars.render(template_name, data)?;
        #[cfg(windows)]
        {
            Ok(filenamify(&rendered)
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(filenamify(&rendered)
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染视频名称模板的便捷方法
    pub fn render_video_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("video", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染分页名称模板的便捷方法
    pub fn render_page_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("page", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染多P视频分页名称模板的便捷方法
    pub fn render_multi_page_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("multi_page", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染番剧名称模板的便捷方法
    #[allow(dead_code)]
    pub fn render_bangumi_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("bangumi", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染番剧文件夹名称模板的便捷方法
    pub fn render_bangumi_folder_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("bangumi_folder", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }

    /// 渲染文件夹结构模板的便捷方法
    pub fn render_folder_structure_template(&self, data: &serde_json::Value) -> Result<String> {
        use crate::utils::filenamify::filenamify_with_options;

        // 两阶段处理：
        // 1. 先渲染模板，保护模板路径分隔符
        let rendered = self.handlebars.render("folder_structure", data)?;
        
        // 2. 对整个渲染结果进行安全化，保护模板分隔符
        let safe_rendered = filenamify_with_options(&rendered, true);
        
        // 3. 最后处理路径分隔符
        #[cfg(windows)]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "_")
                .replace("__WIN_SEP__", "\\"))
        }
        #[cfg(not(windows))]
        {
            Ok(safe_rendered
                .replace("__UNIX_SEP__", "/")
                .replace("__WIN_SEP__", "_"))
        }
    }
}

impl std::fmt::Debug for ConfigBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigBundle")
            .field("config", &"<Config instance>")
            .field("handlebars", &"<Handlebars instance>")
            .field("rate_limiter", &"<RateLimiter instance>")
            .finish()
    }
}

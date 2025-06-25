use std::io;

use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Request too frequently")]
pub struct DownloadAbortError();

#[derive(Error, Debug)]
#[error("Process page error")]
pub struct ProcessPageError();

/// 错误类型枚举，用于更精确的错误分类
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ErrorType {
    #[error("网络连接错误")]
    Network,
    #[error("权限不足")]
    Permission,
    #[error("认证失败")]
    Authentication,
    #[error("授权失败")]
    Authorization,
    #[error("资源未找到")]
    NotFound,
    #[error("请求过于频繁")]
    RateLimit,
    #[error("服务器内部错误")]
    ServerError,
    #[error("客户端错误")]
    ClientError,
    #[error("解析错误")]
    Parse,
    #[error("超时错误")]
    Timeout,
    #[error("文件系统错误")]
    FileSystem,
    #[error("配置错误")]
    Configuration,
    #[error("风控触发")]
    RiskControl,
    #[error("未知错误")]
    Unknown,
}

/// 分类后的错误信息
#[derive(Error, Debug)]
#[error("{error_type}: {message}")]
pub struct ClassifiedError {
    pub error_type: ErrorType,
    pub message: String,
    pub status_code: Option<u16>,
    pub should_retry: bool,
    pub should_ignore: bool,
}

impl ClassifiedError {
    pub fn new(error_type: ErrorType, message: String) -> Self {
        let (should_retry, should_ignore) = match error_type {
            ErrorType::Network | ErrorType::Timeout | ErrorType::RateLimit => (true, false),
            ErrorType::Permission | ErrorType::FileSystem => (false, true),
            ErrorType::NotFound => (false, true),
            ErrorType::Authentication | ErrorType::Authorization => (false, false),
            ErrorType::RiskControl => (false, false),
            ErrorType::ServerError => (true, false),
            ErrorType::ClientError | ErrorType::Parse | ErrorType::Configuration => (false, false),
            ErrorType::Unknown => (false, false),
        };

        Self {
            error_type,
            message,
            status_code: None,
            should_retry,
            should_ignore,
        }
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn with_retry_policy(mut self, should_retry: bool, should_ignore: bool) -> Self {
        self.should_retry = should_retry;
        self.should_ignore = should_ignore;
        self
    }
}

/// 错误分类器
pub struct ErrorClassifier;

impl ErrorClassifier {
    /// 分析并分类错误
    pub fn classify_error(err: &anyhow::Error) -> ClassifiedError {
        for cause in err.chain() {
            // HTTP 状态码错误
            if let Some(reqwest_err) = cause.downcast_ref::<reqwest::Error>() {
                return Self::classify_reqwest_error(reqwest_err);
            }

            // IO 错误
            if let Some(io_err) = cause.downcast_ref::<io::Error>() {
                return Self::classify_io_error(io_err);
            }

            // B站特定错误
            if let Some(bili_err) = cause.downcast_ref::<crate::bilibili::BiliError>() {
                return Self::classify_bili_error(bili_err);
            }

            // JSON解析错误
            if cause.downcast_ref::<serde_json::Error>().is_some() {
                return ClassifiedError::new(ErrorType::Parse, "JSON解析失败".to_string());
            }
        }

        ClassifiedError::new(ErrorType::Unknown, err.to_string())
    }

    fn classify_reqwest_error(err: &reqwest::Error) -> ClassifiedError {
        if err.is_timeout() {
            return ClassifiedError::new(ErrorType::Timeout, "请求超时".to_string());
        }

        if err.is_connect() {
            return ClassifiedError::new(ErrorType::Network, "网络连接失败".to_string());
        }

        if err.is_decode() || err.is_body() {
            return ClassifiedError::new(ErrorType::Parse, "响应解析失败".to_string());
        }

        if let Some(status) = err.status() {
            let status_code = status.as_u16();
            let error_type = match status_code {
                401 => ErrorType::Authentication,
                403 => ErrorType::Authorization,
                404 => ErrorType::NotFound,
                429 => ErrorType::RateLimit,
                500..=599 => ErrorType::ServerError,
                400..=499 => ErrorType::ClientError,
                _ => ErrorType::Unknown,
            };

            let message = match status_code {
                401 => "认证失败，请检查登录状态".to_string(),
                403 => "权限不足，无法访问该资源".to_string(),
                404 => "请求的资源不存在".to_string(),
                429 => "请求过于频繁，请稍后重试".to_string(),
                500..=599 => "服务器内部错误".to_string(),
                _ => format!("HTTP错误: {}", status_code),
            };

            return ClassifiedError::new(error_type, message).with_status_code(status_code);
        }

        ClassifiedError::new(ErrorType::Network, "网络请求失败".to_string())
    }

    fn classify_io_error(err: &io::Error) -> ClassifiedError {
        match err.kind() {
            io::ErrorKind::PermissionDenied => ClassifiedError::new(ErrorType::Permission, "文件权限不足".to_string()),
            io::ErrorKind::NotFound => ClassifiedError::new(ErrorType::FileSystem, "文件或目录不存在".to_string()),
            io::ErrorKind::ConnectionRefused => ClassifiedError::new(ErrorType::Network, "连接被拒绝".to_string()),
            io::ErrorKind::TimedOut => ClassifiedError::new(ErrorType::Timeout, "操作超时".to_string()),
            io::ErrorKind::WriteZero | io::ErrorKind::UnexpectedEof => {
                ClassifiedError::new(ErrorType::Network, "网络连接中断".to_string())
            }
            _ => ClassifiedError::new(ErrorType::FileSystem, format!("文件系统错误: {}", err)),
        }
    }

    fn classify_bili_error(err: &crate::bilibili::BiliError) -> ClassifiedError {
        match err {
            crate::bilibili::BiliError::RiskControlOccurred => {
                ClassifiedError::new(ErrorType::RiskControl, "触发B站风控，请稍后重试".to_string())
                    .with_retry_policy(false, false) // 风控不重试，不忽略
            }
            crate::bilibili::BiliError::NetworkTimeout => {
                ClassifiedError::new(ErrorType::Timeout, "网络超时或DNS解析失败".to_string())
                    .with_retry_policy(true, false) // 网络超时可重试
            }
            crate::bilibili::BiliError::VideoStreamDenied(code) => {
                ClassifiedError::new(ErrorType::NotFound, format!("视频流访问被拒绝: {}", code))
                    .with_retry_policy(false, true) // 不重试，可忽略
            }
            crate::bilibili::BiliError::RequestFailed(code, msg) => {
                let error_type = match *code {
                    87008 | -352 | -412 => ErrorType::RiskControl, // 特定风控错误码
                    -401 | -403 => ErrorType::Authentication,
                    -404 => ErrorType::NotFound,
                    -429 => ErrorType::RateLimit,
                    -500..=-400 => ErrorType::ServerError,
                    _ => ErrorType::ClientError,
                };

                let should_retry = match *code {
                    87008 | -352 | -412 => false, // 风控不重试
                    -500..=-400 | -1 => true,     // 服务器错误或网络错误可重试
                    _ => false,
                };

                ClassifiedError::new(error_type, format!("B站API错误: {}", msg)).with_retry_policy(should_retry, false)
            }
        }
    }
}

pub enum ExecutionStatus {
    Skipped,
    Succeeded,
    Ignored(anyhow::Error),
    Failed(anyhow::Error),
    // 任务可以返回该状态固定自己的 status
    FixedFailed(u32, anyhow::Error),
    // 新增：分类后的错误状态
    ClassifiedFailed(ClassifiedError),
}

// 目前 stable rust 似乎不支持自定义类型使用 ? 运算符，只能先在返回值使用 Result，再这样套层娃
impl From<Result<ExecutionStatus>> for ExecutionStatus {
    fn from(res: Result<ExecutionStatus>) -> Self {
        match res {
            Ok(status) => status,
            Err(err) => {
                let classified_error = ErrorClassifier::classify_error(&err);

                // 根据分类结果决定处理方式
                if classified_error.should_ignore {
                    return ExecutionStatus::Ignored(err);
                }

                // 检查传统的忽略条件（向后兼容）
                for cause in err.chain() {
                    if let Some(io_err) = cause.downcast_ref::<io::Error>() {
                        // 权限错误
                        if io_err.kind() == io::ErrorKind::PermissionDenied {
                            return ExecutionStatus::Ignored(err);
                        }
                        // 使用 io::Error 包裹的 reqwest::Error
                        if io_err.kind() == io::ErrorKind::Other
                            && io_err.get_ref().is_some_and(|e| {
                                e.downcast_ref::<reqwest::Error>().is_some_and(is_ignored_reqwest_error)
                            })
                        {
                            return ExecutionStatus::Ignored(err);
                        }
                    }
                    // 未包裹的 reqwest::Error
                    if let Some(error) = cause.downcast_ref::<reqwest::Error>() {
                        if is_ignored_reqwest_error(error) {
                            return ExecutionStatus::Ignored(err);
                        }
                    }
                }
                ExecutionStatus::ClassifiedFailed(classified_error)
            }
        }
    }
}

fn is_ignored_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_decode() || err.is_body() || err.is_timeout()
}

pub mod convert;
pub mod filenamify;
pub mod format_arg;
pub mod model;
pub mod nfo;
pub mod signal;
pub mod status;

use std::fmt;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

// 自定义日志层，用于将日志添加到API缓冲区
struct LogCaptureLayer;

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        use crate::api::handler::{add_log_entry, LogLevel};

        let level = match *event.metadata().level() {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::TRACE => LogLevel::Debug, // 将TRACE映射到DEBUG
        };

        // 提取日志消息
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            add_log_entry(level, message, Some(event.metadata().target().to_string()));
        }
    }
}

// 用于提取日志消息的访问者
struct MessageVisitor {
    message: Option<String>,
}

impl MessageVisitor {
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

pub fn init_logger(log_level: &str) {
    // 控制台输出层 - 使用用户指定的日志级别（如info）
    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%b %d %H:%M:%S".to_owned(),
        ))
        .with_filter(tracing_subscriber::EnvFilter::builder().parse_lossy(log_level));

    // API日志捕获层 - 捕获debug级别及以上的所有日志
    let log_capture_layer = LogCaptureLayer.with_filter(tracing_subscriber::EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(log_capture_layer)
        .try_init()
        .expect("初始化日志失败");
}

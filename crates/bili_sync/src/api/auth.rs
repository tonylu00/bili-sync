use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::Modify;

use crate::api::wrapper::ApiResponse;

pub async fn auth(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    // 排除不需要认证的路径
    let path = request.uri().path();
    let excluded_paths = [
        "/api/search",           // 搜索API不需要认证
        "/api/proxy/image",      // 图片代理不需要认证
        "/api/setup/check",      // 初始设置检查不需要认证
        "/api/setup/auth-token", // 设置auth token不需要认证
        "/api/credential",       // 更新凭证在初始设置时不需要认证
        "/api/videos/stream",    // 视频流API不需要认证（供播放器使用）
        "/api/videos/proxy-stream", // 视频流代理API不需要认证（供在线播放器使用）
    ];

    let needs_auth = path.starts_with("/api/") && !excluded_paths.iter().any(|&excluded| path.starts_with(excluded));

    let current_config = crate::config::reload_config();
    if needs_auth && get_token(&headers) != current_config.auth_token {
        return Ok(ApiResponse::unauthorized(()).into_response());
    }
    Ok(next.run(request).await)
}

fn get_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(Into::into)
}

pub(super) struct OpenAPIAuth;

impl Modify for OpenAPIAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(schema) = openapi.components.as_mut() {
            schema.add_security_scheme(
                "Token",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                    "Authorization",
                    "与配置文件中的 auth_token 相同",
                ))),
            );
        }
    }
}

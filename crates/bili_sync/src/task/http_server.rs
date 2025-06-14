use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::{header, Uri};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{middleware, Extension, Router, ServiceExt};
use reqwest::StatusCode;
use rust_embed::Embed;
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::api::auth;
use crate::api::handler::{
    add_video_source, delete_video_source, get_bangumi_seasons, get_config, get_logs, get_queue_status,
    get_subscribed_collections, get_user_collections, get_user_favorites, get_user_followings, get_video,
    get_video_sources, get_videos, proxy_image, reload_config, reset_all_videos, reset_video, search_bilibili, update_config,
    update_video_source_enabled, update_video_status, ApiDoc,
};
use crate::config::CONFIG;

#[derive(Embed)]
#[folder = "../../web/build"]
struct Asset;

pub async fn http_server(database_connection: Arc<DatabaseConnection>) -> Result<()> {
    let app = Router::new()
        .route("/api/video-sources", get(get_video_sources))
        .route("/api/video-sources", post(add_video_source))
        .route("/api/video-sources/{source_type}/{id}/enabled", put(update_video_source_enabled))
        .route("/api/video-sources/{source_type}/{id}", delete(delete_video_source))
        .route("/api/videos", get(get_videos))
        .route("/api/videos/{id}", get(get_video))
        .route("/api/videos/{id}/reset", post(reset_video))
        .route("/api/videos/{id}/update-status", post(update_video_status))
        .route("/api/videos/reset-all", post(reset_all_videos))
        .route("/api/reload-config", post(reload_config))
        .route("/api/config", get(get_config))
        .route("/api/config", put(update_config))
        .route("/api/bangumi/seasons/{season_id}", get(get_bangumi_seasons))
        .route("/api/search", get(search_bilibili))
        .route("/api/user/favorites", get(get_user_favorites))
        .route("/api/user/collections/{mid}", get(get_user_collections))
        .route("/api/user/followings", get(get_user_followings))
        .route("/api/user/subscribed-collections", get(get_subscribed_collections))
        .route("/api/logs", get(get_logs))
        .route("/api/queue-status", get(get_queue_status))
        .route("/api/proxy/image", get(proxy_image))
        .merge(
            SwaggerUi::new("/swagger-ui/")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
                .config(
                    Config::default()
                        .try_it_out_enabled(true)
                        .persist_authorization(true)
                        .validator_url("none"),
                ),
        )
        .fallback_service(get(frontend_files))
        .layer(Extension(database_connection))
        .layer(middleware::from_fn(auth::auth));
    let listener = tokio::net::TcpListener::bind(&CONFIG.bind_address)
        .await
        .context("bind address failed")?;
    info!("开始运行管理页: http://{}", CONFIG.bind_address);
    Ok(axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?)
}

async fn frontend_files(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // 对于SPA路由，如果请求的路径不是文件（没有扩展名），则返回index.html
            if !path.contains('.') {
                match Asset::get("index.html") {
                    Some(content) => {
                        let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                        ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
                    }
                    None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
                }
            } else {
                (StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        }
    }
}

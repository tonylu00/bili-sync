use super::*;
use crate::bilibili::test_support::mock_api;
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{Database, IntoActiveModel, PaginatorTrait};
use serde_json::{json, Value};

const SHORT_ID: i64 = 8584954;
const FULL_ID: i64 = 858495490;
const OWNER: i64 = 11262990;

fn folder() -> Value {
    json!({"id": FULL_ID, "fid": SHORT_ID, "mid": OWNER, "title": "My favorite videos"})
}

fn response(data: Value) -> (u16, String) {
    (200, json!({"code": 0, "message": "OK", "data": data}).to_string())
}

fn resolution_responses() -> Vec<(u16, String)> {
    vec![
        response(json!({"list": [folder()]})),
        response(json!({"list": [
            {"id": 1850028, "fid": 0, "mid": 249697245, "type": 21},
            {"id": 1502689, "fid": 0, "mid": 266765166, "type": 21}
        ]})),
        response(folder()),
    ]
}

async fn database() -> (DatabaseConnection, favorite::Model) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    Migrator::up(&db, None).await.unwrap();
    let source = favorite::ActiveModel {
        f_id: Set(SHORT_ID),
        name: Set("默认收藏夹".into()),
        path: Set("/saved/original".into()),
        created_at: Set("2025-01-01 00:00:00".into()),
        latest_row_at: Set("2026-09-01 00:00:00".into()),
        enabled: Set(true),
        scan_deleted_videos: Set(true),
        include_keywords: Set(Some("[\"include\"]".into())),
        exclude_keywords: Set(Some("[\"exclude\"]".into())),
        min_duration_seconds: Set(Some(10)),
        max_duration_seconds: Set(Some(100)),
        min_page_duration_seconds: Set(Some(2)),
        max_page_duration_seconds: Set(Some(80)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    (db, source)
}

async fn stored(db: &DatabaseConnection, id: i32) -> favorite::Model {
    favorite::Entity::find_by_id(id).one(db).await.unwrap().unwrap()
}

#[tokio::test]
async fn favorite_scan_migrates_short_id_before_reading_videos_and_reuses_identity() {
    let (db, original) = database().await;
    let queued = crate::adapter::Args::Favorite { source_id: original.id };
    let saved_video = video::Model {
        id: 1,
        favorite_id: Some(original.id),
        bvid: "BVexisting".into(),
        name: "Downloaded video".into(),
        path: "/saved/video".into(),
        download_status: 123,
        ..Default::default()
    }
    .into_active_model()
    .insert(&db)
    .await
    .unwrap();
    let mut responses = resolution_responses();
    responses.push(response(json!({"info": {"media_count": 1}, "medias": [{
        "title": "Recovered include video", "type": 2, "bvid": "BVrecovered", "intro": "", "cover": "",
        "ctime": 1750000000, "pubtime": 1750000000, "fav_time": 1750000000,
        "upper": {"mid": OWNER, "name": "Uploader", "face": ""}, "attr": 0
    }], "has_more": false})));
    let (base, server) = mock_api(responses).await;
    let client = BiliClient::new(String::new());
    let crate::adapter::Args::Favorite { source_id } = queued else {
        unreachable!()
    };
    // Production already holds a read guard around the whole scan. Migration
    // must not try to upgrade it to the manual repair's exclusive lock.
    let _scan_guard = FAVORITE_SOURCE_LOCK.read().await;
    let (source, stream) = favorite_from_with_identity_context(source_id, &client, &db, &base, Some(OWNER))
        .await
        .unwrap();
    let mut expected = original.clone();
    expected.f_id = FULL_ID;
    expected.owner_mid = Some(OWNER);
    expected.name = "My favorite videos".into();
    assert_eq!(stored(&db, original.id).await, expected);
    let (count, _) = crate::workflow::refresh_video_source(
        &source,
        stream,
        &db,
        tokio_util::sync::CancellationToken::new(),
        &client,
    )
    .await
    .unwrap();
    assert_eq!(count, 1, "the saved scan timestamp must not skip recovered videos");
    let recovered = video::Entity::find()
        .filter(video::Column::Bvid.eq("BVrecovered"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.favorite_id, Some(original.id));
    assert_eq!(
        video::Entity::find_by_id(saved_video.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap(),
        saved_video
    );
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].contains("created/list-all?up_mid=11262990"));
    assert!(requests[2].starts_with("GET /x/v3/fav/folder/info?media_id=858495490 "));
    assert!(requests[3].starts_with("GET /x/v3/fav/resource/list?media_id=858495490&"));
    assert!(requests.iter().all(|request| !request.contains("media_id=8584954 ")));
    assert_eq!(favorite::Entity::find().count(&db).await.unwrap(), 1);

    // A new invocation reloads persisted identity, as after a restart. It does
    // not need the account lists again and still accepts legitimate renames.
    let (base, server) = mock_api(vec![response(json!({
        "id": FULL_ID, "fid": SHORT_ID, "mid": OWNER, "title": "Renamed favorite"
    }))])
    .await;
    let (source, _) = favorite_from_with_identity_context(original.id, &client, &db, &base, None)
        .await
        .unwrap();
    assert_eq!(source.source_name_display(), "Renamed favorite");
    let saved = stored(&db, original.id).await;
    assert_eq!(saved.f_id, FULL_ID);
    assert_eq!(saved.owner_mid, Some(OWNER));
    assert_eq!(saved.name, "Renamed favorite");
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn favorite_scan_migrates_own_empty_favorite_without_local_or_available_videos() {
    for (video_response, scan_succeeds) in [
        (
            response(json!({"info": {"media_count": 0}, "medias": [], "has_more": false})),
            true,
        ),
        (
            response(json!({"info": {"media_count": 0}, "medias": null, "has_more": false})),
            true,
        ),
        (
            response(json!({"info": {"media_count": 5}, "medias": [], "has_more": false})),
            true,
        ),
        (
            (
                200,
                json!({"code": -500, "message": "video list temporarily unavailable"}).to_string(),
            ),
            false,
        ),
    ] {
        let (db, original) = database().await;
        assert_eq!(video::Entity::find().count(&db).await.unwrap(), 0);
        let mut empty_folder = folder();
        empty_folder["media_count"] = json!(0);
        let (base, server) = mock_api(vec![
            response(json!({"list": [empty_folder.clone()]})),
            response(json!({"list": []})),
            response(empty_folder),
            video_response,
        ])
        .await;
        let client = BiliClient::new(String::new());
        let (source, stream) = favorite_from_with_identity_context(original.id, &client, &db, &base, Some(OWNER))
            .await
            .unwrap();
        let mut expected = original.clone();
        expected.f_id = FULL_ID;
        expected.owner_mid = Some(OWNER);
        expected.name = "My favorite videos".into();
        // Identity is persisted before the video API runs, without deriving the
        // owner from a local download or requiring any videos in the folder.
        assert_eq!(stored(&db, original.id).await, expected);
        let result = crate::workflow::refresh_video_source(
            &source,
            stream,
            &db,
            tokio_util::sync::CancellationToken::new(),
            &client,
        )
        .await;
        if scan_succeeds {
            assert_eq!(result.unwrap().0, 0);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(stored(&db, original.id).await, expected);
        assert_eq!(video::Entity::find().count(&db).await.unwrap(), 0);
        assert_eq!(favorite::Entity::find().count(&db).await.unwrap(), 1);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].contains("created/list-all?up_mid=11262990"));
        assert!(requests[2].starts_with("GET /x/v3/fav/folder/info?media_id=858495490 "));
        assert!(requests[3].starts_with("GET /x/v3/fav/resource/list?media_id=858495490&"));
    }
}

#[tokio::test]
async fn favorite_scan_preserves_legacy_source_when_identity_cannot_be_resolved() {
    let other = json!({"id": 858495427, "fid": SHORT_ID, "mid": 12327, "title": "Other owner"});
    for responses in [
        vec![response(json!({"list": []})), response(json!({"list": []}))],
        vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": [other]})),
        ],
        vec![
            response(json!({"list": [folder()]})),
            (200, json!({"code": -412, "message": "blocked"}).to_string()),
        ],
        vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": []})),
            response(json!({"id": FULL_ID, "fid": SHORT_ID, "mid": 99990, "title": "Wrong owner"})),
        ],
    ] {
        let (db, original) = database().await;
        let count = responses.len();
        let (base, server) = mock_api(responses).await;
        let client = BiliClient::new(String::new());
        let result = favorite_from_with_identity_context(original.id, &client, &db, &base, Some(OWNER)).await;
        let Err(error) = result else {
            panic!("unresolved identity must not produce a video stream")
        };
        assert!(format!("{error:#}").contains("修复收藏夹ID"));
        assert_eq!(stored(&db, original.id).await, original);
        assert_eq!(server.await.unwrap().len(), count);
    }
}

#[tokio::test]
async fn favorite_scan_without_login_keeps_unknown_identity_and_name() {
    let (db, original) = database().await;
    let client = BiliClient::new(String::new());
    assert!(
        favorite_from_with_identity_context(original.id, &client, &db, "http://127.0.0.1:1", None)
            .await
            .is_err()
    );
    assert_eq!(stored(&db, original.id).await, original);
}

#[tokio::test]
async fn favorite_migration_preserves_risk_control_classification_and_stops_requests() {
    for code in [-352, -412] {
        for failed_list in 0..2 {
            let (db, original) = database().await;
            let mut responses = Vec::new();
            if failed_list == 1 {
                responses.push(response(json!({"list": [folder()]})));
            }
            responses.push((200, json!({"code": code, "message": "blocked"}).to_string()));
            let (base, server) = mock_api(responses).await;
            let client = BiliClient::new(String::new());
            let result = favorite_from_with_identity_context(original.id, &client, &db, &base, Some(OWNER)).await;
            let Err(error) = result else {
                panic!("risk control must stop the scan")
            };
            assert_eq!(
                crate::error::ErrorClassifier::classify_error(&error).error_type,
                crate::error::ErrorType::RiskControl
            );
            assert_eq!(stored(&db, original.id).await, original);
            assert_eq!(server.await.unwrap().len(), failed_list + 1);
        }
    }
}

#[tokio::test]
async fn favorite_scan_rejects_duplicate_canonical_id_without_changing_either_source() {
    let (db, original) = database().await;
    let mut duplicate = original.clone().into_active_model();
    duplicate.id = sea_orm::ActiveValue::NotSet;
    duplicate.f_id = Set(FULL_ID);
    duplicate.owner_mid = Set(Some(OWNER));
    let duplicate = duplicate.insert(&db).await.unwrap();
    let (base, server) = mock_api(resolution_responses()).await;
    let client = BiliClient::new(String::new());
    let result = favorite_from_with_identity_context(original.id, &client, &db, &base, Some(OWNER)).await;
    let Err(error) = result else {
        panic!("duplicate identity must not produce a video stream")
    };
    assert!(error.to_string().contains("已经存在"));
    assert_eq!(stored(&db, original.id).await, original);
    assert_eq!(stored(&db, duplicate.id).await, duplicate);
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn favorite_identity_migration_converges_without_overwriting_concurrent_settings() {
    let (db, original) = database().await;
    let info: FavoriteListInfo = serde_json::from_value(folder()).unwrap();
    let saved = save_resolved_identity(&db, &original, &info).await.unwrap();
    let mut changed = saved.into_active_model();
    changed.enabled = Set(false);
    changed.path = Set("/changed/path".into());
    let changed = changed.update(&db).await.unwrap();
    // A second scan's stale pre-migration row converges without overwriting settings.
    assert_eq!(save_resolved_identity(&db, &original, &info).await.unwrap(), changed);
    let mut other = info;
    other.id = 858495427;
    other.mid = Some(12327);
    assert!(save_resolved_identity(&db, &original, &other).await.is_err());
    assert_eq!(stored(&db, original.id).await, changed);
}

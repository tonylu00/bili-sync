use anyhow::{anyhow, ensure, Context, Result};
use async_stream::try_stream;
use futures::Stream;
use serde_json::Value;
use tracing::warn;

use crate::bilibili::{BiliClient, Validate, VideoInfo};
pub struct FavoriteList<'a> {
    client: &'a BiliClient,
    fid: String,
    api_base: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::test_support::mock_api;
    use serde_json::json;

    fn response(data: Value) -> (u16, String) {
        (200, json!({"code": 0, "message": "OK", "data": data}).to_string())
    }

    fn folder() -> Value {
        json!({"id": 3800171409_i64, "fid": 38001714, "mid": 443891409, "title": "Recovered"})
    }

    #[tokio::test]
    async fn favorite_resolver_matches_owned_folder_after_checking_both_lists() {
        let (base, server) = mock_api(vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": []})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, Some(443891409), None, false)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(info.mid, Some(443891409));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].contains("created/list-all?up_mid=443891409"));
        assert!(requests[2].contains("folder/info?media_id=3800171409"));
    }

    #[tokio::test]
    async fn favorite_resolver_paginates_followed_folders_even_after_owned_list_failure() {
        let (base, server) = mock_api(vec![
            (200, json!({"code": -101, "message": "not logged in"}).to_string()),
            response(json!({"list": [{"id": 123, "fid": 1}], "has_more": true})),
            response(json!({"list": [folder()], "has_more": false})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, Some(99), None, true)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(info.id, 3800171409);
        let requests = server.await.unwrap();
        assert!(requests[1].contains("collected/list?up_mid=99&pn=1&ps=20"));
        assert!(requests[2].contains("collected/list?up_mid=99&pn=2&ps=20"));
        assert!(requests[3].contains("folder/info?media_id=3800171409"));
    }

    #[tokio::test]
    async fn favorite_resolver_default_finds_followed_folder_without_probing() {
        let (base, server) = mock_api(vec![
            response(json!({"list": []})),
            response(json!({"list": [{"id": 123, "fid": 1}], "has_more": true})),
            response(json!({"list": [folder()], "has_more": false})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, Some(99), None, false)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(info.mid, Some(443891409));
        assert_eq!(server.await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn favorite_resolver_recovers_other_creator_uid_using_zero_padded_suffix() {
        let mut responses = vec![response(json!({"list": null})), response(json!({"list": []}))];
        responses.extend((0..9).map(|_| response(Value::Null)));
        responses.push(response(folder()));
        responses.extend((10..100).map(|_| response(Value::Null)));
        let (base, server) = mock_api(responses).await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, Some(12345), None, true)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(info.mid, Some(443891409));
        let requests = server.await.unwrap();
        assert!(requests[2].contains("media_id=3800171400"));
        assert!(requests[11].contains("media_id=3800171409"));
        assert_eq!(requests.len(), 102);
        assert!(requests.iter().all(|request| !request.contains("media_id=38001714 ")));
    }

    #[tokio::test]
    async fn favorite_resolver_rejects_wrong_metadata_and_stops_on_risk_control() {
        let (base, server) = mock_api(vec![
            response(json!({"id": 3800171400_i64, "fid": 38001714, "mid": 443891409, "title": "Wrong suffix"})),
            response(json!({"id": 3800171401_i64, "fid": 1, "mid": 101, "title": "Unrelated"})),
            (200, json!({"code": -412, "message": "blocked"}).to_string()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let error = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, None, None, true)
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("blocked"));
        assert_eq!(server.await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn favorite_resolver_rejects_ambiguous_owners_across_both_lists() {
        let other = json!({"id": 3800171427_i64, "fid": 38001714, "mid": 686127, "title": "Different creator"});
        let (base, server) = mock_api(vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": [other]})),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let error = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_short_id(38001714, Some(443891409), None, false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("多个收藏夹"));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn favorite_resolver_uses_known_owner_once_and_checks_full_uid() {
        let (base, server) = mock_api(vec![response(folder()), response(folder())]).await;
        let client = BiliClient::new(String::new());
        let resolver = FavoriteList::with_api_base(&client, "38001714".into(), &base);
        let info = resolver
            .resolve_short_id(38001714, None, Some(443891409), false)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(info.mid, Some(443891409));
        // Same suffix does not establish the same creator.
        assert!(resolver
            .resolve_short_id(38001714, None, Some(109), false)
            .await
            .is_err());
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn favorite_resolver_bounds_missing_folder_search_and_rejects_overflow() {
        let (base, server) = mock_api((0..100).map(|_| response(Value::Null)).collect()).await;
        let client = BiliClient::new(String::new());
        let resolver = FavoriteList::with_api_base(&client, "1".into(), &base);
        assert!(resolver.resolve_short_id(i64::MAX, None, None, true).await.is_err());
        assert!(resolver.resolve_short_id(0, None, None, true).await.is_err());
        assert!(resolver
            .resolve_short_id(1, None, None, true)
            .await
            .unwrap_err()
            .to_string()
            .contains("未找到"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 100);
        assert!(requests.last().unwrap().contains("media_id=199"));
    }

    #[tokio::test]
    #[ignore = "contacts Bilibili to verify public short-ID recovery"]
    async fn favorite_resolver_live_public_folder() {
        let client = BiliClient::new(String::new());
        let candidates = FavoriteList::new(&client, "38001714".into())
            .resolve_short_id(38001714, None, None, true)
            .await
            .unwrap();
        assert!(candidates
            .iter()
            .any(|info| info.id == 3800171409 && info.mid == Some(443891409)));
        assert!(candidates.iter().all(|info| info.matches_short_id(38001714)));
        for info in candidates {
            println!("candidate media_id={} creator_uid={:?}", info.id, info.mid);
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FavoriteListInfo {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub fid: Option<i64>,
    #[serde(default)]
    pub mid: Option<i64>,
}

impl FavoriteListInfo {
    /// Arithmetic only generates candidates; upstream must attest both IDs and the owner.
    pub(crate) fn matches_short_id(&self, short_id: i64) -> bool {
        short_id > 0
            && self.fid == Some(short_id)
            && self.mid.is_some_and(|mid| {
                mid > 0 && short_id.checked_mul(100).and_then(|id| id.checked_add(mid % 100)) == Some(self.id)
            })
    }
}

/// Folder entries returned by Bilibili's created/list-all API.
/// Its `id` is the media_id; the separate upstream `fid` is not usable as media_id.
#[derive(Debug, serde::Deserialize)]
pub struct FavoriteFolder {
    pub id: i64,
    pub title: String,
    pub media_count: i32,
    #[serde(default)]
    pub fid: Option<i64>,
    #[serde(default)]
    pub mid: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Upper<T> {
    pub mid: T,
    pub name: String,
    pub face: String,
}
impl<'a> FavoriteList<'a> {
    pub fn new(client: &'a BiliClient, fid: String) -> Self {
        Self::with_api_base(client, fid, "https://api.bilibili.com")
    }

    pub(crate) fn with_api_base(client: &'a BiliClient, fid: String, api_base: &str) -> Self {
        Self {
            client,
            fid,
            api_base: api_base.to_owned(),
        }
    }

    pub async fn get_info(&self) -> Result<FavoriteListInfo> {
        let info = self
            .get_optional_info()
            .await?
            .ok_or(crate::bilibili::SourceUnavailable)?;
        if super::error::is_unavailable_name(&info.title) {
            return Err(crate::bilibili::SourceUnavailable.into());
        }
        ensure!(!info.title.trim().is_empty(), "收藏夹名称为空");
        Ok(info)
    }

    pub(crate) async fn get_optional_info(&self) -> Result<Option<FavoriteListInfo>> {
        let mut res = self
            .client
            .request(reqwest::Method::GET, &format!("{}/x/v3/fav/folder/info", self.api_base))
            .await
            .query(&[("media_id", &self.fid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(
            res.get_mut("data").context("收藏夹响应缺少data")?.take(),
        )?)
    }

    /// Recover a legacy short fid, including public folders owned by other users.
    pub(crate) async fn resolve_short_id(
        &self,
        short_id: i64,
        uid: Option<i64>,
        owner_mid: Option<i64>,
        allow_probe: bool,
    ) -> Result<Vec<FavoriteListInfo>> {
        let base_id = short_id
            .checked_mul(100)
            .filter(|id| *id > 0)
            .and_then(|id| id.checked_add(99).map(|_| id))
            .context("收藏夹原始ID超出范围")?;
        if let Some(owner) = owner_mid {
            ensure!(owner > 0, "创建者UID必须大于0");
            let id = base_id + owner % 100;
            let info = Self::with_api_base(self.client, id.to_string(), &self.api_base)
                .get_info()
                .await?;
            ensure!(
                info.id == id && info.matches_short_id(short_id) && info.mid == Some(owner),
                "收藏夹与指定创建者UID不匹配，请检查完整URL"
            );
            return Ok(vec![info]);
        }
        let mut tried = std::collections::HashSet::new();
        let mut candidates = std::collections::BTreeMap::new();
        let mut list_errors = Vec::new();
        if let Some(uid) = uid {
            for followed in [false, true] {
                let mut page = 1;
                loop {
                    let path = if followed { "collected/list" } else { "created/list-all" };
                    let response = async {
                        self.client
                            .request(
                                reqwest::Method::GET,
                                &format!("{}/x/v3/fav/folder/{path}", self.api_base),
                            )
                            .await
                            .query(&[
                                ("up_mid", uid.to_string()),
                                ("pn", page.to_string()),
                                ("ps", "20".into()),
                                ("platform", "web".into()),
                            ])
                            .send()
                            .await?
                            .error_for_status()?
                            .json::<Value>()
                            .await?
                            .validate()
                    }
                    .await;
                    let response = match response {
                        Ok(response) => response,
                        Err(error) => {
                            // A failed owned list must not prevent trying followed folders.
                            list_errors.push(format!("{path}: {error:#}"));
                            break;
                        }
                    };
                    let Some(list) = response["data"]["list"].as_array() else {
                        break;
                    };
                    for folder in list {
                        if folder["fid"].as_i64() != Some(short_id) {
                            continue;
                        }
                        let Some(id) = folder["id"].as_i64() else { continue };
                        if id < base_id || id > base_id + 99 {
                            continue;
                        }
                        tried.insert(id);
                        let owner = folder["mid"].as_i64().or(if followed { None } else { Some(uid) });
                        candidates.insert((id, owner), ());
                    }
                    if !followed || list.is_empty() {
                        break;
                    }
                    if response["data"]["has_more"].as_bool() == Some(false) {
                        break;
                    }
                    if response["data"]["has_more"].as_bool() != Some(true)
                        && response["data"]["count"]
                            .as_u64()
                            .is_some_and(|count| page * 20 >= count)
                    {
                        break;
                    }
                    if list.len() < 20 && response["data"]["has_more"].as_bool() != Some(true) {
                        break;
                    }
                    page += 1;
                    ensure!(page <= 1000, "关注收藏夹分页异常，请使用完整URL修复");
                }
            }
        }
        ensure!(
            allow_probe || candidates.len() <= 1,
            "原始ID匹配到多个收藏夹，无法确定创建者，请填写创建者UID或完整URL"
        );
        if !allow_probe {
            ensure!(
                list_errors.is_empty(),
                "未能完整读取收藏夹列表：{}，请重试或填写完整URL",
                list_errors.join("; ")
            );
        }
        let mut listed = Vec::new();
        for ((id, owner), _) in candidates {
            let info = Self::with_api_base(self.client, id.to_string(), &self.api_base)
                .get_info()
                .await?;
            ensure!(
                info.id == id && info.matches_short_id(short_id) && owner.is_none_or(|mid| info.mid == Some(mid)),
                "收藏夹列表与详情不一致，请填写完整URL"
            );
            listed.push(info);
        }
        if !listed.is_empty() {
            return Ok(listed);
        }
        ensure!(
            allow_probe,
            "未在创建或关注列表中找到收藏夹，请填写创建者UID或完整URL，或手动启用旧收藏夹候选查找"
        );
        // Opt-in legacy lookup only; callers must present every candidate for confirmation.
        let mut matches = Vec::new();
        // Only 100 possible media_ids. Sequential requests use BiliClient's rate limiter.
        // Do not query the short ID as a media_id: it may be a different valid folder.
        for suffix in 0..100 {
            let id = base_id + suffix;
            if tried.insert(id) {
                if let Some(info) = self.probe_short_id(id, short_id).await? {
                    matches.push(info);
                }
            }
        }
        if !matches.is_empty() {
            return Ok(matches);
        }
        Err(anyhow!(
            "未找到原始ID为 {short_id} 的可访问收藏夹；收藏夹可能已删除或设为私密，请填写完整URL重试"
        ))
    }

    async fn probe_short_id(&self, id: i64, short_id: i64) -> Result<Option<FavoriteListInfo>> {
        let result = Self::with_api_base(self.client, id.to_string(), &self.api_base)
            .get_optional_info()
            .await;
        match result {
            Ok(Some(info)) if info.id == id && info.matches_short_id(short_id) => Ok(Some(info)),
            Ok(_) => Ok(None),
            Err(error)
                if matches!(
                    error.downcast_ref::<crate::bilibili::BiliError>(),
                    Some(crate::bilibili::BiliError::RequestFailed(-403 | -404 | 11010, _))
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(error.context("自动查找已停止，请检查网络或B站登录状态后重试")),
        }
    }

    async fn get_videos(&self, page: u32) -> Result<Value> {
        let response = self
            .client
            .request(
                reqwest::Method::GET,
                &format!("{}/x/v3/fav/resource/list", self.api_base),
            )
            .await
            .query(&[
                ("media_id", self.fid.as_str()),
                ("pn", page.to_string().as_str()),
                ("ps", "20"),
                ("order", "mtime"),
                ("type", "0"),
                ("tid", "0"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        let data = response.get("data").context("视频列表响应缺少data")?;
        if data.is_null() {
            return Err(crate::bilibili::SourceUnavailable.into());
        }
        Ok(response)
    }

    // 拿到收藏夹的所有权，返回一个收藏夹下的视频流
    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut page = 1;
            loop {
                let mut videos = self
                    .get_videos(page)
                    .await
                    .with_context(|| format!("failed to get videos of favorite {} page {}", self.fid, page))?;

                let media_count = videos["data"]["info"]["media_count"].as_u64().unwrap_or(0);
                let medias = &mut videos["data"]["medias"];

                if medias.as_array().is_none_or(|v| v.is_empty()) {
                    if media_count > 0 {
                        // 统计显示有视频但medias为空，说明内容被B站API过滤
                        // 只记录警告，不抛出错误，正常结束扫描
                        warn!("收藏夹 {} 中的 {} 个视频被B站API过滤，无法通过API获取（可能是番剧、纪录片等特殊内容类型）", self.fid, media_count);
                        break;
                    } else {
                        // 正常的空页面情况
                        break;
                    }
                }
                let videos_info: Vec<VideoInfo> = serde_json::from_value(medias.take())
                    .with_context(|| format!("failed to parse videos of favorite {} page {}", self.fid, page))?;
                for video_info in videos_info {
                    yield video_info;
                }
                let has_more = &videos["data"]["has_more"];
                if let Some(v) = has_more.as_bool() {
                    if v {
                        page += 1;
                        continue;
                    }
                } else {
                    Err(anyhow!("has_more is not a bool"))?;
                }
                break;
            }
        }
    }
}

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
    async fn stored_favorite_resolver_maps_short_id_without_querying_it_as_media_id() {
        let (base, server) = mock_api(vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": []})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(Some(443891409))
            .await
            .unwrap();
        assert_eq!(info.id, 3800171409);
        assert_eq!(info.mid, Some(443891409));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2].contains("folder/info?media_id=3800171409"));
        assert!(requests.iter().all(|request| !request.contains("media_id=38001714 ")));
    }

    #[tokio::test]
    async fn stored_favorite_resolver_preserves_exact_full_id_and_default_folder_title() {
        let mut default_folder = folder();
        default_folder["title"] = json!("默认收藏夹");
        let (base, server) = mock_api(vec![
            // Created folders may omit fid and mid; details still must prove both.
            response(json!({"list": [{"id": 3800171409_i64}]})),
            response(json!({"list": []})),
            response(default_folder),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "3800171409".into(), &base)
            .resolve_stored_id(Some(443891409))
            .await
            .unwrap();
        assert_eq!(info.id, 3800171409);
        assert_eq!(info.title, "默认收藏夹");
        assert!(server.await.unwrap()[2].contains("folder/info?media_id=3800171409"));
    }

    #[tokio::test]
    async fn stored_favorite_resolver_rejects_short_and_full_id_collision_before_details() {
        let (base, server) = mock_api(vec![
            response(json!({"list": [{"id": 38001714, "fid": 380017, "mid": 114}]})),
            response(json!({"list": [folder()]})),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let error = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(Some(114))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("多个收藏夹"));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stored_favorite_resolver_requires_both_lists_to_succeed() {
        for failed_list in 0..2 {
            let mut responses = vec![response(json!({"list": [folder()]})); 2];
            responses[failed_list] = (200, json!({"code": -101, "message": "not logged in"}).to_string());
            let (base, server) = mock_api(responses).await;
            let client = BiliClient::new(String::new());
            let error = FavoriteList::with_api_base(&client, "38001714".into(), &base)
                .resolve_stored_id(Some(99))
                .await
                .unwrap_err();
            assert!(error.to_string().contains("未能完整读取"));
            let requests = server.await.unwrap();
            assert_eq!(requests.len(), 2);
            assert!(requests[1].contains("collected/list"));
        }
    }

    #[tokio::test]
    async fn stored_favorite_resolver_rejects_missing_login_without_requests() {
        let (base, server) = mock_api(Vec::new()).await;
        let client = BiliClient::new(String::new());
        assert!(FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(None)
            .await
            .unwrap_err()
            .to_string()
            .contains("登录状态"));
        assert!(server.await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn stored_favorite_resolver_rejects_unlisted_id_without_probing() {
        let (base, server) = mock_api(vec![response(json!({"list": null})), response(json!({"list": []}))]).await;
        let client = BiliClient::new(String::new());
        assert!(FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(Some(99))
            .await
            .unwrap_err()
            .to_string()
            .contains("未在创建或关注列表中找到"));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stored_favorite_resolver_rejects_malformed_list_even_with_valid_candidate() {
        let (base, server) = mock_api(vec![
            response(json!({"unexpected": []})),
            response(json!({"list": [folder()]})),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let error = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(Some(99))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("缺少list"));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stored_favorite_resolver_paginates_followed_folders_and_checks_nested_owner() {
        let mut followed = folder();
        followed.as_object_mut().unwrap().remove("mid");
        followed["upper"] = json!({"mid": 443891409});
        let (base, server) = mock_api(vec![
            response(json!({"list": []})),
            response(json!({"list": [{"id": 123, "fid": 1}], "has_more": true})),
            response(json!({"list": [followed], "has_more": false})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        let info = FavoriteList::with_api_base(&client, "38001714".into(), &base)
            .resolve_stored_id(Some(99))
            .await
            .unwrap();
        assert_eq!(info.id, 3800171409);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[2].contains("collected/list?up_mid=99&pn=2&ps=20"));
    }

    #[tokio::test]
    async fn stored_favorite_resolver_deduplicates_consistent_folder_evidence() {
        let (base, server) = mock_api(vec![
            response(json!({"list": [folder()]})),
            response(json!({"list": [{"id": 3800171409_i64}]})),
            response(folder()),
        ])
        .await;
        let client = BiliClient::new(String::new());
        assert_eq!(
            FavoriteList::with_api_base(&client, "3800171409".into(), &base)
                .resolve_stored_id(Some(443891409))
                .await
                .unwrap()
                .id,
            3800171409
        );
        assert_eq!(server.await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn stored_favorite_resolver_rejects_detail_identity_disagreement() {
        for detail in [
            json!({"id": 3800171408_i64, "fid": 38001714, "mid": 443891408, "title": "Wrong ID"}),
            json!({"id": 3800171409_i64, "fid": 1, "mid": 443891409, "title": "Wrong fid"}),
            json!({"id": 3800171409_i64, "fid": 38001714, "mid": 109, "title": "Wrong owner"}),
        ] {
            let (base, server) = mock_api(vec![
                response(json!({"list": [folder()]})),
                response(json!({"list": []})),
                response(detail),
            ])
            .await;
            let client = BiliClient::new(String::new());
            assert!(FavoriteList::with_api_base(&client, "38001714".into(), &base)
                .resolve_stored_id(Some(443891409))
                .await
                .unwrap_err()
                .to_string()
                .contains("列表与详情不一致"));
            assert_eq!(server.await.unwrap().len(), 3);
        }
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

/// Identity evidence from an account's created or followed folder list.
#[derive(Debug)]
struct ListedFavoriteFolder {
    id: i64,
    fid: Option<i64>,
    mid: Option<i64>,
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
            let (folders, errors) = self.read_folder_lists(uid).await?;
            list_errors = errors;
            for folder in folders {
                if folder.fid != Some(short_id) || folder.id < base_id || folder.id > base_id + 99 {
                    continue;
                }
                tried.insert(folder.id);
                candidates.insert((folder.id, folder.mid), ());
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
                list_errors
                    .iter()
                    .map(|error| format!("{error:#}"))
                    .collect::<Vec<_>>()
                    .join("; ")
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

    /// Resolve an unknown-owner persisted ID without ever treating a legacy fid as a media_id.
    /// Both account lists must agree on one identity before folder/info is queried.
    pub(crate) async fn resolve_stored_id(&self, uid: Option<i64>) -> Result<FavoriteListInfo> {
        let stored_id = self.fid.parse::<i64>().context("收藏夹ID格式错误")?;
        ensure!(stored_id > 0, "收藏夹ID必须大于0");
        let uid = uid
            .filter(|uid| *uid > 0)
            .context("未获取到用户ID，请检查B站登录状态后重试")?;
        let (folders, errors) = self.read_folder_lists(uid).await?;
        if let Some(error) = errors.into_iter().next() {
            return Err(error.context("未能完整读取收藏夹列表，请重试或填写完整URL"));
        }
        let mut candidates = std::collections::BTreeMap::<i64, ListedFavoriteFolder>::new();
        for folder in folders {
            let matches = folder.id == stored_id
                || (folder.fid == Some(stored_id)
                    && stored_id.checked_mul(100).is_some_and(|base| {
                        folder.id >= base && folder.id.checked_sub(base).is_some_and(|suffix| suffix < 100)
                    }));
            if !matches {
                continue;
            }
            if let Some(previous) = candidates.get_mut(&folder.id) {
                ensure!(
                    previous.fid.zip(folder.fid).is_none_or(|(a, b)| a == b)
                        && previous.mid.zip(folder.mid).is_none_or(|(a, b)| a == b),
                    "收藏夹列表身份不一致，请填写完整URL"
                );
                previous.fid = previous.fid.or(folder.fid);
                previous.mid = previous.mid.or(folder.mid);
            } else {
                candidates.insert(folder.id, folder);
            }
        }
        ensure!(
            candidates.len() <= 1,
            "存储的ID匹配到多个收藏夹，无法确定创建者，请填写创建者UID或完整URL"
        );
        let folder = candidates
            .into_values()
            .next()
            .context("未在创建或关注列表中找到收藏夹，请填写创建者UID或完整URL")?;
        let info = Self::with_api_base(self.client, folder.id.to_string(), &self.api_base)
            .get_info()
            .await?;
        ensure!(
            info.id == folder.id
                && info.fid.is_some_and(|fid| info.matches_short_id(fid))
                && folder.fid.is_none_or(|fid| info.fid == Some(fid))
                && folder.mid.is_none_or(|mid| info.mid == Some(mid)),
            "收藏夹列表与详情不一致，请填写完整URL"
        );
        Ok(info)
    }

    /// Read both account lists even if one fails, sharing pagination with manual recovery.
    async fn read_folder_lists(&self, uid: i64) -> Result<(Vec<ListedFavoriteFolder>, Vec<anyhow::Error>)> {
        let mut folders = Vec::new();
        let mut errors = Vec::new();
        for followed in [false, true] {
            let path = if followed { "collected/list" } else { "created/list-all" };
            let mut page = 1;
            loop {
                let response = async {
                    let response = self
                        .client
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
                        .validate()?;
                    let list = response["data"].get("list").context("收藏夹列表响应缺少list")?;
                    ensure!(list.is_null() || list.is_array(), "收藏夹列表响应格式错误");
                    Ok::<_, anyhow::Error>(response)
                }
                .await;
                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        // Keep typed causes for the scan's retry/risk-control handling,
                        // and stop immediately when Bilibili asks us to back off.
                        if matches!(
                            error.downcast_ref::<crate::bilibili::BiliError>(),
                            Some(
                                crate::bilibili::BiliError::RiskControlOccurred
                                    | crate::bilibili::BiliError::RiskControlVerificationRequired(_)
                                    | crate::bilibili::BiliError::RequestFailed(-352 | -412, _)
                            )
                        ) {
                            return Err(error.context(format!("读取收藏夹列表 {path} 时触发风控")));
                        }
                        errors.push(error.context(format!("读取收藏夹列表 {path} 失败")));
                        break;
                    }
                };
                let Some(list) = response["data"]["list"].as_array() else {
                    break;
                };
                for folder in list {
                    let evidence = (|| -> Result<ListedFavoriteFolder> {
                        let id = folder["id"].as_i64().filter(|id| *id > 0).context("收藏夹列表ID无效")?;
                        let optional_id = |value: Option<&Value>| -> Result<Option<i64>> {
                            match value {
                                None | Some(Value::Null) => Ok(None),
                                Some(value) => {
                                    Ok(Some(value.as_i64().filter(|id| *id > 0).context("收藏夹列表身份无效")?))
                                }
                            }
                        };
                        let fid = optional_id(folder.get("fid"))?;
                        let direct_mid = optional_id(folder.get("mid"))?;
                        let upper_mid = optional_id(folder.get("upper").and_then(|upper| upper.get("mid")))?;
                        ensure!(
                            direct_mid.zip(upper_mid).is_none_or(|(a, b)| a == b),
                            "收藏夹列表创建者身份不一致"
                        );
                        let mid = direct_mid.or(upper_mid).or(if followed { None } else { Some(uid) });
                        Ok(ListedFavoriteFolder { id, fid, mid })
                    })();
                    match evidence {
                        Ok(folder) => folders.push(folder),
                        Err(error) => errors.push(error.context(format!("读取收藏夹列表 {path} 失败"))),
                    }
                }
                if !followed || list.is_empty() || response["data"]["has_more"].as_bool() == Some(false) {
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
        Ok((folders, errors))
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

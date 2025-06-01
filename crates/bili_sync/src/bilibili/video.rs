use anyhow::{Result, ensure, bail};
use futures::TryStreamExt;
use futures::stream::FuturesUnordered;
use prost::Message;
use reqwest::Method;
use tracing::{info, warn, error};

use crate::bilibili::analyzer::PageAnalyzer;
use crate::bilibili::client::BiliClient;
use crate::bilibili::credential::encoded_query;
use crate::bilibili::danmaku::{DanmakuElem, DanmakuWriter, DmSegMobileReply};
use crate::bilibili::subtitle::{SubTitle, SubTitleBody, SubTitleInfo, SubTitlesInfo};
use crate::bilibili::{MIXIN_KEY, Validate, VideoInfo};

static MASK_CODE: u64 = 2251799813685247;
static XOR_CODE: u64 = 23442827791579;
static BASE: u64 = 58;
static DATA: &[char] = &[
    'F', 'c', 'w', 'A', 'P', 'N', 'K', 'T', 'M', 'u', 'g', '3', 'G', 'V', '5', 'L', 'j', '7', 'E', 'J', 'n', 'H', 'p',
    'W', 's', 'x', '4', 't', 'b', '8', 'h', 'a', 'Y', 'e', 'v', 'i', 'q', 'B', 'z', '6', 'r', 'k', 'C', 'y', '1', '2',
    'm', 'U', 'S', 'D', 'Q', 'X', '9', 'R', 'd', 'o', 'Z', 'f',
];

pub struct Video<'a> {
    client: &'a BiliClient,
    pub aid: String,
    pub bvid: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Tag {
    pub tag_name: String,
}

impl serde::Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.tag_name)
    }
}
#[derive(Debug, serde::Deserialize, Default)]
pub struct PageInfo {
    pub cid: i64,
    pub page: i32,
    #[serde(rename = "part")]
    pub name: String,
    pub duration: u32,
    pub first_frame: Option<String>,
    pub dimension: Option<Dimension>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
    pub rotate: u32,
}

impl<'a> Video<'a> {
    pub fn new(client: &'a BiliClient, bvid: String) -> Self {
        let aid = bvid_to_aid(&bvid).to_string();
        Self { client, aid, bvid }
    }

    /// 直接调用视频信息接口获取详细的视频信息，视频信息中包含了视频的分页信息
    pub async fn get_view_info(&self) -> Result<VideoInfo> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/view")
            .await
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    #[allow(unused)]
    pub async fn get_pages(&self) -> Result<Vec<PageInfo>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/pagelist")
            .await
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_tags(&self) -> Result<Vec<Tag>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/view/detail/tag")
            .await
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_danmaku_writer(&self, page: &'a PageInfo) -> Result<DanmakuWriter> {
        let tasks = FuturesUnordered::new();
        for i in 1..=page.duration.div_ceil(360) {
            tasks.push(self.get_danmaku_segment(page, i as i64));
        }
        let result: Vec<Vec<DanmakuElem>> = tasks.try_collect().await?;
        let mut result: Vec<DanmakuElem> = result.into_iter().flatten().collect();
        result.sort_by_key(|d| d.progress);
        Ok(DanmakuWriter::new(page, result.into_iter().map(|x| x.into()).collect()))
    }

    /// 获取番剧弹幕的专用方法
    pub async fn get_bangumi_danmaku_writer(&self, page: &'a PageInfo, ep_id: Option<&str>) -> Result<DanmakuWriter> {
        // 首先尝试使用普通方法
        match self.get_danmaku_writer(page).await {
            Ok(writer) => {
                // 成功获取弹幕
                info!("成功获取番剧弹幕，CID: {}, EP: {:?}", page.cid, ep_id);
                Ok(writer)
            }
            Err(e) => {
                // 如果是权限错误，记录特殊信息
                if e.to_string().contains("empty") || e.to_string().contains("权限") {
                    warn!("番剧弹幕获取失败（可能需要会员权限）: {}, CID: {}, EP: {:?}", e, page.cid, ep_id);
                } else {
                    error!("番剧弹幕获取失败: {}, CID: {}, EP: {:?}", e, page.cid, ep_id);
                }
                // 返回空弹幕而不是错误，避免影响下载
                Ok(DanmakuWriter::new(page, vec![]))
            }
        }
    }

    async fn get_danmaku_segment(&self, page: &PageInfo, segment_idx: i64) -> Result<Vec<DanmakuElem>> {
        let mut req = self
            .client
            .request(Method::GET, "http://api.bilibili.com/x/v2/dm/web/seg.so")
            .await
            .query(&[("type", 1), ("oid", page.cid), ("segment_index", segment_idx)]);
        
        // 添加必要的headers，特别是对于番剧内容
        req = req.header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
        
        let res = req.send().await?;
        
        // 检查响应状态
        if !res.status().is_success() {
            warn!("弹幕获取失败，状态码: {}, CID: {}, 分段: {}", res.status(), page.cid, segment_idx);
            return Ok(vec![]);
        }
        
        let headers = res.headers().clone();
        let content_type = headers.get("content-type");
        
        // 如果没有content-type或者为空，说明可能是权限问题
        if content_type.is_none() {
            let body = res.text().await?;
            if body.is_empty() {
                warn!("弹幕API返回空内容，可能需要特殊权限。CID: {}, 分段: {}", page.cid, segment_idx);
                return Ok(vec![]);
            } else {
                bail!("unexpected response: {:?}", body);
            }
        }
        
        ensure!(
            content_type.is_some_and(|v| v == "application/octet-stream"),
            "unexpected content type: {:?}, cid: {}, segment: {}",
            content_type,
            page.cid,
            segment_idx
        );
        
        let bytes = res.bytes().await?;
        if bytes.is_empty() {
            warn!("弹幕数据为空，CID: {}, 分段: {}", page.cid, segment_idx);
            return Ok(vec![]);
        }
        
        Ok(DmSegMobileReply::decode(bytes)?.elems)
    }

    pub async fn get_page_analyzer(&self, page: &PageInfo) -> Result<PageAnalyzer> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/wbi/playurl")
            .await
            .query(&encoded_query(
                vec![
                    ("avid", self.aid.as_str()),
                    ("cid", page.cid.to_string().as_str()),
                    ("qn", "127"),
                    ("otype", "json"),
                    ("fnval", "4048"),
                    ("fourk", "1"),
                ],
                MIXIN_KEY.load().as_deref(),
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(PageAnalyzer::new(res["data"].take()))
    }

    pub async fn get_subtitles(&self, page: &PageInfo) -> Result<Vec<SubTitle>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/wbi/v2")
            .await
            .query(&encoded_query(
                vec![("cid", &page.cid.to_string()), ("bvid", &self.bvid), ("aid", &self.aid)],
                MIXIN_KEY.load().as_deref(),
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        // 接口返回的信息，包含了一系列的字幕，每个字幕包含了字幕的语言和 json 下载地址
        let subtitles_info: SubTitlesInfo = serde_json::from_value(res["data"]["subtitle"].take())?;
        let tasks = subtitles_info
            .subtitles
            .into_iter()
            .filter(|v| !v.is_ai_sub())
            .map(|v| self.get_subtitle(v))
            .collect::<FuturesUnordered<_>>();
        tasks.try_collect().await
    }

    async fn get_subtitle(&self, info: SubTitleInfo) -> Result<SubTitle> {
        let mut res = self
            .client
            .client // 这里可以直接使用 inner_client，因为该请求不需要鉴权
            .request(Method::GET, format!("https:{}", &info.subtitle_url).as_str(), None)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let body: SubTitleBody = serde_json::from_value(res["body"].take())?;
        Ok(SubTitle { lan: info.lan, body })
    }
}

fn bvid_to_aid(bvid: &str) -> u64 {
    let mut bvid = bvid.chars().collect::<Vec<_>>();
    (bvid[3], bvid[9]) = (bvid[9], bvid[3]);
    (bvid[4], bvid[7]) = (bvid[7], bvid[4]);
    let mut tmp = 0u64;
    for char in bvid.into_iter().skip(3) {
        let idx = DATA.iter().position(|&x| x == char).expect("invalid bvid");
        tmp = tmp * BASE + idx as u64;
    }
    (tmp & MASK_CODE) ^ XOR_CODE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bvid_to_aid() {
        assert_eq!(bvid_to_aid("BV1Tr421n746"), 1401752220u64);
        assert_eq!(bvid_to_aid("BV1sH4y1s7fe"), 1051892992u64);
    }
}

use anyhow::Result;
use bili_sync_entity::*;
use chrono::NaiveDateTime;
use quick_xml::events::{BytesCData, BytesText};
use quick_xml::writer::Writer;
use quick_xml::Error;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::config::{EmptyUpperStrategy, NFOConfig, NFOTimeType};

#[allow(clippy::upper_case_acronyms)]
pub enum NFO<'a> {
    Movie(Movie<'a>),
    TVShow(TVShow<'a>),
    Upper(Upper),
    Episode(Episode<'a>),
}

pub struct Movie<'a> {
    pub name: &'a str,
    pub original_title: &'a str,
    pub intro: &'a str,
    pub bvid: &'a str,
    pub upper_id: i64,
    pub upper_name: &'a str,
    pub aired: NaiveDateTime,
    pub premiered: NaiveDateTime,
    pub tags: Option<Vec<String>>,
    pub user_rating: Option<f32>,
    pub mpaa: Option<&'a str>,
    pub country: Option<&'a str>,
    pub studio: Option<&'a str>,
    pub director: Option<&'a str>,
    pub credits: Option<&'a str>,
    pub duration: Option<i32>, // 视频时长（分钟）
    pub view_count: Option<i64>, // 播放量
    pub like_count: Option<i64>, // 点赞数
    pub category: i32, // 视频分类（用于番剧检测）
    pub tagline: Option<String>, // 标语/副标题（从share_copy提取）
    pub set: Option<String>, // 系列名称
    pub sorttitle: Option<String>, // 排序标题
    pub actors_info: Option<String>, // 演员信息字符串（从API获取）
}

pub struct TVShow<'a> {
    pub name: &'a str,
    pub original_title: &'a str,
    pub intro: &'a str,
    pub bvid: &'a str,
    pub upper_id: i64,
    pub upper_name: &'a str,
    pub aired: NaiveDateTime,
    pub premiered: NaiveDateTime,
    pub tags: Option<Vec<String>>,
    pub user_rating: Option<f32>,
    pub mpaa: Option<&'a str>,
    pub country: Option<&'a str>,
    pub studio: Option<&'a str>,
    pub status: Option<&'a str>, // 播出状态：Continuing, Ended
    pub total_seasons: Option<i32>,
    pub total_episodes: Option<i32>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub category: i32, // 视频分类（用于番剧检测）
    pub tagline: Option<String>, // 标语/副标题（从share_copy提取）
    pub set: Option<String>, // 系列名称
    pub sorttitle: Option<String>, // 排序标题
    pub actors_info: Option<String>, // 演员信息字符串（从API获取）
}

pub struct Upper {
    pub upper_id: String,
    pub pubtime: NaiveDateTime,
}

pub struct Episode<'a> {
    pub name: &'a str,
    pub original_title: &'a str,
    pub pid: String,
    pub plot: Option<&'a str>,
    pub season: i32,
    pub episode_number: i32,
    pub aired: Option<NaiveDateTime>,
    pub duration: Option<i32>, // 时长（分钟）
    pub user_rating: Option<f32>,
    pub director: Option<&'a str>,
    pub credits: Option<&'a str>,
}

impl NFO<'_> {
    pub async fn generate_nfo(self) -> Result<String> {
        let config = crate::config::reload_config();
        let mut buffer = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
"#
        .as_bytes()
        .to_vec();
        let mut tokio_buffer = BufWriter::new(&mut buffer);
        let writer = Writer::new_with_indent(&mut tokio_buffer, b' ', 4);
        match self {
            NFO::Movie(movie) => {
                Self::write_movie_nfo(writer, movie, &config.nfo_config).await?;
            }
            NFO::TVShow(tvshow) => {
                Self::write_tvshow_nfo(writer, tvshow, &config.nfo_config).await?;
            }
            NFO::Upper(upper) => {
                Self::write_upper_nfo(writer, upper).await?;
            }
            NFO::Episode(episode) => {
                Self::write_episode_nfo(writer, episode, &config.nfo_config).await?;
            }
        }
        tokio_buffer.flush().await?;
        Ok(String::from_utf8(buffer)?)
    }

    async fn write_movie_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, movie: Movie<'_>, config: &NFOConfig) -> Result<()> {
        // 验证数据有效性
        if !Self::validate_nfo_data(movie.name, movie.bvid, movie.upper_name) {
            return Err(anyhow::anyhow!("Invalid NFO data: name='{}', bvid='{}', upper_name='{}'", movie.name, movie.bvid, movie.upper_name));
        }

        writer
            .create_element("movie")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                // 标题信息
                let (display_title, original_title) = if Self::is_bangumi_video(movie.category) {
                    // 对于番剧，尝试提取番剧名称作为主标题
                    if let Some(bangumi_title) = Self::extract_bangumi_title_from_full_name(movie.name) {
                        (bangumi_title, movie.name.to_string())
                    } else {
                        (movie.name.to_string(), movie.original_title.to_string())
                    }
                } else {
                    (movie.name.to_string(), movie.original_title.to_string())
                };

                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(&display_title))
                    .await?;
                writer
                    .create_element("originaltitle")
                    .write_text_content_async(BytesText::new(&original_title))
                    .await?;

                // 标语/副标题
                if let Some(ref tagline) = movie.tagline {
                    writer
                        .create_element("tagline")
                        .write_text_content_async(BytesText::new(tagline))
                        .await?;
                }

                // 排序标题
                if let Some(ref sorttitle) = movie.sorttitle {
                    writer
                        .create_element("sorttitle")
                        .write_text_content_async(BytesText::new(sorttitle))
                        .await?;
                } else {
                    // 使用显示标题作为默认排序标题
                    writer
                        .create_element("sorttitle")
                        .write_text_content_async(BytesText::new(&display_title))
                        .await?;
                }

                // 系列信息
                if let Some(ref set_name) = movie.set {
                    writer
                        .create_element("set")
                        .write_inner_content_async::<_, _, Error>(|writer| async move {
                            writer
                                .create_element("name")
                                .write_text_content_async(BytesText::new(set_name))
                                .await?;
                            Ok(writer)
                        })
                        .await?;
                }
                
                // 评分信息
                if let Some(rating) = movie.user_rating {
                    writer
                        .create_element("userrating")
                        .write_text_content_async(BytesText::new(&rating.to_string()))
                        .await?;
                }
                
                // 剧情简介
                writer
                    .create_element("plot")
                    .write_cdata_content_async(BytesCData::new(Self::format_plot(movie.bvid, movie.intro)))
                    .await?;
                writer.create_element("outline").write_empty_async().await?;
                
                // 分级信息
                if let Some(mpaa) = movie.mpaa {
                    writer
                        .create_element("mpaa")
                        .write_text_content_async(BytesText::new(mpaa))
                        .await?;
                }
                
                // 唯一标识符
                writer
                    .create_element("uniqueid")
                    .with_attribute(("type", "bilibili"))
                    .with_attribute(("default", "true"))
                    .write_text_content_async(BytesText::new(movie.bvid))
                    .await?;
                
                // 类型标签
                if let Some(tags) = movie.tags {
                    for tag in tags {
                        writer
                            .create_element("genre")
                            .write_text_content_async(BytesText::new(&tag))
                            .await?;
                    }
                }
                
                // 为番剧剧场版添加默认类型标签
                if Self::is_bangumi_video(movie.category) {
                    writer
                        .create_element("genre")
                        .write_text_content_async(BytesText::new("动画"))
                        .await?;
                    writer
                        .create_element("genre")
                        .write_text_content_async(BytesText::new("剧场版"))
                        .await?;
                }
                
                // 国家信息
                if let Some(country) = movie.country {
                    writer
                        .create_element("country")
                        .write_text_content_async(BytesText::new(country))
                        .await?;
                } else {
                    writer
                        .create_element("country")
                        .write_text_content_async(BytesText::new(&config.default_country))
                        .await?;
                }
                
                // 创作人员信息
                if let Some(credits) = movie.credits {
                    writer
                        .create_element("credits")
                        .write_text_content_async(BytesText::new(credits))
                        .await?;
                }
                
                if let Some(director) = movie.director {
                    writer
                        .create_element("director")
                        .write_text_content_async(BytesText::new(director))
                        .await?;
                }
                
                // 时间信息
                writer
                    .create_element("year")
                    .write_text_content_async(BytesText::new(&movie.aired.format("%Y").to_string()))
                    .await?;
                writer
                    .create_element("premiered")
                    .write_text_content_async(BytesText::new(&movie.premiered.format("%Y-%m-%d").to_string()))
                    .await?;
                writer
                    .create_element("aired")
                    .write_text_content_async(BytesText::new(&movie.aired.format("%Y-%m-%d").to_string()))
                    .await?;
                
                // 制作信息
                if let Some(studio) = movie.studio {
                    writer
                        .create_element("studio")
                        .write_text_content_async(BytesText::new(studio))
                        .await?;
                } else {
                    writer
                        .create_element("studio")
                        .write_text_content_async(BytesText::new(&config.default_studio))
                        .await?;
                }
                
                // 演员信息（优先使用真实演员信息，备选UP主）
                if config.include_actor_info {
                    // 首先尝试使用真实演员信息
                    if let Some(ref actors_str) = movie.actors_info {
                        let actors = Self::parse_actors_string(actors_str);
                        for (index, (character, actor)) in actors.iter().enumerate() {
                            writer
                                .create_element("actor")
                                .write_inner_content_async::<_, _, Error>(|writer| async move {
                                    writer
                                        .create_element("name")
                                        .write_text_content_async(BytesText::new(actor))
                                        .await?;
                                    writer
                                        .create_element("role")
                                        .write_text_content_async(BytesText::new(character))
                                        .await?;
                                    writer
                                        .create_element("order")
                                        .write_text_content_async(BytesText::new(&(index + 1).to_string()))
                                        .await?;
                                    Ok(writer)
                                })
                                .await?;
                        }
                    } else {
                        // 备选：使用UP主信息作为创作者
                        let actor_name = Self::get_actor_name(movie.upper_name, config);
                        if let Some(name) = actor_name {
                            writer
                                .create_element("actor")
                                .write_inner_content_async::<_, _, Error>(|writer| async move {
                                    writer
                                        .create_element("name")
                                        .write_text_content_async(BytesText::new(&name))
                                        .await?;
                                    writer
                                        .create_element("role")
                                        .write_text_content_async(BytesText::new("创作者"))
                                        .await?;
                                    writer
                                        .create_element("order")
                                        .write_text_content_async(BytesText::new("1"))
                                        .await?;
                                    Ok(writer)
                                })
                                .await?;
                        }
                    }
                }
                
                // 时长信息
                if let Some(duration) = movie.duration {
                    writer
                        .create_element("runtime")
                        .write_text_content_async(BytesText::new(&duration.to_string()))
                        .await?;
                }
                
                // B站特有信息作为自定义标签
                if config.include_bilibili_info {
                    if let Some(view_count) = movie.view_count {
                        writer
                            .create_element("tag")
                            .write_text_content_async(BytesText::new(&format!("播放量: {}", view_count)))
                            .await?;
                    }
                    
                    if let Some(like_count) = movie.like_count {
                        writer
                            .create_element("tag")
                            .write_text_content_async(BytesText::new(&format!("点赞数: {}", like_count)))
                            .await?;
                    }
                }
                
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_tvshow_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, tvshow: TVShow<'_>, config: &NFOConfig) -> Result<()> {
        // 验证数据有效性
        if !Self::validate_nfo_data(tvshow.name, tvshow.bvid, tvshow.upper_name) {
            return Err(anyhow::anyhow!("Invalid NFO data: name='{}', bvid='{}', upper_name='{}'", tvshow.name, tvshow.bvid, tvshow.upper_name));
        }

        writer
            .create_element("tvshow")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                // 标题信息
                let (display_title, original_title) = if Self::is_bangumi_video(tvshow.category) {
                    // 对于番剧，尝试提取番剧名称作为主标题
                    if let Some(bangumi_title) = Self::extract_bangumi_title_from_full_name(tvshow.name) {
                        (bangumi_title, tvshow.name.to_string())
                    } else {
                        (tvshow.name.to_string(), tvshow.original_title.to_string())
                    }
                } else {
                    (tvshow.name.to_string(), tvshow.original_title.to_string())
                };

                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(&display_title))
                    .await?;
                writer
                    .create_element("originaltitle")
                    .write_text_content_async(BytesText::new(&original_title))
                    .await?;

                // 标语/副标题
                if let Some(ref tagline) = tvshow.tagline {
                    writer
                        .create_element("tagline")
                        .write_text_content_async(BytesText::new(tagline))
                        .await?;
                }

                // 排序标题
                if let Some(ref sorttitle) = tvshow.sorttitle {
                    writer
                        .create_element("sorttitle")
                        .write_text_content_async(BytesText::new(sorttitle))
                        .await?;
                } else {
                    // 使用显示标题作为默认排序标题
                    writer
                        .create_element("sorttitle")
                        .write_text_content_async(BytesText::new(&display_title))
                        .await?;
                }

                // 系列信息
                if let Some(ref set_name) = tvshow.set {
                    writer
                        .create_element("set")
                        .write_inner_content_async::<_, _, Error>(|writer| async move {
                            writer
                                .create_element("name")
                                .write_text_content_async(BytesText::new(set_name))
                                .await?;
                            Ok(writer)
                        })
                        .await?;
                }
                    
                // 剧情简介
                writer
                    .create_element("plot")
                    .write_cdata_content_async(BytesCData::new(Self::format_plot(tvshow.bvid, tvshow.intro)))
                    .await?;
                writer.create_element("outline").write_empty_async().await?;
                
                // 评分信息
                if let Some(rating) = tvshow.user_rating {
                    writer
                        .create_element("userrating")
                        .write_text_content_async(BytesText::new(&rating.to_string()))
                        .await?;
                }
                
                // 分级信息
                if let Some(mpaa) = tvshow.mpaa {
                    writer
                        .create_element("mpaa")
                        .write_text_content_async(BytesText::new(mpaa))
                        .await?;
                }
                
                // 唯一标识符
                writer
                    .create_element("uniqueid")
                    .with_attribute(("type", "bilibili"))
                    .with_attribute(("default", "true"))
                    .write_text_content_async(BytesText::new(tvshow.bvid))
                    .await?;
                
                // 类型标签
                if let Some(tags) = tvshow.tags {
                    for tag in tags {
                        writer
                            .create_element("genre")
                            .write_text_content_async(BytesText::new(&tag))
                            .await?;
                    }
                }
                
                // 国家信息
                if let Some(country) = tvshow.country {
                    writer
                        .create_element("country")
                        .write_text_content_async(BytesText::new(country))
                        .await?;
                } else {
                    writer
                        .create_element("country")
                        .write_text_content_async(BytesText::new(&config.default_country))
                        .await?;
                }
                
                // 播出状态
                if let Some(status) = tvshow.status {
                    writer
                        .create_element("status")
                        .write_text_content_async(BytesText::new(status))
                        .await?;
                } else {
                    writer
                        .create_element("status")
                        .write_text_content_async(BytesText::new(&config.default_tvshow_status))
                        .await?;
                }
                
                // 季数和集数信息
                if let Some(total_seasons) = tvshow.total_seasons {
                    writer
                        .create_element("totalseasons")
                        .write_text_content_async(BytesText::new(&total_seasons.to_string()))
                        .await?;
                }
                
                if let Some(total_episodes) = tvshow.total_episodes {
                    writer
                        .create_element("totalepisodes")
                        .write_text_content_async(BytesText::new(&total_episodes.to_string()))
                        .await?;
                }
                
                // 时间信息
                writer
                    .create_element("year")
                    .write_text_content_async(BytesText::new(&tvshow.aired.format("%Y").to_string()))
                    .await?;
                writer
                    .create_element("premiered")
                    .write_text_content_async(BytesText::new(&tvshow.premiered.format("%Y-%m-%d").to_string()))
                    .await?;
                writer
                    .create_element("aired")
                    .write_text_content_async(BytesText::new(&tvshow.aired.format("%Y-%m-%d").to_string()))
                    .await?;
                
                // 制作信息
                if let Some(studio) = tvshow.studio {
                    writer
                        .create_element("studio")
                        .write_text_content_async(BytesText::new(studio))
                        .await?;
                } else {
                    writer
                        .create_element("studio")
                        .write_text_content_async(BytesText::new(&config.default_studio))
                        .await?;
                }
                
                // 演员信息（优先使用真实演员信息，备选UP主）
                if config.include_actor_info {
                    // 首先尝试使用真实演员信息
                    if let Some(ref actors_str) = tvshow.actors_info {
                        let actors = Self::parse_actors_string(actors_str);
                        for (index, (character, actor)) in actors.iter().enumerate() {
                            writer
                                .create_element("actor")
                                .write_inner_content_async::<_, _, Error>(|writer| async move {
                                    writer
                                        .create_element("name")
                                        .write_text_content_async(BytesText::new(actor))
                                        .await?;
                                    writer
                                        .create_element("role")
                                        .write_text_content_async(BytesText::new(character))
                                        .await?;
                                    writer
                                        .create_element("order")
                                        .write_text_content_async(BytesText::new(&(index + 1).to_string()))
                                        .await?;
                                    Ok(writer)
                                })
                                .await?;
                        }
                    } else {
                        // 备选：使用UP主信息作为创作者
                        let actor_name = Self::get_actor_name(tvshow.upper_name, config);
                        if let Some(name) = actor_name {
                            writer
                                .create_element("actor")
                                .write_inner_content_async::<_, _, Error>(|writer| async move {
                                    writer
                                        .create_element("name")
                                        .write_text_content_async(BytesText::new(&name))
                                        .await?;
                                    writer
                                        .create_element("role")
                                        .write_text_content_async(BytesText::new("创作者"))
                                        .await?;
                                    writer
                                        .create_element("order")
                                        .write_text_content_async(BytesText::new("1"))
                                        .await?;
                                    Ok(writer)
                                })
                                .await?;
                        }
                    }
                }
                
                // B站特有信息作为自定义标签
                if config.include_bilibili_info {
                    if let Some(view_count) = tvshow.view_count {
                        writer
                            .create_element("tag")
                            .write_text_content_async(BytesText::new(&format!("播放量: {}", view_count)))
                            .await?;
                    }
                    
                    if let Some(like_count) = tvshow.like_count {
                        writer
                            .create_element("tag")
                            .write_text_content_async(BytesText::new(&format!("点赞数: {}", like_count)))
                            .await?;
                    }
                }
                
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_upper_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, upper: Upper) -> Result<()> {
        writer
            .create_element("person")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                writer.create_element("plot").write_empty_async().await?;
                writer.create_element("outline").write_empty_async().await?;
                writer
                    .create_element("lockdata")
                    .write_text_content_async(BytesText::new("false"))
                    .await?;
                writer
                    .create_element("dateadded")
                    .write_text_content_async(BytesText::new(&upper.pubtime.format("%Y-%m-%d %H:%M:%S").to_string()))
                    .await?;
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(&upper.upper_id))
                    .await?;
                writer
                    .create_element("sorttitle")
                    .write_text_content_async(BytesText::new(&upper.upper_id))
                    .await?;
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_episode_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, episode: Episode<'_>, _config: &NFOConfig) -> Result<()> {
        writer
            .create_element("episodedetails")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                // 标题信息
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(episode.name))
                    .await?;
                writer
                    .create_element("originaltitle")
                    .write_text_content_async(BytesText::new(episode.original_title))
                    .await?;
                
                // 剧情简介
                if let Some(plot) = episode.plot {
                    writer
                        .create_element("plot")
                        .write_cdata_content_async(BytesCData::new(plot))
                        .await?;
                } else {
                    writer.create_element("plot").write_empty_async().await?;
                }
                writer.create_element("outline").write_empty_async().await?;
                
                // 季集信息
                writer
                    .create_element("season")
                    .write_text_content_async(BytesText::new(&episode.season.to_string()))
                    .await?;
                writer
                    .create_element("episode")
                    .write_text_content_async(BytesText::new(&episode.episode_number.to_string()))
                    .await?;
                
                // 播出时间
                if let Some(aired) = episode.aired {
                    writer
                        .create_element("aired")
                        .write_text_content_async(BytesText::new(&aired.format("%Y-%m-%d").to_string()))
                        .await?;
                }
                
                // 时长信息
                if let Some(duration) = episode.duration {
                    writer
                        .create_element("runtime")
                        .write_text_content_async(BytesText::new(&duration.to_string()))
                        .await?;
                }
                
                // 评分信息
                if let Some(rating) = episode.user_rating {
                    writer
                        .create_element("userrating")
                        .write_text_content_async(BytesText::new(&rating.to_string()))
                        .await?;
                }
                
                // 创作人员信息
                if let Some(director) = episode.director {
                    writer
                        .create_element("director")
                        .write_text_content_async(BytesText::new(director))
                        .await?;
                }
                
                if let Some(credits) = episode.credits {
                    writer
                        .create_element("credits")
                        .write_text_content_async(BytesText::new(credits))
                        .await?;
                }
                
                // 唯一标识符
                writer
                    .create_element("uniqueid")
                    .with_attribute(("type", "bilibili"))
                    .with_attribute(("default", "true"))
                    .write_text_content_async(BytesText::new(&episode.pid))
                    .await?;
                
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    #[inline]
    fn format_plot(bvid: &str, intro: &str) -> String {
        format!(
            r#"原始视频：<a href="https://www.bilibili.com/video/{}/">{}</a><br/><br/>{}"#,
            bvid, bvid, intro,
        )
    }

    /// 检测是否为番剧视频（基于 category 字段）
    fn is_bangumi_video(category: i32) -> bool {
        category == 1
    }

    /// 从完整标题中提取番剧名称
    fn extract_bangumi_title_from_full_name(full_name: &str) -> Option<String> {
        // 匹配 "《番剧名称》第X话/集 副标题" 格式
        let pattern1 = regex::Regex::new(r"《([^》]+)》").unwrap();
        if let Some(caps) = pattern1.captures(full_name) {
            return Some(caps[1].to_string());
        }
        
        // 匹配 "番剧名称 第X话/集" 格式
        let pattern2 = regex::Regex::new(r"^([^第]+)\s*第\d+[话集]").unwrap();
        if let Some(caps) = pattern2.captures(full_name) {
            return Some(caps[1].trim().to_string());
        }
        
        // 匹配番剧标题后跟描述性文本（如"柯南剧场版开山之作"）
        let pattern3 = regex::Regex::new(r"《([^》]+)》(.+)").unwrap();
        if let Some(caps) = pattern3.captures(full_name) {
            let title = caps[1].trim();
            let subtitle = caps[2].trim();
            // 如果副标题不是集数信息，则只返回主标题
            if !subtitle.contains("第") && !subtitle.contains("话") && !subtitle.contains("集") {
                return Some(title.to_string());
            }
        }
        
        None
    }

    /// 从share_copy或标题中提取副标题信息
    fn extract_subtitle_from_share_copy(share_copy: &str) -> Option<String> {
        // 匹配 "《番剧名称》副标题" 格式，提取副标题
        let pattern = regex::Regex::new(r"《[^》]+》\s*(.+)").unwrap();
        if let Some(caps) = pattern.captures(share_copy) {
            let subtitle = caps[1].trim();
            // 过滤掉一些常见的无意义副标题
            if !subtitle.is_empty() && 
               !subtitle.contains("第") && 
               !subtitle.contains("话") && 
               !subtitle.contains("集") &&
               subtitle != "日配" &&  // 过滤掉语言标识
               subtitle != "中配" &&
               subtitle != "国语" &&
               subtitle != "粤语" &&
               subtitle.len() > 2 {
                return Some(subtitle.to_string());
            }
        }
        None
    }

    /// 从完整标题中提取集数信息
    fn extract_episode_info_from_full_name(full_name: &str) -> String {
        // 匹配 "第X话/集 副标题" 格式
        let pattern = regex::Regex::new(r"第(\d+)[话集]\s*(.*)").unwrap();
        if let Some(caps) = pattern.captures(full_name) {
            let episode_num = &caps[1];
            let subtitle = caps.get(2).map_or("", |m| m.as_str().trim());
            
            if subtitle.is_empty() {
                format!("第{}集", episode_num)
            } else {
                format!("第{}集 {}", episode_num, subtitle)
            }
        } else {
            // 如果无法提取，返回原标题
            full_name.to_string()
        }
    }

    /// 验证NFO数据的有效性
    fn validate_nfo_data(name: &str, bvid: &str, _upper_name: &str) -> bool {
        // 检查基本字段是否有效
        !name.trim().is_empty() && 
        !bvid.trim().is_empty() && 
        bvid.starts_with("BV") &&
        bvid.len() >= 10 // BV号最少10位
    }

    /// 根据配置策略获取演员名称
    fn get_actor_name(upper_name: &str, config: &NFOConfig) -> Option<String> {
        let trimmed_name = upper_name.trim();
        
        if !trimmed_name.is_empty() {
            // UP主名称不为空，直接使用
            return Some(trimmed_name.to_string());
        }
        
        // UP主名称为空，根据策略处理
        match config.empty_upper_strategy {
            EmptyUpperStrategy::Skip => None,
            EmptyUpperStrategy::Placeholder => Some(config.empty_upper_placeholder.clone()),
            EmptyUpperStrategy::Default => Some(config.empty_upper_default_name.clone()),
        }
    }

    /// 解析演员信息字符串，返回 (角色名, 声优名) 的向量
    /// 输入格式如："江户川柯南：高山南\n毛利兰：山崎和佳奈\n毛利小五郎：神谷明"
    fn parse_actors_string(actors_str: &str) -> Vec<(String, String)> {
        let mut actors = Vec::new();
        
        for line in actors_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // 支持多种分隔符：全角冒号、半角冒号
            if let Some(pos) = line.find('：') {
                let character = line[..pos].trim().to_string();
                let actor = line[pos + 3..].trim().to_string(); // 全角冒号占3字节
                if !character.is_empty() && !actor.is_empty() {
                    actors.push((character, actor));
                }
            } else if let Some(pos) = line.find(':') {
                let character = line[..pos].trim().to_string();
                let actor = line[pos + 1..].trim().to_string();
                if !character.is_empty() && !actor.is_empty() {
                    actors.push((character, actor));
                }
            } else {
                // 如果没有分隔符，可能是单独的演员名，作为通用演员处理
                if !line.is_empty() {
                    actors.push(("演员".to_string(), line.to_string()));
                }
            }
        }
        
        actors
    }
}

impl<'a> From<&'a video::Model> for Movie<'a> {
    fn from(video: &'a video::Model) -> Self {
        // 使用动态配置而非静态CONFIG
        let config = crate::config::reload_config();

        // 对于番剧影视类型（show_season_type=2），使用share_copy作为标题
        // 其他类型继续使用video.name
        let nfo_title = if video.show_season_type == Some(2) {
            video.share_copy.as_deref().unwrap_or(&video.name)
        } else {
            &video.name
        };


        let aired_time = match config.nfo_config.time_type {
            NFOTimeType::FavTime => video.favtime,
            NFOTimeType::PubTime => video.pubtime,
        };

        // 提取标语/副标题
        let tagline = if video.show_season_type == Some(2) {
            video.share_copy.as_ref().and_then(|sc| NFO::extract_subtitle_from_share_copy(sc))
        } else {
            None
        };

        // 生成排序标题（去除特殊字符，便于排序）
        let sorttitle = Some({
            // 对于番剧，使用提取的系列名称；否则使用原标题
            if NFO::is_bangumi_video(video.category) {
                if let Some(bangumi_title) = NFO::extract_bangumi_title_from_full_name(nfo_title) {
                    bangumi_title
                } else {
                    // 如果提取失败，手动清理标题
                    nfo_title.replace("《", "").replace("》", "").split_whitespace().next().unwrap_or(nfo_title).to_string()
                }
            } else {
                nfo_title.to_string()
            }
        });

        // 对于番剧，尝试提取系列名称
        let set_name = if NFO::is_bangumi_video(video.category) {
            NFO::extract_bangumi_title_from_full_name(nfo_title)
        } else {
            None
        };

        Self {
            name: nfo_title,
            original_title: &video.name,
            intro: &video.intro,
            bvid: &video.bvid,
            upper_id: video.upper_id,
            upper_name: &video.upper_name,
            aired: aired_time,
            premiered: aired_time,
            tags: video
                .tags
                .as_ref()
                .and_then(|tags| serde_json::from_value(tags.clone()).ok()),
            user_rating: None, // B站没有评分系统
            mpaa: None, // 使用默认分级
            country: None, // 使用默认值（中国）
            studio: None, // 使用默认值（哔哩哔哩）
            director: None, // UP主信息在actor中
            credits: None, // UP主信息在actor中
            duration: None, // video模型中没有duration字段
            view_count: None, // video模型中没有view_count字段
            like_count: None, // video模型中没有like_count字段
            category: video.category,
            tagline,
            set: set_name,
            sorttitle,
            actors_info: video.actors.clone(),
        }
    }
}

impl<'a> From<&'a video::Model> for TVShow<'a> {
    fn from(video: &'a video::Model) -> Self {
        // 使用动态配置而非静态CONFIG
        let config = crate::config::reload_config();

        // 对于番剧影视类型（show_season_type=2），使用share_copy作为标题
        // 其他类型继续使用video.name
        let nfo_title = if video.show_season_type == Some(2) {
            video.share_copy.as_deref().unwrap_or(&video.name)
        } else {
            &video.name
        };

        let aired_time = match config.nfo_config.time_type {
            NFOTimeType::FavTime => video.favtime,
            NFOTimeType::PubTime => video.pubtime,
        };

        // 提取标语/副标题
        let tagline = if video.show_season_type == Some(2) {
            video.share_copy.as_ref().and_then(|sc| NFO::extract_subtitle_from_share_copy(sc))
        } else {
            None
        };

        // 生成排序标题（去除特殊字符，便于排序）
        let sorttitle = Some({
            // 对于番剧，使用提取的系列名称；否则使用原标题
            if NFO::is_bangumi_video(video.category) {
                if let Some(bangumi_title) = NFO::extract_bangumi_title_from_full_name(nfo_title) {
                    bangumi_title
                } else {
                    // 如果提取失败，手动清理标题
                    nfo_title.replace("《", "").replace("》", "").split_whitespace().next().unwrap_or(nfo_title).to_string()
                }
            } else {
                nfo_title.to_string()
            }
        });

        // 对于番剧，尝试提取系列名称
        let set_name = if NFO::is_bangumi_video(video.category) {
            NFO::extract_bangumi_title_from_full_name(nfo_title)
        } else {
            None
        };

        Self {
            name: nfo_title,
            original_title: &video.name,
            intro: &video.intro,
            bvid: &video.bvid,
            upper_id: video.upper_id,
            upper_name: &video.upper_name,
            aired: aired_time,
            premiered: aired_time,
            tags: video
                .tags
                .as_ref()
                .and_then(|tags| serde_json::from_value(tags.clone()).ok()),
            user_rating: None, // B站没有评分系统
            mpaa: None, // 使用默认分级
            country: None, // 使用默认值（中国）
            studio: None, // 使用默认值（哔哩哔哩）
            status: Some("Continuing"), // 默认持续播出状态
            total_seasons: Some(1), // 默认单季
            total_episodes: None, // 从分页数量推断
            view_count: None, // video模型中没有view_count字段
            like_count: None, // video模型中没有like_count字段
            category: video.category,
            tagline,
            set: set_name,
            sorttitle,
            actors_info: video.actors.clone(),
        }
    }
}

impl<'a> From<&'a video::Model> for Upper {
    fn from(video: &'a video::Model) -> Self {
        Self {
            upper_id: video.upper_id.to_string(),
            pubtime: video.pubtime,
        }
    }
}

impl<'a> From<&'a page::Model> for Episode<'a> {
    fn from(page: &'a page::Model) -> Self {
        Self {
            name: &page.name,
            original_title: &page.name,
            pid: page.pid.to_string(),
            plot: None, // 分页没有单独简介
            season: 1, // 默认第一季
            episode_number: page.pid, // 使用页面ID作为集数
            aired: None, // 分页没有单独播出时间
            duration: Some(page.duration as i32 / 60), // 分页时长转换为分钟
            user_rating: None, // 分页没有单独评分
            director: None, // 分页没有单独导演信息
            credits: None, // 分页没有单独创作人员信息
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_nfo() {
        let video = video::Model {
            intro: "intro".to_string(),
            name: "name".to_string(),
            upper_id: 1,
            upper_name: "upper_name".to_string(),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2022, 2, 2).unwrap(),
                chrono::NaiveTime::from_hms_opt(2, 2, 2).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2033, 3, 3).unwrap(),
                chrono::NaiveTime::from_hms_opt(3, 3, 3).unwrap(),
            ),
            bvid: "BV1nWcSeeEkV".to_string(),
            tags: Some(serde_json::json!(["tag1", "tag2"])),
            ..Default::default()
        };

        
        let generated_movie = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        // 由于XML字段顺序可能有差异，我们检查关键字段是否存在
        assert!(generated_movie.contains("<title>name</title>"));
        assert!(generated_movie.contains("<originaltitle>name</originaltitle>"));
        assert!(generated_movie.contains(r#"<uniqueid type="bilibili" default="true">BV1nWcSeeEkV</uniqueid>"#));
        assert!(generated_movie.contains("<country>中国</country>"));
        assert!(generated_movie.contains("<studio>哔哩哔哩</studio>"));
        assert!(generated_movie.contains("<role>创作者</role>"));

        let generated_tvshow = NFO::TVShow((&video).into()).generate_nfo().await.unwrap();
        // 检查TVShow的关键字段
        assert!(generated_tvshow.contains("<title>name</title>"));
        assert!(generated_tvshow.contains("<originaltitle>name</originaltitle>"));
        assert!(generated_tvshow.contains(r#"<uniqueid type="bilibili" default="true">BV1nWcSeeEkV</uniqueid>"#));
        assert!(generated_tvshow.contains("<status>Continuing</status>"));
        assert!(generated_tvshow.contains("<totalseasons>1</totalseasons>"));
        assert!(generated_tvshow.contains("<country>中国</country>"));
        assert!(generated_tvshow.contains("<studio>哔哩哔哩</studio>"));
        assert!(generated_tvshow.contains("<role>创作者</role>"));

        assert_eq!(
            NFO::Upper((&video).into()).generate_nfo().await.unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<person>
    <plot/>
    <outline/>
    <lockdata>false</lockdata>
    <dateadded>2033-03-03 03:03:03</dateadded>
    <title>1</title>
    <sorttitle>1</sorttitle>
</person>"#,
        );

        let page = page::Model {
            name: "name".to_string(),
            pid: 3,
            ..Default::default()
        };
        
        let generated_episode = NFO::Episode((&page).into()).generate_nfo().await.unwrap();
        // 检查Episode的关键字段
        assert!(generated_episode.contains("<title>name</title>"));
        assert!(generated_episode.contains("<originaltitle>name</originaltitle>"));
        assert!(generated_episode.contains("<season>1</season>"));
        assert!(generated_episode.contains("<episode>3</episode>"));
        assert!(generated_episode.contains(r#"<uniqueid type="bilibili" default="true">3</uniqueid>"#));
    }

    #[tokio::test]
    async fn test_empty_upper_name() {
        // 测试空UP主名称的处理
        let video = video::Model {
            intro: "测试视频介绍".to_string(),
            name: "官方内容".to_string(),
            upper_id: 0,
            upper_name: "".to_string(), // 空的UP主名称
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            bvid: "BV1234567890".to_string(),
            tags: Some(serde_json::json!(["官方", "番剧"])),
            ..Default::default()
        };

        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证没有生成空的演员信息
        assert!(!movie_nfo.contains("<name></name>"));
        assert!(!movie_nfo.contains("<actor>"));
        
        let tvshow_nfo = NFO::TVShow((&video).into()).generate_nfo().await.unwrap();
        
        // 验证没有生成空的演员信息
        assert!(!tvshow_nfo.contains("<name></name>"));
        assert!(!tvshow_nfo.contains("<actor>"));
        
        println!("空UP主名称的Movie NFO:");
        println!("{}", movie_nfo);
        println!("\n空UP主名称的TVShow NFO:");
        println!("{}", tvshow_nfo);
    }

    #[tokio::test]
    async fn test_bangumi_title_optimization() {
        // 测试番剧标题优化
        let video = video::Model {
            intro: "数百年前，欲望催生了几乎灭绝人类的玛娜生态。".to_string(),
            name: "《灵笼 第二季》第1话 末世桃源".to_string(),
            upper_id: 0,
            upper_name: "".to_string(),
            category: 1, // 番剧分类
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2025, 5, 23).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2025, 5, 23).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            bvid: "BV1bSJez1Et8".to_string(),
            tags: Some(serde_json::json!(["动画", "科幻"])),
            ..Default::default()
        };

        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证标题优化
        assert!(movie_nfo.contains("<title>灵笼 第二季</title>"));
        assert!(movie_nfo.contains("<originaltitle>《灵笼 第二季》第1话 末世桃源</originaltitle>"));
        
        // 验证没有空的演员信息
        assert!(!movie_nfo.contains("<actor>"));
        
        println!("优化后的番剧Movie NFO:");
        println!("{}", movie_nfo);
    }

    #[tokio::test]
    async fn test_nfo_kodi_compatibility() {
        // 测试生成的NFO文件是否符合Kodi标准
        let video = video::Model {
            intro: "测试视频介绍".to_string(),
            name: "测试视频".to_string(),
            upper_id: 123456,
            upper_name: "测试UP主".to_string(),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            bvid: "BV1234567890".to_string(),
            tags: Some(serde_json::json!(["科技", "教程", "编程"])),
            ..Default::default()
        };

        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证Kodi Movie必需字段
        assert!(movie_nfo.contains("<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>"));
        assert!(movie_nfo.contains("<movie>"));
        assert!(movie_nfo.contains("</movie>"));
        assert!(movie_nfo.contains("<title>"));
        assert!(movie_nfo.contains("<originaltitle>"));
        assert!(movie_nfo.contains("<plot>"));
        assert!(movie_nfo.contains("<uniqueid"));
        assert!(movie_nfo.contains("type=\"bilibili\""));
        assert!(movie_nfo.contains("<year>"));
        assert!(movie_nfo.contains("<premiered>"));
        assert!(movie_nfo.contains("<aired>"));
        assert!(movie_nfo.contains("<studio>"));
        assert!(movie_nfo.contains("<country>"));
        
        let tvshow_nfo = NFO::TVShow((&video).into()).generate_nfo().await.unwrap();
        
        // 验证Kodi TVShow必需字段
        assert!(tvshow_nfo.contains("<tvshow>"));
        assert!(tvshow_nfo.contains("</tvshow>"));
        assert!(tvshow_nfo.contains("<status>"));
        assert!(tvshow_nfo.contains("<totalseasons>"));
        
        println!("生成的Movie NFO:");
        println!("{}", movie_nfo);
        println!("\n生成的TVShow NFO:");
        println!("{}", tvshow_nfo);
    }

    #[tokio::test]
    async fn test_empty_upper_strategy_configurations() {
        use crate::config::{EmptyUpperStrategy, NFOConfig};
        
        // 测试空UP主名称的各种处理策略
        let video = video::Model {
            intro: "测试视频介绍".to_string(),
            name: "官方内容".to_string(),
            upper_id: 0,
            upper_name: "".to_string(), // 空的UP主名称
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            bvid: "BV1234567890".to_string(),
            tags: Some(serde_json::json!(["官方", "番剧"])),
            ..Default::default()
        };

        // 测试Skip策略（默认）
        let mut config = NFOConfig::default();
        config.empty_upper_strategy = EmptyUpperStrategy::Skip;
        
        // 创建一个自定义的Movie结构并手动生成NFO
        let movie = Movie::from(&video);
        let actor_name = NFO::get_actor_name(movie.upper_name, &config);
        assert_eq!(actor_name, None);

        // 测试Placeholder策略
        config.empty_upper_strategy = EmptyUpperStrategy::Placeholder;
        config.empty_upper_placeholder = "官方内容".to_string();
        
        let actor_name = NFO::get_actor_name(movie.upper_name, &config);
        assert_eq!(actor_name, Some("官方内容".to_string()));

        // 测试Default策略
        config.empty_upper_strategy = EmptyUpperStrategy::Default;
        config.empty_upper_default_name = "哔哩哔哩".to_string();
        
        let actor_name = NFO::get_actor_name(movie.upper_name, &config);
        assert_eq!(actor_name, Some("哔哩哔哩".to_string()));

        // 测试非空UP主名称（应该直接使用原名称，不受策略影响）
        let actor_name = NFO::get_actor_name("测试UP主", &config);
        assert_eq!(actor_name, Some("测试UP主".to_string()));
        
        println!("空UP主处理策略测试通过");
    }

    #[tokio::test]
    async fn test_enhanced_nfo_features() {
        // 测试增强的NFO功能，包括tagline、set、sorttitle等
        let video = video::Model {
            intro: "数百年前，欲望催生了几乎灭绝人类的玛娜生态。".to_string(),
            name: "《名侦探柯南 计时引爆摩天楼》柯南剧场版开山之作".to_string(),
            upper_id: 0,
            upper_name: "".to_string(),
            category: 1, // 番剧分类
            show_season_type: Some(2), // 影视剧场版
            share_copy: Some("《名侦探柯南 计时引爆摩天楼》柯南剧场版开山之作".to_string()),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2025, 5, 23).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2025, 5, 23).unwrap(),
                chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            ),
            bvid: "BV1bSJez1Et8".to_string(),
            tags: Some(serde_json::json!(["推理", "智斗", "漫画改"])),
            ..Default::default()
        };

        let movie = Movie::from(&video);
        
        // 验证新字段被正确设置
        assert_eq!(movie.tagline, Some("柯南剧场版开山之作".to_string()));
        assert_eq!(movie.set, Some("名侦探柯南 计时引爆摩天楼".to_string()));
        assert!(movie.sorttitle.is_some());
        
        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证NFO包含新字段
        assert!(movie_nfo.contains("<tagline>柯南剧场版开山之作</tagline>"));
        assert!(movie_nfo.contains("<sorttitle>名侦探柯南 计时引爆摩天楼</sorttitle>"));
        assert!(movie_nfo.contains("<set>"));
        assert!(movie_nfo.contains("<name>名侦探柯南 计时引爆摩天楼</name>"));
        
        // 验证标题提取
        assert!(movie_nfo.contains("<title>名侦探柯南 计时引爆摩天楼</title>"));
        assert!(movie_nfo.contains("<originaltitle>《名侦探柯南 计时引爆摩天楼》柯南剧场版开山之作</originaltitle>"));
        
        println!("增强NFO功能测试通过");
    }

    #[tokio::test]
    async fn test_subtitle_extraction() {
        // 测试副标题提取功能
        let test_cases = vec![
            ("《名侦探柯南 计时引爆摩天楼》柯南剧场版开山之作", Some("柯南剧场版开山之作")),
            ("《灵笼 第二季》第1话 末世桃源", None), // 包含集数信息，应该被过滤
            ("《进击的巨人》最终季", Some("最终季")),
            ("《名侦探柯南 水平线上的阴谋》日配 ", None), // 语言标签，应该被过滤
            ("《某某番剧》中配", None), // 语言标签，应该被过滤
            ("普通视频标题", None),
        ];

        for (input, expected) in test_cases {
            let result = NFO::extract_subtitle_from_share_copy(input);
            assert_eq!(result.as_deref(), expected, "Failed for input: {}", input);
        }

        println!("副标题提取测试通过");
    }

    #[tokio::test]
    async fn test_language_tag_filtering() {
        // 测试语言标签过滤功能
        let video = video::Model {
            intro: "故事以15年前的北大西洋上一件海难作序幕...".to_string(),
            name: "《名侦探柯南 水平线上的阴谋》日配 ".to_string(),
            upper_id: 0,
            upper_name: "".to_string(),
            category: 1, // 番剧分类
            show_season_type: Some(2), // 影视剧场版
            share_copy: Some("《名侦探柯南 水平线上的阴谋》日配 ".to_string()),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2020, 5, 22).unwrap(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2020, 5, 22).unwrap(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            bvid: "BV1Hz411q7vB".to_string(),
            tags: Some(serde_json::json!(["推理", "悬疑"])),
            ..Default::default()
        };

        let movie = Movie::from(&video);
        
        // 验证语言标签被过滤掉了
        assert_eq!(movie.tagline, None); // "日配"应该被过滤掉
        
        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证NFO不包含语言标签作为tagline
        assert!(!movie_nfo.contains("<tagline>日配</tagline>"));
        assert!(!movie_nfo.contains("<tagline>中配</tagline>"));
        
        // 验证包含番剧默认类型标签
        assert!(movie_nfo.contains("<genre>动画</genre>"));
        assert!(movie_nfo.contains("<genre>剧场版</genre>"));
        
        // 验证标题提取正确
        assert!(movie_nfo.contains("<title>名侦探柯南 水平线上的阴谋</title>"));
        
        println!("语言标签过滤测试通过");
    }

    #[tokio::test]
    async fn test_actor_info_parsing() {
        // 测试演员信息解析功能
        let actors_str = "江户川柯南：高山南\n毛利兰：山崎和佳奈\n毛利小五郎：神谷明\n工藤新一：山口胜平\n目暮警部：茶风林";
        
        let actors = NFO::parse_actors_string(actors_str);
        
        assert_eq!(actors.len(), 5);
        assert_eq!(actors[0], ("江户川柯南".to_string(), "高山南".to_string()));
        assert_eq!(actors[1], ("毛利兰".to_string(), "山崎和佳奈".to_string()));
        assert_eq!(actors[2], ("毛利小五郎".to_string(), "神谷明".to_string()));
        assert_eq!(actors[3], ("工藤新一".to_string(), "山口胜平".to_string()));
        assert_eq!(actors[4], ("目暮警部".to_string(), "茶风林".to_string()));
        
        // 测试半角冒号格式
        let actors_en = NFO::parse_actors_string("Character1:Actor1\nCharacter2:Actor2");
        assert_eq!(actors_en.len(), 2);
        assert_eq!(actors_en[0], ("Character1".to_string(), "Actor1".to_string()));
        
        // 测试单独演员名（无角色分隔符）
        let actors_simple = NFO::parse_actors_string("演员名1\n演员名2");
        assert_eq!(actors_simple.len(), 2);
        assert_eq!(actors_simple[0], ("演员".to_string(), "演员名1".to_string()));
        
        println!("演员信息解析测试通过");
    }

    #[tokio::test]
    async fn test_nfo_with_real_actors() {
        // 测试使用真实演员信息生成NFO
        let video = video::Model {
            intro: "故事以15年前的北大西洋上一件海难作序幕...".to_string(),
            name: "《名侦探柯南 计时引爆摩天楼》".to_string(),
            upper_id: 0,
            upper_name: "".to_string(),
            category: 1, // 番剧分类
            show_season_type: Some(2), // 影视剧场版
            share_copy: Some("《名侦探柯南 计时引爆摩天楼》柯南剧场版开山之作".to_string()),
            actors: Some("江户川柯南：高山南\n毛利兰：山崎和佳奈\n毛利小五郎：神谷明".to_string()),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2020, 5, 22).unwrap(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2020, 5, 22).unwrap(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            bvid: "BV1Hz411q7vB".to_string(),
            tags: Some(serde_json::json!(["推理", "悬疑"])),
            ..Default::default()
        };

        let movie_nfo = NFO::Movie((&video).into()).generate_nfo().await.unwrap();
        
        // 验证包含真实演员信息
        assert!(movie_nfo.contains("<actor>"));
        assert!(movie_nfo.contains("<name>高山南</name>"));
        assert!(movie_nfo.contains("<role>江户川柯南</role>"));
        assert!(movie_nfo.contains("<name>山崎和佳奈</name>"));
        assert!(movie_nfo.contains("<role>毛利兰</role>"));
        assert!(movie_nfo.contains("<order>1</order>"));
        assert!(movie_nfo.contains("<order>2</order>"));
        
        // 验证不包含UP主作为演员（因为有真实演员信息）
        assert!(!movie_nfo.contains("<role>创作者</role>"));
        
        println!("真实演员信息NFO生成测试通过");
    }
}

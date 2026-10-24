use bili_sync_entity::video;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, PartialEq)]
pub struct Creator {
    pub uid: String,
    pub name: String,
    pub role: String,
    pub primary: bool,
}

/// Names are never split on punctuation: identity comes from Bilibili's staff UID.
pub fn from_video(video: &video::Model) -> Vec<Creator> {
    if video.source_type == Some(1) {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut authors = Vec::new();
    if video.upper_id > 0 {
        seen.insert(video.upper_id);
        authors.push(Creator {
            uid: video.upper_id.to_string(),
            name: video.upper_name.clone(),
            role: "UP主".into(),
            primary: true,
        });
    }
    if let Some(staff) = video.staff_info.as_ref().and_then(|s| s.as_array()) {
        for person in staff {
            let Some(uid) = person["mid"].as_i64().filter(|id| *id > 0) else {
                continue;
            };
            if !seen.insert(uid) {
                continue;
            }
            authors.push(Creator {
                uid: uid.to_string(),
                name: person["name"].as_str().unwrap_or("").to_string(),
                role: person["title"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("联合创作者")
                    .to_string(),
                primary: false,
            });
        }
    }
    authors
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creators_keep_names_separate_and_deduplicate_by_uid() {
        let video = video::Model {
            upper_id: 1,
            upper_name: "A & B".into(),
            staff_info: Some(serde_json::json!([
                {"mid": 1, "name": "Remote primary"}, {"mid": 2, "name": "Other", "title": "配音"},
                {"mid": 2, "name": "Duplicate"}, {"mid": 0, "name": "Unknown"}
            ])),
            ..Default::default()
        };
        let authors = from_video(&video);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].name, "A & B");
        assert_eq!(authors[1].uid, "2");
        assert!(!authors[1].primary);
    }
    #[test]
    fn refreshed_details_restore_primary_owner_and_store_each_staff_identity() {
        let details: crate::bilibili::VideoInfo = serde_json::from_value(serde_json::json!({
            "title":"Video", "bvid":"BV1nWcSeeEkV", "desc":"", "pic":"",
            "owner":{"mid":11,"name":"Primary","face":""}, "ctime":0,"pubdate":0,"state":0,
            "staff":[{"mid":11,"name":"Primary","face":"","title":"UP主"}, {"mid":22,"name":"Collaborator","face":"","title":"配音"}]
        })).unwrap();
        let model = details.into_detail_model(video::Model { upper_id:22, upper_name:"Old reassignment".into(), ..Default::default() });
        assert_eq!(model.upper_id, sea_orm::Set(11));
        assert_eq!(model.upper_name, sea_orm::Set("Primary".to_string()));
        let staff = model.staff_info.unwrap().unwrap();
        assert_eq!(staff[1]["mid"], 22);
        assert_eq!(staff[1]["name"], "Collaborator");
    }

}

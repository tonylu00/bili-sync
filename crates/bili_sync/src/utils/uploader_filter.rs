use anyhow::{ensure, Result};
use bili_sync_entity::video;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(default)]
pub struct UploaderFilter {
    pub min_duration_seconds: Option<i32>,
    pub max_duration_seconds: Option<i32>,
    pub min_page_duration_seconds: Option<i32>,
    pub max_page_duration_seconds: Option<i32>,
    pub include_keywords: Option<Vec<String>>,
    pub exclude_keywords: Option<Vec<String>>,
}
impl UploaderFilter {
    pub fn validate(&mut self) -> Result<()> {
        for (min, max) in [
            (self.min_duration_seconds, self.max_duration_seconds),
            (self.min_page_duration_seconds, self.max_page_duration_seconds),
        ] {
            ensure!(
                min.is_none_or(|v| v >= 0) && max.is_none_or(|v| v >= 0),
                "时长不能为负数"
            );
            ensure!(min.zip(max).is_none_or(|(a, b)| a <= b), "时长下限不能大于上限");
        }
        for list in [&mut self.include_keywords, &mut self.exclude_keywords]
            .into_iter()
            .flatten()
        {
            *list = list
                .iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            ensure!(
                list.len() <= 100 && list.iter().all(|s| s.len() <= 300),
                "关键词过多或过长"
            );
        }
        Ok(())
    }
    pub fn allows_title(&self, title: &str) -> bool {
        super::keyword_filter::matches_keyword_filters(
            title,
            self.include_keywords.as_deref(),
            self.exclude_keywords.as_deref(),
        )
    }
    pub fn allows_page(&self, seconds: u32) -> bool {
        self.min_page_duration_seconds
            .is_none_or(|v| seconds as i64 >= v as i64)
            && self
                .max_page_duration_seconds
                .is_none_or(|v| seconds as i64 <= v as i64)
    }
    pub fn allows_total(&self, seconds: i64) -> bool {
        self.min_duration_seconds.is_none_or(|v| seconds >= v as i64)
            && self.max_duration_seconds.is_none_or(|v| seconds <= v as i64)
    }
}

pub async fn for_video(db: &DatabaseConnection, video: &video::Model) -> Result<Vec<UploaderFilter>> {
    // Ownership comes from the API owner, never a collaborator or a subscribed source.
    // Legacy JSON may contain include_coauthored=true; it must not expand the scope.
    if video.source_type == Some(1) || video.upper_id <= 0 {
        return Ok(vec![]);
    }
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT settings FROM uploader_filter WHERE upper_id = ?",
            [video.upper_id.into()],
        ))
        .await?;
    row.map(|row| {
        serde_json::from_str(&row.try_get::<String>("", "settings")?)
            .map(|rule| vec![rule])
            .map_err(Into::into)
    })
    .unwrap_or_else(|| Ok(vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bili_sync_migration::{Migrator, MigratorTrait};
    #[tokio::test]
    async fn filters_apply_only_to_primary_uid_even_with_legacy_coauthor_setting() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        let video = video::Model {
            upper_id: 11,
            upper_name: "Same name".into(),
            staff_info: Some(serde_json::json!([{"mid":22,"name":"Same name"}])),
            ..Default::default()
        };
        let mut rule = UploaderFilter {
            exclude_keywords: Some(vec!["Blocked".into()]),
            ..Default::default()
        };
        rule.validate().unwrap();
        for uid in [22, 33] {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO uploader_filter VALUES (?, ?, '')",
                [uid.into(), serde_json::to_string(&rule).unwrap().into()],
            ))
            .await
            .unwrap();
        }
        assert!(for_video(&db, &video).await.unwrap().is_empty());
        let mut legacy = serde_json::to_value(&rule).unwrap();
        legacy["include_coauthored"] = serde_json::json!(true);
        db.execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE uploader_filter SET settings=? WHERE upper_id=22",
            [serde_json::to_string(&legacy).unwrap().into()],
        ))
        .await
        .unwrap();
        assert!(for_video(&db, &video).await.unwrap().is_empty());
        // The same UID's rule still applies to its own uploads.
        let primary = video::Model {
            upper_id: 22,
            ..Default::default()
        };
        let rules = for_video(&db, &primary).await.unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!rules[0].allows_title("A BLOCKED title"));
        assert!(serde_json::to_value(&rules[0])
            .unwrap()
            .get("include_coauthored")
            .is_none());
        assert!(for_video(
            &db,
            &video::Model {
                source_type: Some(1),
                ..primary
            }
        )
        .await
        .unwrap()
        .is_empty());
        assert!(for_video(
            &db,
            &video::Model {
                upper_id: 44,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .is_empty());
        let own = video::Model {
            upper_id: 33,
            ..Default::default()
        };
        assert_eq!(for_video(&db, &own).await.unwrap().len(), 1);
    }
    #[test]
    fn validates_duration_ranges_and_normalizes_keywords() {
        let mut rule = UploaderFilter {
            min_duration_seconds: Some(20),
            max_duration_seconds: Some(10),
            ..Default::default()
        };
        assert!(rule.validate().is_err());
        rule.max_duration_seconds = Some(30);
        rule.include_keywords = Some(vec![" A ".into(), "  ".into()]);
        rule.validate().unwrap();
        assert_eq!(rule.include_keywords, Some(vec!["a".into()]));
        assert!(rule.allows_total(25));
        assert!(!rule.allows_total(31));
    }
}

use crate::app::db;
use crate::app::github::GitHubClient;
use crate::app::model::SoftwareEntry;
use anyhow::Result;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Updater {
    github_client: GitHubClient,
}

impl Updater {
    pub fn new(token: Option<String>) -> Self {
        Self {
            github_client: GitHubClient::new(token),
        }
    }

    /// 检查单个软件的更新
    pub async fn check_for_updates(
        &self,
        entry: &SoftwareEntry,
    ) -> Result<Option<String>> {
        let latest = self.github_client
            .fetch_latest_release(&entry.repo_owner, &entry.repo_name)
            .await?;
        
        if let Some(release) = latest {
            let latest_tag = release.tag_name;
            if crate::utils::version::is_newer(&latest_tag, &entry.current_version) {
                return Ok(Some(latest_tag));
            }
        }
        Ok(None)
    }

    /// 批量检查所有软件，并更新数据库中的 latest_version 字段
    pub async fn check_all_and_update_db(&self, conn: &Connection) -> Result<Vec<(i64, String)>> {
        let entries = db::get_all_software(conn)?;
        let mut updated = Vec::new();
        
        for entry in entries {
            if let Some(latest) = self.check_for_updates(&entry).await? {
                // 更新数据库中的 latest_version
                let mut updated_entry = entry.clone();
                updated_entry.latest_version = Some(latest.clone());
                updated_entry.updated_at = chrono::Local::now();
                db::update_software(conn, &updated_entry)?;
                updated.push((entry.id.unwrap(), latest));
            }
        }
        Ok(updated)
    }
}

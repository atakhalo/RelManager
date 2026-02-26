use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};
use rusqlite::Connection;
use crate::app::db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareEntry {
    pub id: Option<i64>,
	pub name: String,
    pub alias: String,              // 用户自定义别名（显示名称）
	pub repo_url: String,            // GitHub 仓库完整 URL
    pub current_version: String,
    pub latest_version: Option<String>,
    pub asset_name: String,          // 下载的资产文件名（用于显示软件包选项）
    pub install_path: Option<String>,
    pub executable_path: Option<String>,
    pub notes: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl SoftwareEntry {
    /// 从 repo_url 解析出 owner 和 repo，用于 API 请求
    pub fn parse_repo(&self) -> Option<(String, String)> {
        crate::app::github::GitHubClient::parse_repo_url(&self.repo_url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub github_token: Option<String>,
    pub auto_check_interval_hours: u64,
    pub last_check_time: Option<DateTime<Local>>,
    pub download_dir: Option<String>,
}

impl Settings {
    pub fn load_from_db(conn: &Connection) -> Result<Self, Box<dyn std::error::Error>> {
        let github_token = db::get_setting(conn, "github_token")?;
        let auto_check_interval = db::get_setting(conn, "auto_check_interval")?
            .unwrap_or_else(|| "24".to_string())
            .parse::<u64>()?;
        let last_check_time = db::get_setting(conn, "last_check_time")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Local));
        let download_dir = db::get_setting(conn, "download_dir")?;
        
        Ok(Settings {
            github_token,
            auto_check_interval_hours: auto_check_interval,
            last_check_time,
            download_dir,
        })
    }
}

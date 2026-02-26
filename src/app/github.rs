use anyhow::{Result, Context};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: Option<u64>,
}

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("User-Agent", "GitHub-Release-Manager".parse().unwrap());
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client, token }
    }

    /// 解析 GitHub 仓库 URL，返回 (owner, repo)
    pub fn parse_repo_url(url: &str) -> Option<(String, String)> {
        let url = url.trim_end_matches('/');
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[parts.len() - 2].to_string();
            let repo = parts[parts.len() - 1].to_string();
            Some((owner, repo))
        } else {
            None
        }
    }

    /// 获取所有 releases
    pub async fn fetch_releases(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
        let mut req = self.client.get(&url);
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let releases = req.send().await?
            .error_for_status()?
            .json::<Vec<Release>>()
            .await?;
        Ok(releases)
    }

    /// 获取最新 release（按发布时间排序）
    pub async fn fetch_latest_release(&self, owner: &str, repo: &str) -> Result<Option<Release>> {
        let releases = self.fetch_releases(owner, repo).await?;
        Ok(releases.into_iter().next())
    }
}

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub index: IndexConfig,
    pub pagination: PaginationConfig,
    pub crawler: CrawlerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexConfig {
    pub shard_count: u32,
    pub data_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationConfig {
    pub default_page_size: u32,
    pub max_page_size: u32,
    pub cursor_ttl_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrawlerConfig {
    pub concurrency: usize,
    pub user_agent: String,
    #[serde(default)]
    pub seed_urls: Vec<String>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}

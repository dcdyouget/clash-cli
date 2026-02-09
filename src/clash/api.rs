use anyhow::{Result};
use reqwest::{Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_API_URL: &str = "http://127.0.0.1:9090";

/// Clash API 客户端
///
/// 用于与 Clash 外部控制 API 进行交互
pub struct ClashClient {
    client: Client,
    base_url: String,
}

impl ClashClient {
    /// 创建一个新的 API 客户端实例
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: DEFAULT_API_URL.to_string(),
        }
    }

    /// 获取所有代理组和节点信息
    pub async fn get_proxies(&self) -> Result<HashMap<String, ProxyItem>> {
        let url = format!("{}/proxies", self.base_url);
        let resp: ProxiesResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.proxies)
    }

    /// 切换指定代理组的选中节点
    pub async fn select_proxy(&self, group_name: &str, proxy_name: &str) -> Result<()> {
        let url = format!("{}/proxies/{}", self.base_url, group_name);
        let payload = serde_json::json!({ "name": proxy_name });
        let resp = self.client.put(&url).json(&payload).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to select proxy: {}", resp.status()))
        }
    }
    
    /// 获取当前配置信息
    pub async fn get_config(&self) -> Result<Config> {
        let url = format!("{}/configs", self.base_url);
        let config: Config = self.client.get(&url).send().await?.json().await?;
        Ok(config)
    }

    /// 更新 Clash 配置 (如切换模式)
    pub async fn update_config(&self, payload: &serde_json::Value) -> Result<()> {
        let url = format!("{}/configs", self.base_url);
        let resp = self.client.patch(&url).json(payload).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to update config: {}", resp.status()))
        }
    }
    
    /// 测试指定节点的延迟
    pub async fn delay_test(&self, proxy_name: &str) -> Result<u64> {
         // URL encoded proxy name? reqwest handles path segments but if it has slash?
         // Assuming proxy name is safe or simple.
         // Actually, delay test usually works on specific nodes, not groups.
         // But user wants "Check current node status".
         let url = format!("{}/proxies/{}/delay?timeout=5000&url=http://www.gstatic.com/generate_204", self.base_url, urlencoding::encode(proxy_name));
         let resp: DelayResponse = self.client.get(&url).send().await?.json().await?;
         Ok(resp.delay)
    }
}

#[derive(Debug, Deserialize)]
struct ProxiesResponse {
    proxies: HashMap<String, ProxyItem>,
}

/// 代理节点/组信息
#[derive(Debug, Deserialize, Clone)]
pub struct ProxyItem {
    /// 代理名称
    pub name: String,
    /// 代理类型 (Selector, URLTest, Direct, etc.)
    #[serde(rename = "type")]
    pub proxy_type: String,
    /// 包含的子节点列表 (仅 Selector/URLTest 有效)
    pub all: Option<Vec<String>>, 
    /// 当前选中的节点 (仅 Selector 有效)
    pub now: Option<String>,      
    /// 历史延迟数据
    pub history: Option<Vec<History>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct History {
    pub time: String,
    pub delay: u64,
}

#[derive(Debug, Deserialize)]
struct DelayResponse {
    delay: u64,
}

/// Clash 配置结构
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub port: Option<u16>,
    /// 运行模式: Global, Rule, Direct
    pub mode: String, 
    #[serde(rename = "log-level")]
    pub log_level: Option<String>,
}

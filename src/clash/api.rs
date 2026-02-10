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

    /// 获取当前流量信息 (快照)
    pub async fn get_traffic(&self) -> Result<Traffic> {
        let url = format!("{}/traffic", self.base_url);
        let mut response = self.client.get(&url).send().await?;
        
        // 读取第一个数据包
        if let Some(chunk) = response.chunk().await? {
            let s = String::from_utf8_lossy(&chunk);
            // Clash 返回的数据可能是 "{"up":0,"down":0}\n"
            // 我们只需要解析第一行 JSON
            if let Some(line) = s.lines().next() {
                let traffic: Traffic = serde_json::from_str(line)?;
                return Ok(traffic);
            }
        }
        Err(anyhow::anyhow!("No traffic data received"))
    }

    /// 获取版本信息
    pub async fn get_version(&self) -> Result<Version> {
        let url = format!("{}/version", self.base_url);
        let version: Version = self.client.get(&url).send().await?.json().await?;
        Ok(version)
    }

    /// 获取当前活跃连接数
    pub async fn get_connection_count(&self) -> Result<usize> {
        let url = format!("{}/connections", self.base_url);
        let resp: ConnectionsResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.connections.len())
    }

    /// 获取流量流 (Streaming Response)
    pub async fn stream_traffic(&self) -> Result<reqwest::Response> {
        let url = format!("{}/traffic", self.base_url);
        Ok(self.client.get(&url).send().await?)
    }

    /// 获取日志流 (Streaming Response)
    pub async fn stream_logs(&self) -> Result<reqwest::Response> {
        let url = format!("{}/logs?level=info", self.base_url);
        Ok(self.client.get(&url).send().await?)
    }
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct ConnectionsResponse {
    connections: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Traffic {
    pub up: u64,
    pub down: u64,
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
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u16>,
    /// 运行模式: Global, Rule, Direct
    pub mode: String, 
    #[serde(rename = "log-level")]
    pub log_level: Option<String>,
    pub tun: Option<TunConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TunConfig {
    pub enable: bool,
}

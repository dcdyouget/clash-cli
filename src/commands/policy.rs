use anyhow::Result;
use crate::cli::PolicyMode;
use crate::clash::api::ClashClient;
use colored::*;

/// 策略管理命令入口
pub async fn run(mode: PolicyMode) -> Result<()> {
    set_policy(mode).await
}

/// 设置路由策略 (Global, Rule, Direct)
/// 
/// 通过调用 Clash API 实时修改
async fn set_policy(mode: PolicyMode) -> Result<()> {
    let client = ClashClient::new();
    let payload = serde_json::json!({ "mode": mode.to_string() });
    client.update_config(&payload).await?;
    println!("策略已切换为 {}", mode.to_string().green());
    Ok(())
}

use anyhow::{Result, Context};
use crate::clash::api::ClashClient;
use colored::*;

/// 状态检测命令入口
/// 
/// 检测所有 Selector 类型的代理组当前选中节点的延迟
pub async fn run() -> Result<()> {
    let client = ClashClient::new();
    let proxies = client.get_proxies().await.context("无法连接到 Clash API。Clash 是否正在运行？")?;
    
    let mut groups: Vec<&String> = proxies.iter()
        .filter(|(_, p)| p.proxy_type == "Selector")
        .map(|(n, _)| n)
        .collect();
    groups.sort();
    
    if groups.is_empty() {
        println!("未找到代理组。");
        return Ok(());
    }
    
    println!("正在检查节点状态...");
    
    for group_name in groups {
        if let Some(group) = proxies.get(group_name) {
            if let Some(now) = &group.now {
                 print!("代理组 {}: 当前选中节点 [{}] ... ", group_name.cyan(), now.yellow());
                 
                 // 测试节点延迟
                 match client.delay_test(now).await {
                     Ok(delay) => {
                         let status = if delay < 200 {
                             format!("{} ms", delay).green()
                         } else if delay < 500 {
                             format!("{} ms", delay).yellow()
                         } else {
                             format!("{} ms", delay).red()
                         };
                         println!("延迟: {}", status);
                     },
                     Err(_) => {
                         println!("{}", "超时/错误".red());
                     }
                 }
            }
        }
    }
    
    Ok(())
}

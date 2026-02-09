use anyhow::{Result, Context};
use crate::clash::api::ClashClient;
use dialoguer::{Select, theme::ColorfulTheme};
use colored::*;

/// 节点管理命令入口
///
/// 负责获取代理组列表，并进行交互式节点选择
pub async fn run(_select: bool) -> Result<()> {
    let client = ClashClient::new();
    // 获取所有代理信息
    let proxies = client.get_proxies().await.context("连接 Clash API 失败。Clash 是否正在运行？")?;

    // 筛选出类型为 "Selector" 的代理组
    let mut groups: Vec<&String> = proxies.iter()
        .filter(|(_, p)| p.proxy_type == "Selector")
        .map(|(n, _)| n)
        .collect();
        
    if groups.is_empty() {
        println!("未找到代理组 (Selector)。");
        return Ok(());
    }
    
    // 对组名进行排序
    groups.sort();

    // 交互式选择代理组
    let group_idx = if groups.len() == 1 {
        0
    } else {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("请选择代理组")
            .items(&groups)
            .interact()?
    };
    
    let group_name = groups[group_idx];
    let group = &proxies[group_name];
    
    let all_nodes = group.all.as_ref().context("该组没有节点")?;
    
    // 获取当前选中的节点
    let current = group.now.as_deref().unwrap_or("");
    
    // 构造显示列表，标记当前选中的节点
    let items: Vec<String> = all_nodes.iter().map(|n| {
        if n == current {
            format!("{} (当前)", n)
        } else {
            n.clone()
        }
    }).collect();
    
    // 交互式选择节点
    let node_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("请选择 {} 的节点", group_name))
        .items(&items)
        .default(all_nodes.iter().position(|x| x == current).unwrap_or(0))
        .interact()?;
        
    let selected_node = &all_nodes[node_idx];
    
    // 如果选择未改变，直接返回
    if selected_node == current {
        println!("{} 已经是当前选择。", selected_node.green());
        return Ok(());
    }
    
    // 调用 API 切换节点
    client.select_proxy(group_name, selected_node).await?;
    println!("已将 {} 切换至 {}", group_name, selected_node.green());

    Ok(())
}

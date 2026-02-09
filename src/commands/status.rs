use anyhow::Result;
use colored::*;
use crate::clash::api::ClashClient;
use std::process::Command;
use std::fs;
use std::path::Path;

const CONFIG_DIR: &str = "/etc/clash";
const ACTIVE_CONFIG: &str = "config.yaml";

/// 显示 Clash 状态
pub async fn run() -> Result<()> {
    println!("{}", "=== Clash 状态概览 ===".cyan().bold());

    // 1. 检查 Systemd 服务状态
    check_service_status();

    // 2. 检查当前配置文件
    check_active_config();

    // 3. 尝试连接 API 获取实时信息
    let client = ClashClient::new();

    // 获取并显示模式 (Global/Rule/Direct)
    match client.get_config().await {
        Ok(config) => {
            println!("- {}: {}", "路由模式".bold(), config.mode.green());
            if let Some(port) = config.port {
                println!("- {}: {}", "HTTP 端口".bold(), port);
            }
            if let Some(mixed) = config.mixed_port {
                println!("- {}: {}", "混合端口".bold(), mixed);
            }
        },
        Err(_) => {
            println!("- {}: {}", "API 连接".bold(), "失败 (Clash 可能未运行)".red());
        }
    }

    // 获取并显示当前节点
    // 这里我们尝试获取所有的 proxies，并找出当前选中的
    // 由于 Clash API 比较复杂，这里简化处理，只显示 GLOBAL 组的选中项
    // 或者显示特定策略组的状态
    if let Ok(proxies) = client.get_proxies().await {
        // 尝试查找常见的策略组名称
        let group_names = vec!["GLOBAL", "Proxy", "PROXY", "Select", "节点选择"];
        let mut found = false;
        for name in group_names {
            if let Some(proxy) = proxies.get(name) {
                if let Some(now) = &proxy.now {
                    println!("- {}: {} -> {}", "当前节点".bold(), name, now.green());
                    found = true;
                    break;
                }
            }
        }
        if !found {
                println!("- {}: {}", "当前节点".bold(), "未找到主要策略组".yellow());
        }
    }

    // 4. 显示最近日志
    println!("\n{}", "=== 最近日志 (最后 10 行) ===".cyan().bold());
    let output = Command::new("journalctl")
        .arg("-u")
        .arg("clash")
        .arg("-n")
        .arg("10")
        .arg("--no-pager")
        .output();

    match output {
        Ok(o) => {
            let log = String::from_utf8_lossy(&o.stdout);
            println!("{}", log);
        },
        Err(_) => println!("{}", "无法读取日志".red()),
    }

    Ok(())
}

fn check_service_status() {
    let output = Command::new("systemctl")
        .arg("is-active")
        .arg("clash")
        .output();
    
    match output {
        Ok(o) => {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let color_status = if status == "active" { status.green() } else { status.red() };
            println!("- {}: {}", "服务状态".bold(), color_status);
        },
        Err(_) => println!("- {}: {}", "服务状态".bold(), "未知".red()),
    }
}

fn check_active_config() {
    let config_path = Path::new(CONFIG_DIR).join(ACTIVE_CONFIG);
    if config_path.exists() {
        // 尝试读取它是否是一个软链接，或者直接读取内容摘要
        // 这里简单地列出 config.yaml 的实际指向（如果是软链）或大小
        if let Ok(metadata) = fs::symlink_metadata(&config_path) {
            if metadata.file_type().is_symlink() {
                 if let Ok(target) = fs::read_link(&config_path) {
                     println!("- {}: {} -> {}", "配置文件".bold(), ACTIVE_CONFIG, target.display().to_string().blue());
                 }
            } else {
                 println!("- {}: {} (普通文件, {} bytes)", "配置文件".bold(), ACTIVE_CONFIG, metadata.len());
            }
        }
    } else {
        println!("- {}: {}", "配置文件".bold(), "未找到".red());
    }
}
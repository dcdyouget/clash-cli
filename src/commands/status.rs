use anyhow::Result;
use colored::*;
use crate::clash::api::ClashClient;
use std::process::Command;

/// 显示 Clash 状态
pub async fn run() -> Result<()> {
    println!("{}", "=== Clash 状态概览 ===".cyan().bold());

    // 1. 检查 Systemd 服务状态
    check_service_status();

    // 2. 尝试连接 API 获取实时信息
    let client = ClashClient::new();

    // 获取并显示模式
    match client.get_config().await {
        Ok(config) => {
            // 判断是否为 Tun 模式
            let tun_status = if let Some(tun) = config.tun {
                if tun.enable { "TUN 模式".green() } else { "HTTP 代理模式".yellow() }
            } else {
                "HTTP 代理模式".yellow()
            };
            println!("- {}: {}", "运行模式".bold(), tun_status);
            
            // 显示策略模式 (Global/Rule/Direct)
            println!("- {}: {}", "策略模式".bold(), config.mode.green());
        },
        Err(_) => {
            println!("- {}: {}", "API 连接".bold(), "失败 (Clash 可能未运行)".red());
        }
    }

    // 获取并显示当前流量
    match client.get_traffic().await {
        Ok(traffic) => {
            println!("- {}: {}", "上传速度".bold(), format_speed(traffic.up).green());
            println!("- {}: {}", "下载速度".bold(), format_speed(traffic.down).green());
        },
        Err(_) => {
             // 流量获取失败通常不致命，可能暂时忽略
        }
    }

    // 获取连接数
    if let Ok(count) = client.get_connection_count().await {
        println!("- {}: {}", "当前连接".bold(), count.to_string().cyan());
    }

    // 获取版本
    if let Ok(version) = client.get_version().await {
        println!("- {}: {}", "内核版本".bold(), version.version.blue());
    }

    // 获取内存占用
    if let Some(mem) = get_memory_usage() {
        println!("- {}: {}", "内存占用".bold(), mem.yellow());
    }

    // 4. 显示代理组选择与延迟
    if let Ok(proxies) = client.get_proxies().await {
        println!("\n{}", "=== 代理组状态 ===".cyan().bold());
        
        let mut groups: Vec<_> = proxies.iter()
            .filter(|(_, p)| p.proxy_type == "Selector" || p.proxy_type == "URLTest")
            .collect();
        
        // 简单排序，把 GLOBAL 放在最前面
        groups.sort_by(|(name_a, _), (name_b, _)| {
            if *name_a == "GLOBAL" { std::cmp::Ordering::Less }
            else if *name_b == "GLOBAL" { std::cmp::Ordering::Greater }
            else { name_a.cmp(name_b) }
        });

        for (name, group) in groups {
            if let Some(now) = &group.now {
                let delay_str = if let Some(node) = proxies.get(now) {
                    if let Some(history) = &node.history {
                        if let Some(last) = history.last() {
                            if last.delay == 0 {
                                "无数据".red().to_string()
                            } else {
                                format!("{} ms", last.delay).green().to_string()
                            }
                        } else {
                            "无数据".red().to_string()
                        }
                    } else {
                        "无数据".red().to_string()
                    }
                } else {
                    "未知".dimmed().to_string()
                };

                println!("- {}: {} ({})", name.bold(), now.cyan(), delay_str);
            }
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

fn format_speed(speed: u64) -> String {
    if speed < 1024 {
        format!("{} B/s", speed)
    } else if speed < 1024 * 1024 {
        format!("{:.2} KB/s", speed as f64 / 1024.0)
    } else {
        format!("{:.2} MB/s", speed as f64 / 1024.0 / 1024.0)
    }
}

fn get_memory_usage() -> Option<String> {
    // 1. 获取 PID
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("clash")
        .output()
        .ok()?;
    
    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if pid_str.is_empty() {
        return None;
    }

    // 2. 获取 RSS 内存 (KB)
    let output = Command::new("ps")
        .arg("-o")
        .arg("rss=")
        .arg("-p")
        .arg(&pid_str)
        .output()
        .ok()?;

    let rss_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if let Ok(rss_kb) = rss_str.parse::<u64>() {
        let rss_mb = rss_kb as f64 / 1024.0;
        return Some(format!("{:.1} MB", rss_mb));
    }
    None
}
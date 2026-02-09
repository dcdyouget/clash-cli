use anyhow::{Result, Context};
use crate::cli::InboundMode;
use std::fs;
use std::process::Command;
use colored::*;

const CONFIG_FILE: &str = "/etc/clash/config.yaml";

/// 入站模式管理命令入口
pub async fn run(mode: InboundMode) -> Result<()> {
    set_inbound(mode).await
}

/// 设置入站模式 (Tun, Http Proxy)
/// 
/// 通过修改配置文件并重启服务实现
async fn set_inbound(mode: InboundMode) -> Result<()> {
    // 检查配置文件是否存在
    if fs::metadata(CONFIG_FILE).is_err() {
        println!("在 {} 未找到配置文件。Clash 是否已安装并配置？", CONFIG_FILE);
        return Ok(());
    }
    
    // 读取配置文件内容 (需要 root 权限则使用 sudo cat)
    let content = match fs::read_to_string(CONFIG_FILE) {
        Ok(c) => c,
        Err(_) => {
             let output = Command::new("sudo").arg("cat").arg(CONFIG_FILE).output()?;
             String::from_utf8(output.stdout)?
        }
    };

    let mut doc: serde_yaml::Value = serde_yaml::from_str(&content).context("解析配置文件失败")?;
    
    match mode {
        InboundMode::Tun => {
            // 启用 Tun 模式
            if let Some(tun) = doc.get_mut("tun") {
                if let Some(enable) = tun.get_mut("enable") {
                    *enable = serde_yaml::Value::Bool(true);
                } else {
                     if let serde_yaml::Value::Mapping(m) = tun {
                         m.insert(serde_yaml::Value::String("enable".to_string()), serde_yaml::Value::Bool(true));
                     }
                }
            } else {
                // 如果 tun 字段不存在则创建
                let mut tun_map = serde_yaml::Mapping::new();
                tun_map.insert(serde_yaml::Value::String("enable".to_string()), serde_yaml::Value::Bool(true));
                tun_map.insert(serde_yaml::Value::String("stack".to_string()), serde_yaml::Value::String("system".to_string()));
                // Add dns hijack if needed, but minimal for now
                 if let serde_yaml::Value::Mapping(m) = &mut doc {
                     m.insert(serde_yaml::Value::String("tun".to_string()), serde_yaml::Value::Mapping(tun_map));
                 }
            }
            println!("已在配置中启用 Tun 模式。");
        },
        InboundMode::Http => {
             // 禁用 Tun
             disable_tun(&mut doc);
             println!("已禁用 Tun 模式。");
             println!("{}", "已切换到 HTTP/Socks 代理模式。".green());
             println!("{}", "要在终端中使用代理，请导出以下环境变量：".yellow());
             println!("export https_proxy=http://127.0.0.1:7890 http_proxy=http://127.0.0.1:7890 all_proxy=socks5://127.0.0.1:7890");
        },
    }
    
    // 写回配置文件
    let new_content = serde_yaml::to_string(&doc)?;
    let temp_path = "/tmp/clash_config_update.yaml";
    fs::write(temp_path, new_content)?;
    
    let status = Command::new("sudo")
        .arg("cp")
        .arg(temp_path)
        .arg(CONFIG_FILE)
        .status()?;
        
    if !status.success() {
        return Err(anyhow::anyhow!("更新配置文件失败"));
    }
    
    println!("正在重启 Clash...");
    Command::new("sudo").arg("systemctl").arg("restart").arg("clash").status()?;
    
    Ok(())
}

fn disable_tun(doc: &mut serde_yaml::Value) {
    if let Some(tun) = doc.get_mut("tun") {
        if let Some(enable) = tun.get_mut("enable") {
            *enable = serde_yaml::Value::Bool(false);
        }
    }
}

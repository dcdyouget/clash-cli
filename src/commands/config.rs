use anyhow::{Result, Context, anyhow};
use crate::cli::ConfigAction;
use crate::utils::download;
use std::path::{Path};
use std::fs;
use colored::*;
use dialoguer::{Select, theme::ColorfulTheme};
use std::process::Command;

const CONFIG_DIR: &str = "/etc/clash";
const ACTIVE_CONFIG: &str = "config.yaml";

/// 配置管理命令入口
pub async fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Add { url, name } => add_config(url, name).await?,
        ConfigAction::List => { list_configs()?; },
        ConfigAction::Select => select_config()?,
    }
    Ok(())
}

/// 添加新的配置
/// 
/// 支持从 URL 下载或从本地文件复制
async fn add_config(url: String, name: Option<String>) -> Result<()> {
    let filename = if let Some(n) = name {
        if n.ends_with(".yaml") || n.ends_with(".yml") {
            n
        } else {
            format!("{}.yaml", n)
        }
    } else {
        // 从 URL 推断文件名
        let url_path = url.split('?').next().unwrap(); // remove query params
        let name = Path::new(url_path).file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("subscription.yaml");
         if name.ends_with(".yaml") || name.ends_with(".yml") {
            name.to_string()
        } else {
            format!("{}.yaml", name)
        }
    };

    println!("正在添加配置: {}", filename);
    
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join(&filename);
    
    if url.starts_with("http") {
        download::download_file(&url, &temp_path).await?;
    } else {
        // 本地文件
        fs::copy(&url, &temp_path).context("复制本地文件失败")?;
    }
    
    // 移动到配置目录 /etc/clash/
    let target_path = Path::new(CONFIG_DIR).join(&filename);
    println!("正在安装到 {}...", target_path.display());
    
    let status = Command::new("sudo")
        .arg("cp")
        .arg(temp_path)
        .arg(&target_path)
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("复制配置文件失败"));
    }
    
    println!("{}", "配置添加成功。".green());
    Ok(())
}

/// 列出可用配置
fn list_configs() -> Result<Vec<String>> {
    let mut configs = Vec::new();
    
    // 检查目录是否存在
    if !Path::new(CONFIG_DIR).exists() {
        println!("配置目录 {} 不存在。请先安装 Clash。", CONFIG_DIR);
        return Ok(configs);
    }

    let entries = fs::read_dir(CONFIG_DIR).context(format!("读取目录 {} 失败", CONFIG_DIR))?;
    
    println!("{} 下的可用配置:", CONFIG_DIR);
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                             // 不列出当前激活的软链接/副本目标
                             if name != ACTIVE_CONFIG {
                                 configs.push(name.to_string());
                                 println!("  - {}", name);
                             }
                        }
                    }
                }
            }
        }
    }
    Ok(configs)
}

/// 交互式选择并切换配置
fn select_config() -> Result<()> {
    let configs = list_configs()?;
    if configs.is_empty() {
        println!("未找到配置文件。");
        return Ok(());
    }
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("请选择要使用的配置")
        .default(0)
        .items(&configs)
        .interact()?;
        
    let selected_config = &configs[selection];
    println!("正在切换到 {}", selected_config);
    
    let source = Path::new(CONFIG_DIR).join(selected_config);
    let target = Path::new(CONFIG_DIR).join(ACTIVE_CONFIG);
    
    let status = Command::new("sudo")
        .arg("cp")
        .arg(source)
        .arg(target)
        .status()?;
        
    if !status.success() {
        return Err(anyhow!("切换配置失败"));
    }
    
    println!("正在重启 Clash 服务...");
    Command::new("sudo").arg("systemctl").arg("restart").arg("clash").status()?;
    
    println!("{}", "配置已切换并重启服务。".green());
    Ok(())
}

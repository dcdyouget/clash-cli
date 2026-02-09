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
    
    // 验证配置文件格式
    if let Err(e) = validate_config_file(&temp_path) {
        return Err(anyhow!("配置文件校验失败: {}\n提示: 请确保订阅链接是 Clash 格式 (通常包含 &flag=clash)", e));
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

    // 检查是否应该自动应用此配置
    // 如果当前只有一个有效配置（即刚添加的这个），或者之前的 config.yaml 是默认生成的（简单检查）
    let configs = list_configs_internal()?;
    if configs.len() == 1 {
        println!("检测到这是唯一的配置文件，正在自动应用...");
        apply_config(&filename)?;
    }

    Ok(())
}

/// 内部列出配置函数，不打印输出
fn list_configs_internal() -> Result<Vec<String>> {
    let mut configs = Vec::new();
    if !Path::new(CONFIG_DIR).exists() {
        return Ok(configs);
    }
    
    let entries = fs::read_dir(CONFIG_DIR)?;
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                            if name != ACTIVE_CONFIG {
                                configs.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(configs)
}

/// 验证配置文件格式
fn validate_config_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path).context("读取下载的文件失败")?;
    
    // 尝试解析为 YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
        if yaml.is_mapping() {
            // 简单的结构检查
            if yaml.get("proxies").is_some() || yaml.get("proxy-providers").is_some() || yaml.get("mixed-port").is_some() || yaml.get("port").is_some() {
                return Ok(());
            }
            // 可能是仅包含 proxies 的片段，但也可能是无效的
        }
    }

    // 检查常见错误格式
    let trimmed = content.trim();
    
    // 1. Base64 格式 (无空格，长字符串)
    if trimmed.len() > 50 && !trimmed.contains(' ') && !trimmed.contains('\n') {
        return Err(anyhow!("检测到 Base64 编码的内容。这是通用的订阅格式，不是 Clash 配置文件。"));
    }
    
    // 2. 直接的节点列表 (包含 trojan://, ss://, vmess:// 等)
    if content.contains("trojan://") || content.contains("ss://") || content.contains("vmess://") || content.contains("vless://") {
        return Err(anyhow!("检测到原始节点列表。Clash 需要 YAML 格式的配置文件。"));
    }

    // 3. HTML 内容 (可能是下载到了登录页或验证页)
    if content.to_lowercase().contains("<!doctype html>") || content.to_lowercase().contains("<html") {
        return Err(anyhow!("下载的内容似乎是 HTML 页面。可能是订阅链接失效或需要网页验证。"));
    }

    Err(anyhow!("文件不是有效的 Clash YAML 配置文件。"))
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

/// 应用指定的配置文件
fn apply_config(config_name: &str) -> Result<()> {
    println!("正在切换到 {}", config_name);
    
    let source = Path::new(CONFIG_DIR).join(config_name);
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
    
    println!("{}", "配置已应用并重启服务。".green());
    Ok(())
}

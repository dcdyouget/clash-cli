use anyhow::Result;
use crate::cli::ProxyAction;
use std::process::Command;

/// 服务控制命令入口
pub async fn run(action: ProxyAction) -> Result<()> {
    match action {
        ProxyAction::Start => service_control("start")?,
        ProxyAction::Stop => service_control("stop")?,
        ProxyAction::Restart => service_control("restart")?,
        ProxyAction::Status => service_status()?,
    }
    Ok(())
}

/// 执行 systemctl 命令控制服务
fn service_control(action: &str) -> Result<()> {
    let action_cn = match action {
        "start" => "启动",
        "stop" => "停止",
        "restart" => "重启",
        _ => action,
    };
    
    println!("正在{} Clash 服务...", action_cn);
    let status = Command::new("sudo")
        .arg("systemctl")
        .arg(action)
        .arg("clash")
        .status()?;
        
    if status.success() {
        println!("Clash 服务{}成功。", action_cn);
    } else {
        println!("Clash 服务{}失败。", action_cn);
    }
    Ok(())
}

/// 查看服务状态
fn service_status() -> Result<()> {
    let _ = Command::new("systemctl")
        .arg("status")
        .arg("clash")
        .status()?;
    Ok(())
}

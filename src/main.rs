mod cli;
mod commands;
mod utils;
mod clash;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 根据子命令执行对应的功能模块
    match cli.command {
        Commands::Install { version, file } => {
            // 安装命令
            commands::install::run(version, file).await?;
        }
        Commands::Uninstall => {
            // 卸载命令
            commands::install::uninstall().await?;
        }
        Commands::Config { action } => {
            // 配置管理命令
            commands::config::run(action).await?;
        }
        Commands::Node { select } => {
            // 节点管理命令
            commands::node::run(select).await?;
        }
        Commands::Policy { mode } => {
            // 路由策略切换命令
            commands::policy::run(mode).await?;
        }
        Commands::Tun { mode } => {
            // 入站模式切换命令
            commands::tun::run(mode).await?;
        }
        Commands::Check => {
            // 状态检测命令
            commands::check::run().await?;
        }
        Commands::Proxy { action } => {
            // 服务控制命令
            commands::proxy::run(action).await?;
        }
        Commands::Status => {
            // 状态查看命令
            commands::status::run().await?;
        }
    }

    Ok(())
}

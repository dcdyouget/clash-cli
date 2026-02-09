use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

#[derive(Parser)]
#[command(name = "clash-cli")]
#[command(about = "Clash 管理命令行工具", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 安装 Clash 并注册为系统服务
    Install {
        /// 安装版本 (默认: latest)
        #[arg(short, long)]
        version: Option<String>,
        
        /// 从本地文件安装 (Mihomo .gz 包)
        #[arg(short, long)]
        file: Option<String>,
    },
    /// 卸载 Clash 服务和二进制文件
    Uninstall,
    
    /// 管理配置文件
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// 管理节点/代理
    Node {
        /// 交互式选择代理
        #[arg(short, long)]
        select: bool,
    },
    
    /// 设置路由策略 (Global, Rule, Direct)
    Policy {
        #[arg(value_enum)]
        mode: PolicyMode,
    },

    /// 设置入站模式 (Tun, Http Proxy)
    Tun {
        #[arg(value_enum)]
        mode: InboundMode,
    },
    
    /// 检测当前节点状态
    Check,
    
    /// 控制代理服务 (start/stop)
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 从 URL 添加配置
    Add {
        /// 订阅 URL 或本地文件路径
        url: String,
        /// 配置名称 (可选, 默认从文件名获取)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// 列出可用配置
    List,
    /// 选择当前激活的配置
    Select,
}

#[derive(Subcommand)]
pub enum ProxyAction {
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 重启服务
    Restart,
    /// 查看状态
    Status,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum PolicyMode {
    Global,
    Rule,
    Direct,
}

impl fmt::Display for PolicyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyMode::Global => write!(f, "Global"),
            PolicyMode::Rule => write!(f, "Rule"),
            PolicyMode::Direct => write!(f, "Direct"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InboundMode {
    /// 开启 Tun 模式 (流量接管)
    Tun,
    /// 关闭 Tun，使用传统 HTTP/Socks 代理 (需配置环境变量)
    Http,
}

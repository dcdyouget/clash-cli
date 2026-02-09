# Clash CLI

一个用 Rust 编写的 Linux 命令行 Clash (Mihomo 内核) 管理工具。

## 功能特性

- **安装/卸载**: 自动检测系统架构，下载 Mihomo 内核，并注册 systemd 服务。
- **配置管理**: 支持从 URL 或本地文件添加订阅，支持在多个配置文件间切换。
- **节点管理**: 交互式选择代理组和节点。
- **模式切换**: 切换 Tun/Http 代理模式，切换路由策略 (Global/Rule/Direct)。
- **状态检测**: 检测当前节点延迟。
- **服务控制**: 启动、停止、重启 Clash 服务。

## 安装

### 源码编译

依赖要求: Rust 工具链, `gcc` (用于编译依赖)。

```bash
git clone https://github.com/yourusername/clash-cli.git
cd clash-cli
cargo build --release
sudo cp target/release/clash-cli /usr/local/bin/
```

## 使用说明

### 1. 安装 Clash 内核

此命令将下载最新的 Mihomo 内核并配置 systemd 服务。

```bash
clash-cli install
```

### 2. 添加配置

添加订阅 URL:

```bash
clash-cli config add "https://example.com/subscribe?token=xxx" --name my-sub
```

列出和切换配置:

```bash
clash-cli config list
clash-cli config select
```

### 3. 启动服务

```bash
clash-cli proxy start
```

### 4. 管理节点

交互式选择代理组和节点:

```bash
clash-cli node --select
```

### 5. 切换模式

启用 Tun 模式 (透明代理):

```bash
clash-cli tun tun
```

禁用 Tun (切换回 HTTP/Socks 代理模式):

```bash
clash-cli tun http
```

切换路由策略:

```bash
clash-cli policy global
clash-cli policy rule
```

### 6. 检查状态

检查当前选中节点和延迟:

```bash
clash-cli check
```

## 项目结构

- `src/main.rs`: 程序入口，命令分发。
- `src/cli.rs`: CLI 参数定义 (Clap)。
- `src/commands/`: 命令具体实现。
  - `install.rs`: 安装逻辑。
  - `config.rs`: 配置管理。
  - `node.rs`: 节点选择。
  - `policy.rs`: 路由策略切换。
  - `tun.rs`: 入站模式 (Tun) 切换。
  - `proxy.rs`: 服务控制。
  - `check.rs`: 状态检测。
- `src/clash/`: Clash API 客户端。
- `src/utils/`: 辅助工具 (下载, 系统信息)。
- `src/service/`: 服务管理辅助。

## License

MIT

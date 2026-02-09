use std::env;
use anyhow::{Result, anyhow};

/// 获取系统架构 (amd64/arm64)
pub fn get_arch() -> Result<&'static str> {
    match env::consts::ARCH {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        "arm" => Ok("armv7"), // Simplified, might need more check for armv6/v7
        _ => Err(anyhow!("Unsupported architecture: {}", env::consts::ARCH)),
    }
}

/// 获取操作系统 (linux/darwin)
pub fn get_os() -> Result<&'static str> {
    match env::consts::OS {
        "linux" => Ok("linux"),
        "macos" => Ok("darwin"), // Support macos just in case, though user asked for linux tool
        _ => Err(anyhow!("Unsupported OS: {}", env::consts::OS)),
    }
}

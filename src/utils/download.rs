use std::fs::File;
use std::io::copy;
use std::path::Path;
use anyhow::{Result, Context};
use reqwest::Client;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use std::io::Write;
use indicatif::{ProgressBar, ProgressStyle};

/// 下载文件并显示进度条
pub async fn download_file(url: &str, target_path: &Path) -> Result<()> {
    let client = Client::new();
    let res = client.get(url).send().await.context("Failed to send request")?;
    let total_size = res.content_length().unwrap_or(0);

    // 设置进度条样式
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let mut file = File::create(target_path).context("Failed to create file")?;
    let mut stream = res.bytes_stream();

    // 流式下载
    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading chunk")?;
        file.write_all(&chunk).context("Error while writing to file")?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("Download complete");
    Ok(())
}

/// 解压 .gz 文件
/// 
/// 自动为解压后的文件赋予 755 权限 (Unix)
pub fn extract_gzip(archive_path: &Path, output_path: &Path) -> Result<()> {
    let file = File::open(archive_path).context("Failed to open archive")?;
    let mut decoder = GzDecoder::new(file);
    let mut output = File::create(output_path).context("Failed to create output file")?;
    copy(&mut decoder, &mut output).context("Failed to extract content")?;
    
    // chmod +x
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = output.metadata()?.permissions();
        perms.set_mode(0o755);
        output.set_permissions(perms)?;
    }
    
    Ok(())
}

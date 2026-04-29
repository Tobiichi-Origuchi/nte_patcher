use crate::error::Error;
use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::Client;
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};

pub struct DownloadReceipt {
    pub hash: String,
    pub bytes: u64,
}

pub async fn download_file(
    client: &Client,
    url: &str,
    path: impl AsRef<Path>,
) -> Result<DownloadReceipt, Error> {
    let response = client.get(url).send().await?.error_for_status()?;

    let file = File::create(path).await?;
    let mut writer = BufWriter::new(file);
    let mut stream = response.bytes_stream();

    let mut md5 = Md5::new();
    let mut bytes = 0u64;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        md5.update(&chunk);
        bytes += chunk.len() as u64;
        writer.write_all(&chunk).await?;
    }

    writer.flush().await?;
    let hash = hex::encode(md5.finalize());

    Ok(DownloadReceipt { hash, bytes })
}

use crate::error::Error;
use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::Client;
use std::path::Path;
use tokio::{fs::File, io::AsyncWriteExt};

pub struct DownloadReceipt {
    pub hash: String,
    pub bytes: u64,
}

pub async fn download_file(url: &str, path: impl AsRef<Path>) -> Result<DownloadReceipt, Error> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let mut file = File::create(path).await?;
    let mut stream = response.bytes_stream();

    let mut md5 = Md5::new();
    let mut bytes = 0u64;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        md5.update(&chunk);
        bytes += chunk.len() as u64;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    let digest = md5.finalize();
    let hash = hex::encode(digest);

    Ok(DownloadReceipt { hash, bytes })
}

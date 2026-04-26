use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::Client;
use std::error::Error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(url: &str, path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let mut file = File::create(path).await?;
    let mut stream = response.bytes_stream();

    let mut md5 = Md5::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        md5.update(&chunk);
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    let digest = md5.finalize();
    let hash = hex::encode(digest);

    Ok(hash)
}

use futures_util::StreamExt;
use reqwest::Client;
use std::error::Error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(url: &str, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    let mut file = File::create(path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    Ok(())
}

use md5::{Digest, Md5};
use nte_patcher::{
    crypto::aes_cbc,
    error::Error,
    manager::DownloadManager,
    model::{Config, ResTask},
    parser::{get_config, get_reslist},
    unzip::extract,
};
use reqwest::Url;
use std::{
    fs::read,
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
};
use tempfile::tempdir;

async fn download_file(client: &reqwest::Client, url: &str, path: &str) -> Result<(), Error> {
    let response = client.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(path).await.map_err(Error::Io)?;
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(Error::Reqwest)?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &data)
            .await
            .map_err(Error::Io)?;
    }
    Ok(())
}

fn compute_md5(file_path: &str) -> Result<String, Error> {
    let data = read(file_path)?;
    let mut hasher = Md5::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

#[tokio::test]
async fn test_perfectworld_cdn_workflow() -> Result<(), Error> {
    let base_url = "https://ntecdn1.perfectworld.com/clientRes/publish_PC";

    // 1. Setup temp dirs
    let temp_dir = tempdir().map_err(Error::Io)?;
    let base_path = temp_dir.path();
    let bucket_dir = base_path.join("bucket");
    let game_dir = base_path.join("game");

    tokio::fs::create_dir_all(&bucket_dir)
        .await
        .map_err(Error::Io)?;
    tokio::fs::create_dir_all(&game_dir)
        .await
        .map_err(Error::Io)?;

    // 2. Fetch config
    let config_url = Url::parse(&format!("{}/Version/Windows/config.xml", base_url))?;
    let config: Config = get_config(config_url).await?;
    let version = config.resversion.to_string();

    // 3. Download ResList.bin.zip
    let reslist_zip_url = format!(
        "{}/Version/Windows/version/{}/ResList.bin.zip",
        base_url, version
    );
    let reslist_zip_path = base_path.join("ResList.bin.zip");
    let client = reqwest::Client::builder().build().map_err(Error::Reqwest)?;
    download_file(
        &client,
        &reslist_zip_url,
        reslist_zip_path.to_str().unwrap(),
    )
    .await?;

    // 4. Extract ResList.bin.zip
    let zip_data = read(&reslist_zip_path)?;
    extract(&zip_data, base_path)?;

    // 5. Decrypt ResList.bin to ResList.xml
    let reslist_bin_path = base_path.join("ResList.bin");
    let reslist_xml_path = base_path.join("ResList.xml");
    let key = b"3000001@Patcher0";
    let iv = b"PatcherSDK000000";
    aes_cbc(&reslist_bin_path, &reslist_xml_path, key, iv)?;

    // 6. Verify ResList.xml MD5 against config
    let actual_listhash = compute_md5(reslist_xml_path.to_str().unwrap())?;
    assert_eq!(
        config.extra.listhash, actual_listhash,
        "ResList MD5 mismatch"
    );

    // 7. Parse ResList.xml
    let reslist = get_reslist(&reslist_xml_path)?;
    let tasks = ResTask::from_reslist(reslist);
    assert!(!tasks.is_empty(), "ResList should contain tasks");

    // 8. Test DownloadManager
    let mut normal_tasks = Vec::new();
    let mut pak_tasks = Vec::new();
    let mut block_tasks = Vec::new();

    for task in tasks {
        match task.task_type {
            nte_patcher::model::TaskType::Normal => normal_tasks.push(task),
            nte_patcher::model::TaskType::Pak { .. } => pak_tasks.push(task),
            nte_patcher::model::TaskType::Block { .. } => block_tasks.push(task),
        }
    }

    normal_tasks.sort_by_key(|t| t.filesize);
    pak_tasks.sort_by_key(|t| t.filesize);
    block_tasks.sort_by_key(|t| t.filesize);

    let mut small_tasks: Vec<ResTask> = normal_tasks.into_iter().take(3).collect();
    small_tasks.extend(pak_tasks.into_iter().take(1));
    small_tasks.extend(block_tasks.into_iter().take(1));

    let _total_expected_bytes: u64 = small_tasks.iter().map(|t| t.filesize).sum();

    let manager = DownloadManager::new(
        base_url,
        bucket_dir.clone(),
        game_dir.clone(),
        2, // max_concurrent
    );

    let downloaded_bytes = Arc::new(AtomicU64::new(0));
    let downloaded_bytes_clone = downloaded_bytes.clone();

    manager
        .start_all(small_tasks.clone(), move |bytes| {
            downloaded_bytes_clone.fetch_add(bytes, Ordering::Relaxed);
        })
        .await?;

    // Wait a brief moment to ensure progress channel is fully flushed
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Validate symlinks and files
    for task in &small_tasks {
        match &task.task_type {
            nte_patcher::model::TaskType::Pak { entries } => {
                for entry in entries {
                    let target_path = game_dir.join(&entry.name);
                    assert!(
                        target_path.exists(),
                        "Pak entry {:?} should exist",
                        entry.name
                    );
                }
            }
            _ => {
                let target_path = game_dir.join(&task.target_path);
                assert!(
                    target_path.exists(),
                    "Target file {:?} should exist",
                    task.target_path
                );

                // Also verify that the size is correct
                let meta = tokio::fs::metadata(&target_path).await.map_err(Error::Io)?;
                assert_eq!(
                    meta.len(),
                    task.filesize,
                    "Size mismatch for {:?}",
                    task.target_path
                );
            }
        }
    }

    Ok(())
}

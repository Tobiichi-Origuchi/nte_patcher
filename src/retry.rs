use crate::error::Error;
use std::time::Duration;
use std::future::Future;

pub async fn with_retry<F, Fut, T>(max_retries: u32, mut action: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut attempts = 0;
    loop {
        match action().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e);
                }
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
            }
        }
    }
}

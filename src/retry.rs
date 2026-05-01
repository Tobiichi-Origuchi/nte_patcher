use crate::error::Error;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn random_factor() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 10000) as f64 / 10000.0
}

pub async fn with_retry<F, Fut, T>(max_retries: u32, mut action: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut attempts = 0;
    let max_delay = Duration::from_secs(30);

    loop {
        match action().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempts += 1;
                if !e.is_retryable() {
                    return Err(e);
                }
                if attempts >= max_retries {
                    return Err(e);
                }
                let base_delay = Duration::from_secs(2u64.pow(attempts));
                let delay = std::cmp::min(base_delay, max_delay);
                // add jitter to the delay to avoid synchronized retries
                let jitter_range = delay.as_secs_f64() * 0.5;
                let actual_delay = delay.as_secs_f64() - (jitter_range * random_factor());
                tokio::time::sleep(Duration::from_secs_f64(actual_delay)).await;
            }
        }
    }
}

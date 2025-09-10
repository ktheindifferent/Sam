use async_trait::async_trait;
use std::error::Error;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

#[async_trait]
pub trait RetryStrategy: Send + Sync {
    async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
        E: Error + Send;

    fn should_retry(&self, attempt: u32, error: &dyn Error) -> bool;
    fn calculate_delay(&self, attempt: u32) -> Duration;
}

pub struct ExponentialBackoffRetry {
    config: RetryConfig,
}

impl ExponentialBackoffRetry {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.config.max_attempts = max_attempts;
        self
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.config.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.config.max_delay = delay;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.config.jitter = jitter;
        self
    }

    fn add_jitter(&self, delay: Duration) -> Duration {
        if self.config.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter_ms = rng.gen_range(0..=delay.as_millis() / 4) as u64;
            delay + Duration::from_millis(jitter_ms)
        } else {
            delay
        }
    }
}

#[async_trait]
impl RetryStrategy for ExponentialBackoffRetry {
    async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
        E: Error + Send,
    {
        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempt += 1;
                    if !self.should_retry(attempt, &error) {
                        return Err(error);
                    }
                    let delay = self.calculate_delay(attempt);
                    log::warn!(
                        "Retry attempt {} after {:?} due to: {}",
                        attempt,
                        delay,
                        error
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    fn should_retry(&self, attempt: u32, _error: &dyn Error) -> bool {
        if attempt >= self.config.max_attempts {
            return false;
        }

        // Check if error is retryable - simplified for now to avoid lifetime issues
        true  // Always retry within max attempts
        
        /* TODO: Re-enable when lifetime issues are resolved
        if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
            matches!(
                io_error.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted
            )
        } else {
            // Default to retrying for unknown errors
            true
        }
        */
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.config.exponential_base.powi(attempt as i32 - 1);
        let capped_delay = exponential_delay.min(self.config.max_delay.as_millis() as f64);
        let delay = Duration::from_millis(capped_delay as u64);
        self.add_jitter(delay)
    }
}

pub struct LinearBackoffRetry {
    config: RetryConfig,
    increment: Duration,
}

impl LinearBackoffRetry {
    pub fn new(config: RetryConfig, increment: Duration) -> Self {
        Self { config, increment }
    }
}

#[async_trait]
impl RetryStrategy for LinearBackoffRetry {
    async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
        E: Error + Send,
    {
        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempt += 1;
                    if !self.should_retry(attempt, &error) {
                        return Err(error);
                    }
                    let delay = self.calculate_delay(attempt);
                    log::warn!(
                        "Retry attempt {} after {:?} due to: {}",
                        attempt,
                        delay,
                        error
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    fn should_retry(&self, attempt: u32, _error: &dyn Error) -> bool {
        attempt < self.config.max_attempts
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.config.initial_delay + self.increment * (attempt - 1);
        delay.min(self.config.max_delay)
    }
}

pub async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    max_attempts: u32,
) -> Result<T, E>
where
    F: Fn() -> Fut + Send,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send,
    E: Error + Send,
{
    let config = RetryConfig {
        max_attempts,
        ..Default::default()
    };
    let strategy = ExponentialBackoffRetry::new(config);
    strategy.execute(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test error")
        }
    }

    impl Error for TestError {}

    #[tokio::test]
    async fn test_exponential_backoff_success() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            exponential_base: 2.0,
            jitter: false,
        };

        let strategy = ExponentialBackoffRetry::new(config);
        let result = strategy
            .execute(|| {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err(TestError)
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_max_attempts_exceeded() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            exponential_base: 2.0,
            jitter: false,
        };

        let strategy = ExponentialBackoffRetry::new(config);
        let result = strategy
            .execute(|| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async move { Err::<(), TestError>(TestError) }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, Mutex};
use tokio::time::{interval, timeout};
use anyhow::{Result, Context};
use log::{debug, warn, error, info};
use std::collections::VecDeque;
use async_trait::async_trait;

/// Connection pool for managing database connections
pub struct ConnectionPool<C: PooledConnection> {
    config: PoolConfig,
    connections: Arc<RwLock<VecDeque<PoolEntry<C>>>>,
    semaphore: Arc<Semaphore>,
    factory: Arc<dyn ConnectionFactory<C>>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    metrics: Arc<RwLock<PoolMetrics>>,
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub health_check_interval: Duration,
    pub enable_circuit_breaker: bool,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_reset_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(60),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_timeout: Duration::from_secs(60),
        }
    }
}

/// Pool entry with connection and metadata
struct PoolEntry<C> {
    connection: C,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
}

impl<C> PoolEntry<C> {
    fn new(connection: C) -> Self {
        let now = Instant::now();
        PoolEntry {
            connection,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }
    
    fn is_expired(&self, config: &PoolConfig) -> bool {
        let now = Instant::now();
        
        // Check max lifetime
        if now.duration_since(self.created_at) > config.max_lifetime {
            return true;
        }
        
        // Check idle timeout
        if now.duration_since(self.last_used) > config.idle_timeout {
            return true;
        }
        
        false
    }
}

/// Connection factory trait
#[async_trait]
pub trait ConnectionFactory<C>: Send + Sync {
    async fn create(&self) -> Result<C>;
    async fn validate(&self, conn: &C) -> bool;
}

/// Pooled connection trait
#[async_trait]
pub trait PooledConnection: Send + Sync + 'static {
    async fn is_valid(&self) -> bool;
    async fn close(self);
}

/// Pool metrics
#[derive(Debug, Default)]
pub struct PoolMetrics {
    pub total_created: u64,
    pub total_closed: u64,
    pub total_checkouts: u64,
    pub total_returns: u64,
    pub failed_creates: u64,
    pub failed_checkouts: u64,
    pub current_size: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub wait_time_ms: u64,
}

impl<C: PooledConnection> ConnectionPool<C> {
    /// Create a new connection pool
    pub async fn new(
        config: PoolConfig,
        factory: Arc<dyn ConnectionFactory<C>>,
    ) -> Result<Self> {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let connections = Arc::new(RwLock::new(VecDeque::new()));
        let circuit_breaker = Arc::new(Mutex::new(CircuitBreaker::new(
            config.circuit_breaker_threshold,
            config.circuit_breaker_reset_timeout,
        )));
        let metrics = Arc::new(RwLock::new(PoolMetrics::default()));
        
        let mut pool = ConnectionPool {
            config: config.clone(),
            connections,
            semaphore,
            factory,
            circuit_breaker,
            metrics,
            health_check_handle: None,
        };
        
        // Initialize minimum connections
        for _ in 0..config.min_connections {
            if let Ok(conn) = pool.create_connection().await {
                pool.return_connection(conn).await;
            }
        }
        
        // Start health check task
        pool.start_health_check().await;
        
        Ok(pool)
    }
    
    /// Get a connection from the pool
    pub async fn get(&self) -> Result<PooledConnectionGuard<C>> {
        let start = Instant::now();
        
        // Check circuit breaker
        if self.config.enable_circuit_breaker {
            let mut breaker = self.circuit_breaker.lock().await;
            if breaker.is_open() {
                return Err(anyhow::anyhow!("Circuit breaker is open"));
            }
        }
        
        // Try to get connection with timeout
        let permit = timeout(
            self.config.connection_timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .context("Connection pool timeout")?
        .context("Failed to acquire semaphore")?;
        
        // Try to get existing connection
        let conn = {
            let mut connections = self.connections.write().await;
            loop {
                if let Some(mut entry) = connections.pop_front() {
                    if !entry.is_expired(&self.config) {
                        entry.last_used = Instant::now();
                        entry.use_count += 1;
                        
                        // Update metrics
                        let mut metrics = self.metrics.write().await;
                        metrics.total_checkouts += 1;
                        metrics.idle_connections = connections.len();
                        metrics.active_connections += 1;
                        metrics.wait_time_ms = start.elapsed().as_millis() as u64;
                        
                        break Some(entry.connection);
                    } else {
                        // Close expired connection
                        entry.connection.close().await;
                        
                        let mut metrics = self.metrics.write().await;
                        metrics.total_closed += 1;
                        metrics.current_size -= 1;
                    }
                } else {
                    break None;
                }
            }
        };
        
        // Create new connection if needed
        let conn = match conn {
            Some(c) => c,
            None => {
                match self.create_connection().await {
                    Ok(c) => c,
                    Err(e) => {
                        // Record failure in circuit breaker
                        if self.config.enable_circuit_breaker {
                            let mut breaker = self.circuit_breaker.lock().await;
                            breaker.record_failure();
                        }
                        
                        let mut metrics = self.metrics.write().await;
                        metrics.failed_checkouts += 1;
                        
                        return Err(e);
                    }
                }
            }
        };
        
        // Record success in circuit breaker
        if self.config.enable_circuit_breaker {
            let mut breaker = self.circuit_breaker.lock().await;
            breaker.record_success();
        }
        
        Ok(PooledConnectionGuard {
            connection: Some(conn),
            pool: self.clone(),
            permit: Some(permit),
        })
    }
    
    /// Create a new connection
    async fn create_connection(&self) -> Result<C> {
        match self.factory.create().await {
            Ok(conn) => {
                let mut metrics = self.metrics.write().await;
                metrics.total_created += 1;
                metrics.current_size += 1;
                
                Ok(conn)
            }
            Err(e) => {
                let mut metrics = self.metrics.write().await;
                metrics.failed_creates += 1;
                
                Err(e)
            }
        }
    }
    
    /// Return a connection to the pool
    async fn return_connection(&self, conn: C) {
        // Validate connection before returning
        if !self.factory.validate(&conn).await {
            conn.close().await;
            
            let mut metrics = self.metrics.write().await;
            metrics.total_closed += 1;
            metrics.current_size -= 1;
            metrics.active_connections -= 1;
            
            return;
        }
        
        let mut connections = self.connections.write().await;
        connections.push_back(PoolEntry::new(conn));
        
        let mut metrics = self.metrics.write().await;
        metrics.total_returns += 1;
        metrics.active_connections -= 1;
        metrics.idle_connections = connections.len();
    }
    
    /// Start health check task
    async fn start_health_check(&mut self) {
        let connections = self.connections.clone();
        let factory = self.factory.clone();
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                let mut to_remove = Vec::new();
                let mut connections = connections.write().await;
                
                for (i, entry) in connections.iter().enumerate() {
                    if entry.is_expired(&config) || !factory.validate(&entry.connection).await {
                        to_remove.push(i);
                    }
                }
                
                // Remove invalid connections
                for i in to_remove.iter().rev() {
                    if let Some(entry) = connections.remove(*i) {
                        entry.connection.close().await;
                        
                        let mut metrics = metrics.write().await;
                        metrics.total_closed += 1;
                        metrics.current_size -= 1;
                    }
                }
                
                if !to_remove.is_empty() {
                    debug!("Health check removed {} connections", to_remove.len());
                }
            }
        });
        
        self.health_check_handle = Some(handle);
        info!("Started connection pool health check");
    }
    
    /// Get pool metrics
    pub async fn get_metrics(&self) -> PoolMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Close all connections and shutdown pool
    pub async fn shutdown(mut self) {
        // Stop health check
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }
        
        // Close all connections
        let mut connections = self.connections.write().await;
        while let Some(entry) = connections.pop_front() {
            entry.connection.close().await;
        }
        
        info!("Connection pool shutdown complete");
    }
}

impl<C: PooledConnection> Clone for ConnectionPool<C> {
    fn clone(&self) -> Self {
        ConnectionPool {
            config: self.config.clone(),
            connections: self.connections.clone(),
            semaphore: self.semaphore.clone(),
            factory: self.factory.clone(),
            circuit_breaker: self.circuit_breaker.clone(),
            metrics: self.metrics.clone(),
            health_check_handle: None,
        }
    }
}

/// Guard for pooled connection with automatic return
pub struct PooledConnectionGuard<C: PooledConnection> {
    connection: Option<C>,
    pool: ConnectionPool<C>,
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl<C: PooledConnection> PooledConnectionGuard<C> {
    /// Get reference to the connection
    pub fn get(&self) -> &C {
        self.connection.as_ref().expect("Connection already taken")
    }
    
    /// Take the connection without returning to pool
    pub fn take(mut self) -> C {
        self.permit = None; // Release permit
        self.connection.take().expect("Connection already taken")
    }
}

impl<C: PooledConnection> Drop for PooledConnectionGuard<C> {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            let pool = self.pool.clone();
            
            // Return connection to pool asynchronously
            tokio::spawn(async move {
                pool.return_connection(conn).await;
            });
        }
    }
}

impl<C: PooledConnection> std::ops::Deref for PooledConnectionGuard<C> {
    type Target = C;
    
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// Circuit breaker for connection failures
struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    threshold: u32,
    last_failure: Option<Instant>,
    reset_timeout: Duration,
}

#[derive(Debug, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    fn new(threshold: u32, reset_timeout: Duration) -> Self {
        CircuitBreaker {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            threshold,
            last_failure: None,
            reset_timeout,
        }
    }
    
    fn is_open(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                if let Some(last_failure) = self.last_failure {
                    if Instant::now().duration_since(last_failure) > self.reset_timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.failure_count = 0;
                        self.success_count = 0;
                        debug!("Circuit breaker transitioned to half-open");
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            _ => false,
        }
    }
    
    fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= 3 {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    debug!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            _ => {}
        }
    }
    
    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        
        match self.state {
            CircuitBreakerState::Closed => {
                if self.failure_count >= self.threshold {
                    self.state = CircuitBreakerState::Open;
                    warn!("Circuit breaker opened after {} failures", self.failure_count);
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                warn!("Circuit breaker reopened after failure in half-open state");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock connection for testing
    struct MockConnection {
        id: u64,
        valid: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl PooledConnection for MockConnection {
        async fn is_valid(&self) -> bool {
            *self.valid.read().await
        }
        
        async fn close(self) {
            // No-op for testing
        }
    }

    struct MockFactory {
        counter: Arc<RwLock<u64>>,
    }

    #[async_trait]
    impl ConnectionFactory<MockConnection> for MockFactory {
        async fn create(&self) -> Result<MockConnection> {
            let mut counter = self.counter.write().await;
            *counter += 1;
            
            Ok(MockConnection {
                id: *counter,
                valid: Arc::new(RwLock::new(true)),
            })
        }
        
        async fn validate(&self, conn: &MockConnection) -> bool {
            conn.is_valid().await
        }
    }

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let config = PoolConfig {
            max_connections: 3,
            min_connections: 1,
            ..Default::default()
        };
        
        let factory = Arc::new(MockFactory {
            counter: Arc::new(RwLock::new(0)),
        });
        
        let pool = ConnectionPool::new(config, factory).await.unwrap();
        
        // Get connection
        let conn1 = pool.get().await.unwrap();
        assert_eq!(conn1.id, 1);
        
        // Get another connection
        let conn2 = pool.get().await.unwrap();
        assert_eq!(conn2.id, 2);
        
        // Return first connection
        drop(conn1);
        
        // Wait a bit for async return
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Get connection again - should reuse returned one
        let conn3 = pool.get().await.unwrap();
        assert_eq!(conn3.id, 1); // Reused connection
    }

    #[test]
    fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, Duration::from_secs(1));
        
        assert!(!breaker.is_open());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert!(!breaker.is_open());
        
        breaker.record_failure();
        assert!(breaker.is_open());
        
        // Record success doesn't close when open
        breaker.record_success();
        assert!(breaker.is_open());
    }
}
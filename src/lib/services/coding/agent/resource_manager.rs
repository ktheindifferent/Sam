//! Improved resource management with async patterns and automatic cleanup

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use anyhow::{Result, Context};
use log::{info, warn, debug};
use serde::{Serialize, Deserialize};

// Removed unused import: CodingAgentError

/// Resource types that can be managed
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Memory,
    CpuTime,
    FileHandles,
    NetworkConnections,
    ThreadPool,
    GpuMemory,
}

/// Resource allocation tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub resource_type: ResourceType,
    pub amount: u64,
    pub allocated_at: Instant,
    pub owner: String,
}

/// Resource pool for managing shared resources
pub struct ResourcePool {
    /// Available resources by type
    resources: Arc<RwLock<HashMap<ResourceType, ResourceLimit>>>,
    /// Active allocations
    allocations: Arc<RwLock<Vec<ResourceAllocation>>>,
    /// Cleanup tasks
    cleanup_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Global semaphore for rate limiting
    rate_limiter: Arc<Semaphore>,
}

/// Resource limit configuration
#[derive(Debug, Clone)]
struct ResourceLimit {
    max_amount: u64,
    current_usage: u64,
    peak_usage: u64,
    last_cleanup: Instant,
}

impl ResourcePool {
    pub fn new() -> Self {
        let mut resources = HashMap::new();

        // Initialize default resource limits
        resources.insert(ResourceType::Memory, ResourceLimit {
            max_amount: 4 * 1024 * 1024 * 1024, // 4GB
            current_usage: 0,
            peak_usage: 0,
            last_cleanup: Instant::now(),
        });

        resources.insert(ResourceType::FileHandles, ResourceLimit {
            max_amount: 1024,
            current_usage: 0,
            peak_usage: 0,
            last_cleanup: Instant::now(),
        });

        resources.insert(ResourceType::NetworkConnections, ResourceLimit {
            max_amount: 100,
            current_usage: 0,
            peak_usage: 0,
            last_cleanup: Instant::now(),
        });

        resources.insert(ResourceType::ThreadPool, ResourceLimit {
            max_amount: 50,
            current_usage: 0,
            peak_usage: 0,
            last_cleanup: Instant::now(),
        });

        let pool = Self {
            resources: Arc::new(RwLock::new(resources)),
            allocations: Arc::new(RwLock::new(Vec::new())),
            cleanup_handles: Arc::new(RwLock::new(Vec::new())),
            rate_limiter: Arc::new(Semaphore::new(100)), // 100 concurrent operations
        };

        // Start background cleanup task
        pool.start_cleanup_task();

        pool
    }

    /// Start background cleanup task
    fn start_cleanup_task(&self) {
        let resources = self.resources.clone();
        let allocations = self.allocations.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Clean up expired allocations
                let now = Instant::now();
                let mut allocs = allocations.write().await;

                // Remove allocations older than 5 minutes
                let expired: Vec<_> = allocs
                    .iter()
                    .filter(|a| now.duration_since(a.allocated_at) > Duration::from_secs(300))
                    .cloned()
                    .collect();

                for expired_alloc in expired {
                    allocs.retain(|a| a.allocated_at != expired_alloc.allocated_at);

                    // Release the resources
                    let mut res = resources.write().await;
                    if let Some(limit) = res.get_mut(&expired_alloc.resource_type) {
                        limit.current_usage = limit.current_usage.saturating_sub(expired_alloc.amount);
                        debug!("Released expired allocation: {:?} ({} units)",
                               expired_alloc.resource_type, expired_alloc.amount);
                    }
                }

                // Update last cleanup time
                let mut res = resources.write().await;
                for limit in res.values_mut() {
                    limit.last_cleanup = now;
                }
            }
        });

        let mut handles = tokio::runtime::Handle::current().block_on(self.cleanup_handles.write());
        handles.push(handle);
    }

    /// Allocate resources with automatic cleanup
    pub async fn allocate(
        &self,
        resource_type: ResourceType,
        amount: u64,
        owner: String,
    ) -> Result<ResourceGuard> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await
            .context("Failed to acquire rate limit permit")?;

        // Check and allocate resource
        let mut resources = self.resources.write().await;

        let limit = resources.get_mut(&resource_type)
            .ok_or_else(|| anyhow::anyhow!("Resource type {:?} not configured", resource_type))?;

        if limit.current_usage + amount > limit.max_amount {
            return Err(anyhow::anyhow!(
                "Insufficient {:?} resources: requested {}, available {}",
                resource_type,
                amount,
                limit.max_amount - limit.current_usage
            ));
        }

        // Update usage
        limit.current_usage += amount;
        limit.peak_usage = limit.peak_usage.max(limit.current_usage);

        // Record allocation
        let allocation = ResourceAllocation {
            resource_type: resource_type.clone(),
            amount,
            allocated_at: Instant::now(),
            owner,
        };

        self.allocations.write().await.push(allocation.clone());

        info!("Allocated {} units of {:?} for {}",
              amount, resource_type, allocation.owner);

        // Return RAII guard
        Ok(ResourceGuard {
            pool: Arc::new(self.clone_weak()),
            allocation,
        })
    }

    /// Try to allocate with timeout
    pub async fn try_allocate_with_timeout(
        &self,
        resource_type: ResourceType,
        amount: u64,
        owner: String,
        duration: Duration,
    ) -> Result<ResourceGuard> {
        timeout(duration, self.allocate(resource_type, amount, owner))
            .await
            .context("Resource allocation timed out")?
    }

    /// Get current resource usage
    pub async fn get_usage(&self, resource_type: &ResourceType) -> Option<ResourceUsage> {
        let resources = self.resources.read().await;

        resources.get(resource_type).map(|limit| ResourceUsage {
            current: limit.current_usage,
            max: limit.max_amount,
            peak: limit.peak_usage,
            utilization: (limit.current_usage as f64 / limit.max_amount as f64) * 100.0,
        })
    }

    /// Get all resource usage
    pub async fn get_all_usage(&self) -> HashMap<ResourceType, ResourceUsage> {
        let resources = self.resources.read().await;

        resources.iter().map(|(typ, limit)| {
            (typ.clone(), ResourceUsage {
                current: limit.current_usage,
                max: limit.max_amount,
                peak: limit.peak_usage,
                utilization: (limit.current_usage as f64 / limit.max_amount as f64) * 100.0,
            })
        }).collect()
    }

    /// Release resources manually
    async fn release(&self, allocation: &ResourceAllocation) {
        let mut resources = self.resources.write().await;

        if let Some(limit) = resources.get_mut(&allocation.resource_type) {
            limit.current_usage = limit.current_usage.saturating_sub(allocation.amount);
            debug!("Released {} units of {:?}",
                   allocation.amount, allocation.resource_type);
        }

        // Remove from active allocations
        let mut allocs = self.allocations.write().await;
        allocs.retain(|a| a.allocated_at != allocation.allocated_at);
    }

    /// Create a weak reference for guards
    fn clone_weak(&self) -> ResourcePoolWeak {
        ResourcePoolWeak {
            resources: Arc::downgrade(&self.resources),
            allocations: Arc::downgrade(&self.allocations),
        }
    }

    /// Shutdown and cleanup all resources
    pub async fn shutdown(&self) {
        info!("Shutting down resource pool");

        // Cancel all cleanup tasks
        let mut handles = self.cleanup_handles.write().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        // Release all allocations
        let allocations = self.allocations.read().await.clone();
        for allocation in allocations {
            self.release(&allocation).await;
        }

        info!("Resource pool shutdown complete");
    }
}

/// Weak reference for resource pool
#[derive(Clone)]
struct ResourcePoolWeak {
    resources: std::sync::Weak<RwLock<HashMap<ResourceType, ResourceLimit>>>,
    allocations: std::sync::Weak<RwLock<Vec<ResourceAllocation>>>,
}

/// RAII guard for automatic resource cleanup
pub struct ResourceGuard {
    pool: Arc<ResourcePoolWeak>,
    allocation: ResourceAllocation,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // Try to release resources if pool still exists
        if let (Some(resources), Some(allocations)) =
            (self.pool.resources.upgrade(), self.pool.allocations.upgrade()) {

            let allocation = self.allocation.clone();

            // Spawn cleanup task
            tokio::spawn(async move {
                // Release the resource
                let mut res = resources.write().await;
                if let Some(limit) = res.get_mut(&allocation.resource_type) {
                    limit.current_usage = limit.current_usage.saturating_sub(allocation.amount);
                }

                // Remove from allocations
                let mut allocs = allocations.write().await;
                allocs.retain(|a| a.allocated_at != allocation.allocated_at);
            });
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub current: u64,
    pub max: u64,
    pub peak: u64,
    pub utilization: f64,
}

/// Advanced resource manager with policies
pub struct ResourceManager {
    pool: Arc<ResourcePool>,
    policies: Arc<RwLock<ResourcePolicies>>,
    metrics: Arc<RwLock<ResourceMetrics>>,
}

/// Resource management policies
#[derive(Debug, Clone)]
struct ResourcePolicies {
    /// Enable automatic resource scaling
    auto_scale: bool,
    /// Maximum resource utilization before throttling
    throttle_threshold: f64,
    /// Enable resource preemption
    allow_preemption: bool,
    /// Resource allocation priority levels
    priority_levels: HashMap<String, u8>,
}

impl Default for ResourcePolicies {
    fn default() -> Self {
        Self {
            auto_scale: true,
            throttle_threshold: 80.0,
            allow_preemption: false,
            priority_levels: HashMap::new(),
        }
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Default)]
struct ResourceMetrics {
    total_allocations: u64,
    failed_allocations: u64,
    total_releases: u64,
    average_hold_time: Duration,
    peak_utilization: HashMap<ResourceType, f64>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(ResourcePool::new()),
            policies: Arc::new(RwLock::new(ResourcePolicies::default())),
            metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
        }
    }

    /// Allocate with priority and policies
    pub async fn allocate_with_priority(
        &self,
        resource_type: ResourceType,
        amount: u64,
        owner: String,
        priority: u8,
    ) -> Result<ResourceGuard> {
        let policies = self.policies.read().await;

        // Check if we need to throttle based on utilization
        if let Some(usage) = self.pool.get_usage(&resource_type).await {
            if usage.utilization > policies.throttle_threshold {
                if priority < 5 {
                    // Low priority requests are throttled
                    warn!("Throttling allocation request for {} due to high utilization",
                          owner);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_allocations += 1;
        }

        // Attempt allocation
        match self.pool.allocate(resource_type.clone(), amount, owner).await {
            Ok(guard) => {
                // Update peak utilization
                if let Some(usage) = self.pool.get_usage(&resource_type).await {
                    let mut metrics = self.metrics.write().await;
                    metrics.peak_utilization
                        .entry(resource_type)
                        .and_modify(|peak| *peak = peak.max(usage.utilization))
                        .or_insert(usage.utilization);
                }
                Ok(guard)
            }
            Err(e) => {
                let mut metrics = self.metrics.write().await;
                metrics.failed_allocations += 1;
                Err(e)
            }
        }
    }

    /// Get resource metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        self.metrics.read().await.clone()
    }

    /// Update resource policies
    pub async fn update_policies<F>(&self, updater: F)
    where
        F: FnOnce(&mut ResourcePolicies),
    {
        let mut policies = self.policies.write().await;
        updater(&mut policies);
    }

    /// Shutdown the resource manager
    pub async fn shutdown(&self) {
        self.pool.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_allocation() {
        let pool = ResourcePool::new();

        let guard = pool.allocate(
            ResourceType::Memory,
            1024,
            "test".to_string()
        ).await.unwrap();

        let usage = pool.get_usage(&ResourceType::Memory).await.unwrap();
        assert_eq!(usage.current, 1024);

        drop(guard);
        tokio::time::sleep(Duration::from_millis(100)).await;

        let usage = pool.get_usage(&ResourceType::Memory).await.unwrap();
        assert_eq!(usage.current, 0);
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let pool = ResourcePool::new();

        // Set a small limit for testing
        {
            let mut resources = pool.resources.write().await;
            resources.get_mut(&ResourceType::FileHandles).unwrap().max_amount = 10;
        }

        // Allocate up to limit
        let _g1 = pool.allocate(ResourceType::FileHandles, 5, "test1".to_string())
            .await.unwrap();
        let _g2 = pool.allocate(ResourceType::FileHandles, 5, "test2".to_string())
            .await.unwrap();

        // This should fail
        let result = pool.allocate(ResourceType::FileHandles, 1, "test3".to_string()).await;
        assert!(result.is_err());
    }
}
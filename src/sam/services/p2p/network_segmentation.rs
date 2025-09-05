// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Semaphore};
use serde::{Deserialize, Serialize};
use log::{info, warn, error, debug};

/// Different types of network channels for data segregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Control,      // Control messages (handshakes, heartbeats)
    Data,         // Regular data transfer
    FileTransfer, // Large file transfers
    Sync,         // State synchronization
    Emergency,    // High-priority emergency messages
}

/// Priority levels for message queuing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Rate limiting configuration per peer
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_messages_per_second: u32,
    pub max_bytes_per_second: usize,
    pub burst_size: u32,
    pub penalty_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_second: 100,
            max_bytes_per_second: 10 * 1024 * 1024, // 10MB/s
            burst_size: 200,
            penalty_duration: Duration::from_secs(60),
        }
    }
}

/// Rate limiter for individual peers
#[derive(Debug)]
struct PeerRateLimiter {
    message_tokens: Arc<Semaphore>,
    byte_tokens: Arc<RwLock<usize>>,
    last_refill: Arc<RwLock<SystemTime>>,
    violations: Arc<RwLock<u32>>,
    blocked_until: Arc<RwLock<Option<SystemTime>>>,
}

impl PeerRateLimiter {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            message_tokens: Arc::new(Semaphore::new(config.burst_size as usize)),
            byte_tokens: Arc::new(RwLock::new(config.max_bytes_per_second)),
            last_refill: Arc::new(RwLock::new(SystemTime::now())),
            violations: Arc::new(RwLock::new(0)),
            blocked_until: Arc::new(RwLock::new(None)),
        }
    }

    async fn check_rate_limit(&self, message_size: usize, config: &RateLimitConfig) -> Result<(), String> {
        // Check if peer is blocked
        let blocked_until = self.blocked_until.read().await;
        if let Some(until) = *blocked_until {
            if SystemTime::now() < until {
                return Err("Peer is temporarily blocked due to rate limit violations".to_string());
            }
        }
        drop(blocked_until);

        // Refill tokens based on elapsed time
        self.refill_tokens(config).await;

        // Check message rate
        match self.message_tokens.try_acquire() {
            Ok(_permit) => {
                // Check byte rate
                let mut byte_tokens = self.byte_tokens.write().await;
                if *byte_tokens >= message_size {
                    *byte_tokens -= message_size;
                    Ok(())
                } else {
                    // Rate limit exceeded
                    self.record_violation(config).await;
                    Err("Byte rate limit exceeded".to_string())
                }
            }
            Err(_) => {
                // Rate limit exceeded
                self.record_violation(config).await;
                Err("Message rate limit exceeded".to_string())
            }
        }
    }

    async fn refill_tokens(&self, config: &RateLimitConfig) {
        let now = SystemTime::now();
        let mut last_refill = self.last_refill.write().await;
        
        if let Ok(elapsed) = now.duration_since(*last_refill) {
            let seconds = elapsed.as_secs_f64();
            
            // Refill message tokens
            let messages_to_add = (config.max_messages_per_second as f64 * seconds) as u32;
            for _ in 0..messages_to_add.min(config.burst_size) {
                self.message_tokens.add_permits(1);
            }
            
            // Refill byte tokens
            let mut byte_tokens = self.byte_tokens.write().await;
            let bytes_to_add = (config.max_bytes_per_second as f64 * seconds) as usize;
            *byte_tokens = (*byte_tokens + bytes_to_add).min(config.max_bytes_per_second * 2);
            
            *last_refill = now;
        }
    }

    async fn record_violation(&self, config: &RateLimitConfig) {
        let mut violations = self.violations.write().await;
        *violations += 1;
        
        // Block peer if too many violations
        if *violations > 5 {
            let mut blocked_until = self.blocked_until.write().await;
            *blocked_until = Some(SystemTime::now() + config.penalty_duration);
            warn!("Peer blocked for {:?} due to rate limit violations", config.penalty_duration);
        }
    }
}

/// Message queue with priority handling
#[derive(Debug)]
struct PriorityQueue<T> {
    queues: HashMap<Priority, VecDeque<T>>,
    max_size: usize,
    current_size: Arc<RwLock<usize>>,
}

impl<T> PriorityQueue<T> {
    fn new(max_size: usize) -> Self {
        let mut queues = HashMap::new();
        queues.insert(Priority::Critical, VecDeque::new());
        queues.insert(Priority::High, VecDeque::new());
        queues.insert(Priority::Normal, VecDeque::new());
        queues.insert(Priority::Low, VecDeque::new());
        
        Self {
            queues,
            max_size,
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    async fn push(&mut self, item: T, priority: Priority) -> Result<(), String> {
        let size = *self.current_size.read().await;
        if size >= self.max_size {
            // Try to drop low priority messages first
            if priority > Priority::Low {
                if let Some(queue) = self.queues.get_mut(&Priority::Low) {
                    if !queue.is_empty() {
                        queue.pop_front();
                        let mut size = self.current_size.write().await;
                        *size -= 1;
                    }
                }
            } else {
                return Err("Queue is full".to_string());
            }
        }
        
        if let Some(queue) = self.queues.get_mut(&priority) {
            queue.push_back(item);
            let mut size = self.current_size.write().await;
            *size += 1;
            Ok(())
        } else {
            Err("Invalid priority".to_string())
        }
    }

    async fn pop(&mut self) -> Option<T> {
        // Pop from highest priority queue first
        for priority in [Priority::Critical, Priority::High, Priority::Normal, Priority::Low] {
            if let Some(queue) = self.queues.get_mut(&priority) {
                if let Some(item) = queue.pop_front() {
                    let mut size = self.current_size.write().await;
                    *size -= 1;
                    return Some(item);
                }
            }
        }
        None
    }
}

/// Network segmentation manager
pub struct NetworkSegmentation {
    channels: Arc<RwLock<HashMap<ChannelType, ChannelConfig>>>,
    peer_rate_limiters: Arc<RwLock<HashMap<String, PeerRateLimiter>>>,
    message_queues: Arc<RwLock<HashMap<String, PriorityQueue<QueuedMessage>>>>,
    rate_limit_config: RateLimitConfig,
}

#[derive(Debug, Clone)]
struct ChannelConfig {
    enabled: bool,
    max_bandwidth: usize,
    priority: Priority,
    encryption_required: bool,
}

#[derive(Debug)]
struct QueuedMessage {
    channel: ChannelType,
    priority: Priority,
    data: Vec<u8>,
    timestamp: SystemTime,
}

impl NetworkSegmentation {
    pub fn new() -> Self {
        let mut channels = HashMap::new();
        
        // Configure default channels
        channels.insert(ChannelType::Control, ChannelConfig {
            enabled: true,
            max_bandwidth: 1024 * 1024, // 1MB/s
            priority: Priority::High,
            encryption_required: true,
        });
        
        channels.insert(ChannelType::Data, ChannelConfig {
            enabled: true,
            max_bandwidth: 10 * 1024 * 1024, // 10MB/s
            priority: Priority::Normal,
            encryption_required: true,
        });
        
        channels.insert(ChannelType::FileTransfer, ChannelConfig {
            enabled: true,
            max_bandwidth: 50 * 1024 * 1024, // 50MB/s
            priority: Priority::Low,
            encryption_required: true,
        });
        
        channels.insert(ChannelType::Sync, ChannelConfig {
            enabled: true,
            max_bandwidth: 5 * 1024 * 1024, // 5MB/s
            priority: Priority::Normal,
            encryption_required: true,
        });
        
        channels.insert(ChannelType::Emergency, ChannelConfig {
            enabled: true,
            max_bandwidth: 100 * 1024, // 100KB/s
            priority: Priority::Critical,
            encryption_required: true,
        });
        
        Self {
            channels: Arc::new(RwLock::new(channels)),
            peer_rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            message_queues: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_config: RateLimitConfig::default(),
        }
    }

    /// Check if a message can be sent on a specific channel
    pub async fn can_send(&self, peer_id: &str, channel: ChannelType, message_size: usize) -> Result<(), String> {
        // Check if channel is enabled
        let channels = self.channels.read().await;
        let channel_config = channels.get(&channel)
            .ok_or("Unknown channel type")?;
        
        if !channel_config.enabled {
            return Err("Channel is disabled".to_string());
        }
        
        // Check channel bandwidth limit
        if message_size > channel_config.max_bandwidth {
            return Err("Message exceeds channel bandwidth limit".to_string());
        }
        
        // Get or create rate limiter for peer
        let mut rate_limiters = self.peer_rate_limiters.write().await;
        let rate_limiter = rate_limiters.entry(peer_id.to_string())
            .or_insert_with(|| PeerRateLimiter::new(&self.rate_limit_config));
        
        // Check rate limit
        rate_limiter.check_rate_limit(message_size, &self.rate_limit_config).await
    }

    /// Queue a message for sending
    pub async fn queue_message(
        &self,
        peer_id: &str,
        channel: ChannelType,
        priority: Priority,
        data: Vec<u8>,
    ) -> Result<(), String> {
        // Check if we can send
        self.can_send(peer_id, channel, data.len()).await?;
        
        // Get or create message queue for peer
        let mut queues = self.message_queues.write().await;
        let queue = queues.entry(peer_id.to_string())
            .or_insert_with(|| PriorityQueue::new(1000));
        
        // Queue the message
        let message = QueuedMessage {
            channel,
            priority,
            data,
            timestamp: SystemTime::now(),
        };
        
        queue.push(message, priority).await
    }

    /// Get next message to send for a peer
    pub async fn get_next_message(&self, peer_id: &str) -> Option<(ChannelType, Vec<u8>)> {
        let mut queues = self.message_queues.write().await;
        if let Some(queue) = queues.get_mut(peer_id) {
            if let Some(message) = queue.pop().await {
                // Check if message hasn't expired (5 minutes timeout)
                if let Ok(elapsed) = SystemTime::now().duration_since(message.timestamp) {
                    if elapsed < Duration::from_secs(300) {
                        return Some((message.channel, message.data));
                    } else {
                        debug!("Dropped expired message for peer {}", peer_id);
                    }
                }
            }
        }
        None
    }

    /// Update channel configuration
    pub async fn configure_channel(
        &self,
        channel: ChannelType,
        enabled: bool,
        max_bandwidth: usize,
        priority: Priority,
        encryption_required: bool,
    ) {
        let mut channels = self.channels.write().await;
        channels.insert(channel, ChannelConfig {
            enabled,
            max_bandwidth,
            priority,
            encryption_required,
        });
        
        info!("Updated configuration for channel {:?}", channel);
    }

    /// Reset rate limit violations for a peer
    pub async fn reset_peer_violations(&self, peer_id: &str) {
        let mut rate_limiters = self.peer_rate_limiters.write().await;
        if let Some(rate_limiter) = rate_limiters.get_mut(peer_id) {
            let mut violations = rate_limiter.violations.write().await;
            *violations = 0;
            let mut blocked_until = rate_limiter.blocked_until.write().await;
            *blocked_until = None;
            
            info!("Reset rate limit violations for peer {}", peer_id);
        }
    }

    /// Get statistics for a peer
    pub async fn get_peer_stats(&self, peer_id: &str) -> Option<PeerStats> {
        let rate_limiters = self.peer_rate_limiters.read().await;
        if let Some(rate_limiter) = rate_limiters.get(peer_id) {
            let violations = *rate_limiter.violations.read().await;
            let blocked_until = *rate_limiter.blocked_until.read().await;
            
            let queues = self.message_queues.read().await;
            let queue_size = if let Some(queue) = queues.get(peer_id) {
                *queue.current_size.read().await
            } else {
                0
            };
            
            Some(PeerStats {
                violations,
                blocked_until,
                queued_messages: queue_size,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeerStats {
    pub violations: u32,
    pub blocked_until: Option<SystemTime>,
    pub queued_messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_segmentation() {
        let segmentation = NetworkSegmentation::new();
        
        // Test channel configuration
        segmentation.configure_channel(
            ChannelType::Data,
            true,
            1024 * 1024,
            Priority::High,
            true,
        ).await;
        
        // Test message queueing
        let result = segmentation.queue_message(
            "peer1",
            ChannelType::Data,
            Priority::Normal,
            vec![1, 2, 3, 4],
        ).await;
        
        assert!(result.is_ok());
        
        // Test getting next message
        let message = segmentation.get_next_message("peer1").await;
        assert!(message.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let segmentation = NetworkSegmentation::new();
        
        // Send many messages quickly to trigger rate limit
        for i in 0..200 {
            let _ = segmentation.can_send(
                "peer2",
                ChannelType::Data,
                1024,
            ).await;
        }
        
        // Check that peer has violations
        let stats = segmentation.get_peer_stats("peer2").await;
        assert!(stats.is_some());
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let mut queue = PriorityQueue::new(10);
        
        // Add messages with different priorities
        queue.push("low", Priority::Low).await.unwrap();
        queue.push("critical", Priority::Critical).await.unwrap();
        queue.push("normal", Priority::Normal).await.unwrap();
        queue.push("high", Priority::High).await.unwrap();
        
        // Should get critical first
        assert_eq!(queue.pop().await, Some("critical"));
        assert_eq!(queue.pop().await, Some("high"));
        assert_eq!(queue.pop().await, Some("normal"));
        assert_eq!(queue.pop().await, Some("low"));
    }
}
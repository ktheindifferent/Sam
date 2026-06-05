// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub auto_sync: bool,
    pub sync_interval: Duration,
    pub conflict_resolution: ConflictResolution,
    pub sync_types: HashSet<String>,
    pub excluded_paths: Vec<String>,
    pub bandwidth_limit: Option<usize>, // bytes per second
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    LatestWins,
    OldestWins,
    Manual,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub id: String,
    pub timestamp: u64,
    pub hash: String,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub from_version: u64,
    pub to_version: u64,
    pub operations: Vec<SyncOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    Insert {
        key: String,
        value: Vec<u8>,
    },
    Update {
        key: String,
        old_value: Vec<u8>,
        new_value: Vec<u8>,
    },
    Delete {
        key: String,
    },
    Move {
        old_key: String,
        new_key: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub sync_type: String,
    pub last_sync: Option<u64>,
    pub state_hash: Option<String>,
    pub full_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub sync_type: String,
    pub timestamp: u64,
    pub delta: Option<SyncDelta>,
    pub full_state: Option<SyncState>,
    pub conflicts: Vec<SyncConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub key: String,
    pub local_value: Vec<u8>,
    pub remote_value: Vec<u8>,
    pub local_timestamp: u64,
    pub remote_timestamp: u64,
}

pub struct SyncManager {
    config: SyncConfig,
    local_states: Arc<RwLock<HashMap<String, SyncState>>>,
    sync_history: Arc<RwLock<Vec<SyncEvent>>>,
    pending_conflicts: Arc<RwLock<Vec<SyncConflict>>>,
    sync_peers: Arc<RwLock<HashMap<String, PeerSyncInfo>>>,
}

#[derive(Debug, Clone)]
struct SyncEvent {
    timestamp: u64,
    sync_type: String,
    peer_id: String,
    success: bool,
    changes: usize,
    conflicts: usize,
}

#[derive(Debug, Clone)]
struct PeerSyncInfo {
    peer_id: String,
    last_sync: u64,
    sync_types: HashSet<String>,
    reliability_score: f32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_interval: Duration::from_secs(300), // 5 minutes
            conflict_resolution: ConflictResolution::LatestWins,
            sync_types: HashSet::from([
                "configuration".to_string(),
                "state".to_string(),
                "cache".to_string(),
            ]),
            excluded_paths: Vec::new(),
            bandwidth_limit: None,
        }
    }
}

impl SyncManager {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            local_states: Arc::new(RwLock::new(HashMap::new())),
            sync_history: Arc::new(RwLock::new(Vec::new())),
            pending_conflicts: Arc::new(RwLock::new(Vec::new())),
            sync_peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_auto_sync(&self) {
        if !self.config.auto_sync {
            return;
        }

        let manager = self.clone_internal();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(manager.config.sync_interval).await;
                if let Err(e) = manager.sync_all_peers().await {
                    error!("Auto-sync error: {}", e);
                }
            }
        });
    }

    pub async fn sync_with_peer(
        &self,
        peer_id: String,
        sync_type: String,
    ) -> Result<SyncResponse, Box<dyn std::error::Error>> {
        let local_states = self.local_states.read().await;
        let local_state = local_states.get(&sync_type);

        let request = SyncRequest {
            sync_type: sync_type.clone(),
            last_sync: self.get_last_sync_time(&peer_id, &sync_type).await,
            state_hash: local_state.map(|s| s.hash.clone()),
            full_sync: false,
        };

        // Send sync request to peer (implementation depends on P2P layer)
        let response = self.send_sync_request(&peer_id, request).await?;

        // Process response
        self.process_sync_response(&peer_id, response.clone())
            .await?;

        Ok(response)
    }

    pub async fn handle_sync_request(
        &self,
        peer_id: String,
        request: SyncRequest,
    ) -> Result<SyncResponse, Box<dyn std::error::Error>> {
        let local_states = self.local_states.read().await;

        if let Some(local_state) = local_states.get(&request.sync_type) {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

            if request.full_sync || request.state_hash != Some(local_state.hash.clone()) {
                // Send full state
                Ok(SyncResponse {
                    sync_type: request.sync_type,
                    timestamp,
                    delta: None,
                    full_state: Some(local_state.clone()),
                    conflicts: Vec::new(),
                })
            } else if let Some(last_sync) = request.last_sync {
                // Generate and send delta
                let delta = self.generate_delta(&request.sync_type, last_sync).await?;
                Ok(SyncResponse {
                    sync_type: request.sync_type,
                    timestamp,
                    delta: Some(delta),
                    full_state: None,
                    conflicts: Vec::new(),
                })
            } else {
                // No sync needed
                Ok(SyncResponse {
                    sync_type: request.sync_type,
                    timestamp,
                    delta: None,
                    full_state: None,
                    conflicts: Vec::new(),
                })
            }
        } else {
            Err(format!("Unknown sync type: {}", request.sync_type).into())
        }
    }

    async fn process_sync_response(
        &self,
        peer_id: &str,
        response: SyncResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut local_states = self.local_states.write().await;

        if let Some(full_state) = response.full_state {
            // Apply full state
            local_states.insert(response.sync_type.clone(), full_state);
        } else if let Some(delta) = response.delta {
            // Apply delta
            self.apply_delta(&response.sync_type, delta).await?;
        }

        // Handle conflicts
        let conflicts_count = response.conflicts.len();
        if !response.conflicts.is_empty() {
            self.handle_conflicts(response.conflicts).await?;
        }

        // Record sync event
        self.record_sync_event(
            peer_id.to_string(),
            response.sync_type,
            true,
            0,
            conflicts_count,
        )
        .await;

        Ok(())
    }

    async fn apply_delta(
        &self,
        sync_type: &str,
        delta: SyncDelta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut local_states = self.local_states.write().await;

        if let Some(state) = local_states.get_mut(sync_type) {
            for operation in delta.operations {
                match operation {
                    SyncOperation::Insert { key, value } => {
                        // Apply insert operation
                        debug!("Applying insert: {}", key);
                    }
                    SyncOperation::Update {
                        key,
                        old_value,
                        new_value,
                    } => {
                        // Apply update operation
                        debug!("Applying update: {}", key);
                    }
                    SyncOperation::Delete { key } => {
                        // Apply delete operation
                        debug!("Applying delete: {}", key);
                    }
                    SyncOperation::Move { old_key, new_key } => {
                        // Apply move operation
                        debug!("Applying move: {} -> {}", old_key, new_key);
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_conflicts(
        &self,
        conflicts: Vec<SyncConflict>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.config.conflict_resolution {
            ConflictResolution::LatestWins => {
                for conflict in conflicts {
                    if conflict.remote_timestamp > conflict.local_timestamp {
                        // Use remote value
                        self.apply_conflict_resolution(&conflict.key, conflict.remote_value)
                            .await?;
                    }
                    // Otherwise keep local value
                }
            }
            ConflictResolution::OldestWins => {
                for conflict in conflicts {
                    if conflict.remote_timestamp < conflict.local_timestamp {
                        // Use remote value
                        self.apply_conflict_resolution(&conflict.key, conflict.remote_value)
                            .await?;
                    }
                    // Otherwise keep local value
                }
            }
            ConflictResolution::Manual => {
                // Store conflicts for manual resolution
                let mut pending = self.pending_conflicts.write().await;
                pending.extend(conflicts);
            }
            ConflictResolution::Merge => {
                // Attempt to merge conflicts
                for conflict in conflicts {
                    let merged = self.merge_conflict(conflict).await?;
                    self.apply_conflict_resolution(&merged.0, merged.1).await?;
                }
            }
        }

        Ok(())
    }

    async fn merge_conflict(
        &self,
        conflict: SyncConflict,
    ) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
        // Simple merge strategy - combine both values
        // In practice, this would be more sophisticated
        let mut merged = conflict.local_value.clone();
        merged.extend_from_slice(&conflict.remote_value);
        Ok((conflict.key, merged))
    }

    async fn apply_conflict_resolution(
        &self,
        key: &str,
        value: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Applying conflict resolution for key: {}", key);
        // Apply the resolved value
        Ok(())
    }

    async fn generate_delta(
        &self,
        sync_type: &str,
        since: u64,
    ) -> Result<SyncDelta, Box<dyn std::error::Error>> {
        // Generate delta based on changes since timestamp
        Ok(SyncDelta {
            from_version: since,
            to_version: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            operations: Vec::new(),
        })
    }

    async fn send_sync_request(
        &self,
        peer_id: &str,
        request: SyncRequest,
    ) -> Result<SyncResponse, Box<dyn std::error::Error>> {
        // Send request to peer via P2P network
        // This is a placeholder - actual implementation would use P2P layer
        Ok(SyncResponse {
            sync_type: request.sync_type,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            delta: None,
            full_state: None,
            conflicts: Vec::new(),
        })
    }

    async fn sync_all_peers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.sync_peers.read().await;

        for (peer_id, peer_info) in peers.iter() {
            for sync_type in &peer_info.sync_types {
                if self.config.sync_types.contains(sync_type) {
                    if let Err(e) = self
                        .sync_with_peer(peer_id.clone(), sync_type.clone())
                        .await
                    {
                        warn!("Failed to sync {} with peer {}: {}", sync_type, peer_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_last_sync_time(&self, peer_id: &str, sync_type: &str) -> Option<u64> {
        let history = self.sync_history.read().await;

        history
            .iter()
            .filter(|e| e.peer_id == peer_id && e.sync_type == sync_type && e.success)
            .map(|e| e.timestamp)
            .max()
    }

    async fn record_sync_event(
        &self,
        peer_id: String,
        sync_type: String,
        success: bool,
        changes: usize,
        conflicts: usize,
    ) {
        let mut history = self.sync_history.write().await;

        history.push(SyncEvent {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sync_type,
            peer_id,
            success,
            changes,
            conflicts,
        });

        // Keep only last 1000 events
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    pub async fn add_sync_peer(&self, peer_id: String, sync_types: HashSet<String>) {
        let mut peers = self.sync_peers.write().await;

        peers.insert(
            peer_id.clone(),
            PeerSyncInfo {
                peer_id,
                last_sync: 0,
                sync_types,
                reliability_score: 1.0,
            },
        );
    }

    pub async fn remove_sync_peer(&self, peer_id: &str) {
        let mut peers = self.sync_peers.write().await;
        peers.remove(peer_id);
    }

    pub async fn get_sync_status(&self) -> HashMap<String, Vec<SyncEvent>> {
        let history = self.sync_history.read().await;
        let mut status = HashMap::new();

        for event in history.iter() {
            status
                .entry(event.sync_type.clone())
                .or_insert_with(Vec::new)
                .push(event.clone());
        }

        status
    }

    pub async fn get_pending_conflicts(&self) -> Vec<SyncConflict> {
        let conflicts = self.pending_conflicts.read().await;
        conflicts.clone()
    }

    pub async fn resolve_conflict(
        &self,
        key: String,
        use_local: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conflicts = self.pending_conflicts.write().await;

        if let Some(pos) = conflicts.iter().position(|c| c.key == key) {
            let conflict = conflicts.remove(pos);

            if !use_local {
                self.apply_conflict_resolution(&key, conflict.remote_value)
                    .await?;
            }
            // If use_local is true, we keep the local value (do nothing)
        }

        Ok(())
    }

    fn clone_internal(&self) -> SyncManager {
        SyncManager {
            config: self.config.clone(),
            local_states: Arc::clone(&self.local_states),
            sync_history: Arc::clone(&self.sync_history),
            pending_conflicts: Arc::clone(&self.pending_conflicts),
            sync_peers: Arc::clone(&self.sync_peers),
        }
    }
}

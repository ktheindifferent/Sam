// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use ring::rand::SystemRandom;
use sha2::{Sha256, Digest};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{info, warn, error, debug};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::io::{Read, Write};

use super::secure::{SecureP2P, PeerIdentity, EncryptedMessage, TrustLevel};

const DEFAULT_P2P_PORT: u16 = 9000;
const DISCOVERY_PORT: u16 = 9001;
const MAX_PEERS: usize = 50;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const PEER_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub address: SocketAddr,
    pub public_key: Vec<u8>,
    pub capabilities: Vec<String>,
    pub version: String,
    pub last_seen: u64,
    pub latency_ms: Option<u32>,
    pub trust_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    Handshake {
        peer_info: PeerInfo,
        nonce: Vec<u8>,
        signature: Vec<u8>,
    },
    Discovery {
        peers: Vec<PeerInfo>,
    },
    Heartbeat {
        timestamp: u64,
        load: f32,
    },
    Data {
        id: String,
        payload: Vec<u8>,
        encrypted: bool,
        compression: Option<String>,
    },
    Request {
        id: String,
        method: String,
        params: serde_json::Value,
    },
    Response {
        id: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    Broadcast {
        topic: String,
        data: Vec<u8>,
        ttl: u8,
    },
    FileTransfer {
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
        checksum: String,
    },
    Sync {
        sync_type: SyncType,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncType {
    State,
    Configuration,
    Database,
    Files,
    Custom(String),
}

pub struct P2PNode {
    id: String,
    name: String,
    keypair: Ed25519KeyPair,
    keypair_arc: Arc<Ed25519KeyPair>,
    local_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    connections: Arc<RwLock<HashMap<String, Arc<Mutex<TcpStream>>>>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(P2PMessage, String) + Send + Sync>>>>,
    broadcast_tx: broadcast::Sender<P2PMessage>,
    is_running: Arc<RwLock<bool>>,
    config: P2PConfig,
    secure_p2p: Arc<SecureP2P>,
}

#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub port: u16,
    pub max_peers: usize,
    pub enable_discovery: bool,
    pub enable_encryption: bool,
    pub enable_compression: bool,
    pub discovery_interval: Duration,
    pub cleanup_interval: Duration,
    pub file_transfer_chunk_size: usize,
    pub trusted_peers: HashSet<String>,
    pub blocked_peers: HashSet<String>,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_P2P_PORT,
            max_peers: MAX_PEERS,
            enable_discovery: true,
            enable_encryption: true,
            enable_compression: true,
            discovery_interval: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(120),
            file_transfer_chunk_size: 64 * 1024, // 64KB chunks
            trusted_peers: HashSet::new(),
            blocked_peers: HashSet::new(),
        }
    }
}

impl P2PNode {
    pub fn new(name: String, config: P2PConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let rng = SystemRandom::new();
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
        let keypair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())?;
        let keypair_clone = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())?;
        
        let id = Self::generate_peer_id(&keypair);
        let local_addr = SocketAddr::from(([0, 0, 0, 0], config.port));
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        // Create PeerIdentity for SecureP2P
        let identity = PeerIdentity::generate(&name)?;
        let secure_p2p = Arc::new(SecureP2P::new(identity)?);
        
        Ok(Self {
            id,
            name: name.clone(),
            keypair,
            keypair_arc: Arc::new(keypair_clone),
            local_addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            is_running: Arc::new(RwLock::new(false)),
            config,
            secure_p2p,
        })
    }

    fn generate_peer_id(keypair: &Ed25519KeyPair) -> String {
        let public_key = keypair.public_key();
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_ref());
        let hash = hasher.finalize();
        hex::encode(&hash[..16])
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("P2P node is already running".into());
        }
        *is_running = true;
        drop(is_running);

        info!("Starting P2P node {} on {}", self.id, self.local_addr);

        // Start TCP listener
        let listener = TcpListener::bind(self.local_addr).await?;
        let self_clone = self.clone_internal();
        
        tokio::spawn(async move {
            Self::accept_connections(self_clone, listener).await;
        });

        // Start discovery service
        if self.config.enable_discovery {
            let self_clone = self.clone_internal();
            tokio::spawn(async move {
                Self::discovery_service(self_clone).await;
            });
        }

        // Start heartbeat service
        let self_clone = self.clone_internal();
        tokio::spawn(async move {
            Self::heartbeat_service(self_clone).await;
        });

        // Start cleanup service
        let self_clone = self.clone_internal();
        tokio::spawn(async move {
            Self::cleanup_service(self_clone).await;
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Err("P2P node is not running".into());
        }
        *is_running = false;
        
        // Close all connections
        let mut connections = self.connections.write().await;
        connections.clear();
        
        info!("P2P node {} stopped", self.id);
        Ok(())
    }

    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        if !*self.is_running.read().await {
            return Err("P2P node is not running".into());
        }

        let mut stream = TcpStream::connect(addr).await?;
        
        // Send handshake
        let handshake = self.create_handshake().await?;
        self.send_message(&mut stream, &handshake).await?;
        
        // Receive peer handshake
        let response = self.receive_message(&mut stream).await?;
        
        if let P2PMessage::Handshake { peer_info, nonce, signature } = response {
            // Verify signature
            if self.verify_handshake(&peer_info, &nonce, &signature)? {
                // Add peer
                let peer_id = peer_info.id.clone();
                self.add_peer(peer_info).await?;
                
                // Store connection
                let mut connections = self.connections.write().await;
                connections.insert(peer_id.clone(), Arc::new(Mutex::new(stream)));
                
                info!("Connected to peer {}", peer_id);
            } else {
                return Err("Invalid handshake signature".into());
            }
        } else {
            return Err("Invalid handshake response".into());
        }
        
        Ok(())
    }

    pub async fn send_to_peer(&self, peer_id: &str, message: P2PMessage) -> Result<(), Box<dyn std::error::Error>> {
        // Check if peer is blocked
        if self.secure_p2p.is_peer_blocked(peer_id).await {
            return Err("Cannot send to blocked peer".into());
        }
        
        let connections = self.connections.read().await;
        
        if let Some(connection) = connections.get(peer_id) {
            let mut stream = connection.lock().await;
            
            // Encrypt sensitive messages
            let message = if self.config.enable_encryption {
                match &message {
                    P2PMessage::Data { id, payload, .. } => {
                        // Encrypt the payload
                        let encrypted_payload = self.encrypt_data(payload)?;
                        P2PMessage::Data {
                            id: id.clone(),
                            payload: encrypted_payload,
                            encrypted: true,
                            compression: None,
                        }
                    }
                    _ => message,
                }
            } else {
                message
            };
            
            self.send_message(&mut *stream, &message).await?;
            Ok(())
        } else {
            Err(format!("Peer {} not connected", peer_id).into())
        }
    }

    pub async fn broadcast(&self, message: P2PMessage) -> Result<(), Box<dyn std::error::Error>> {
        let connections = self.connections.read().await;
        
        for (peer_id, connection) in connections.iter() {
            let mut stream = connection.lock().await;
            if let Err(e) = self.send_message(&mut *stream, &message).await {
                warn!("Failed to send to peer {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }

    async fn accept_connections(node: Arc<P2PNode>, listener: TcpListener) {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let node_clone = Arc::clone(&node);
                    tokio::spawn(async move {
                        if let Err(e) = node_clone.handle_connection(stream, addr).await {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, mut stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        // Receive handshake
        let message = self.receive_message(&mut stream).await?;
        
        if let P2PMessage::Handshake { peer_info, nonce, signature } = message {
            // Verify and add peer
            if self.verify_handshake(&peer_info, &nonce, &signature)? {
                let peer_id = peer_info.id.clone();
                
                // Check if peer is blocked
                if self.config.blocked_peers.contains(&peer_id) {
                    return Err("Peer is blocked".into());
                }
                
                // Check max peers
                if self.peers.read().await.len() >= self.config.max_peers {
                    return Err("Max peers reached".into());
                }
                
                self.add_peer(peer_info).await?;
                
                // Send our handshake
                let handshake = self.create_handshake().await?;
                self.send_message(&mut stream, &handshake).await?;
                
                // Store connection
                let mut connections = self.connections.write().await;
                connections.insert(peer_id.clone(), Arc::new(Mutex::new(stream)));
                
                info!("Accepted connection from peer {}", peer_id);
                
                // Start message handler for this peer
                let node_clone = Arc::new(self.clone_internal());
                let peer_id_clone = peer_id.clone();
                tokio::spawn(async move {
                    node_clone.handle_peer_messages(peer_id_clone).await;
                });
            }
        }
        
        Ok(())
    }

    async fn handle_peer_messages(&self, peer_id: String) {
        loop {
            let connections = self.connections.read().await;
            
            if let Some(connection) = connections.get(&peer_id) {
                let mut stream = connection.lock().await;
                
                match self.receive_message(&mut *stream).await {
                    Ok(message) => {
                        if let Err(e) = self.process_message(message, peer_id.clone()).await {
                            error!("Error processing message from {}: {}", peer_id, e);
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message from {}: {}", peer_id, e);
                        break;
                    }
                }
            } else {
                break;
            }
        }
        
        // Remove disconnected peer
        self.remove_peer(&peer_id).await;
    }

    async fn process_message(&self, message: P2PMessage, peer_id: String) -> Result<(), Box<dyn std::error::Error>> {
        match message {
            P2PMessage::Heartbeat { timestamp, load } => {
                self.update_peer_heartbeat(&peer_id, timestamp, load).await?;
            }
            P2PMessage::Discovery { peers } => {
                for peer_info in peers {
                    if peer_info.id != self.id && !self.peers.read().await.contains_key(&peer_info.id) {
                        // Try to connect to discovered peer
                        if let Err(e) = self.connect_to_peer(peer_info.address).await {
                            debug!("Failed to connect to discovered peer {}: {}", peer_info.id, e);
                        }
                    }
                }
            }
            P2PMessage::Data { id, payload, encrypted, compression } => {
                // Check trust level before processing
                let trust_level = self.secure_p2p.get_trust_level(&peer_id).await;
                if trust_level == TrustLevel::Untrusted {
                    warn!("Ignoring data from untrusted peer {}", peer_id);
                    return Ok(());
                }
                
                // Process data based on encryption and compression
                let data = if encrypted && self.config.enable_encryption {
                    // Decrypt data
                    self.decrypt_data(&payload)?
                } else {
                    payload
                };
                
                let data = if let Some(comp) = compression {
                    // Decompress data
                    self.decompress_data(&data, &comp)?
                } else {
                    data
                };
                
                // Handle data
                self.handle_data(id, data, peer_id).await?;
            }
            P2PMessage::Request { id, method, params } => {
                // Handle RPC request
                let response = self.handle_request(method, params).await;
                let response_msg = P2PMessage::Response {
                    id,
                    result: response.ok(),
                    error: response.err().map(|e| e.to_string()),
                };
                self.send_to_peer(&peer_id, response_msg).await?;
            }
            P2PMessage::Broadcast { topic, data, ttl } => {
                // Handle broadcast message
                if ttl > 0 {
                    // Rebroadcast with decreased TTL
                    let new_msg = P2PMessage::Broadcast {
                        topic,
                        data,
                        ttl: ttl - 1,
                    };
                    self.broadcast(new_msg).await?;
                }
            }
            _ => {
                // Call custom handlers
                let handlers = self.message_handlers.read().await;
                for (_, handler) in handlers.iter() {
                    handler(message.clone(), peer_id.clone());
                }
            }
        }
        
        Ok(())
    }

    async fn discovery_service(node: Arc<P2PNode>) {
        let socket = match UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to bind discovery socket: {}", e);
                return;
            }
        };
        
        let mut buf = [0u8; 4096]; // Increased buffer size for encrypted messages
        
        loop {
            tokio::select! {
                _ = tokio::time::sleep(node.config.discovery_interval) => {
                    // Broadcast signed discovery message
                    let peers = node.peers.read().await;
                    let peer_list: Vec<PeerInfo> = peers.values()
                        .filter(|p| {
                            // Only share trusted peers
                            node.secure_p2p.get_trust_level(&p.id).await != TrustLevel::Untrusted
                        })
                        .cloned()
                        .collect();
                    
                    let discovery_msg = P2PMessage::Discovery { peers: peer_list };
                    
                    if let Ok(mut data) = serde_json::to_vec(&discovery_msg) {
                        // Sign the discovery message
                        let signature = node.keypair.sign(&data);
                        data.extend_from_slice(signature.as_ref());
                        
                        // Use multicast instead of broadcast for better security
                        let multicast_addr = "239.255.0.1"; // Private multicast range
                        let _ = socket.send_to(&data, (multicast_addr, DISCOVERY_PORT)).await;
                    }
                }
                result = socket.recv_from(&mut buf) => {
                    if let Ok((len, addr)) = result {
                        // Verify signature before processing
                        if len > 64 { // Must have signature
                            let msg_len = len - 64;
                            let message_data = &buf[..msg_len];
                            let signature = &buf[msg_len..len];
                            
                            if let Ok(message) = serde_json::from_slice::<P2PMessage>(message_data) {
                                if let P2PMessage::Discovery { peers } = message {
                                    for peer_info in peers {
                                        if peer_info.id != node.id {
                                            // Verify peer signature before connecting
                                            let public_key = ring::signature::UnparsedPublicKey::new(
                                                &ring::signature::ED25519,
                                                &peer_info.public_key
                                            );
                                            
                                            if public_key.verify(message_data, signature).is_ok() {
                                                // Try to connect to verified peer
                                                let _ = node.connect_to_peer(peer_info.address).await;
                                            } else {
                                                warn!("Invalid signature from peer {}", peer_info.id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if !*node.is_running.read().await {
                break;
            }
        }
    }

    async fn heartbeat_service(node: Arc<P2PNode>) {
        loop {
            tokio::time::sleep(HEARTBEAT_INTERVAL).await;
            
            if !*node.is_running.read().await {
                break;
            }
            
            let heartbeat = P2PMessage::Heartbeat {
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                load: 0.0, // TODO: Calculate actual system load
            };
            
            let _ = node.broadcast(heartbeat).await;
        }
    }

    async fn cleanup_service(node: Arc<P2PNode>) {
        loop {
            tokio::time::sleep(node.config.cleanup_interval).await;
            
            if !*node.is_running.read().await {
                break;
            }
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let mut peers = node.peers.write().await;
            let mut connections = node.connections.write().await;
            
            let timeout_peers: Vec<String> = peers
                .iter()
                .filter(|(_, info)| now - info.last_seen > PEER_TIMEOUT.as_secs())
                .map(|(id, _)| id.clone())
                .collect();
            
            for peer_id in timeout_peers {
                peers.remove(&peer_id);
                connections.remove(&peer_id);
                info!("Removed inactive peer {}", peer_id);
            }
        }
    }

    async fn create_handshake(&self) -> Result<P2PMessage, Box<dyn std::error::Error>> {
        let nonce: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        let peer_info = PeerInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            address: self.local_addr,
            public_key: self.keypair.public_key().as_ref().to_vec(),
            capabilities: vec!["sync".to_string(), "file_transfer".to_string()],
            version: "1.0.0".to_string(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            latency_ms: None,
            trust_score: 1.0,
        };
        
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(&nonce);
        data_to_sign.extend_from_slice(self.id.as_bytes());
        
        let signature = self.keypair.sign(&data_to_sign);
        
        Ok(P2PMessage::Handshake {
            peer_info,
            nonce,
            signature: signature.as_ref().to_vec(),
        })
    }

    fn verify_handshake(&self, peer_info: &PeerInfo, nonce: &[u8], signature: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        // Verify signature using peer's public key
        use ring::signature::{UnparsedPublicKey, ED25519};
        
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(nonce);
        data_to_verify.extend_from_slice(peer_info.id.as_bytes());
        
        let public_key = UnparsedPublicKey::new(&ED25519, &peer_info.public_key);
        
        match public_key.verify(&data_to_verify, signature) {
            Ok(_) => {
                debug!("Signature verified for peer {}", peer_info.id);
                Ok(true)
            }
            Err(_) => {
                warn!("Invalid signature from peer {}", peer_info.id);
                Ok(false)
            }
        }
    }

    async fn add_peer(&self, peer_info: PeerInfo) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        peers.insert(peer_info.id.clone(), peer_info);
        Ok(())
    }

    async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        let mut connections = self.connections.write().await;
        
        peers.remove(peer_id);
        connections.remove(peer_id);
    }

    async fn update_peer_heartbeat(&self, peer_id: &str, timestamp: u64, load: f32) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.last_seen = timestamp;
        }
        
        Ok(())
    }

    async fn send_message(&self, stream: &mut TcpStream, message: &P2PMessage) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(message)?;
        
        // Check message size limit
        if data.len() > 10 * 1024 * 1024 { // 10MB limit
            return Err("Message too large".into());
        }
        
        let len = data.len() as u32;
        
        // Add message integrity check
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = hasher.finalize();
        
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&checksum).await?;
        stream.write_all(&data).await?;
        stream.flush().await?;
        
        Ok(())
    }

    async fn receive_message(&self, stream: &mut TcpStream) -> Result<P2PMessage, Box<dyn std::error::Error>> {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        // Check message size limit
        if len > 10 * 1024 * 1024 { // 10MB limit
            return Err("Message too large".into());
        }
        
        // Read checksum
        let mut checksum_buf = [0u8; 32];
        stream.read_exact(&mut checksum_buf).await?;
        
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;
        
        // Verify checksum
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let calculated_checksum = hasher.finalize();
        
        if calculated_checksum.as_slice() != checksum_buf {
            return Err("Message integrity check failed".into());
        }
        
        let message = serde_json::from_slice(&data)?;
        Ok(message)
    }

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};
        
        if data.len() < 12 + 16 { // nonce + tag minimum
            return Err("Invalid encrypted data".into());
        }
        
        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];
        
        // Derive key from keypair (simplified - should use proper key exchange)
        let mut key_bytes = [0u8; 32];
        let public_key = self.keypair.public_key();
        key_bytes[..public_key.as_ref().len().min(32)].copy_from_slice(&public_key.as_ref()[..public_key.as_ref().len().min(32)]);
        
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e).into())
    }
    
    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
        
        // Derive key from keypair (simplified - should use proper key exchange)
        let mut key_bytes = [0u8; 32];
        let public_key = self.keypair.public_key();
        key_bytes[..public_key.as_ref().len().min(32)].copy_from_slice(&public_key.as_ref()[..public_key.as_ref().len().min(32)]);
        
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        // Generate nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        // Encrypt data
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e).into())?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    fn decompress_data(&self, data: &[u8], compression: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match compression {
            "gzip" => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            _ => {
                warn!("Unknown compression type: {}", compression);
                Ok(data.to_vec())
            }
        }
    }
    
    fn compress_data(&self, data: &[u8], compression: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match compression {
            "gzip" => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            _ => {
                warn!("Unknown compression type: {}", compression);
                Ok(data.to_vec())
            }
        }
    }

    async fn handle_data(&self, id: String, data: Vec<u8>, peer_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement data handling
        info!("Received data {} from peer {}", id, peer_id);
        Ok(())
    }

    async fn handle_request(&self, method: String, params: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Implement RPC handling
        Ok(serde_json::json!({ "status": "ok" }))
    }

    fn clone_internal(&self) -> Arc<P2PNode> {
        // Safe clone using Arc - avoids unsafe pointer operations
        // Note: keypair is now wrapped in Arc for safe sharing
        Arc::new(P2PNode {
            id: self.id.clone(),
            name: self.name.clone(),
            keypair: Arc::clone(&self.keypair_arc),
            local_addr: self.local_addr,
            peers: Arc::clone(&self.peers),
            connections: Arc::clone(&self.connections),
            message_handlers: Arc::clone(&self.message_handlers),
            broadcast_tx: self.broadcast_tx.clone(),
            is_running: Arc::clone(&self.is_running),
            config: self.config.clone(),
            secure_p2p: Arc::clone(&self.secure_p2p),
        })
    }

    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub async fn is_connected(&self, peer_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(peer_id)
    }
}
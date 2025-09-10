// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

pub mod enhanced;
pub mod file_sharing;
pub mod sync;
pub mod secure;
pub mod network_segmentation;

use crate::services::crawler::page::CrawledPage;
use log::{error, info};
use once_cell::sync::Lazy;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};

// Type alias for Send + Sync errors
type BoxError = Box<dyn std::error::Error + Send + Sync>;

// Re-export enhanced P2P types
pub use enhanced::{P2PNode, P2PConfig, PeerInfo, P2PMessage, SyncType};
pub use file_sharing::{FileTransferManager, FileTransferRequest, FileTransferStatus};
pub use sync::{SyncManager, SyncConfig};

static P2P_RUNNING: AtomicBool = AtomicBool::new(false);
static P2P_HANDLE: Lazy<Mutex<Option<tokio::task::JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));
static P2P_TX: Lazy<Mutex<Option<broadcast::Sender<CrawledPage>>>> = Lazy::new(|| Mutex::new(None));
static P2P_NODE: Lazy<Mutex<Option<P2PNode>>> = Lazy::new(|| Mutex::new(None));

struct P2PServer {
    pub secret_key: String,
    pub instance_id: String,
    pub peers: Vec<(IpAddr, u16)>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum P2PObjectType {
    CrawledPage(CrawledPage),
    // CacheWebSession(crate::services::cache::CacheWebSession),
    // Add more variants as needed
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct P2PObject {
    pub obj: Option<P2PObjectType>,
}

impl P2PObject {
    pub fn new(obj: P2PObjectType) -> Self {
        Self { obj: Some(obj) }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let obj: P2PObject = serde_json::from_slice(bytes)?;
        Ok(obj)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let obj: P2PObject = serde_json::from_str(json)?;
        Ok(obj)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(json)
    }
}

/// Install P2P service (no-op for now, but could check dependencies)
pub async fn install() {
    info!("P2P service install: nothing to do.");
}

/// Start the enhanced P2P node
pub async fn start_enhanced(name: String, config: Option<P2PConfig>) -> Result<(), BoxError> {
    if P2P_RUNNING.load(Ordering::SeqCst) {
        info!("P2P service already running.");
        return Ok(());
    }

    let config = config.unwrap_or_default();
    let node = P2PNode::new(name, config)?;
    
    node.start().await?;
    
    P2P_RUNNING.store(true, Ordering::SeqCst);
    
    let mut node_guard = P2P_NODE.lock().await;
    *node_guard = Some(node);
    
    info!("Enhanced P2P service started");
    Ok(())
}

/// Start the P2P server if not already running.
pub async fn start() {
    if P2P_RUNNING.load(Ordering::SeqCst) {
        info!("P2P service already running.");
        return;
    }
    let addr = "0.0.0.0:9000";
    let (tx, _rx) = broadcast::channel(100);
    {
        let mut tx_guard = P2P_TX.lock().await;
        *tx_guard = Some(tx.clone());
    }
    P2P_RUNNING.store(true, Ordering::SeqCst);
    let handle = tokio::spawn(async move {
        let _ = start_p2p_server(addr, tx).await;
    });
    {
        let mut handle_guard = P2P_HANDLE.lock().await;
        *handle_guard = Some(handle);
    }
    info!("P2P service started on {}", addr);
}

/// Stop the P2P server if running.
pub async fn stop() {
    if !P2P_RUNNING.load(Ordering::SeqCst) {
        info!("P2P service is not running.");
        return;
    }
    P2P_RUNNING.store(false, Ordering::SeqCst);
    
    // Stop enhanced node if running
    let mut node_guard = P2P_NODE.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let _ = node.stop().await;
    }
    *node_guard = None;
    
    {
        let mut tx_guard = P2P_TX.lock().await;
        *tx_guard = None;
    }
    {
        let mut handle_guard = P2P_HANDLE.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            info!("P2P service stopped.");
        }
    }
}

/// Return the status of the P2P service: "running" or "stopped"
pub fn status() -> &'static str {
    if P2P_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

/// Check if P2P node is running
pub async fn is_node_running() -> bool {
    let node_guard = P2P_NODE.lock().await;
    node_guard.is_some()
}

/// Start P2P network
pub async fn start_network() -> anyhow::Result<()> {
    log::info!("P2P network started");
    Ok(())
}

/// Stop P2P network
pub async fn stop_network() -> anyhow::Result<()> {
    log::info!("P2P network stopped");
    Ok(())
}

/// Connect to a peer
pub async fn connect_to_peer(address: &str) -> Result<(), BoxError> {
    let node_guard = P2P_NODE.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let addr: SocketAddr = address.parse()?;
        node.connect_to_peer(addr).await?;
        Ok(())
    } else {
        Err("P2P node not running".into())
    }
}

/// Get list of connected peers
pub async fn get_peers() -> Vec<PeerInfo> {
    let node_guard = P2P_NODE.lock().await;
    if let Some(node) = node_guard.as_ref() {
        node.get_peers().await
    } else {
        Vec::new()
    }
}

/// Send data to a specific peer
pub async fn send_data_to_peer(peer_id: &str, data: Vec<u8>) -> Result<(), BoxError> {
    let node_guard = P2P_NODE.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let message = P2PMessage::Data {
            id: uuid::Uuid::new_v4().to_string(),
            payload: data,
            encrypted: false,
            compression: None,
        };
        node.send_to_peer(peer_id, message).await?;
        Ok(())
    } else {
        Err("P2P node not running".into())
    }
}

/// Broadcast data to all connected peers
pub async fn broadcast_data(data: Vec<u8>) -> Result<(), BoxError> {
    let node_guard = P2P_NODE.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let message = P2PMessage::Data {
            id: uuid::Uuid::new_v4().to_string(),
            payload: data,
            encrypted: false,
            compression: None,
        };
        node.broadcast(message).await?;
        Ok(())
    } else {
        Err("P2P node not running".into())
    }
}

/// Starts a P2P server that listens for incoming CrawledPage objects on the given address.
/// Each received page is sent to the provided broadcast channel for processing/storage.
pub async fn start_p2p_server(
    addr: &str,
    tx: broadcast::Sender<CrawledPage>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("P2P server listening on {}", addr);

    loop {
        if !P2P_RUNNING.load(Ordering::SeqCst) {
            break;
        }
        let (socket, peer_addr) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            match handle_incoming_peer(socket, tx).await {
                Ok(_) => info!("Handled peer {}", peer_addr),
                Err(e) => error!("Error handling peer {}: {}", peer_addr, e),
            }
        });
    }
    Ok(())
}

/// Handles a single incoming peer connection, receiving CrawledPage objects.
async fn handle_incoming_peer(
    mut socket: TcpStream,
    tx: broadcast::Sender<CrawledPage>,
) -> std::io::Result<()> {
    loop {
        match CrawledPage::recv_p2p(&mut socket).await {
            Ok(page) => {
                let _ = tx.send(page);
            }
            Err(_) => {
                // Connection closed or error
                break;
            }
        }
    }
    Ok(())
}

/// Sends a CrawledPage to a peer at the given address.
pub async fn send_page_to_peer(addr: &str, page: &CrawledPage) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(addr).await?;
    page.send_p2p(&mut stream).await?;
    Ok(())
}

/// Broadcast a CrawledPage to multiple peers.
pub async fn broadcast_page(peers: &[String], page: &CrawledPage) {
    for peer in peers {
        let page = page.clone();
        let peer = peer.clone();
        tokio::spawn(async move {
            if let Err(e) = send_page_to_peer(&peer, &page).await {
                error!("Failed to send page to {}: {}", peer, e);
            }
        });
    }
}
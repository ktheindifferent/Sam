use crate::sam::services::crawler::page::CrawledPage;
use log::{error, info};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};

static P2P_RUNNING: AtomicBool = AtomicBool::new(false);
static P2P_HANDLE: Lazy<Mutex<Option<tokio::task::JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));
static P2P_TX: Lazy<Mutex<Option<broadcast::Sender<CrawledPage>>>> = Lazy::new(|| Mutex::new(None));

/// Install P2P service (no-op for now, but could check dependencies)
pub async fn install() {
    info!("P2P service install: nothing to do.");
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

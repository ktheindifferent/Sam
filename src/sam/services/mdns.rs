use futures_util::{pin_mut, stream::StreamExt};
use libmdns::{Responder, Service};
use mdns::RecordKind;
use rand::{distributions::Alphanumeric, Rng};
use std::sync::{Arc};
use tokio::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use once_cell::sync::Lazy;

const SERVICE_NAME: &str = "_opensam._tcp.local";
const SERVICE_PORT: u16 = 5959;

// Removed MDNS_INSTANCE, only using *_HANDLE globals now
// Global handles for singleton instance and tasks
static DISCOVER_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static BROADCAST_HANDLE: Lazy<Arc<Mutex<Option<JoinHandle<()>>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));


/// Generates a random secret key for the instance.
pub fn generate_secret_key() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

#[derive(Debug, Clone)]
pub struct MDns {
    pub instance_id: String,
    pub secret_key: String,
}

impl MDns {
    pub fn new() -> Self {
        Self {
            instance_id: generate_secret_key(),
            secret_key: generate_secret_key(),
        }
    }

    pub async fn discover_with_output(&self, output_lines: Arc<Mutex<Vec<String>>>) -> Result<(), anyhow::Error> {
        log::debug!("[mDNS] Starting discovery for service: {}", SERVICE_NAME);
        let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
        pin_mut!(stream);

        while let Some(Ok(response)) = stream.next().await {
            log::debug!("[mDNS] Received mDNS response: {:?}", response);
            let ip_addr = response.ip_addr();
            let socket = response.socket_address();
            let txt = response.txt_records();
            let records = response.records();
            
            log::info!("[mDNS] Discovered IP address: {:?}", ip_addr);
            log::info!("[mDNS] Discovered socket address: {:?}", socket);
            {
                let mut lines = output_lines.lock().await;
                lines.push(format!("[mDNS] Discovered IP address: {:?}", ip_addr));
                lines.push(format!("[mDNS] Discovered socket address: {:?}", socket));
            }
            for record in records {
                log::debug!("[mDNS] Record: {:?}", record);
                if let RecordKind::A(addr) = &record.kind {
                    log::info!("[mDNS] Discovered service on addr: {}", addr);
                    {
                        let mut lines = output_lines.lock().await;
                        lines.push(format!("[mDNS] Discovered service on addr: {}", addr));
                    }
                }
                if let RecordKind::AAAA(addr) = &record.kind {
                    log::info!("[mDNS] Discovered service on addr: {}", addr);
                    {
                        let mut lines = output_lines.lock().await;
                        lines.push(format!("[mDNS] Discovered service on addr: {}", addr));
                    }
                }
            }
            for record in txt {
                log::debug!("[mDNS] TXT record: {:?}", record);
                log::info!("[mDNS] Discovered TXT record: {:?}", record);
                {
                    let mut lines = output_lines.lock().await;
                    lines.push(format!("[mDNS] Discovered TXT record: {:?}", record));
                }
            }

            {
                let mut lines = output_lines.lock().await;
                lines.push(format!("[mDNS] Got response: {:?}", response));
            }
        }
        log::debug!("[mDNS] Discovery loop ended for service: {}", SERVICE_NAME);
        Ok(())
    }

    pub async fn init(){
        let responder = libmdns::Responder::new().unwrap();
        let _svc = responder.register(
            "_opensam._tcp".to_owned(),
            "opensam".to_owned(),
            8001,
            &["path=/"],
        );
    }

    /// Broadcasts the mDNS service in a loop.
    pub async fn broadcast_loop(&self) {
        log::debug!("[mDNS] Entering broadcast loop for service: {}", SERVICE_NAME);
        let responder = libmdns::Responder::new().unwrap();

        // No, you generally do not need to loop here.
        // Registering the service once is sufficient; libmdns keeps it alive as long as the Responder and Service are alive.
        log::debug!("[mDNS] Registering service: {} on port {}", SERVICE_NAME, SERVICE_PORT);
        let _svc = responder.register(
            SERVICE_NAME.to_owned(),
            "_tcp.local".to_string(),
            SERVICE_PORT,
            &[
            "path=/",
            &format!("id={}", self.instance_id),
            &format!("secret={}", self.secret_key),
            ],
        );
        log::info!("[mDNS] Broadcast started, responder created");
        // Keep the task alive as long as needed
        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }
}

// stop_broadcast now just aborts the broadcast task
pub async fn stop_broadcast() {
    log::info!("[mDNS] Attempting to stop broadcast (aborting broadcast task)");
    if let Some(handle) = BROADCAST_HANDLE.lock().await.take() {
        handle.abort();
        log::info!("[mDNS] Broadcast task aborted");
    } else {
        log::info!("[mDNS] No broadcast task was active");
    }
}

// Start discovery loop (singleton)
pub async fn start_discovery(output_lines: Arc<Mutex<Vec<String>>>) {
    // Abort early if discovery is already running
    if DISCOVER_HANDLE.lock().await.as_ref().is_some() {
        log::info!("[mDNS] Discovery already running, aborting start_discovery");
        return;
    }
    let discover_handle = tokio::spawn({
        async move {
            let output_lines = output_lines.clone();
            let _ = MDns::new().discover_with_output(output_lines).await.unwrap_or_else(|e| {
                log::error!("[mDNS] Discovery error: {}", e);
            });
        }
    });
    *DISCOVER_HANDLE.lock().await = Some(discover_handle);
}

pub async fn stop_discovery() {
    if let Some(handle) = DISCOVER_HANDLE.lock().await.take() {
        handle.abort();
    }
}

pub async fn start_broadcast() {
    // Abort any existing broadcast task before starting a new one
    if let Some(handle) = BROADCAST_HANDLE.lock().await.take() {
        handle.abort();
    }
    let broadcast_handle = tokio::spawn(async move {
        let mdns = MDns::new();
        let _ = mdns.broadcast_loop().await;
    });
    *BROADCAST_HANDLE.lock().await = Some(broadcast_handle);
}

// stop_broadcast_and_task is now just an alias for stop_broadcast
pub async fn stop_broadcast_and_task() {
    stop_broadcast().await;
}

pub async fn mdns_status() -> (bool, bool) {
    let discover_running = DISCOVER_HANDLE.lock().await.as_ref().map(|h| !h.is_finished()).unwrap_or(false);
    let broadcast_running = BROADCAST_HANDLE.lock().await.as_ref().map(|h| !h.is_finished()).unwrap_or(false);
    (discover_running, broadcast_running)
}

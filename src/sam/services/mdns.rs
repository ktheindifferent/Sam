use futures_util::{pin_mut, stream::StreamExt};
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use rand::{distributions::Alphanumeric, Rng};
use std::sync::{Arc};
use tokio::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use once_cell::sync::Lazy;

const SERVICE_NAME: &str = "_opensam._tcp.local.";
const SERVICE_TYPE: &str = "_opensam._tcp";
const SERVICE_PORT: u16 = 5959;

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
        use mdns::{RecordKind, Error as MdnsError};
        log::info!("[mDNS] Starting discovery for service: {} (mdns crate)", SERVICE_NAME);
        let stream = mdns::discover::all("hive.".to_owned() + SERVICE_NAME, std::time::Duration::from_secs(15))?.listen();
        tokio::pin!(stream);
        while let Some(Ok(response)) = stream.next().await {
            let name = response.records().find_map(|r| {
                if let RecordKind::PTR(ref ptr) = r.kind {
                    Some(ptr.clone())
                } else {
                    None
                }
            }).unwrap_or_else(|| "<unknown>".to_string());

            let addrs = response.records().filter_map(|r| match r.kind {
                RecordKind::A(addr) => Some(addr.to_string()),
                RecordKind::AAAA(addr) => Some(addr.to_string()),
                RecordKind::SRV { priority, weight, port, ref target } => {
                    Some(format!("SRV: {}:{} (prio {}, weight {})", target, port, priority, weight))
                }
                RecordKind::MX { preference, ref exchange } => {
                    Some(format!("MX: {} (pref {})", exchange, preference))
                }
                RecordKind::TXT(ref txt) => {
                    Some(format!("TXT: {}", txt.join(", ")))
                }
                _ => None,
            }).collect::<Vec<_>>();

            log::info!("[mDNS] Discovered: {} at {:?}", name, addrs);
            let mut lines = output_lines.lock().await;
            lines.push(format!("[mDNS] Discovered: {} at {:?}", name, addrs));
        }
        log::info!("[mDNS] Discovery loop ended for service: {} (mdns crate)", SERVICE_NAME);
        Ok(())
    }

    /// Broadcasts the mDNS service in a loop.
    pub async fn broadcast_loop(&self) {
        log::info!("[mDNS] Starting broadcast for service: {} (mdns-sd)", SERVICE_TYPE);
        let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");
        let instance_name = "hive".to_string(); // TODO: Use a more meaningful instance name
        let host_ipv4: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        let ip_list: &[std::net::IpAddr] = &[host_ipv4];
        let txt_records: &[(&str, &str)] = &[("id", self.instance_id.as_str()), ("secret", self.secret_key.as_str())];
        let service_info = ServiceInfo::new(
            SERVICE_NAME, // must end in .local.
            &instance_name,
            "localhost.local.", // host name must end in .local.
            ip_list,
            SERVICE_PORT,
            txt_records,
        ).expect("Valid service info");
        mdns.register(service_info).expect("Failed to register mDNS service");
        log::info!("[mDNS] Broadcast registered for {}", instance_name);
        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }

    pub async fn init() -> Result<(), anyhow::Error> {
        println!("[mDNS] Initializing mDNS broadcast (mdns-sd)");
        let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");
        println!("[mDNS] mDNS daemon created successfully");
        let instance_id = generate_secret_key();
        println!("[mDNS] Generated instance ID: {}", instance_id);
        let instance_name = format!("opensam-{}", instance_id);
        println!("[mDNS] Instance name: {}", instance_name);
        let host_ipv4: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        println!("[mDNS] Host IPv4 address: {}", host_ipv4);
        let ip_list: &[std::net::IpAddr] = &[host_ipv4];
        println!("[mDNS] IP list: {:?}", ip_list);
        let txt_records: &[(&str, &str)] = &[("id", instance_id.as_str())];
        println!("[mDNS] TXT records: {:?}", txt_records);
        let service_info = ServiceInfo::new(
            SERVICE_NAME,
            &instance_name,
            "localhost", // host name
            ip_list,
            SERVICE_PORT,
            txt_records,
        ).expect("Valid service info");
        println!("[mDNS] Service info created successfully: {:?}", service_info);
        // mdns.register(service_info).expect("Failed to register mDNS service");
        if let Err(e) = mdns.register(service_info) {
            println!("[mDNS] Failed to register mDNS service: {}", e);
            return Err(anyhow::anyhow!(e));
        }
        println!("[mDNS] mDNS initialized and service registered: {}", instance_name);

        Ok(())
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

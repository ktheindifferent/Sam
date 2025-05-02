use libmdns::{Responder, Service};
use mdns::RecordKind;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use futures_util::{pin_mut, stream::StreamExt};
use rand::{distributions::Alphanumeric, Rng};

const SERVICE_NAME: &str = "_opensam2";
const SERVICE_PORT: u16 = 5353;

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
}

impl MDns {
    pub fn new() -> Self {
        Self {
            instance_id: generate_secret_key(),
        }
    }

    pub async fn discover_with_output(&self, output_lines: Arc<Mutex<Vec<String>>>) {
        let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15)).unwrap().listen();
        pin_mut!(stream);
        while let Some(Ok(response)) = stream.next().await {
            let addr = response.records().filter_map(|rec| match rec.kind {
                RecordKind::A(addr) => Some(std::net::IpAddr::V4(addr)),
                RecordKind::AAAA(addr) => Some(std::net::IpAddr::V6(addr)),
                _ => None,
            }).next();
            let mut lines = output_lines.lock().await;
            if let Some(addr) = addr {
                lines.push(format!("[mDNS] Found device at {}", addr));
            } else {
                lines.push("[mDNS] Device does not advertise address".to_string());
            }
        }
    }

    /// Broadcasts the mDNS service in a loop.
    pub async fn broadcast_loop(&self) {
        let responder = libmdns::Responder::new().unwrap();
        let _svc = responder.register(
            "_tcp".into(),
            SERVICE_NAME.to_string(),
            SERVICE_PORT,
            &[
                "path=/",
                &format!("id={}", self.instance_id),
            ],
        );
        println!("[mDNS] Broadcast started, responder created");
        {
            let mut global = BROADCAST_RESPONDER.lock().unwrap();
            *global = Some(responder);
            println!("[mDNS] Responder stored in global handle");
        }
    }
}

static BROADCAST_RESPONDER: once_cell::sync::Lazy<Arc<StdMutex<Option<libmdns::Responder>>>> = once_cell::sync::Lazy::new(|| Arc::new(StdMutex::new(None)));

pub fn stop_broadcast() {
    println!("[mDNS] Attempting to stop broadcast and drop responder");
    let mut global = BROADCAST_RESPONDER.lock().unwrap();
    let was_some = global.is_some();
    *global = None;
    if was_some {
        println!("[mDNS] Responder dropped, broadcast should stop");
    } else {
        println!("[mDNS] No responder was active");
    }
}

pub fn start(output_lines: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>) {
    let mut mdns = MDns::new();
    tokio::spawn(async move {
        mdns.discover_with_output(output_lines).await;
    });
}

pub fn stop() {
    // Implement stopping logic here
}

pub fn status() -> String {
    // Implement status logic here
    "mDNS service is running".to_string()
}

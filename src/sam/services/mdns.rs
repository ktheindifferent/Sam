use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use async_std::task;
use mdns::{RecordKind, Service};
use rand::{distributions::Alphanumeric, Rng};

const SERVICE_NAME: &str = "_myapp._udp.local";
const SERVICE_PORT: u16 = 5353;

/// Generates a random secret key for the instance.
pub fn generate_secret_key() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

/// Broadcasts the service with the given secret key.
pub async fn broadcast(secret_key: &str) -> mdns::Service {
    let hostname = format!("instance-{}", secret_key);
    let service = Service::new(
        SERVICE_NAME,
        SERVICE_PORT,
        &hostname,
        &[("secret", secret_key)],
    )
    .unwrap();
    service
}

/// Discovers instances broadcasting the same secret key.
/// Returns a vector of (hostname, ip) pairs.
pub async fn discover(secret_key: &str, timeout: Duration) -> Vec<(String, IpAddr)> {
    let mut discovered = Vec::new();
    let stream = mdns::discover::all(SERVICE_NAME, timeout)
        .unwrap()
        .listen();

    task::block_on(async {
        futures::pin_mut!(stream);
        while let Some(Ok(response)) = stream.next().await {
            for record in response.records() {
                if let RecordKind::TXT(txts) = record.kind {
                    for txt in txts {
                        if txt.starts_with(&format!("secret={}", secret_key)) {
                            if let Some(addr) = response.addr() {
                                discovered.push((response.name().to_string(), addr.ip()));
                            }
                        }
                    }
                }
            }
        }
    });

    discovered
}

// Example usage:
// let secret = generate_secret_key();
// let _svc = broadcast(&secret).await;
// let peers = discover(&secret, Duration::from_secs(5)).await;
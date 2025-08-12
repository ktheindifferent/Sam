use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use log::{info, error, debug, warn};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Client -> Server
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    Ping { timestamp: i64 },
    Command { id: String, command: String, args: serde_json::Value },
    
    // Server -> Client
    Pong { timestamp: i64 },
    ServiceStatus { service: String, status: ServiceStatus },
    SystemStats { stats: SystemStats },
    Activity { activity: ActivityItem },
    Alert { message: String, severity: AlertSeverity },
    CommandResponse { id: String, success: bool, data: serde_json::Value },
    Error { message: String, code: Option<i32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub state: String,
    pub message: Option<String>,
    pub progress: Option<u8>,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_percent: f32,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_percent: f32,
    pub network_speed: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub activity_type: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// WebSocket client connection
#[derive(Debug)]
struct WsClient {
    id: String,
    subscriptions: Vec<String>,
    created_at: DateTime<Utc>,
    last_ping: DateTime<Utc>,
}

/// WebSocket server manager
pub struct WsServer {
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    broadcast_tx: broadcast::Sender<WsMessage>,
    stats_tx: broadcast::Sender<SystemStats>,
}

impl WsServer {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        let (stats_tx, _) = broadcast::channel(128);
        
        WsServer {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            stats_tx,
        }
    }
    
    /// Start the WebSocket server
    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        info!("WebSocket server listening on {}", addr);
        
        // Start background tasks
        self.start_background_tasks();
        
        loop {
            let (stream, addr) = listener.accept().await?;
            info!("New WebSocket connection from {}", addr);
            
            let clients = self.clients.clone();
            let broadcast_tx = self.broadcast_tx.clone();
            let stats_tx = self.stats_tx.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, clients, broadcast_tx, stats_tx).await {
                    error!("Error handling WebSocket connection: {}", e);
                }
            });
        }
    }
    
    /// Start background tasks for periodic updates
    fn start_background_tasks(&self) {
        let broadcast_tx = self.broadcast_tx.clone();
        let stats_tx = self.stats_tx.clone();
        
        // System stats updater (every 5 seconds)
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                if let Ok(stats) = collect_system_stats().await {
                    let _ = stats_tx.send(stats.clone());
                    let _ = broadcast_tx.send(WsMessage::SystemStats { stats });
                }
            }
        });
        
        // Service status updater (every 10 seconds)
        let broadcast_tx = self.broadcast_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                if let Ok(statuses) = collect_service_statuses().await {
                    for (service, status) in statuses {
                        let _ = broadcast_tx.send(WsMessage::ServiceStatus { service, status });
                    }
                }
            }
        });
        
        // Client health checker (every 30 seconds)
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut clients_guard = clients.write().await;
                let now = Utc::now();
                
                // Remove inactive clients (no ping for 60 seconds)
                clients_guard.retain(|id, client| {
                    let inactive_duration = now.signed_duration_since(client.last_ping);
                    if inactive_duration.num_seconds() > 60 {
                        warn!("Removing inactive WebSocket client: {}", id);
                        false
                    } else {
                        true
                    }
                });
            }
        });
    }
    
    /// Broadcast a message to all subscribed clients
    pub async fn broadcast(&self, message: WsMessage) {
        let _ = self.broadcast_tx.send(message);
    }
    
    /// Send a message to a specific client
    pub async fn send_to_client(&self, client_id: &str, message: WsMessage) -> Result<(), String> {
        // This would require storing client senders, implementing if needed
        Ok(())
    }
    
    /// Get current client count
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
    
    /// Get client information
    pub async fn get_clients(&self) -> Vec<(String, Vec<String>)> {
        self.clients
            .read()
            .await
            .iter()
            .map(|(id, client)| (id.clone(), client.subscriptions.clone()))
            .collect()
    }
}

/// Handle individual WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    broadcast_tx: broadcast::Sender<WsMessage>,
    stats_tx: broadcast::Sender<SystemStats>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let client_id = Uuid::new_v4().to_string();
    let client = WsClient {
        id: client_id.clone(),
        subscriptions: vec!["default".to_string()],
        created_at: Utc::now(),
        last_ping: Utc::now(),
    };
    
    // Register client
    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id.clone(), client);
    }
    
    info!("WebSocket client {} connected", client_id);
    
    // Subscribe to broadcasts
    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut stats_rx = stats_tx.subscribe();
    
    // Send initial connection message
    let welcome_msg = WsMessage::Activity {
        activity: ActivityItem {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            message: format!("Connected to S.A.M. WebSocket server"),
            activity_type: "system".to_string(),
            metadata: None,
        },
    };
    
    let msg_json = serde_json::to_string(&welcome_msg)?;
    ws_sender.send(Message::Text(msg_json)).await?;
    
    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            handle_client_message(
                                &client_id,
                                ws_msg,
                                &clients,
                                &mut ws_sender
                            ).await?;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client {} disconnected", client_id);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        ws_sender.send(Message::Pong(data)).await?;
                        
                        // Update last ping time
                        let mut clients_guard = clients.write().await;
                        if let Some(client) = clients_guard.get_mut(&client_id) {
                            client.last_ping = Utc::now();
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            
            // Handle broadcast messages
            msg = broadcast_rx.recv() => {
                if let Ok(msg) = msg {
                    // Check if client is subscribed to this message type
                    if should_send_to_client(&client_id, &msg, &clients).await {
                        let msg_json = serde_json::to_string(&msg)?;
                        if ws_sender.send(Message::Text(msg_json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
            
            // Handle stats updates
            stats = stats_rx.recv() => {
                if let Ok(stats) = stats {
                    let msg = WsMessage::SystemStats { stats };
                    let msg_json = serde_json::to_string(&msg)?;
                    if ws_sender.send(Message::Text(msg_json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
    
    // Remove client on disconnect
    {
        let mut clients_guard = clients.write().await;
        clients_guard.remove(&client_id);
    }
    
    Ok(())
}

/// Handle messages from WebSocket clients
async fn handle_client_message(
    client_id: &str,
    message: WsMessage,
    clients: &Arc<RwLock<HashMap<String, WsClient>>>,
    ws_sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    match message {
        WsMessage::Subscribe { channels } => {
            let mut clients_guard = clients.write().await;
            if let Some(client) = clients_guard.get_mut(client_id) {
                for channel in channels {
                    if !client.subscriptions.contains(&channel) {
                        client.subscriptions.push(channel);
                    }
                }
            }
        }
        
        WsMessage::Unsubscribe { channels } => {
            let mut clients_guard = clients.write().await;
            if let Some(client) = clients_guard.get_mut(client_id) {
                client.subscriptions.retain(|c| !channels.contains(c));
            }
        }
        
        WsMessage::Ping { timestamp } => {
            let pong = WsMessage::Pong { timestamp };
            let msg_json = serde_json::to_string(&pong)?;
            ws_sender.send(Message::Text(msg_json)).await?;
            
            // Update last ping time
            let mut clients_guard = clients.write().await;
            if let Some(client) = clients_guard.get_mut(client_id) {
                client.last_ping = Utc::now();
            }
        }
        
        WsMessage::Command { id, command, args } => {
            // Process command and send response
            let response = process_command(&command, args).await;
            let msg = WsMessage::CommandResponse {
                id,
                success: response.is_ok(),
                data: response.unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
            };
            let msg_json = serde_json::to_string(&msg)?;
            ws_sender.send(Message::Text(msg_json)).await?;
        }
        
        _ => {
            debug!("Unexpected message type from client {}: {:?}", client_id, message);
        }
    }
    
    Ok(())
}

/// Check if a message should be sent to a specific client
async fn should_send_to_client(
    client_id: &str,
    message: &WsMessage,
    clients: &Arc<RwLock<HashMap<String, WsClient>>>,
) -> bool {
    let clients_guard = clients.read().await;
    
    if let Some(client) = clients_guard.get(client_id) {
        // Check if client is subscribed to relevant channels
        match message {
            WsMessage::ServiceStatus { service, .. } => {
                client.subscriptions.contains(&"services".to_string()) ||
                client.subscriptions.contains(&format!("service:{}", service))
            }
            WsMessage::SystemStats { .. } => {
                client.subscriptions.contains(&"stats".to_string())
            }
            WsMessage::Activity { .. } => {
                client.subscriptions.contains(&"activity".to_string())
            }
            WsMessage::Alert { .. } => {
                client.subscriptions.contains(&"alerts".to_string())
            }
            _ => true, // Send other messages to all clients
        }
    } else {
        false
    }
}

/// Process commands from WebSocket clients
async fn process_command(
    command: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        "get_stats" => {
            let stats = collect_system_stats().await?;
            Ok(serde_json::to_value(stats)?)
        }
        
        "get_services" => {
            let services = collect_service_statuses().await?;
            Ok(serde_json::to_value(services)?)
        }
        
        "restart_service" => {
            if let Some(service_name) = args.get("service").and_then(|s| s.as_str()) {
                // Implement service restart logic
                Ok(serde_json::json!({ "message": format!("Service {} restart initiated", service_name) }))
            } else {
                Err("Missing service name".into())
            }
        }
        
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}

/// Collect system statistics
async fn collect_system_stats() -> Result<SystemStats, Box<dyn std::error::Error>> {
    use sysinfo::{System, SystemExt, CpuExt};
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu = sys.global_cpu_info().cpu_usage();
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    let memory_percent = (memory_used as f32 / memory_total as f32) * 100.0;
    
    let mut disk_used = 0u64;
    let mut disk_total = 0u64;
    
    for disk in sys.disks() {
        disk_used += disk.total_space() - disk.available_space();
        disk_total += disk.total_space();
    }
    
    let disk_percent = if disk_total > 0 {
        (disk_used as f32 / disk_total as f32) * 100.0
    } else {
        0.0
    };
    
    Ok(SystemStats {
        cpu,
        memory_used,
        memory_total,
        memory_percent,
        disk_used,
        disk_total,
        disk_percent,
        network_speed: 0.0, // TODO: Implement network speed monitoring
        timestamp: Utc::now(),
    })
}

/// Collect service statuses
async fn collect_service_statuses() -> Result<HashMap<String, ServiceStatus>, Box<dyn std::error::Error>> {
    let mut statuses = HashMap::new();
    
    // Check Redis
    if crate::sam::services::redis::is_running().await {
        statuses.insert(
            "redis".to_string(),
            ServiceStatus {
                state: "healthy".to_string(),
                message: Some("Redis is running".to_string()),
                progress: None,
                last_check: Utc::now(),
            },
        );
    } else {
        statuses.insert(
            "redis".to_string(),
            ServiceStatus {
                state: "stopped".to_string(),
                message: Some("Redis is not running".to_string()),
                progress: None,
                last_check: Utc::now(),
            },
        );
    }
    
    // Check Docker
    if crate::sam::services::docker::is_running() {
        statuses.insert(
            "docker".to_string(),
            ServiceStatus {
                state: "healthy".to_string(),
                message: Some("Docker daemon is running".to_string()),
                progress: None,
                last_check: Utc::now(),
            },
        );
    } else {
        statuses.insert(
            "docker".to_string(),
            ServiceStatus {
                state: "stopped".to_string(),
                message: Some("Docker daemon is not running".to_string()),
                progress: None,
                last_check: Utc::now(),
            },
        );
    }
    
    // Add more service checks as needed
    
    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = WsMessage::Ping { timestamp: 1234567890 };
        let json = serde_json::to_string(&msg).expect("Should serialize test message");
        assert!(json.contains("ping"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_service_status() {
        let status = ServiceStatus {
            state: "healthy".to_string(),
            message: Some("Service is running".to_string()),
            progress: Some(75),
            last_check: Utc::now(),
        };
        
        let json = serde_json::to_string(&status).expect("Should serialize test status");
        assert!(json.contains("healthy"));
        assert!(json.contains("Service is running"));
        assert!(json.contains("75"));
    }

    #[tokio::test]
    async fn test_ws_server_creation() {
        let server = WsServer::new();
        assert_eq!(server.client_count().await, 0);
    }

    #[test]
    fn test_alert_severity() {
        let alert = WsMessage::Alert {
            message: "Test alert".to_string(),
            severity: AlertSeverity::Warning,
        };
        
        let json = serde_json::to_string(&alert).expect("Should serialize test alert");
        assert!(json.contains("alert"));
        assert!(json.contains("warning"));
    }
}
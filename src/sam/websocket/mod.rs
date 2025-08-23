use std::collections::HashMap;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use log::{info, error, debug, warn};

mod security;
use crate::sam::network_monitor::NetworkMonitor;
use security::{WebSocketLimits, WebSocketSecurityConfig, WsSecurityError, MessagePriority, SessionInfo};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Client -> Server
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    Ping { timestamp: i64 },
    Command { id: String, command: String, args: serde_json::Value },
    Authenticate { token: String },
    Heartbeat { timestamp: i64 },
    
    // Server -> Client
    Pong { timestamp: i64 },
    ServiceStatus { service: String, status: ServiceStatus },
    SystemStats { stats: SystemStats },
    NetworkStats { stats: NetworkStatsDetail },
    Activity { activity: ActivityItem },
    Alert { message: String, severity: AlertSeverity },
    CommandResponse { id: String, success: bool, data: serde_json::Value },
    Error { message: String, code: Option<i32> },
    AuthenticationRequired { reason: String },
    AuthenticationSuccess { permissions: Vec<String> },
    HeartbeatAck { timestamp: i64 },
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
pub struct NetworkStatsDetail {
    pub interfaces: HashMap<String, InterfaceStats>,
    pub total_download_mbps: f32,
    pub total_upload_mbps: f32,
    pub average_latency_ms: f32,
    pub packet_loss_percent: f32,
    pub connection_count: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceStats {
    pub name: String,
    pub download_mbps: f32,
    pub upload_mbps: f32,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
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
    session_info: SessionInfo,
    remote_addr: SocketAddr,
}

/// WebSocket server manager
pub struct WsServer {
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    broadcast_tx: broadcast::Sender<WsMessage>,
    stats_tx: broadcast::Sender<SystemStats>,
    security_limits: Arc<WebSocketLimits>,
    audit_tx: mpsc::UnboundedSender<AuditEvent>,
}

/// Audit event for security logging
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    timestamp: DateTime<Utc>,
    client_id: String,
    event_type: String,
    details: serde_json::Value,
    severity: AuditSeverity,
}

#[derive(Debug, Clone, Serialize)]
enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl WsServer {
    pub fn new() -> Self {
        Self::with_config(WebSocketSecurityConfig::default())
    }
    
    pub fn with_config(security_config: WebSocketSecurityConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        let (stats_tx, _) = broadcast::channel(128);
        let (audit_tx, mut audit_rx) = mpsc::unbounded_channel();
        
        // Spawn audit logger
        tokio::spawn(async move {
            while let Some(event) = audit_rx.recv().await {
                log_audit_event(event);
            }
        });
        
        WsServer {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            stats_tx,
            security_limits: Arc::new(WebSocketLimits::new(security_config)),
            audit_tx,
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
            let security_limits = self.security_limits.clone();
            let audit_tx = self.audit_tx.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(
                    stream, 
                    addr,
                    clients, 
                    broadcast_tx, 
                    stats_tx,
                    security_limits,
                    audit_tx
                ).await {
                    error!("Error handling WebSocket connection from {}: {}", addr, e);
                }
            });
        }
    }
    
    /// Start background tasks for periodic updates
    fn start_background_tasks(&self) {
        // Security cleanup task (every 30 seconds)
        let security_limits = self.security_limits.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                security_limits.cleanup().await;
            }
        });
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
    addr: SocketAddr,
    clients: Arc<RwLock<HashMap<String, WsClient>>>,
    broadcast_tx: broadcast::Sender<WsMessage>,
    stats_tx: broadcast::Sender<SystemStats>,
    security_limits: Arc<WebSocketLimits>,
    audit_tx: mpsc::UnboundedSender<AuditEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client_id = Uuid::new_v4().to_string();
    let ip = addr.ip();
    
    // Validate connection limits and create session
    let session_info = match security_limits.validate_connection(ip, client_id.clone()).await {
        Ok(session) => session,
        Err(e) => {
            error!("Connection validation failed for {}: {}", addr, e);
            audit_tx.send(AuditEvent {
                timestamp: Utc::now(),
                client_id: client_id.clone(),
                event_type: "connection_rejected".to_string(),
                details: serde_json::json!({ "reason": e.to_string(), "ip": addr.to_string() }),
                severity: AuditSeverity::Warning,
            })?;
            return Err(Box::new(e));
        }
    };
    
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let client = WsClient {
        id: client_id.clone(),
        subscriptions: vec!["default".to_string()],
        created_at: Utc::now(),
        last_ping: Utc::now(),
        session_info: session_info.clone(),
        remote_addr: addr,
    };
    
    // Register client
    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id.clone(), client);
    }
    
    info!("WebSocket client {} connected from {}", client_id, addr);
    
    // Log successful connection
    audit_tx.send(AuditEvent {
        timestamp: Utc::now(),
        client_id: client_id.clone(),
        event_type: "connection_established".to_string(),
        details: serde_json::json!({ "ip": addr.to_string() }),
        severity: AuditSeverity::Info,
    })?;
    
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
                        // Validate message
                        match security_limits.validate_message(&client_id, &text).await {
                            Ok(_) => {
                                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                    if let Err(e) = handle_client_message(
                                        &client_id,
                                        ws_msg,
                                        &clients,
                                        &mut ws_sender,
                                        &security_limits,
                                        &audit_tx
                                    ).await {
                                        error!("Error handling message from {}: {}", client_id, e);
                                        let error_msg = WsMessage::Error {
                                            message: "Failed to process message".to_string(),
                                            code: Some(400),
                                        };
                                        let _ = ws_sender.send(Message::Text(
                                            serde_json::to_string(&error_msg).unwrap()
                                        )).await;
                                    }
                                } else {
                                    warn!("Invalid message format from client {}", client_id);
                                    audit_tx.send(AuditEvent {
                                        timestamp: Utc::now(),
                                        client_id: client_id.clone(),
                                        event_type: "invalid_message_format".to_string(),
                                        details: serde_json::json!({ "message_preview": &text[..text.len().min(100)] }),
                                        severity: AuditSeverity::Warning,
                                    })?;
                                }
                            }
                            Err(e) => {
                                warn!("Message validation failed for client {}: {}", client_id, e);
                                audit_tx.send(AuditEvent {
                                    timestamp: Utc::now(),
                                    client_id: client_id.clone(),
                                    event_type: "message_validation_failed".to_string(),
                                    details: serde_json::json!({ "error": e.to_string() }),
                                    severity: AuditSeverity::Warning,
                                })?;
                                
                                let error_msg = WsMessage::Error {
                                    message: match e {
                                        WsSecurityError::RateLimitExceeded { .. } => "Rate limit exceeded. Please slow down.".to_string(),
                                        WsSecurityError::MessageTooLarge { .. } => "Message too large".to_string(),
                                        WsSecurityError::SessionExpired => "Session expired. Please reauthenticate.".to_string(),
                                        _ => "Message validation failed".to_string(),
                                    },
                                    code: Some(429),
                                };
                                let _ = ws_sender.send(Message::Text(
                                    serde_json::to_string(&error_msg).unwrap()
                                )).await;
                                
                                // For session expiry, close connection
                                if matches!(e, WsSecurityError::SessionExpired) {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client {} disconnected", client_id);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        ws_sender.send(Message::Pong(data)).await?;
                        
                        // Update last ping time and activity
                        let mut clients_guard = clients.write().await;
                        if let Some(client) = clients_guard.get_mut(&client_id) {
                            client.last_ping = Utc::now();
                        }
                        
                        // Update connection activity
                        security_limits.connection_tracker.update_activity(ip, &client_id).await;
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
    
    // Clean up security tracking
    security_limits.connection_tracker.remove_connection(ip, &client_id).await;
    security_limits.session_manager.remove_session(&client_id).await;
    security_limits.message_queue.clear_queue(&client_id).await;
    
    // Log disconnection
    let _ = audit_tx.send(AuditEvent {
        timestamp: Utc::now(),
        client_id: client_id.clone(),
        event_type: "connection_closed".to_string(),
        details: serde_json::json!({ "ip": addr.to_string() }),
        severity: AuditSeverity::Info,
    });
    
    info!("WebSocket client {} disconnected", client_id);
    
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
    security_limits: &Arc<WebSocketLimits>,
    audit_tx: &mpsc::UnboundedSender<AuditEvent>,
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
        
        WsMessage::Authenticate { token } => {
            match security_limits.session_manager.reauthenticate(client_id, &token).await {
                Ok(_) => {
                    let session = security_limits.session_manager.validate_session(client_id).await?;
                    
                    audit_tx.send(AuditEvent {
                        timestamp: Utc::now(),
                        client_id: client_id.to_string(),
                        event_type: "reauthentication_success".to_string(),
                        details: serde_json::json!({}),
                        severity: AuditSeverity::Info,
                    })?;
                    
                    let msg = WsMessage::AuthenticationSuccess {
                        permissions: session.permissions,
                    };
                    let msg_json = serde_json::to_string(&msg)?;
                    ws_sender.send(Message::Text(msg_json)).await?;
                }
                Err(e) => {
                    audit_tx.send(AuditEvent {
                        timestamp: Utc::now(),
                        client_id: client_id.to_string(),
                        event_type: "reauthentication_failed".to_string(),
                        details: serde_json::json!({ "error": e.to_string() }),
                        severity: AuditSeverity::Warning,
                    })?;
                    
                    let msg = WsMessage::Error {
                        message: "Authentication failed".to_string(),
                        code: Some(401),
                    };
                    let msg_json = serde_json::to_string(&msg)?;
                    ws_sender.send(Message::Text(msg_json)).await?;
                }
            }
        }
        
        WsMessage::Heartbeat { timestamp } => {
            // Update activity
            security_limits.session_manager.update_activity(client_id).await;
            security_limits.connection_tracker.update_activity(
                clients.read().await.get(client_id)
                    .map(|c| c.remote_addr.ip())
                    .unwrap_or_else(|| "127.0.0.1".parse().unwrap()),
                client_id
            ).await;
            
            let ack = WsMessage::HeartbeatAck { timestamp };
            let msg_json = serde_json::to_string(&ack)?;
            ws_sender.send(Message::Text(msg_json)).await?;
        }
        
        WsMessage::Command { id, command, args } => {
            // Validate command permissions
            let session = security_limits.session_manager.validate_session(client_id).await?;
            
            if let Err(e) = security_limits.message_validator.validate_command(&command, &session.permissions) {
                audit_tx.send(AuditEvent {
                    timestamp: Utc::now(),
                    client_id: client_id.to_string(),
                    event_type: "unauthorized_command".to_string(),
                    details: serde_json::json!({ "command": command, "error": e.to_string() }),
                    severity: AuditSeverity::Warning,
                })?;
                
                let msg = WsMessage::CommandResponse {
                    id,
                    success: false,
                    data: serde_json::json!({ "error": "Unauthorized" }),
                };
                let msg_json = serde_json::to_string(&msg)?;
                ws_sender.send(Message::Text(msg_json)).await?;
            } else {
                // Process command and send response
                let response = process_command(&command, args).await;
                
                // Log command execution
                audit_tx.send(AuditEvent {
                    timestamp: Utc::now(),
                    client_id: client_id.to_string(),
                    event_type: "command_executed".to_string(),
                    details: serde_json::json!({ 
                        "command": command, 
                        "success": response.is_ok() 
                    }),
                    severity: AuditSeverity::Info,
                })?;
                
                let msg = WsMessage::CommandResponse {
                    id,
                    success: response.is_ok(),
                    data: response.unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
                };
                let msg_json = serde_json::to_string(&msg)?;
                ws_sender.send(Message::Text(msg_json)).await?;
            }
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
        
        "get_network_stats" => {
            let stats = collect_network_stats().await?;
            Ok(serde_json::to_value(stats)?)
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
    
    // Get network speed from the network monitor
    let network_monitor = NetworkMonitor::new();
    let network_speed = match network_monitor.get_total_speed_mbps().await {
        Ok(speed) => speed as f32,
        Err(e) => {
            debug!("Failed to get network speed: {}", e);
            0.0
        }
    };
    
    Ok(SystemStats {
        cpu,
        memory_used,
        memory_total,
        memory_percent,
        disk_used,
        disk_total,
        disk_percent,
        network_speed,
        timestamp: Utc::now(),
    })
}

/// Collect detailed network statistics
async fn collect_network_stats() -> Result<NetworkStatsDetail, Box<dyn std::error::Error>> {
    use crate::sam::network_monitor::{NetworkMonitor, ConnectionStats};
    
    let monitor = NetworkMonitor::new();
    let metrics = monitor.get_metrics().await?;
    
    // Convert network speeds to interface stats
    let mut interfaces = HashMap::new();
    for (name, speed) in metrics.speeds {
        interfaces.insert(name.clone(), InterfaceStats {
            name: name.clone(),
            download_mbps: speed.download_speed_mbps as f32,
            upload_mbps: speed.upload_speed_mbps as f32,
            rx_packets: 0, // Would need to be tracked separately
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
        });
    }
    
    // Get connection statistics
    let connection_stats = ConnectionStats::gather().await.unwrap_or(ConnectionStats {
        tcp_established: 0,
        tcp_listen: 0,
        tcp_time_wait: 0,
        udp_connections: 0,
        total_connections: 0,
    });
    
    Ok(NetworkStatsDetail {
        interfaces,
        total_download_mbps: metrics.total_download_mbps as f32,
        total_upload_mbps: metrics.total_upload_mbps as f32,
        average_latency_ms: metrics.average_latency_ms as f32,
        packet_loss_percent: metrics.packet_loss_percent as f32,
        connection_count: connection_stats.total_connections,
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

/// Log audit events to file and console
fn log_audit_event(event: AuditEvent) {
    let log_message = format!(
        "[WEBSOCKET AUDIT] {} - Client: {} - Event: {} - Details: {}",
        event.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
        event.client_id,
        event.event_type,
        event.details
    );
    
    match event.severity {
        AuditSeverity::Info => info!("{}", log_message),
        AuditSeverity::Warning => warn!("{}", log_message),
        AuditSeverity::Error => error!("{}", log_message),
        AuditSeverity::Critical => error!("[CRITICAL] {}", log_message),
    }
    
    // Optionally write to a dedicated audit log file
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("websocket_audit.log")
    {
        use std::io::Write;
        let _ = writeln!(file, "{}: {}", event.timestamp.to_rfc3339(), serde_json::to_string(&event).unwrap_or_default());
    }
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
    
    #[tokio::test]
    async fn test_ws_server_with_custom_config() {
        let config = WebSocketSecurityConfig {
            max_message_size: 1024,
            max_messages_per_minute: 10,
            max_connections_per_ip: 2,
            ..Default::default()
        };
        let server = WsServer::with_config(config);
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
    
    #[test]
    fn test_authentication_messages() {
        let auth_msg = WsMessage::Authenticate {
            token: "test_token".to_string(),
        };
        let json = serde_json::to_string(&auth_msg).expect("Should serialize auth message");
        assert!(json.contains("authenticate"));
        assert!(json.contains("test_token"));
        
        let auth_success = WsMessage::AuthenticationSuccess {
            permissions: vec!["read".to_string(), "write".to_string()],
        };
        let json = serde_json::to_string(&auth_success).expect("Should serialize auth success");
        assert!(json.contains("authentication_success"));
        assert!(json.contains("read"));
    }
    
    #[test]
    fn test_heartbeat_messages() {
        let heartbeat = WsMessage::Heartbeat { timestamp: 1234567890 };
        let json = serde_json::to_string(&heartbeat).expect("Should serialize heartbeat");
        assert!(json.contains("heartbeat"));
        assert!(json.contains("1234567890"));
        
        let ack = WsMessage::HeartbeatAck { timestamp: 1234567890 };
        let json = serde_json::to_string(&ack).expect("Should serialize heartbeat ack");
        assert!(json.contains("heartbeat_ack"));
    }
    
    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent {
            timestamp: Utc::now(),
            client_id: "test_client".to_string(),
            event_type: "test_event".to_string(),
            details: serde_json::json!({ "test": "data" }),
            severity: AuditSeverity::Info,
        };
        
        let json = serde_json::to_string(&event).expect("Should serialize audit event");
        assert!(json.contains("test_client"));
        assert!(json.contains("test_event"));
    }
}

pub mod security;

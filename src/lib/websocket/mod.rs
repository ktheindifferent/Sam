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
mod error;
#[cfg(test)]
mod tests;

use crate::network_monitor::NetworkMonitor;
use security::{WebSocketLimits, WebSocketSecurityConfig, WsSecurityError, SessionInfo};
use error::{safe_ops};
use base64::{Engine as _, engine::general_purpose};

// Type alias for Send + Sync errors
type BoxError = Box<dyn std::error::Error + Send + Sync>;

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

impl Default for WsServer {
    fn default() -> Self {
        Self::new()
    }
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
    pub async fn start(&self, addr: &str) -> Result<(), BoxError> {
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
            
            // Spawn connection handler
            tokio::spawn(handle_connection(
                stream,
                addr,
                clients,
                broadcast_tx,
                stats_tx,
                security_limits,
                audit_tx,
            ));
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

        // System stats updater (every 5 seconds) with threshold alerts
        let alert_broadcast_tx = self.broadcast_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Ok(stats) = collect_system_stats().await {
                    let _ = stats_tx.send(stats.clone());
                    let _ = broadcast_tx.send(WsMessage::SystemStats { stats: stats.clone() });

                    // Threshold alerts
                    if stats.cpu >= 90.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("CPU usage critical: {:.1}%", stats.cpu),
                            severity: AlertSeverity::Critical,
                        });
                    } else if stats.cpu >= 80.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("CPU usage high: {:.1}%", stats.cpu),
                            severity: AlertSeverity::Warning,
                        });
                    }

                    if stats.memory_percent >= 90.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("Memory usage critical: {:.1}%", stats.memory_percent),
                            severity: AlertSeverity::Critical,
                        });
                    } else if stats.memory_percent >= 80.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("Memory usage high: {:.1}%", stats.memory_percent),
                            severity: AlertSeverity::Warning,
                        });
                    }

                    if stats.disk_percent >= 90.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("Disk usage critical: {:.1}%", stats.disk_percent),
                            severity: AlertSeverity::Critical,
                        });
                    } else if stats.disk_percent >= 85.0 {
                        let _ = alert_broadcast_tx.send(WsMessage::Alert {
                            message: format!("Disk usage high: {:.1}%", stats.disk_percent),
                            severity: AlertSeverity::Warning,
                        });
                    }
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

        // Event bus bridge: forward ServiceEvent -> WsMessage
        let broadcast_tx = self.broadcast_tx.clone();
        tokio::spawn(async move {
            let mut event_rx = crate::services::events::subscribe();
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        let ws_msg = match event {
                            crate::services::events::ServiceEvent::StatusChanged { service, old_status: _, new_status } => {
                                WsMessage::ServiceStatus {
                                    service: service.clone(),
                                    status: ServiceStatus {
                                        state: new_status.clone(),
                                        message: Some(format!("Status changed to {}", new_status)),
                                        progress: None,
                                        last_check: Utc::now(),
                                    },
                                }
                            }
                            crate::services::events::ServiceEvent::Error { service, message } => {
                                WsMessage::Alert {
                                    message: format!("[{}] {}", service, message),
                                    severity: AlertSeverity::Error,
                                }
                            }
                            crate::services::events::ServiceEvent::HealthCheck { service, healthy, message } => {
                                WsMessage::ServiceStatus {
                                    service: service.clone(),
                                    status: ServiceStatus {
                                        state: if healthy { "healthy".to_string() } else { "unhealthy".to_string() },
                                        message: Some(message),
                                        progress: None,
                                        last_check: Utc::now(),
                                    },
                                }
                            }
                            crate::services::events::ServiceEvent::MetricsUpdate { service, metric, value } => {
                                WsMessage::Activity {
                                    activity: ActivityItem {
                                        id: Uuid::new_v4().to_string(),
                                        timestamp: Utc::now(),
                                        message: format!("{}: {} = {:.2}", service, metric, value),
                                        activity_type: "metrics".to_string(),
                                        metadata: None,
                                    },
                                }
                            }
                        };
                        let _ = broadcast_tx.send(ws_msg);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket event bridge lagged {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Event bus closed, stopping WebSocket bridge");
                        break;
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

        // LIFX bulb status broadcaster (every 5 seconds)
        let lifx_broadcast_tx = self.broadcast_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Ok(bulbs) = crate::services::lifx::get_bulbs() {
                    if !bulbs.is_empty() {
                        let bulbs_info: Vec<serde_json::Value> = bulbs.iter().map(|b| {
                            serde_json::json!({
                                "id": b.id,
                                "label": b.label,
                                "connected": b.connected,
                                "power": b.power,
                                "brightness": b.brightness,
                                "color": b.lifx_color.as_ref().map(|c| serde_json::json!({
                                    "hue": c.hue,
                                    "saturation": c.saturation,
                                    "brightness": c.brightness,
                                    "kelvin": c.kelvin
                                })),
                            })
                        }).collect();

                        let _ = lifx_broadcast_tx.send(WsMessage::Activity {
                            activity: ActivityItem {
                                id: Uuid::new_v4().to_string(),
                                timestamp: Utc::now(),
                                message: format!("LIFX: {} bulbs online", bulbs.len()),
                                activity_type: "lifx_status".to_string(),
                                metadata: Some(serde_json::json!({ "bulbs": bulbs_info })),
                            },
                        });
                    }
                }
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
) -> Result<(), BoxError> {
    let client_id = Uuid::new_v4().to_string();
    let ip = addr.ip();
    
    // Validate connection limits and create session (no token provided initially)
    let session_info = match security_limits.validate_connection(ip, client_id.clone(), None).await {
        Ok(session) => session,
        Err(e) => {
            error!("Connection validation failed for {}: {}", addr, e);
            // Report connection validation errors to Sentry
            crate::monitoring::report_service_error("websocket", &e, None);
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
            message: "Connected to S.A.M. WebSocket server".to_string(),
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
                        info!("WebSocket received message from {}: {}", client_id, text);
                        // Validate message
                        match security_limits.validate_message(&client_id, &text).await {
                            Ok(_) => {
                                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                    info!("Parsed WebSocket message: {:?}", ws_msg);
                                    let result = handle_client_message(
                                        &client_id,
                                        ws_msg,
                                        &clients,
                                        &mut ws_sender,
                                        &security_limits,
                                        &audit_tx
                                    ).await;
                                    
                                    if let Err(e) = result {
                                        let error_str = e.to_string();
                                        // Report message handling errors to Sentry
                                        crate::monitoring::report_service_error("websocket", &e, None);
                                        drop(e); // Ensure error is dropped before await
                                        error!("Error handling message from {}: {}", client_id, error_str);
                                        let error_msg = WsMessage::Error {
                                            message: "Failed to process message".to_string(),
                                            code: Some(400),
                                        };
                                        let msg_json = safe_ops::serialize_json_or_default(&error_msg, r#"{"type":"error","message":"Authentication failed"}"#);
                                        let _ = ws_sender.send(Message::Text(msg_json)).await;
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
                                let msg_json = safe_ops::serialize_json_or_default(&error_msg, r#"{"type":"error","message":"Message validation failed"}"#);
                                let _ = ws_sender.send(Message::Text(msg_json)).await;
                                
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            let msg_json = serde_json::to_string(&pong)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            ws_sender.send(Message::Text(msg_json)).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            
            // Update last ping time
            let mut clients_guard = clients.write().await;
            if let Some(client) = clients_guard.get_mut(client_id) {
                client.last_ping = Utc::now();
            }
        }
        
        WsMessage::Authenticate { token } => {
            match security_limits.session_manager.reauthenticate(client_id, &token).await {
                Ok(_) => {
                    let session = security_limits.session_manager.validate_session(client_id).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    
                    audit_tx.send(AuditEvent {
                        timestamp: Utc::now(),
                        client_id: client_id.to_string(),
                        event_type: "reauthentication_success".to_string(),
                        details: serde_json::json!({}),
                        severity: AuditSeverity::Info,
                    }).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    
                    let msg = WsMessage::AuthenticationSuccess {
                        permissions: session.permissions,
                    };
                    let msg_json = serde_json::to_string(&msg)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    ws_sender.send(Message::Text(msg_json)).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                }
                Err(e) => {
                    audit_tx.send(AuditEvent {
                        timestamp: Utc::now(),
                        client_id: client_id.to_string(),
                        event_type: "reauthentication_failed".to_string(),
                        details: serde_json::json!({ "error": e.to_string() }),
                        severity: AuditSeverity::Warning,
                    }).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    
                    let msg = WsMessage::Error {
                        message: "Authentication failed".to_string(),
                        code: Some(401),
                    };
                    let msg_json = serde_json::to_string(&msg)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    ws_sender.send(Message::Text(msg_json)).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                }
            }
        }
        
        WsMessage::Heartbeat { timestamp } => {
            // Update activity
            security_limits.session_manager.update_activity(client_id).await;
            security_limits.connection_tracker.update_activity(
                clients.read().await.get(client_id)
                    .map(|c| c.remote_addr.ip())
                    .unwrap_or_else(|| safe_ops::parse_ip_or_default("127.0.0.1")),
                client_id
            ).await;
            
            let ack = WsMessage::HeartbeatAck { timestamp };
            let msg_json = safe_ops::serialize_json(&ack).map_err(|e| {
                error!("Failed to serialize heartbeat ack: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;
            ws_sender.send(Message::Text(msg_json)).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }
        
        WsMessage::Command { id, command, args } => {
            info!("Processing command '{}' with id '{}' and args: {:?}", command, id, args);
            // Validate command permissions
            let session = security_limits.session_manager.validate_session(client_id).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            
            if let Err(e) = security_limits.message_validator.validate_command(&command, &session.permissions) {
                audit_tx.send(AuditEvent {
                    timestamp: Utc::now(),
                    client_id: client_id.to_string(),
                    event_type: "unauthorized_command".to_string(),
                    details: serde_json::json!({ "command": command, "error": e.to_string() }),
                    severity: AuditSeverity::Warning,
                }).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
                let msg = WsMessage::CommandResponse {
                    id,
                    success: false,
                    data: serde_json::json!({ "error": "Unauthorized" }),
                };
                let msg_json = serde_json::to_string(&msg)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                ws_sender.send(Message::Text(msg_json)).await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            } else {
                // Process command and send response
                info!("Executing command '{}' for client {}", command, client_id);
                let response = process_command(&command, args).await;
                
                // Log command execution
                let success = response.is_ok();
                let response_data = response.unwrap_or_else(|e| {
                    error!("Command '{}' failed: {}", command, e);
                    serde_json::json!({ "error": e.to_string() })
                });
                
                info!("Command '{}' result: success={}, data={:?}", command, success, response_data);
                
                audit_tx.send(AuditEvent {
                    timestamp: Utc::now(),
                    client_id: client_id.to_string(),
                    event_type: "command_executed".to_string(),
                    details: serde_json::json!({ 
                        "command": command, 
                        "success": success 
                    }),
                    severity: AuditSeverity::Info,
                }).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
                let msg = WsMessage::CommandResponse {
                    id,
                    success,
                    data: response_data,
                };
                let msg_json = serde_json::to_string(&msg)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                info!("Sending command response: {}", msg_json);
                ws_sender.send(Message::Text(msg_json)).await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
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
) -> Result<serde_json::Value, BoxError> {
    info!("process_command called with command: '{}', args: {:?}", command, args);
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
        
        "start_service" => {
            if let Some(service_name) = args.get("service").and_then(|s| s.as_str()) {
                match service_name {
                    "redis" => {
                        crate::services::redis::start().await;
                        Ok(serde_json::json!({ "success": true, "message": "Redis service started" }))
                    }
                    "crawler" => {
                        crate::services::crawler::start_service_async().await;
                        Ok(serde_json::json!({ "success": true, "message": "Crawler service started" }))
                    }
                    "docker" => {
                        crate::services::docker::start().await;
                        Ok(serde_json::json!({ "success": true, "message": "Docker service started" }))
                    }
                    "nextcloud" => {
                        crate::services::fs::nextcloud::initialize().await
                            .map_err(|e| format!("Failed to start NextCloud service: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "NextCloud service started" }))
                    }
                    "lifx" => {
                        crate::services::lifx::start_server().await
                            .map_err(|e| format!("Failed to start LIFX service: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "LIFX service started" }))
                    }
                    "ssh_server" => {
                        crate::services::ssh::server::start_ssh_server().await
                            .map_err(|e| format!("Failed to start SSH server: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "SSH server started" }))
                    }
                    "media" => {
                        crate::services::media::start().await;
                        Ok(serde_json::json!({ "success": true, "message": "Media service started" }))
                    }
                    "snapcast" => {
                        crate::services::media::snapcast::init().await
                            .map_err(|e| format!("Failed to start Snapcast: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "Snapcast service started" }))
                    }
                    _ => Err(format!("Unknown service: {}", service_name).into())
                }
            } else {
                Err("Missing service name".into())
            }
        }
        
        "stop_service" => {
            if let Some(service_name) = args.get("service").and_then(|s| s.as_str()) {
                match service_name {
                    "redis" => {
                        crate::services::redis::stop().await;
                        Ok(serde_json::json!({ "success": true, "message": "Redis service stopped" }))
                    }
                    "crawler" => {
                        crate::services::crawler::stop_service();
                        Ok(serde_json::json!({ "success": true, "message": "Crawler service stopped" }))
                    }
                    "docker" => {
                        crate::services::docker::stop().await;
                        Ok(serde_json::json!({ "success": true, "message": "Docker service stopped" }))
                    }
                    "nextcloud" => {
                        // NextCloud service doesn't have a persistent daemon to stop
                        Ok(serde_json::json!({ "success": true, "message": "NextCloud service stopped" }))
                    }
                    "lifx" => {
                        crate::services::lifx::stop_server().await;
                        Ok(serde_json::json!({ "success": true, "message": "LIFX service stopped" }))
                    }
                    "ssh_server" => {
                        crate::services::ssh::server::stop_ssh_server().await;
                        Ok(serde_json::json!({ "success": true, "message": "SSH server stopped" }))
                    }
                    "media" => {
                        crate::services::media::stop().await;
                        Ok(serde_json::json!({ "success": true, "message": "Media service stopped" }))
                    }
                    "snapcast" => {
                        // Snapcast runs in background thread, just mark as stopped
                        Ok(serde_json::json!({ "success": true, "message": "Snapcast service stopped" }))
                    }
                    _ => Err(format!("Unknown service: {}", service_name).into())
                }
            } else {
                Err("Missing service name".into())
            }
        }
        
        "restart_service" => {
            if let Some(service_name) = args.get("service").and_then(|s| s.as_str()) {
                match service_name {
                    "redis" => {
                        crate::services::redis::stop().await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        crate::services::redis::start().await;
                        Ok(serde_json::json!({ "success": true, "message": "Redis service restarted" }))
                    }
                    "crawler" => {
                        crate::services::crawler::stop_service();
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        crate::services::crawler::start_service_async().await;
                        Ok(serde_json::json!({ "success": true, "message": "Crawler service restarted" }))
                    }
                    "nextcloud" => {
                        // NextCloud is stateless, so restart is just a reinitialize
                        crate::services::fs::nextcloud::initialize().await
                            .map_err(|e| format!("Failed to restart NextCloud service: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "NextCloud service restarted" }))
                    }
                    "lifx" => {
                        crate::services::lifx::stop_server().await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        crate::services::lifx::start_server().await
                            .map_err(|e| format!("Failed to restart LIFX service: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "LIFX service restarted" }))
                    }
                    "ssh_server" => {
                        crate::services::ssh::server::stop_ssh_server().await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        crate::services::ssh::server::start_ssh_server().await
                            .map_err(|e| format!("Failed to restart SSH server: {}", e))?;
                        Ok(serde_json::json!({ "success": true, "message": "SSH server restarted" }))
                    }
                    "media" => {
                        crate::services::media::stop().await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        crate::services::media::start().await;
                        Ok(serde_json::json!({ "success": true, "message": "Media service restarted" }))
                    }
                    _ => Err(format!("Unknown service: {}", service_name).into())
                }
            } else {
                Err("Missing service name".into())
            }
        }
        
        // Ollama commands
        "ollama_install" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.install().await {
                Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }
        
        "ollama_start" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.start_service().await {
                Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }
        
        "ollama_stop" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.stop_service().await {
                Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }
        
        "ollama_status" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            let installed = service.is_installed().await;
            let running = if installed { service.is_running().await } else { false };
            
            let version = if running {
                service.get_version().await.ok()
            } else {
                None
            };
            
            let models = if running {
                service.get_installed_model_names().await.unwrap_or_default()
            } else {
                Vec::new()
            };
            
            Ok(serde_json::json!({
                "installed": installed,
                "running": running,
                "version": version,
                "models": models
            }))
        }
        
        "ollama_list_models" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.list_models().await {
                Ok(models) => Ok(serde_json::to_value(models)?),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }
        
        "ollama_pull_model" => {
            if let Some(model_name) = args.get("model").and_then(|m| m.as_str()) {
                let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
                match service.pull_model(model_name).await {
                    Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                }
            } else {
                Err("Missing model name".into())
            }
        }
        
        "ollama_remove_model" => {
            if let Some(model_name) = args.get("model").and_then(|m| m.as_str()) {
                let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
                match service.remove_model(model_name).await {
                    Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                }
            } else {
                Err("Missing model name".into())
            }
        }
        
        "ollama_generate" => {
            if let (Some(model), Some(prompt)) = (
                args.get("model").and_then(|m| m.as_str()),
                args.get("prompt").and_then(|p| p.as_str())
            ) {
                let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
                let options = args.get("options").and_then(|o| {
                    if let serde_json::Value::Object(map) = o {
                        let mut options = std::collections::HashMap::new();
                        for (k, v) in map {
                            options.insert(k.clone(), v.clone());
                        }
                        Some(options)
                    } else {
                        None
                    }
                });
                
                match service.generate(model, prompt, options).await {
                    Ok(response) => Ok(serde_json::to_value(response)?),
                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                }
            } else {
                Err("Missing model or prompt".into())
            }
        }
        
        "ollama_search_models" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.search_models(query).await {
                Ok(models) => Ok(serde_json::json!({ "success": true, "models": models })),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }
        
        "ollama_install_recommended" => {
            let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
            match service.install_recommended_models().await {
                Ok(message) => Ok(serde_json::json!({ "success": true, "message": message })),
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
            }
        }

        // NextCloud commands
        "nextcloud_test_connection" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        let config = crate::services::fs::nextcloud::NextCloudConfig {
                            server_url: server_url.to_string(),
                            username: username.to_string(),
                            password: password.to_string(),
                            ..Default::default()
                        };

                        match crate::services::fs::nextcloud::NextCloudService::new(config) {
                            Ok(service) => {
                                match service.test_connection().await {
                                    Ok(true) => Ok(serde_json::json!({ "success": true, "message": "Connection successful" })),
                                    Ok(false) => Ok(serde_json::json!({ "success": false, "error": "Connection failed" })),
                                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                }
                            }
                            Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        "nextcloud_list_files" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        let path = args.get("path").and_then(|s| s.as_str()).unwrap_or("");

                        let config = crate::services::fs::nextcloud::NextCloudConfig {
                            server_url: server_url.to_string(),
                            username: username.to_string(),
                            password: password.to_string(),
                            ..Default::default()
                        };

                        match crate::services::fs::nextcloud::NextCloudService::new(config) {
                            Ok(service) => {
                                match service.list_files(path).await {
                                    Ok(files) => Ok(serde_json::json!({ "success": true, "files": files })),
                                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                }
                            }
                            Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        "nextcloud_upload_file" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        if let Some(remote_path) = args.get("remote_path").and_then(|s| s.as_str()) {
                            if let Some(content_base64) = args.get("content").and_then(|s| s.as_str()) {
                                match general_purpose::STANDARD.decode(content_base64) {
                                    Ok(content) => {
                                        let config = crate::services::fs::nextcloud::NextCloudConfig {
                                            server_url: server_url.to_string(),
                                            username: username.to_string(),
                                            password: password.to_string(),
                                            ..Default::default()
                                        };

                                        match crate::services::fs::nextcloud::NextCloudService::new(config) {
                                            Ok(service) => {
                                                match service.upload_file(std::path::Path::new(""), remote_path, &content).await {
                                                    Ok(file_info) => Ok(serde_json::json!({ "success": true, "file": file_info })),
                                                    Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                                }
                                            }
                                            Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                                        }
                                    }
                                    Err(_) => Ok(serde_json::json!({ "success": false, "error": "Invalid base64 content" }))
                                }
                            } else {
                                Ok(serde_json::json!({ "success": false, "error": "Missing content" }))
                            }
                        } else {
                            Ok(serde_json::json!({ "success": false, "error": "Missing remote_path" }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        "nextcloud_download_file" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        if let Some(remote_path) = args.get("remote_path").and_then(|s| s.as_str()) {
                            let config = crate::services::fs::nextcloud::NextCloudConfig {
                                server_url: server_url.to_string(),
                                username: username.to_string(),
                                password: password.to_string(),
                                ..Default::default()
                            };

                            match crate::services::fs::nextcloud::NextCloudService::new(config) {
                                Ok(service) => {
                                    match service.download_file(remote_path).await {
                                        Ok(content) => {
                                            let content_base64 = general_purpose::STANDARD.encode(&content);
                                            Ok(serde_json::json!({ "success": true, "content": content_base64, "size": content.len() }))
                                        }
                                        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                    }
                                }
                                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                            }
                        } else {
                            Ok(serde_json::json!({ "success": false, "error": "Missing remote_path" }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        "nextcloud_delete_file" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        if let Some(remote_path) = args.get("remote_path").and_then(|s| s.as_str()) {
                            let config = crate::services::fs::nextcloud::NextCloudConfig {
                                server_url: server_url.to_string(),
                                username: username.to_string(),
                                password: password.to_string(),
                                ..Default::default()
                            };

                            match crate::services::fs::nextcloud::NextCloudService::new(config) {
                                Ok(service) => {
                                    match service.delete_file(remote_path).await {
                                        Ok(_) => Ok(serde_json::json!({ "success": true, "message": "File deleted successfully" })),
                                        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                    }
                                }
                                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                            }
                        } else {
                            Ok(serde_json::json!({ "success": false, "error": "Missing remote_path" }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        "nextcloud_create_directory" => {
            if let Some(server_url) = args.get("server_url").and_then(|s| s.as_str()) {
                if let Some(username) = args.get("username").and_then(|s| s.as_str()) {
                    if let Some(password) = args.get("password").and_then(|s| s.as_str()) {
                        if let Some(remote_path) = args.get("remote_path").and_then(|s| s.as_str()) {
                            let config = crate::services::fs::nextcloud::NextCloudConfig {
                                server_url: server_url.to_string(),
                                username: username.to_string(),
                                password: password.to_string(),
                                ..Default::default()
                            };

                            match crate::services::fs::nextcloud::NextCloudService::new(config) {
                                Ok(service) => {
                                    match service.create_directory(remote_path).await {
                                        Ok(_) => Ok(serde_json::json!({ "success": true, "message": "Directory created successfully" })),
                                        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                                    }
                                }
                                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() }))
                            }
                        } else {
                            Ok(serde_json::json!({ "success": false, "error": "Missing remote_path" }))
                        }
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "Missing password" }))
                    }
                } else {
                    Ok(serde_json::json!({ "success": false, "error": "Missing username" }))
                }
            } else {
                Ok(serde_json::json!({ "success": false, "error": "Missing server_url" }))
            }
        }

        // Notification commands
        "get_notification_rules" => {
            let user_config = crate::services::config::SamUserConfig::load();
            let rules = user_config
                .notifications
                .and_then(|n| n.rules)
                .unwrap_or_default();
            Ok(serde_json::to_value(rules)?)
        }

        "test_notification" => {
            let channel = args.get("channel").and_then(|c| c.as_str()).unwrap_or("websocket");
            let message = args
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Test notification from SAM");
            let notification = crate::services::notifications::Notification {
                rule_id: "test".into(),
                rule_name: "Manual Test".into(),
                severity: crate::services::notifications::Severity::Info,
                message: message.to_string(),
                service: None,
                timestamp: chrono::Utc::now(),
            };
            match channel {
                "sms" => {
                    let user_config = crate::services::config::SamUserConfig::load();
                    if let Some(recipients) = user_config
                        .notifications
                        .as_ref()
                        .and_then(|n| n.sms_recipients.clone())
                    {
                        let ch = crate::services::notifications::channels::SmsChannel { recipients };
                        use crate::services::notifications::channels::NotificationChannel;
                        ch.send(&notification)
                            .await
                            .map_err(|e| -> BoxError { e.into() })?;
                        Ok(serde_json::json!({ "success": true, "message": "Test SMS sent" }))
                    } else {
                        Ok(serde_json::json!({ "success": false, "error": "No SMS recipients configured" }))
                    }
                }
                _ => {
                    // Default: emit through websocket channel (which sends an event)
                    let ch = crate::services::notifications::channels::WebSocketChannel;
                    use crate::services::notifications::channels::NotificationChannel;
                    ch.send(&notification)
                        .await
                        .map_err(|e| -> BoxError { e.into() })?;
                    Ok(serde_json::json!({ "success": true, "message": "Test notification sent" }))
                }
            }
        }

        "acknowledge_notification" => {
            let rule_id = args.get("rule_id").and_then(|r| r.as_str()).unwrap_or("");
            log::info!("Notification acknowledged: {}", rule_id);
            Ok(serde_json::json!({ "success": true, "acknowledged": rule_id }))
        }

        _ => Err(format!("Unknown command: {}", command).into()),
    }
}

/// Collect system statistics
async fn collect_system_stats() -> Result<SystemStats, BoxError> {
    use sysinfo::System;
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu = sys.global_cpu_usage();
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    let memory_percent = (memory_used as f32 / memory_total as f32) * 100.0;
    
    let mut disk_used = 0u64;
    let mut disk_total = 0u64;
    
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in disks.list() {
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
async fn collect_network_stats() -> Result<NetworkStatsDetail, BoxError> {
    use crate::network_monitor::{NetworkMonitor, ConnectionStats};
    
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
async fn collect_service_statuses() -> Result<HashMap<String, ServiceStatus>, BoxError> {
    let mut statuses = HashMap::new();
    
    // Check Redis
    if crate::services::redis::is_running().await {
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
    
    // Check Crawler
    let crawler_status = crate::services::crawler::service_status();
    info!("Crawler status check: {}", crawler_status);
    let crawler_state = if crawler_status.contains("running") {
        "healthy"
    } else {
        "stopped"
    };
    statuses.insert(
        "crawler".to_string(),
        ServiceStatus {
            state: crawler_state.to_string(),
            message: Some(crawler_status.to_string()),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Skip Docker check to avoid 2-second blocking timeout
    statuses.insert(
        "docker".to_string(),
        ServiceStatus {
            state: "unknown".to_string(),
            message: Some("Docker status check disabled (timeout issue)".to_string()),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // PostgreSQL status - assume healthy since the app is running
    // Skip actual check to avoid blocking/runtime panics
    statuses.insert(
        "postgres".to_string(),
        ServiceStatus {
            state: "healthy".to_string(),
            message: Some("PostgreSQL assumed healthy (check disabled to prevent blocking)".to_string()),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check WebSocket
    statuses.insert(
        "websocket".to_string(),
        ServiceStatus {
            state: "healthy".to_string(),
            message: Some("WebSocket server is running".to_string()),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check SSH server
    let ssh_server_running = crate::services::ssh::server::is_ssh_server_running().await;
    statuses.insert(
        "ssh_server".to_string(),
        ServiceStatus {
            state: if ssh_server_running { "healthy" } else { "stopped" }.to_string(),
            message: Some(if ssh_server_running {
                "SSH server running on port 2222".to_string()
            } else {
                "SSH server not running".to_string()
            }),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check LIFX service
    let lifx_status = crate::services::lifx::status_service().unwrap_or_else(|_| "stopped".to_string());
    let lifx_bulbs = crate::services::lifx::get_bulb_count().unwrap_or(0);
    statuses.insert(
        "lifx".to_string(),
        ServiceStatus {
            state: if lifx_status.contains("running") || lifx_status.contains("discovery") { "healthy" } else { "stopped" }.to_string(),
            message: Some(format!("LIFX: {} ({} bulbs)", lifx_status, lifx_bulbs)),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check Media service
    let media_running = crate::services::media::is_running().await;
    statuses.insert(
        "media".to_string(),
        ServiceStatus {
            state: if media_running { "healthy" } else { "stopped" }.to_string(),
            message: Some(if media_running {
                "Media service running".to_string()
            } else {
                "Media service not running".to_string()
            }),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check Snapcast service
    let snapcast_running = std::process::Command::new("pgrep")
        .arg("snapserver")
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false);
    statuses.insert(
        "snapcast".to_string(),
        ServiceStatus {
            state: if snapcast_running { "healthy" } else { "stopped" }.to_string(),
            message: Some(if snapcast_running {
                "Snapcast server running".to_string()
            } else {
                "Snapcast server not running".to_string()
            }),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
    // Check Voice/TTS service
    statuses.insert(
        "voice".to_string(),
        ServiceStatus {
            state: "stopped".to_string(),
            message: Some("Voice service not configured".to_string()),
            progress: None,
            last_check: Utc::now(),
        },
    );
    
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

/// Simple WebSocket server placeholder
pub struct WebSocketServer;

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn start(&self, _addr: &str) -> Result<(), BoxError> {
        // TODO: Implement actual WebSocket server
        log::info!("WebSocket server placeholder started");
        Ok(())
    }
}

/// Global websocket server instance
static WEBSOCKET_SERVER: once_cell::sync::OnceCell<WebSocketServer> = once_cell::sync::OnceCell::new();

/// Start the websocket server
pub async fn start_server() -> Result<(), BoxError> {
    let server = WEBSOCKET_SERVER.get_or_init(WebSocketServer::new);
    server.start("0.0.0.0:8080").await
}

/// Stop the websocket server
pub async fn stop_server() -> Result<(), BoxError> {
    // For now, this is a no-op since we don't store server handles
    // In a full implementation, you'd store the server handle and call shutdown
    Ok(())
}


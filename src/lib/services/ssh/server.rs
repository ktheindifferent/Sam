use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::services::orchestrator::ServiceStatus;
use std::env;

// Default SSH server port (can be overridden with SSH_SERVER_PORT env var)
const DEFAULT_SSH_SERVER_PORT: u16 = 2222;

/// Get the SSH server port from environment or use default
fn get_ssh_server_port() -> u16 {
    env::var("SSH_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_SSH_SERVER_PORT)
}

/// Get SSH username from environment or use default
fn get_ssh_username() -> String {
    env::var("SSH_USERNAME").unwrap_or_else(|_| "sam".to_string())
}

/// Get SSH password from environment or use default
fn get_ssh_password() -> String {
    env::var("SSH_PASSWORD").unwrap_or_else(|_| "sam".to_string())
}

/// Remote Access Server that supports SSH tunneling
pub struct RemoteAccessServer {
    sessions: Arc<RwLock<HashMap<usize, SessionData>>>,
    session_counter: Arc<Mutex<usize>>,
}

/// Session data
struct SessionData {
    session_id: usize,
    username: String,
    authenticated: bool,
}

/// TUI handle placeholder for future integration
pub struct TuiHandle;

impl TuiHandle {
    pub fn new() -> Self {
        TuiHandle
    }
}

impl RemoteAccessServer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(RemoteAccessServer {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_counter: Arc::new(Mutex::new(0)),
        })
    }

    /// Start the server with SSH tunneling support  
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = get_ssh_server_port();
        info!("Starting SAM Remote Access Server on port {}", port);
        info!("SSH clients can connect using: ssh sam@hostname -p {} 'nc localhost {}'", port, port);
        
        let socket = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        
        loop {
            let (stream, addr) = socket.accept().await?;
            debug!("New connection from: {}", addr);
            
            let sessions = self.sessions.clone();
            let session_counter = self.session_counter.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, sessions, session_counter).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }
}

/// Handle individual connection
async fn handle_connection(
    mut stream: TcpStream,
    sessions: Arc<RwLock<HashMap<usize, SessionData>>>,
    session_counter: Arc<Mutex<usize>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create session
    let mut counter = session_counter.lock().await;
    *counter += 1;
    let session_id = *counter;
    drop(counter);

    // Send welcome message
    let welcome = format!(
        "Welcome to SAM (Smart Artificial Mind) Remote Access\r\n\
         \r\n\
         For SSH access, use: ssh sam@hostname -p {} 'nc localhost {}'\r\n\
         \r\n\
         Login: ",
        get_ssh_server_port(), get_ssh_server_port()
    );
    stream.write_all(welcome.as_bytes()).await?;
    
    let mut buffer = [0; 1024];
    let mut authenticated = false;
    let mut username = String::new();
    
    // Authentication loop
    while !authenticated {
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            return Ok(());
        }
        
        let input = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        
        if username.is_empty() {
            username = input;
            stream.write_all(b"Password: ").await?;
        } else {
            let expected_username = get_ssh_username();
            let expected_password = get_ssh_password();
            
            if username == expected_username && input == expected_password {
                authenticated = true;
                let welcome = format!(
                    "\r\nAuthentication successful!\r\n\
                     Welcome to SAM Remote Access Interface\r\n\
                     Type 'help' for available commands.\r\n\r\n"
                );
                stream.write_all(welcome.as_bytes()).await?;
                info!("User {} authenticated successfully", username);
            } else {
                stream.write_all(b"\r\nAuthentication failed.\r\nLogin: ").await?;
                username.clear();
                warn!("Authentication failed for user: {}", username);
            }
        }
    }

    // Add session
    {
        let session = SessionData {
            session_id,
            username: username.clone(),
            authenticated,
        };
        let mut sessions = sessions.write().await;
        sessions.insert(session_id, session);
    }

    // Command loop
    loop {
        stream.write_all(b"sam> ").await?;
        
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        
        let input = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        let response = handle_command(&input).await;
        
        stream.write_all(response.as_bytes()).await?;
        
        if input == "exit" || input == "quit" {
            break;
        }
    }

    // Cleanup
    {
        let mut sessions = sessions.write().await;
        sessions.remove(&session_id);
    }
    
    info!("Session {} ended for user {}", session_id, username);
    Ok(())
}

/// Handle command input
async fn handle_command(input: &str) -> String {
    match input.trim() {
        "help" => {
            format!(
                "Available SSH Commands:\r\n\
                 help          - Show this help message\r\n\
                 tui           - Enter TUI mode (interactive terminal)\r\n\
                 status        - Show service status\r\n\
                 services      - List all services\r\n\
                 logs          - Show recent logs\r\n\
                 system        - Show system information\r\n\
                 exit, quit    - Disconnect from SSH session\r\n\
                 \r\n\
                 Note: TUI mode provides full interactive access to SAM\r\n\
                 sam> "
            )
        }
        "tui" => {
            // This would integrate with the TUI system
            format!(
                "Entering TUI mode...\r\n\
                 Press Ctrl+C to return to command mode.\r\n\
                 [TUI mode not yet implemented - coming soon!]\r\n\
                 sam> "
            )
        }
        "status" => {
            // Get service status (would integrate with actual service manager)
            format!(
                "SAM Service Status:\r\n\
                 HTTP Server: Running (port 8000)\r\n\
                 WebSocket: Running (port 8080)\r\n\
                 SSH Server: Running (port 2222)\r\n\
                 Redis: Unknown\r\n\
                 PostgreSQL: Unknown\r\n\
                 Docker: Unknown\r\n\
                 \r\n\
                 sam> "
            )
        }
        "services" => {
            format!(
                "Available Services:\r\n\
                 - HTTP Server (Web Dashboard)\r\n\
                 - WebSocket Server (Real-time Communication)\r\n\
                 - SSH Server (Remote TUI Access)\r\n\
                 - Redis (Caching)\r\n\
                 - PostgreSQL (Database)\r\n\
                 - Docker (Container Management)\r\n\
                 - AI Services (OpenAI, Llama, RiveScript)\r\n\
                 - Home Automation (LIFX, Matter)\r\n\
                 \r\n\
                 Use 'status' for current service states\r\n\
                 sam> "
            )
        }
        "logs" => {
            format!(
                "Recent SAM Logs:\r\n\
                 [Log integration not yet implemented]\r\n\
                 Use TUI mode for full log viewing capabilities\r\n\
                 \r\n\
                 sam> "
            )
        }
        "system" => {
            format!(
                "SAM System Information:\r\n\
                 Version: 0.0.2\r\n\
                 Platform: {}\r\n\
                 Architecture: {}\r\n\
                 \r\n\
                 Use TUI mode for detailed system monitoring\r\n\
                 sam> ",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        }
        "exit" | "quit" => {
            "Goodbye!\r\n".to_string()
        }
        "" => "\r\n".to_string(),
        _ => {
            format!(
                "Unknown command: '{}'\r\n\
                 Type 'help' for available commands\r\n\
                 sam> ",
                input
            )
        }
    }
}

/// Initialize and start the remote access server
pub async fn start_ssh_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = RemoteAccessServer::new()?;
    server.start().await
}

/// Check if SSH server is running
pub async fn is_ssh_server_running() -> bool {
    let port = get_ssh_server_port();
    match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Get SSH server status
pub fn get_ssh_server_status() -> ServiceStatus {
    if tokio::runtime::Handle::try_current()
        .map(|_| futures::executor::block_on(is_ssh_server_running()))
        .unwrap_or(false)
    {
        ServiceStatus::Running
    } else {
        ServiceStatus::Stopped
    }
}

/// Stop the SSH server
pub async fn stop_ssh_server() {
    info!("SSH server stop requested");
    // SSH server runs in background, stopping is handled by OS when process ends
    // For now, just log the stop request - actual shutdown would require storing the server handle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_server_creation() {
        let server = RemoteAccessServer::new();
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_ssh_server_port_available() {
        let result = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", DEFAULT_SSH_SERVER_PORT)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_tui_handle_creation() {
        let handle = TuiHandle::new();
        // Just verify it can be created
        assert_eq!(std::mem::size_of_val(&handle), 0);
    }
}
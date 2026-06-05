use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// SSH connection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub timeout_seconds: u64,
    pub keepalive_interval: Option<u64>,
    pub strict_host_key_checking: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    Password {
        password: String,
    },
    PublicKey {
        private_key_path: String,
        passphrase: Option<String>,
    },
    Agent,
}

/// SSH connection manager
pub struct SshManager {
    connections: Arc<RwLock<HashMap<String, SshConnection>>>,
    default_config: SshConfig,
}

/// Individual SSH connection
pub struct SshConnection {
    id: String,
    session: Session,
    config: SshConfig,
    created_at: std::time::Instant,
    last_used: std::time::Instant,
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// File transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub success: bool,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl SshManager {
    /// Create a new SSH manager
    pub fn new(default_config: SshConfig) -> Self {
        SshManager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        }
    }

    /// Connect to a remote host
    pub async fn connect(
        &self,
        config: Option<SshConfig>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let connection_id = format!("{}@{}:{}", config.username, config.host, config.port);

        // Check if connection already exists
        {
            let connections = self.connections.read().await;
            if connections.contains_key(&connection_id) {
                debug!("Reusing existing SSH connection to {}", connection_id);
                return Ok(connection_id);
            }
        }

        // Create new connection
        let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(config.timeout_seconds))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Authenticate
        match &config.auth_method {
            AuthMethod::Password { password } => {
                session.userauth_password(&config.username, password)?;
            }
            AuthMethod::PublicKey {
                private_key_path,
                passphrase,
            } => {
                let key_path = Path::new(private_key_path);
                session.userauth_pubkey_file(
                    &config.username,
                    None,
                    key_path,
                    passphrase.as_deref(),
                )?;
            }
            AuthMethod::Agent => {
                session.userauth_agent(&config.username)?;
            }
        }

        if !session.authenticated() {
            return Err("SSH authentication failed".into());
        }

        // Set keepalive if configured
        if let Some(interval) = config.keepalive_interval {
            session.set_keepalive(true, interval as u32);
        }

        info!("Successfully connected to SSH host: {}", connection_id);

        let connection = SshConnection {
            id: connection_id.clone(),
            session,
            config,
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
        };

        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), connection);

        Ok(connection_id)
    }

    /// Execute a command on a remote host
    pub async fn execute_command(
        &self,
        connection_id: &str,
        command: &str,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();

        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let mut channel = connection.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        channel.wait_close()?;
        let exit_code = channel.exit_status()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(
            "Command '{}' executed in {}ms with exit code {}",
            command, duration_ms, exit_code
        );

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
            duration_ms,
        })
    }

    /// Execute multiple commands in sequence
    pub async fn execute_commands(
        &self,
        connection_id: &str,
        commands: Vec<String>,
    ) -> Result<Vec<CommandResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute_command(connection_id, &command).await?;

            // Stop on error unless it's a continue-on-error command
            if result.exit_code != 0 && !command.starts_with('-') {
                error!("Command failed: {}", command);
                results.push(result);
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Execute a command with streaming output
    pub async fn execute_command_stream<F>(
        &self,
        connection_id: &str,
        command: &str,
        mut callback: F,
    ) -> Result<CommandResult, Box<dyn std::error::Error>>
    where
        F: FnMut(&str),
    {
        let start = std::time::Instant::now();

        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let mut channel = connection.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout_full = String::new();
        let mut buffer = [0u8; 1024];

        loop {
            match channel.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = std::str::from_utf8(&buffer[..n])?;
                    stdout_full.push_str(chunk);
                    callback(chunk);
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        return Err(e.into());
                    }
                }
            }
        }

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        channel.wait_close()?;
        let exit_code = channel.exit_status()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(CommandResult {
            stdout: stdout_full,
            stderr,
            exit_code,
            duration_ms,
        })
    }

    /// Upload a file to remote host
    pub async fn upload_file(
        &self,
        connection_id: &str,
        local_path: &Path,
        remote_path: &Path,
    ) -> Result<TransferResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();

        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let sftp = connection.session.sftp()?;

        let local_file = std::fs::File::open(local_path)?;
        let metadata = local_file.metadata()?;
        let file_size = metadata.len();

        let mut remote_file = sftp.create(remote_path)?;
        let mut local_file = BufReader::new(local_file);

        let bytes_transferred = std::io::copy(&mut local_file, &mut remote_file)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Uploaded {} bytes to {} in {}ms",
            bytes_transferred,
            remote_path.display(),
            duration_ms
        );

        Ok(TransferResult {
            success: true,
            bytes_transferred,
            duration_ms,
            error: None,
        })
    }

    /// Download a file from remote host
    pub async fn download_file(
        &self,
        connection_id: &str,
        remote_path: &Path,
        local_path: &Path,
    ) -> Result<TransferResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();

        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let sftp = connection.session.sftp()?;

        let mut remote_file = sftp.open(remote_path)?;
        let mut local_file = std::fs::File::create(local_path)?;

        let bytes_transferred = std::io::copy(&mut remote_file, &mut local_file)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Downloaded {} bytes from {} in {}ms",
            bytes_transferred,
            remote_path.display(),
            duration_ms
        );

        Ok(TransferResult {
            success: true,
            bytes_transferred,
            duration_ms,
            error: None,
        })
    }

    /// List directory contents on remote host
    pub async fn list_directory(
        &self,
        connection_id: &str,
        path: &Path,
    ) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let sftp = connection.session.sftp()?;
        let entries = sftp.readdir(path)?;

        let mut files = Vec::new();
        for (path, stat) in entries {
            files.push(FileInfo {
                path: path.to_string_lossy().to_string(),
                size: stat.size.unwrap_or(0),
                is_directory: stat.is_dir(),
                is_file: stat.is_file(),
                permissions: stat.perm.unwrap_or(0),
                modified: stat.mtime,
            });
        }

        Ok(files)
    }

    /// Create an SSH tunnel
    pub async fn create_tunnel(
        &self,
        connection_id: &str,
        local_port: u16,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut connections = self.connections.write().await;
        let connection = connections
            .get_mut(connection_id)
            .ok_or("Connection not found")?;

        connection.last_used = std::time::Instant::now();

        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", local_port))?;
        info!("SSH tunnel listening on localhost:{}", local_port);

        // This would need to be spawned in a separate thread/task
        // to handle incoming connections and forward them

        Ok(())
    }

    /// Disconnect from a host
    pub async fn disconnect(&self, connection_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.remove(connection_id) {
            connection
                .session
                .disconnect(None, "User disconnect", None)?;
            info!("Disconnected from SSH host: {}", connection_id);
            Ok(())
        } else {
            Err("Connection not found".into())
        }
    }

    /// Disconnect all connections
    pub async fn disconnect_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut connections = self.connections.write().await;

        for (id, connection) in connections.drain() {
            if let Err(e) = connection.session.disconnect(None, "Shutdown", None) {
                warn!("Error disconnecting from {}: {}", id, e);
            }
        }

        info!("All SSH connections closed");
        Ok(())
    }

    /// Get active connections
    pub async fn get_connections(&self) -> Vec<ConnectionInfo> {
        let connections = self.connections.read().await;

        connections
            .values()
            .map(|conn| ConnectionInfo {
                id: conn.id.clone(),
                host: conn.config.host.clone(),
                port: conn.config.port,
                username: conn.config.username.clone(),
                connected_duration_seconds: conn.created_at.elapsed().as_secs(),
                last_used_seconds_ago: conn.last_used.elapsed().as_secs(),
            })
            .collect()
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self, max_idle_seconds: u64) {
        let mut connections = self.connections.write().await;
        let now = std::time::Instant::now();

        connections.retain(|id, conn| {
            let idle_time = now.duration_since(conn.last_used).as_secs();
            if idle_time > max_idle_seconds {
                info!("Closing idle SSH connection: {}", id);
                let _ = conn.session.disconnect(None, "Idle timeout", None);
                false
            } else {
                true
            }
        });
    }
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub is_file: bool,
    pub permissions: u32,
    pub modified: Option<u64>,
}

/// Connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected_duration_seconds: u64,
    pub last_used_seconds_ago: u64,
}

/// SSH command builder for complex operations
pub struct SshCommandBuilder {
    commands: Vec<String>,
    environment: HashMap<String, String>,
    working_directory: Option<String>,
}

impl Default for SshCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SshCommandBuilder {
    pub fn new() -> Self {
        SshCommandBuilder {
            commands: Vec::new(),
            environment: HashMap::new(),
            working_directory: None,
        }
    }

    pub fn add_command(mut self, command: &str) -> Self {
        self.commands.push(command.to_string());
        self
    }

    pub fn set_env(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }

    pub fn set_working_directory(mut self, dir: &str) -> Self {
        self.working_directory = Some(dir.to_string());
        self
    }

    pub fn build(self) -> String {
        let mut script = String::new();

        // Set environment variables
        for (key, value) in self.environment {
            script.push_str(&format!("export {}='{}'\n", key, value));
        }

        // Change directory if specified
        if let Some(dir) = self.working_directory {
            script.push_str(&format!("cd '{}'\n", dir));
        }

        // Add commands
        for command in self.commands {
            script.push_str(&format!("{}\n", command));
        }

        script
    }
}

/// SSH session for interactive operations
pub struct SshSession {
    manager: Arc<SshManager>,
    connection_id: String,
    current_directory: PathBuf,
}

impl SshSession {
    pub async fn new(
        manager: Arc<SshManager>,
        config: SshConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let connection_id = manager.connect(Some(config)).await?;

        // Get home directory
        let result = manager.execute_command(&connection_id, "pwd").await?;
        let current_directory = PathBuf::from(result.stdout.trim());

        Ok(SshSession {
            manager,
            connection_id,
            current_directory,
        })
    }

    pub async fn execute(
        &self,
        command: &str,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        self.manager
            .execute_command(&self.connection_id, command)
            .await
    }

    pub async fn cd(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("cd '{}' && pwd", path);
        let result = self.execute(&command).await?;

        if result.exit_code == 0 {
            self.current_directory = PathBuf::from(result.stdout.trim());
            Ok(())
        } else {
            Err(format!("Failed to change directory: {}", result.stderr).into())
        }
    }

    pub async fn pwd(&self) -> PathBuf {
        self.current_directory.clone()
    }

    pub async fn upload(
        &self,
        local_path: &Path,
        remote_path: &Path,
    ) -> Result<TransferResult, Box<dyn std::error::Error>> {
        self.manager
            .upload_file(&self.connection_id, local_path, remote_path)
            .await
    }

    pub async fn download(
        &self,
        remote_path: &Path,
        local_path: &Path,
    ) -> Result<TransferResult, Box<dyn std::error::Error>> {
        self.manager
            .download_file(&self.connection_id, remote_path, local_path)
            .await
    }

    pub async fn ls(
        &self,
        path: Option<&Path>,
    ) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let path = path.unwrap_or(&self.current_directory);
        self.manager.list_directory(&self.connection_id, path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config() {
        let config = SshConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: AuthMethod::Password {
                password: "secret".to_string(),
            },
            timeout_seconds: 30,
            keepalive_interval: Some(60),
            strict_host_key_checking: true,
        };

        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn test_command_builder() {
        let script = SshCommandBuilder::new()
            .set_env("PATH", "/usr/local/bin:$PATH")
            .set_working_directory("/home/user/project")
            .add_command("ls -la")
            .add_command("git status")
            .build();

        assert!(script.contains("export PATH='/usr/local/bin:$PATH'"));
        assert!(script.contains("cd '/home/user/project'"));
        assert!(script.contains("ls -la"));
        assert!(script.contains("git status"));
    }

    #[test]
    fn test_command_result() {
        let result = CommandResult {
            stdout: "Hello World".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
            duration_ms: 100,
        };

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "Hello World");
        assert!(result.stderr.is_empty());
    }
}

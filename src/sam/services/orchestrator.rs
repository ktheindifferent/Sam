use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, interval};
use futures::future::join_all;
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceName {
    // Core Infrastructure
    PostgreSQL,
    Redis,
    Docker,
    
    // Security Services
    ClamAV,
    VulnerabilityScanner,
    
    // AI/ML Services
    Whisper,
    Llama,
    OpenAI,
    Copilot,
    
    // Communication Services
    Voice,
    SMS,
    Notifications,
    P2P,
    
    // Storage Services
    FileStorage,
    Backup,
    Dropbox,
    
    // Development Services
    Git,
    GitHub,
    SSH,
    
    // Smart Home
    Lifx,
    Matter,
    
    // Media Services
    Media,
    Spotify,
    Snapcast,
    
    // Web Services
    Crawler,
    MDNS,
    WebSocket,
    
    // Other Services
    PasswordManager,
    RiveScript,
    RTSP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed(String),
    Degraded(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: ServiceStatus,
    pub uptime: Duration,
    pub last_check: Instant,
    pub error_count: u32,
    pub restart_count: u32,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f32>,
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for ServiceHealth {
    fn default() -> Self {
        Self {
            status: ServiceStatus::Stopped,
            uptime: Duration::from_secs(0),
            last_check: Instant::now(),
            error_count: 0,
            restart_count: 0,
            memory_usage: None,
            cpu_usage: None,
            custom_metrics: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: ServiceName,
    pub enabled: bool,
    pub auto_restart: bool,
    pub max_restarts: u32,
    pub health_check_interval: Duration,
    pub startup_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub dependencies: Vec<ServiceName>,
    pub environment: HashMap<String, String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: ServiceName::Redis,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(30),
            startup_timeout: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(30),
            dependencies: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

pub struct ServiceOrchestrator {
    services: Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
    configs: Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ServiceOrchestrator {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    pub fn register_service(&self, config: ServiceConfig) -> Result<()> {
        let name = config.name.clone();
        
        self.configs.write().unwrap().insert(name.clone(), config);
        self.services.write().unwrap().insert(name.clone(), ServiceHealth::default());
        
        info!("Registered service: {:?}", name);
        Ok(())
    }

    pub async fn start_all(&mut self) -> Result<()> {
        info!("Starting all services...");
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Get dependency order
        let start_order = self.get_startup_order()?;
        
        // Start services in order
        for service_name in start_order {
            if let Err(e) = self.start_service(&service_name).await {
                error!("Failed to start {:?}: {}", service_name, e);
                
                // Check if this is a critical service
                if self.is_critical_service(&service_name) {
                    return Err(anyhow::anyhow!("Critical service {:?} failed to start", service_name));
                }
            }
        }
        
        // Start health monitoring
        let services = self.services.clone();
        let configs = self.configs.clone();
        
        tokio::spawn(async move {
            let mut health_interval = interval(Duration::from_secs(10));
            
            loop {
                tokio::select! {
                    _ = health_interval.tick() => {
                        Self::check_all_health(&services, &configs).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Health monitor shutting down");
                        break;
                    }
                }
            }
        });
        
        info!("All services started successfully");
        Ok(())
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        info!("Stopping all services...");
        
        // Signal health monitor to stop
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        
        // Get shutdown order (reverse of startup)
        let mut stop_order = self.get_startup_order()?;
        stop_order.reverse();
        
        // Stop services in order
        for service_name in stop_order {
            if let Err(e) = self.stop_service(&service_name).await {
                warn!("Error stopping {:?}: {}", service_name, e);
            }
        }
        
        info!("All services stopped");
        Ok(())
    }

    async fn start_service(&self, name: &ServiceName) -> Result<()> {
        info!("Starting service: {:?}", name);
        
        // Update status
        {
            let mut services = self.services.write().unwrap();
            if let Some(health) = services.get_mut(name) {
                health.status = ServiceStatus::Starting;
            }
        }
        
        // Start the actual service
        let result = match name {
            ServiceName::PostgreSQL => self.start_postgresql().await,
            ServiceName::Redis => self.start_redis().await,
            ServiceName::Docker => self.start_docker().await,
            ServiceName::FileStorage => self.start_file_storage().await,
            ServiceName::Backup => self.start_backup().await,
            ServiceName::SSH => self.start_ssh().await,
            ServiceName::Crawler => self.start_crawler().await,
            ServiceName::Voice => self.start_voice().await,
            ServiceName::P2P => self.start_p2p().await,
            ServiceName::VulnerabilityScanner => self.start_vulnerability_scanner().await,
            ServiceName::Whisper => self.start_whisper().await,
            ServiceName::Lifx => self.start_lifx().await,
            ServiceName::Media => self.start_media().await,
            ServiceName::WebSocket => self.start_websocket().await,
            ServiceName::MDNS => self.start_mdns().await,
            _ => {
                warn!("Service {:?} not yet implemented", name);
                Ok(())
            }
        };
        
        // Update status based on result
        {
            let mut services = self.services.write().unwrap();
            if let Some(health) = services.get_mut(name) {
                match result {
                    Ok(_) => {
                        health.status = ServiceStatus::Running;
                        health.uptime = Duration::from_secs(0);
                        info!("Service {:?} started successfully", name);
                    }
                    Err(ref e) => {
                        health.status = ServiceStatus::Failed(e.to_string());
                        health.error_count += 1;
                        error!("Service {:?} failed to start: {}", name, e);
                    }
                }
            }
        }
        
        result
    }

    async fn stop_service(&self, name: &ServiceName) -> Result<()> {
        info!("Stopping service: {:?}", name);
        
        // Update status
        {
            let mut services = self.services.write().unwrap();
            if let Some(health) = services.get_mut(name) {
                health.status = ServiceStatus::Stopping;
            }
        }
        
        // Stop the actual service
        let result = match name {
            ServiceName::PostgreSQL => self.stop_postgresql().await,
            ServiceName::Redis => self.stop_redis().await,
            ServiceName::Docker => self.stop_docker().await,
            ServiceName::Crawler => self.stop_crawler().await,
            ServiceName::P2P => self.stop_p2p().await,
            ServiceName::WebSocket => self.stop_websocket().await,
            _ => Ok(()),
        };
        
        // Update status
        {
            let mut services = self.services.write().unwrap();
            if let Some(health) = services.get_mut(name) {
                health.status = ServiceStatus::Stopped;
            }
        }
        
        result
    }

    // Service-specific start methods
    async fn start_postgresql(&self) -> Result<()> {
        // Check if PostgreSQL is already running
        match crate::sam::services::pg::connect().await {
            Ok(_) => {
                info!("PostgreSQL is already running");
                Ok(())
            }
            Err(_) => {
                // Try to start PostgreSQL using Docker or system service
                if let Ok(_) = crate::sam::services::docker::is_running().await {
                    // Start PostgreSQL container
                    info!("Starting PostgreSQL via Docker");
                    crate::sam::services::docker::start_postgres().await
                        .context("Failed to start PostgreSQL container")?;
                } else {
                    // Try system service
                    warn!("Docker not available, attempting system PostgreSQL");
                    // This would require system-specific commands
                }
                Ok(())
            }
        }
    }

    async fn start_redis(&self) -> Result<()> {
        match crate::sam::services::redis::connect().await {
            Ok(_) => {
                info!("Redis is already running");
                Ok(())
            }
            Err(_) => {
                if let Ok(_) = crate::sam::services::docker::is_running().await {
                    info!("Starting Redis via Docker");
                    crate::sam::services::docker::start_redis().await
                        .context("Failed to start Redis container")?;
                } else {
                    warn!("Docker not available, attempting system Redis");
                }
                Ok(())
            }
        }
    }

    async fn start_docker(&self) -> Result<()> {
        crate::sam::services::docker::ensure_running().await
            .context("Failed to ensure Docker is running")
    }

    async fn start_file_storage(&self) -> Result<()> {
        info!("Initializing file storage service");
        crate::sam::services::file_storage::initialize().await
            .context("Failed to initialize file storage")
    }

    async fn start_backup(&self) -> Result<()> {
        info!("Starting backup service");
        crate::sam::services::backup::start_scheduler().await
            .context("Failed to start backup scheduler")
    }

    async fn start_ssh(&self) -> Result<()> {
        info!("Initializing SSH service");
        // SSH service is typically on-demand, no continuous process
        Ok(())
    }

    async fn start_crawler(&self) -> Result<()> {
        info!("Starting crawler service");
        crate::sam::services::crawler::start_service_async().await
            .context("Failed to start crawler service")
    }

    async fn start_voice(&self) -> Result<()> {
        info!("Starting voice service");
        crate::sam::services::voice::initialize().await
            .context("Failed to initialize voice service")
    }

    async fn start_p2p(&self) -> Result<()> {
        info!("Starting P2P service");
        crate::sam::services::p2p::start_network().await
            .context("Failed to start P2P network")
    }

    async fn start_vulnerability_scanner(&self) -> Result<()> {
        info!("Initializing vulnerability scanner");
        // Scanner is typically on-demand
        Ok(())
    }

    async fn start_whisper(&self) -> Result<()> {
        info!("Loading Whisper models");
        crate::sam::services::stt::whisper_enhanced::initialize().await
            .context("Failed to initialize Whisper")
    }

    async fn start_lifx(&self) -> Result<()> {
        info!("Starting Lifx service");
        crate::sam::services::lifx::start_server().await
            .context("Failed to start Lifx server")
    }

    async fn start_media(&self) -> Result<()> {
        info!("Starting media service");
        crate::sam::services::media::initialize().await
            .context("Failed to initialize media service")
    }

    async fn start_websocket(&self) -> Result<()> {
        info!("Starting WebSocket server");
        crate::sam::websocket::start_server().await
            .context("Failed to start WebSocket server")
    }

    async fn start_mdns(&self) -> Result<()> {
        info!("Starting mDNS service");
        crate::sam::services::mdns::start_discovery().await
            .context("Failed to start mDNS discovery")
    }

    // Service-specific stop methods
    async fn stop_postgresql(&self) -> Result<()> {
        if let Ok(_) = crate::sam::services::docker::is_running().await {
            crate::sam::services::docker::stop_postgres().await?;
        }
        Ok(())
    }

    async fn stop_redis(&self) -> Result<()> {
        if let Ok(_) = crate::sam::services::docker::is_running().await {
            crate::sam::services::docker::stop_redis().await?;
        }
        Ok(())
    }

    async fn stop_docker(&self) -> Result<()> {
        // Docker daemon is system-managed
        Ok(())
    }

    async fn stop_crawler(&self) -> Result<()> {
        crate::sam::services::crawler::stop_service().await
    }

    async fn stop_p2p(&self) -> Result<()> {
        crate::sam::services::p2p::stop_network().await
    }

    async fn stop_websocket(&self) -> Result<()> {
        crate::sam::websocket::stop_server().await
    }

    // Helper methods
    fn get_startup_order(&self) -> Result<Vec<ServiceName>> {
        let configs = self.configs.read().unwrap();
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        
        for (name, config) in configs.iter() {
            if config.enabled {
                self.topological_sort(name, &configs, &mut visited, &mut visiting, &mut order)?;
            }
        }
        
        Ok(order)
    }

    fn topological_sort(
        &self,
        name: &ServiceName,
        configs: &HashMap<ServiceName, ServiceConfig>,
        visited: &mut HashSet<ServiceName>,
        visiting: &mut HashSet<ServiceName>,
        order: &mut Vec<ServiceName>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(());
        }
        
        if visiting.contains(name) {
            return Err(anyhow::anyhow!("Circular dependency detected for {:?}", name));
        }
        
        visiting.insert(name.clone());
        
        if let Some(config) = configs.get(name) {
            for dep in &config.dependencies {
                self.topological_sort(dep, configs, visited, visiting, order)?;
            }
        }
        
        visiting.remove(name);
        visited.insert(name.clone());
        order.push(name.clone());
        
        Ok(())
    }

    fn is_critical_service(&self, name: &ServiceName) -> bool {
        matches!(name, ServiceName::PostgreSQL | ServiceName::Redis)
    }

    async fn check_all_health(
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
        configs: &Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
    ) {
        let service_names: Vec<ServiceName> = {
            services.read().unwrap().keys().cloned().collect()
        };
        
        for name in service_names {
            if let Err(e) = Self::check_service_health(&name, services, configs).await {
                warn!("Health check failed for {:?}: {}", name, e);
            }
        }
    }

    async fn check_service_health(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
        configs: &Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
    ) -> Result<()> {
        let healthy = match name {
            ServiceName::PostgreSQL => {
                crate::sam::services::pg::health_check().await.is_ok()
            }
            ServiceName::Redis => {
                crate::sam::services::redis::health_check().await.is_ok()
            }
            ServiceName::Docker => {
                crate::sam::services::docker::is_running().await.is_ok()
            }
            _ => true, // Default to healthy for unimplemented checks
        };
        
        let mut services = services.write().unwrap();
        if let Some(health) = services.get_mut(name) {
            health.last_check = Instant::now();
            
            if !healthy && matches!(health.status, ServiceStatus::Running) {
                health.status = ServiceStatus::Degraded("Health check failed".to_string());
                health.error_count += 1;
                
                // Check if we should restart
                let should_restart = {
                    let configs = configs.read().unwrap();
                    configs.get(name)
                        .map(|c| c.auto_restart && health.restart_count < c.max_restarts)
                        .unwrap_or(false)
                };
                
                if should_restart {
                    health.restart_count += 1;
                    warn!("Restarting unhealthy service {:?} (attempt {})", name, health.restart_count);
                    // TODO: Implement restart logic
                }
            }
        }
        
        Ok(())
    }

    pub fn get_status(&self) -> HashMap<ServiceName, ServiceStatus> {
        self.services.read().unwrap()
            .iter()
            .map(|(name, health)| (name.clone(), health.status.clone()))
            .collect()
    }

    pub fn get_health(&self, name: &ServiceName) -> Option<ServiceHealth> {
        self.services.read().unwrap().get(name).cloned()
    }

    pub fn get_all_health(&self) -> HashMap<ServiceName, ServiceHealth> {
        self.services.read().unwrap().clone()
    }
}

// Default service configurations
pub fn default_configs() -> Vec<ServiceConfig> {
    vec![
        ServiceConfig {
            name: ServiceName::PostgreSQL,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(30),
            startup_timeout: Duration::from_secs(120),
            shutdown_timeout: Duration::from_secs(30),
            dependencies: vec![],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::Redis,
            enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval: Duration::from_secs(30),
            startup_timeout: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(10),
            dependencies: vec![],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::Docker,
            enabled: true,
            auto_restart: false,
            max_restarts: 1,
            health_check_interval: Duration::from_secs(60),
            startup_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
            dependencies: vec![],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::FileStorage,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::PostgreSQL],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::Backup,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::FileStorage, ServiceName::PostgreSQL],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::Crawler,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::PostgreSQL, ServiceName::Redis],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::Voice,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::Whisper],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::P2P,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::Redis],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::WebSocket,
            enabled: true,
            auto_restart: true,
            dependencies: vec![ServiceName::Redis],
            ..Default::default()
        },
        ServiceConfig {
            name: ServiceName::MDNS,
            enabled: true,
            auto_restart: true,
            dependencies: vec![],
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_registration() {
        let orchestrator = ServiceOrchestrator::new();
        let config = ServiceConfig {
            name: ServiceName::Redis,
            ..Default::default()
        };
        
        assert!(orchestrator.register_service(config).is_ok());
        assert!(orchestrator.get_health(&ServiceName::Redis).is_some());
    }

    #[tokio::test]
    async fn test_dependency_resolution() {
        let orchestrator = ServiceOrchestrator::new();
        
        // Register services with dependencies
        orchestrator.register_service(ServiceConfig {
            name: ServiceName::PostgreSQL,
            enabled: true,
            dependencies: vec![],
            ..Default::default()
        }).unwrap();
        
        orchestrator.register_service(ServiceConfig {
            name: ServiceName::FileStorage,
            enabled: true,
            dependencies: vec![ServiceName::PostgreSQL],
            ..Default::default()
        }).unwrap();
        
        orchestrator.register_service(ServiceConfig {
            name: ServiceName::Backup,
            enabled: true,
            dependencies: vec![ServiceName::FileStorage],
            ..Default::default()
        }).unwrap();
        
        let order = orchestrator.get_startup_order().unwrap();
        
        // PostgreSQL should start before FileStorage
        let pg_index = order.iter().position(|s| *s == ServiceName::PostgreSQL).unwrap();
        let fs_index = order.iter().position(|s| *s == ServiceName::FileStorage).unwrap();
        let backup_index = order.iter().position(|s| *s == ServiceName::Backup).unwrap();
        
        assert!(pg_index < fs_index);
        assert!(fs_index < backup_index);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let orchestrator = ServiceOrchestrator::new();
        
        // Create circular dependency
        orchestrator.register_service(ServiceConfig {
            name: ServiceName::Redis,
            enabled: true,
            dependencies: vec![ServiceName::PostgreSQL],
            ..Default::default()
        }).unwrap();
        
        orchestrator.register_service(ServiceConfig {
            name: ServiceName::PostgreSQL,
            enabled: true,
            dependencies: vec![ServiceName::Redis],
            ..Default::default()
        }).unwrap();
        
        assert!(orchestrator.get_startup_order().is_err());
    }
}
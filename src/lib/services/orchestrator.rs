use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, sleep};

use super::restart::{RestartConfig, RestartManager, RestartStrategy};

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

#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    pub status: ServiceStatus,
    pub uptime: Duration,
    #[serde(skip)]
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
    restart_manager: Arc<RestartManager>,
    restart_tasks: Arc<Mutex<HashMap<ServiceName, tokio::task::JoinHandle<()>>>>,
}

impl Default for ServiceOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceOrchestrator {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
            restart_manager: Arc::new(RestartManager::new()),
            restart_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_service(&self, config: ServiceConfig) -> Result<()> {
        let name = config.name.clone();

        // Configure restart behavior
        let restart_config = RestartConfig {
            strategy: RestartStrategy::ExponentialBackoff {
                base_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                multiplier: 2.0,
            },
            max_attempts: config.max_restarts,
            health_check_timeout: Duration::from_secs(30),
            health_check_retries: 3,
            dependency_check: true,
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(300),
            notify_on_restart: true,
            notify_on_failure: true,
        };

        self.restart_manager
            .register_config(name.clone(), restart_config)?;

        self.configs
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire configs write lock: {}", e))?
            .insert(name.clone(), config);
        self.services
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
            .insert(name.clone(), ServiceHealth::default());

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
                    return Err(anyhow::anyhow!(
                        "Critical service {:?} failed to start",
                        service_name
                    ));
                }
            }
        }

        // Start health monitoring
        let services = self.services.clone();
        let configs = self.configs.clone();

        {
            let services = services.clone();
            let configs = configs.clone();
            tokio::spawn(async move {
                let mut health_interval = interval(Duration::from_secs(10));

                loop {
                    tokio::select! {
                        _ = health_interval.tick() => {
                            // Create a simple health check without complex locking
                            // TODO: Implement proper health monitoring
                        }
                        _ = shutdown_rx.recv() => {
                            info!("Health monitor shutting down");
                            break;
                        }
                    }
                }
            });
        }

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
            let mut services = self
                .services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?;
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
            let mut services = self
                .services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?;
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
            let mut services = self
                .services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?;
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
            let mut services = self
                .services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?;
            if let Some(health) = services.get_mut(name) {
                health.status = ServiceStatus::Stopped;
            }
        }

        result
    }

    // Service-specific start methods
    async fn start_postgresql(&self) -> Result<()> {
        // Check if PostgreSQL is already running
        match crate::services::pg::connect().await {
            Ok(_) => {
                info!("PostgreSQL is already running");
                Ok(())
            }
            Err(_) => {
                // Try to start PostgreSQL using Docker or system service
                if let Ok(_) = crate::services::docker::is_running_async().await {
                    // Start PostgreSQL container
                    info!("Starting PostgreSQL via Docker");
                    crate::services::docker::start_postgres()
                        .await
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
        match crate::services::redis::connect().await {
            Ok(_) => {
                info!("Redis is already running");
                Ok(())
            }
            Err(_) => {
                if let Ok(_) = crate::services::docker::is_running_async().await {
                    info!("Starting Redis via Docker");
                    crate::services::docker::start_redis()
                        .await
                        .context("Failed to start Redis container")?;
                } else {
                    warn!("Docker not available, attempting system Redis");
                }
                Ok(())
            }
        }
    }

    async fn start_docker(&self) -> Result<()> {
        crate::services::docker::ensure_running()
            .await
            .context("Failed to ensure Docker is running")
    }

    async fn start_file_storage(&self) -> Result<()> {
        info!("Initializing file storage service");
        crate::services::fs::initialize()
            .await
            .context("Failed to initialize file storage")
    }

    async fn start_backup(&self) -> Result<()> {
        info!("Starting backup service");
        crate::services::backup::start_scheduler()
            .await
            .context("Failed to start backup scheduler")
    }

    async fn start_ssh(&self) -> Result<()> {
        info!("Initializing SSH service");
        // SSH service is typically on-demand, no continuous process
        Ok(())
    }

    async fn start_crawler(&self) -> Result<()> {
        info!("Starting crawler service");
        crate::services::crawler::start_service_async().await;
        Ok(())
    }

    async fn start_voice(&self) -> Result<()> {
        info!("Starting voice service");
        crate::services::voice::initialize()
            .await
            .context("Failed to initialize voice service")
    }

    async fn start_p2p(&self) -> Result<()> {
        info!("Starting P2P service");
        crate::services::p2p::start_network()
            .await
            .context("Failed to start P2P network")
    }

    async fn start_vulnerability_scanner(&self) -> Result<()> {
        info!("Initializing vulnerability scanner");
        // Scanner is typically on-demand
        Ok(())
    }

    async fn start_whisper(&self) -> Result<()> {
        info!("Loading Whisper models");
        crate::services::stt::whisper_enhanced::initialize()
            .await
            .context("Failed to initialize Whisper")
    }

    async fn start_lifx(&self) -> Result<()> {
        info!("Starting Lifx service");
        crate::services::lifx::start_server()
            .await
            .context("Failed to start Lifx server")
    }

    async fn start_media(&self) -> Result<()> {
        info!("Starting media service");
        crate::services::media::initialize()
            .await
            .context("Failed to initialize media service")
    }

    async fn start_websocket(&self) -> Result<()> {
        info!("Starting WebSocket server");
        crate::websocket::start_server()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start WebSocket server: {}", e))
    }

    async fn start_mdns(&self) -> Result<()> {
        info!("Starting mDNS service");
        let output_lines = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        crate::services::mdns::start_discovery(output_lines).await;
        Ok(())
    }

    // Service-specific stop methods
    async fn stop_postgresql(&self) -> Result<()> {
        if let Ok(_) = crate::services::docker::is_running_async().await {
            crate::services::docker::stop_postgres().await?;
        }
        Ok(())
    }

    async fn stop_redis(&self) -> Result<()> {
        if let Ok(_) = crate::services::docker::is_running_async().await {
            crate::services::docker::stop_redis().await?;
        }
        Ok(())
    }

    async fn stop_docker(&self) -> Result<()> {
        // Docker daemon is system-managed
        Ok(())
    }

    async fn stop_crawler(&self) -> Result<()> {
        crate::services::crawler::stop_service();
        Ok(())
    }

    async fn stop_p2p(&self) -> Result<()> {
        crate::services::p2p::stop_network().await
    }

    async fn stop_websocket(&self) -> Result<()> {
        crate::websocket::stop_server()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to stop WebSocket server: {}", e))
    }

    // Helper methods
    fn get_startup_order(&self) -> Result<Vec<ServiceName>> {
        let configs = self
            .configs
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire configs read lock: {}", e))?;
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
            return Err(anyhow::anyhow!(
                "Circular dependency detected for {:?}",
                name
            ));
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
            match services.read() {
                Ok(guard) => guard.keys().cloned().collect(),
                Err(e) => {
                    error!("Failed to acquire services read lock: {}", e);
                    vec![]
                }
            }
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
            ServiceName::PostgreSQL => crate::services::pg::health_check().await.is_ok(),
            ServiceName::Redis => crate::services::redis::health_check().await.is_ok(),
            ServiceName::Docker => crate::services::docker::is_running_async().await.is_ok(),
            _ => true, // Default to healthy for unimplemented checks
        };

        // Clone the Arc parameters before acquiring locks (for potential restart task)
        let services_arc = services.clone();
        let configs_arc = configs.clone();

        let mut services_guard = services
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?;
        if let Some(health) = services_guard.get_mut(name) {
            health.last_check = Instant::now();

            if !healthy && matches!(health.status, ServiceStatus::Running) {
                health.status = ServiceStatus::Degraded("Health check failed".to_string());
                health.error_count += 1;

                // Check if we should restart
                let should_restart = {
                    let configs = configs.read().map_err(|e| {
                        anyhow::anyhow!("Failed to acquire configs read lock: {}", e)
                    })?;
                    configs
                        .get(name)
                        .map(|c| c.auto_restart && health.restart_count < c.max_restarts)
                        .unwrap_or(false)
                };

                if should_restart {
                    health.restart_count += 1;
                    warn!(
                        "Scheduling restart for unhealthy service {:?} (attempt {})",
                        name, health.restart_count
                    );

                    // Drop the write lock before spawning the restart task
                    let service_name = name.clone();
                    let services_clone = services_arc.clone();
                    let configs_clone = configs_arc.clone();

                    // Spawn restart task (simplified to avoid Send issues)
                    tokio::spawn(async move {
                        // TODO: Implement proper service restart when Send issues are resolved
                        warn!(
                            "Service {:?} requires restart but implementation is stubbed",
                            service_name
                        );
                    });
                }
            }
        }

        Ok(())
    }

    pub fn get_status(&self) -> HashMap<ServiceName, ServiceStatus> {
        match self.services.read() {
            Ok(guard) => guard
                .iter()
                .map(|(name, health)| (name.clone(), health.status.clone()))
                .collect(),
            Err(e) => {
                error!("Failed to acquire services read lock: {}", e);
                HashMap::new()
            }
        }
    }

    pub fn get_health(&self, name: &ServiceName) -> Option<ServiceHealth> {
        match self.services.read() {
            Ok(guard) => guard.get(name).cloned(),
            Err(e) => {
                error!("Failed to acquire services read lock: {}", e);
                None
            }
        }
    }

    pub fn get_all_health(&self) -> HashMap<ServiceName, ServiceHealth> {
        match self.services.read() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire services read lock: {}", e);
                HashMap::new()
            }
        }
    }

    /// Restart a service with dependency checking and health validation
    pub async fn restart_service(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
        configs: &Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
        reason: String,
    ) -> Result<()> {
        let start_time = Instant::now();
        info!("Initiating service restart for {:?}: {}", name, reason);

        // Check dependencies first
        if let Err(e) = Self::check_dependencies_ready(name, services, configs).await {
            error!("Dependency check failed for {:?}: {}", name, e);
            return Err(e);
        }

        // Stop the service
        if let Err(e) = Self::stop_service_internal(name, services).await {
            warn!("Error stopping service {:?} before restart: {}", name, e);
        }

        // Wait before restarting (exponential backoff is handled by RestartManager)
        sleep(Duration::from_secs(2)).await;

        // Start the service
        match Self::start_service_internal(name, services, configs).await {
            Ok(_) => {
                info!("Service {:?} restarted successfully", name);

                // Perform health check
                if let Err(e) = Self::validate_service_health(name, services).await {
                    error!(
                        "Health validation failed after restart for {:?}: {}",
                        name, e
                    );
                    return Err(e);
                }

                let duration = start_time.elapsed();
                info!(
                    "Service {:?} fully operational after restart (took {:?})",
                    name, duration
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to restart service {:?}: {}", name, e);
                Err(e)
            }
        }
    }

    /// Check if all dependencies are ready for a service
    async fn check_dependencies_ready(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
        configs: &Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
    ) -> Result<()> {
        let dependencies = {
            configs
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire configs read lock: {}", e))?
                .get(name)
                .map(|c| c.dependencies.clone())
                .unwrap_or_default()
        };

        let mut missing_deps = Vec::new();

        {
            let services_guard = services
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services read lock: {}", e))?;

            for dep in &dependencies {
                if let Some(health) = services_guard.get(dep) {
                    match &health.status {
                        ServiceStatus::Running => continue,
                        ServiceStatus::Degraded(_) => {
                            warn!("Dependency {:?} is degraded but allowing restart", dep);
                            continue;
                        }
                        _ => missing_deps.push(dep.clone()),
                    }
                } else {
                    missing_deps.push(dep.clone());
                }
            }
        }

        if !missing_deps.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot restart {:?}: dependencies not ready: {:?}",
                name,
                missing_deps
            ));
        }

        Ok(())
    }

    /// Validate service health after restart
    async fn validate_service_health(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
    ) -> Result<()> {
        let max_retries = 5;
        let retry_delay = Duration::from_secs(2);

        for attempt in 1..=max_retries {
            debug!("Health check attempt {} for {:?}", attempt, name);

            // Perform service-specific health check
            let healthy = match name {
                ServiceName::PostgreSQL => crate::services::pg::health_check().await.is_ok(),
                ServiceName::Redis => crate::services::redis::health_check().await.is_ok(),
                ServiceName::Docker => crate::services::docker::is_running_async().await.is_ok(),
                _ => {
                    // For services without specific health checks, check status
                    let status = services
                        .read()
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to acquire services read lock: {}", e)
                        })?
                        .get(name)
                        .map(|h| h.status.clone());

                    matches!(status, Some(ServiceStatus::Running))
                }
            };

            if healthy {
                // Update service health
                services
                    .write()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
                    .get_mut(name)
                    .map(|h| {
                        h.status = ServiceStatus::Running;
                        h.error_count = 0;
                    });

                return Ok(());
            }

            if attempt < max_retries {
                sleep(retry_delay).await;
            }
        }

        Err(anyhow::anyhow!(
            "Service {:?} failed health validation after {} attempts",
            name,
            max_retries
        ))
    }

    /// Internal method to stop a service
    async fn stop_service_internal(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
    ) -> Result<()> {
        info!("Stopping service: {:?}", name);

        // Update status
        {
            services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
                .get_mut(name)
                .map(|h| h.status = ServiceStatus::Stopping);
        }

        // Stop the actual service
        let result = match name {
            ServiceName::PostgreSQL => {
                if let Ok(_) = crate::services::docker::is_running_async().await {
                    crate::services::docker::stop_postgres().await
                } else {
                    Ok(())
                }
            }
            ServiceName::Redis => {
                if let Ok(_) = crate::services::docker::is_running_async().await {
                    crate::services::docker::stop_redis().await
                } else {
                    Ok(())
                }
            }
            ServiceName::Docker => Ok(()),
            ServiceName::Crawler => {
                crate::services::crawler::stop_service();
                Ok(())
            }
            ServiceName::P2P => crate::services::p2p::stop_network()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to stop P2P network: {}", e)),
            ServiceName::WebSocket => crate::websocket::stop_server()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to stop WebSocket server: {}", e)),
            _ => Ok(()),
        };

        // Update status
        {
            services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
                .get_mut(name)
                .map(|h| h.status = ServiceStatus::Stopped);
        }

        result
    }

    /// Internal method to start a service
    async fn start_service_internal(
        name: &ServiceName,
        services: &Arc<RwLock<HashMap<ServiceName, ServiceHealth>>>,
        configs: &Arc<RwLock<HashMap<ServiceName, ServiceConfig>>>,
    ) -> Result<()> {
        info!("Starting service: {:?}", name);

        // Update status
        {
            services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
                .get_mut(name)
                .map(|h| h.status = ServiceStatus::Starting);
        }

        // Start the actual service
        let result = match name {
            ServiceName::PostgreSQL => match crate::services::pg::connect().await {
                Ok(_) => {
                    info!("PostgreSQL is already running");
                    Ok(())
                }
                Err(_) => {
                    if let Ok(_) = crate::services::docker::is_running_async().await {
                        info!("Starting PostgreSQL via Docker");
                        crate::services::docker::start_postgres().await
                    } else {
                        Err(anyhow::anyhow!("Docker not available to start PostgreSQL"))
                    }
                }
            },
            ServiceName::Redis => match crate::services::redis::connect().await {
                Ok(_) => {
                    info!("Redis is already running");
                    Ok(())
                }
                Err(_) => {
                    if let Ok(_) = crate::services::docker::is_running_async().await {
                        info!("Starting Redis via Docker");
                        crate::services::docker::start_redis().await
                    } else {
                        Err(anyhow::anyhow!("Docker not available to start Redis"))
                    }
                }
            },
            ServiceName::Docker => crate::services::docker::ensure_running().await,
            ServiceName::FileStorage => crate::services::fs::initialize().await,
            ServiceName::Backup => crate::services::backup::start_scheduler().await,
            ServiceName::SSH => Ok(()),
            ServiceName::Crawler => {
                crate::services::crawler::start_service_async().await;
                Ok(())
            }
            ServiceName::Voice => crate::services::voice::initialize().await,
            ServiceName::P2P => crate::services::p2p::start_network()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start P2P network: {}", e)),
            ServiceName::VulnerabilityScanner => Ok(()),
            ServiceName::Whisper => crate::services::stt::whisper_enhanced::initialize().await,
            ServiceName::Lifx => crate::services::lifx::start_server().await,
            ServiceName::Media => crate::services::media::initialize().await,
            ServiceName::WebSocket => crate::websocket::start_server()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start WebSocket server: {}", e)),
            ServiceName::MDNS => {
                let output_lines = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                crate::services::mdns::start_discovery(output_lines).await;
                Ok(())
            }
            _ => {
                warn!("Service {:?} not yet implemented", name);
                Ok(())
            }
        };

        // Update status based on result
        {
            services
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire services write lock: {}", e))?
                .get_mut(name)
                .map(|h| match &result {
                    Ok(_) => {
                        h.status = ServiceStatus::Running;
                        h.uptime = Duration::from_secs(0);
                        info!("Service {:?} started successfully", name);
                    }
                    Err(e) => {
                        h.status = ServiceStatus::Failed(e.to_string());
                        h.error_count += 1;
                        error!("Service {:?} failed to start: {}", name, e);
                    }
                });
        }

        result
    }

    /// Get restart metrics for all services
    pub fn get_restart_metrics(&self) -> HashMap<ServiceName, super::restart::RestartMetrics> {
        self.restart_manager.get_all_metrics()
    }

    /// Get restart metrics for a specific service
    pub fn get_service_restart_metrics(
        &self,
        name: &ServiceName,
    ) -> Option<super::restart::RestartMetrics> {
        self.restart_manager.get_metrics(name)
    }

    /// Reset restart metrics for a service
    pub fn reset_restart_metrics(&self, name: &ServiceName) -> Result<()> {
        self.restart_manager.reset_metrics(name)
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

/// Petgraph-based dependency graph for service startup ordering.
///
/// Builds a directed acyclic graph from service configs and uses
/// topological sort to determine correct startup order (dependencies first).
pub struct ServiceDependencyGraph {
    graph: DiGraph<ServiceName, ()>,
    node_map: HashMap<ServiceName, petgraph::graph::NodeIndex>,
}

impl ServiceDependencyGraph {
    /// Build a dependency graph from service configurations.
    pub fn from_configs(configs: &[ServiceConfig]) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add all services as nodes
        for config in configs {
            let idx = graph.add_node(config.name.clone());
            node_map.insert(config.name.clone(), idx);
        }

        // Add dependency edges (dependency → dependent)
        for config in configs {
            if let Some(&dependent_idx) = node_map.get(&config.name) {
                for dep in &config.dependencies {
                    if let Some(&dep_idx) = node_map.get(dep) {
                        graph.add_edge(dep_idx, dependent_idx, ());
                    }
                }
            }
        }

        Self { graph, node_map }
    }

    /// Return services in topological startup order (dependencies first).
    /// Returns Err if a cycle is detected.
    pub fn startup_order(&self) -> Result<Vec<ServiceName>> {
        match toposort(&self.graph, None) {
            Ok(indices) => Ok(indices
                .into_iter()
                .map(|idx| self.graph[idx].clone())
                .collect()),
            Err(cycle) => Err(anyhow::anyhow!(
                "Circular dependency detected involving {:?}",
                self.graph[cycle.node_id()]
            )),
        }
    }

    /// Return services in reverse topological order (for shutdown).
    pub fn shutdown_order(&self) -> Result<Vec<ServiceName>> {
        let mut order = self.startup_order()?;
        order.reverse();
        Ok(order)
    }

    /// Get direct dependencies of a service.
    pub fn dependencies_of(&self, name: &ServiceName) -> Vec<ServiceName> {
        if let Some(&idx) = self.node_map.get(name) {
            self.graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .map(|dep_idx| self.graph[dep_idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get services that depend on the given service.
    pub fn dependents_of(&self, name: &ServiceName) -> Vec<ServiceName> {
        if let Some(&idx) = self.node_map.get(name) {
            self.graph
                .neighbors_directed(idx, petgraph::Direction::Outgoing)
                .map(|dep_idx| self.graph[dep_idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
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
        orchestrator
            .register_service(ServiceConfig {
                name: ServiceName::PostgreSQL,
                enabled: true,
                dependencies: vec![],
                ..Default::default()
            })
            .expect("Failed to register service");

        orchestrator
            .register_service(ServiceConfig {
                name: ServiceName::FileStorage,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL],
                ..Default::default()
            })
            .expect("Failed to register service");

        orchestrator
            .register_service(ServiceConfig {
                name: ServiceName::Backup,
                enabled: true,
                dependencies: vec![ServiceName::FileStorage],
                ..Default::default()
            })
            .expect("Failed to register service");

        let order = orchestrator
            .get_startup_order()
            .expect("Failed to get startup order");

        // PostgreSQL should start before FileStorage
        let pg_index = order
            .iter()
            .position(|s| *s == ServiceName::PostgreSQL)
            .expect("PostgreSQL not found in startup order");
        let fs_index = order
            .iter()
            .position(|s| *s == ServiceName::FileStorage)
            .expect("FileStorage not found in startup order");
        let backup_index = order
            .iter()
            .position(|s| *s == ServiceName::Backup)
            .expect("Backup not found in startup order");

        assert!(pg_index < fs_index);
        assert!(fs_index < backup_index);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let orchestrator = ServiceOrchestrator::new();

        // Create circular dependency
        orchestrator
            .register_service(ServiceConfig {
                name: ServiceName::Redis,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL],
                ..Default::default()
            })
            .expect("Failed to register service");

        orchestrator
            .register_service(ServiceConfig {
                name: ServiceName::PostgreSQL,
                enabled: true,
                dependencies: vec![ServiceName::Redis],
                ..Default::default()
            })
            .expect("Failed to register service");

        assert!(orchestrator.get_startup_order().is_err());
    }

    #[test]
    fn test_dependency_graph_startup_order() {
        let configs = vec![
            ServiceConfig {
                name: ServiceName::PostgreSQL,
                enabled: true,
                dependencies: vec![],
                ..Default::default()
            },
            ServiceConfig {
                name: ServiceName::Redis,
                enabled: true,
                dependencies: vec![],
                ..Default::default()
            },
            ServiceConfig {
                name: ServiceName::Crawler,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL, ServiceName::Redis],
                ..Default::default()
            },
        ];

        let graph = ServiceDependencyGraph::from_configs(&configs);
        let order = graph.startup_order().expect("Should produce valid order");

        let pg_pos = order
            .iter()
            .position(|s| *s == ServiceName::PostgreSQL)
            .unwrap();
        let redis_pos = order.iter().position(|s| *s == ServiceName::Redis).unwrap();
        let crawler_pos = order
            .iter()
            .position(|s| *s == ServiceName::Crawler)
            .unwrap();

        assert!(pg_pos < crawler_pos);
        assert!(redis_pos < crawler_pos);
    }

    #[test]
    fn test_dependency_graph_circular_detection() {
        let configs = vec![
            ServiceConfig {
                name: ServiceName::Redis,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL],
                ..Default::default()
            },
            ServiceConfig {
                name: ServiceName::PostgreSQL,
                enabled: true,
                dependencies: vec![ServiceName::Redis],
                ..Default::default()
            },
        ];

        let graph = ServiceDependencyGraph::from_configs(&configs);
        assert!(graph.startup_order().is_err());
    }

    #[test]
    fn test_dependency_graph_dependents() {
        let configs = vec![
            ServiceConfig {
                name: ServiceName::PostgreSQL,
                enabled: true,
                dependencies: vec![],
                ..Default::default()
            },
            ServiceConfig {
                name: ServiceName::FileStorage,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL],
                ..Default::default()
            },
            ServiceConfig {
                name: ServiceName::Backup,
                enabled: true,
                dependencies: vec![ServiceName::PostgreSQL],
                ..Default::default()
            },
        ];

        let graph = ServiceDependencyGraph::from_configs(&configs);
        let dependents = graph.dependents_of(&ServiceName::PostgreSQL);
        assert_eq!(dependents.len(), 2);
    }
}

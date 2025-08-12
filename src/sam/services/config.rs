use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub enum ConfigSource {
    File(PathBuf),
    Environment,
    Memory(String),
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
    NotFound(String),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ConfigError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}

pub trait ConfigLoader {
    fn load(&self, source: &ConfigSource) -> Result<String, ConfigError>;
}

pub struct FileConfigLoader;

impl ConfigLoader for FileConfigLoader {
    fn load(&self, source: &ConfigSource) -> Result<String, ConfigError> {
        match source {
            ConfigSource::File(path) => {
                if !path.exists() {
                    return Err(ConfigError::NotFound(format!("File not found: {:?}", path)));
                }
                Ok(fs::read_to_string(path)?)
            }
            ConfigSource::Environment => {
                Err(ConfigError::NotFound("Environment source not supported by FileConfigLoader".to_string()))
            }
            ConfigSource::Memory(content) => Ok(content.clone()),
        }
    }
}

pub trait ConfigParser {
    fn parse<T: for<'de> Deserialize<'de>>(&self, content: &str) -> Result<T, ConfigError>;
}

pub struct JsonConfigParser;

impl ConfigParser for JsonConfigParser {
    fn parse<T: for<'de> Deserialize<'de>>(&self, content: &str) -> Result<T, ConfigError> {
        Ok(serde_json::from_str(content)?)
    }
}

pub struct TomlConfigParser;

impl ConfigParser for TomlConfigParser {
    fn parse<T: for<'de> Deserialize<'de>>(&self, content: &str) -> Result<T, ConfigError> {
        Ok(toml::from_str(content)?)
    }
}

pub struct YamlConfigParser;

impl ConfigParser for YamlConfigParser {
    fn parse<T: for<'de> Deserialize<'de>>(&self, content: &str) -> Result<T, ConfigError> {
        Ok(serde_yaml::from_str(content)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub environment: String,
    pub log_level: String,
    pub services: HashMap<String, ServiceConfig>,
    pub database: Option<DatabaseConfig>,
    pub cache: Option<CacheConfig>,
    pub monitoring: Option<MonitoringConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub provider: String,
    pub url: Option<String>,
    pub ttl_seconds: u64,
    pub max_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_port: u16,
    pub health_check_interval_seconds: u64,
}

pub struct ConfigManager {
    config: Arc<RwLock<GlobalConfig>>,
    watchers: Vec<Box<dyn ConfigWatcher>>,
}

impl ConfigManager {
    pub fn new(config: GlobalConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            watchers: Vec::new(),
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config = Self::parse_config(&content, path)?;
        Ok(Self::new(config))
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let config = GlobalConfig {
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            services: HashMap::new(),
            database: Self::database_from_env(),
            cache: Self::cache_from_env(),
            monitoring: Self::monitoring_from_env(),
        };
        Ok(Self::new(config))
    }

    fn parse_config(content: &str, path: &Path) -> Result<GlobalConfig, ConfigError> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| ConfigError::Parse("Unknown file extension".to_string()))?;

        match extension {
            "json" => JsonConfigParser.parse(content),
            "toml" => TomlConfigParser.parse(content),
            "yaml" | "yml" => YamlConfigParser.parse(content),
            _ => Err(ConfigError::Parse(format!("Unsupported config format: {}", extension))),
        }
    }

    fn database_from_env() -> Option<DatabaseConfig> {
        env::var("DATABASE_URL").ok().map(|url| DatabaseConfig {
            url,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            connection_timeout_seconds: env::var("DATABASE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        })
    }

    fn cache_from_env() -> Option<CacheConfig> {
        env::var("CACHE_PROVIDER").ok().map(|provider| CacheConfig {
            provider,
            url: env::var("CACHE_URL").ok(),
            ttl_seconds: env::var("CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            max_size: env::var("CACHE_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
        })
    }

    fn monitoring_from_env() -> Option<MonitoringConfig> {
        env::var("MONITORING_ENABLED")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .filter(|&enabled| enabled)
            .map(|_| MonitoringConfig {
                enabled: true,
                metrics_port: env::var("METRICS_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(9090),
                health_check_interval_seconds: env::var("HEALTH_CHECK_INTERVAL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30),
            })
    }

    pub fn get(&self) -> Arc<RwLock<GlobalConfig>> {
        self.config.clone()
    }

    pub fn get_service_config(&self, service_name: &str) -> Option<ServiceConfig> {
        self.config.read().ok()?.services.get(service_name).cloned()
    }

    pub fn update_service_config(&self, service_name: &str, config: ServiceConfig) -> Result<(), ConfigError> {
        let mut global_config = self.config.write()
            .map_err(|_| ConfigError::Parse("Failed to acquire write lock".to_string()))?;
        global_config.services.insert(service_name.to_string(), config);
        Ok(())
    }

    pub fn reload(&mut self, source: ConfigSource) -> Result<(), ConfigError> {
        let loader = FileConfigLoader;
        let content = loader.load(&source)?;
        
        let new_config = match source {
            ConfigSource::File(ref path) => Self::parse_config(&content, path)?,
            _ => JsonConfigParser.parse(&content)?,
        };

        *self.config.write()
            .map_err(|_| ConfigError::Parse("Failed to acquire write lock".to_string()))? = new_config;

        // Notify watchers
        for watcher in &self.watchers {
            watcher.on_config_change();
        }

        Ok(())
    }

    pub fn add_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) {
        self.watchers.push(watcher);
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let config = self.config.read()
            .map_err(|_| ConfigError::Parse("Failed to acquire read lock".to_string()))?;

        if config.environment.is_empty() {
            return Err(ConfigError::Validation("Environment cannot be empty".to_string()));
        }

        if let Some(ref db_config) = config.database {
            if db_config.url.is_empty() {
                return Err(ConfigError::Validation("Database URL cannot be empty".to_string()));
            }
            if db_config.max_connections == 0 {
                return Err(ConfigError::Validation("Max connections must be greater than 0".to_string()));
            }
        }

        Ok(())
    }
}

pub trait ConfigWatcher: Send + Sync {
    fn on_config_change(&self);
}

pub struct LoggingConfigWatcher;

impl ConfigWatcher for LoggingConfigWatcher {
    fn on_config_change(&self) {
        log::info!("Configuration has been reloaded");
    }
}

#[derive(Clone)]
pub struct ConfigBuilder {
    environment: Option<String>,
    log_level: Option<String>,
    services: HashMap<String, ServiceConfig>,
    database: Option<DatabaseConfig>,
    cache: Option<CacheConfig>,
    monitoring: Option<MonitoringConfig>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            environment: None,
            log_level: None,
            services: HashMap::new(),
            database: None,
            cache: None,
            monitoring: None,
        }
    }

    pub fn environment(mut self, env: String) -> Self {
        self.environment = Some(env);
        self
    }

    pub fn log_level(mut self, level: String) -> Self {
        self.log_level = Some(level);
        self
    }

    pub fn add_service(mut self, name: String, config: ServiceConfig) -> Self {
        self.services.insert(name, config);
        self
    }

    pub fn database(mut self, config: DatabaseConfig) -> Self {
        self.database = Some(config);
        self
    }

    pub fn cache(mut self, config: CacheConfig) -> Self {
        self.cache = Some(config);
        self
    }

    pub fn monitoring(mut self, config: MonitoringConfig) -> Self {
        self.monitoring = Some(config);
        self
    }

    pub fn build(self) -> GlobalConfig {
        GlobalConfig {
            environment: self.environment.unwrap_or_else(|| "development".to_string()),
            log_level: self.log_level.unwrap_or_else(|| "info".to_string()),
            services: self.services,
            database: self.database,
            cache: self.cache,
            monitoring: self.monitoring,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .environment("production".to_string())
            .log_level("debug".to_string())
            .build();

        assert_eq!(config.environment, "production");
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_config_validation() {
        let config = GlobalConfig {
            environment: "".to_string(),
            log_level: "info".to_string(),
            services: HashMap::new(),
            database: None,
            cache: None,
            monitoring: None,
        };

        let manager = ConfigManager::new(config);
        assert!(manager.validate().is_err());
    }
}
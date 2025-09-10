//! Configuration file support for the crawler
//!
//! This module provides YAML and TOML configuration file support for the crawler,
//! allowing users to configure all aspects of crawling behavior through config files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use anyhow::{Result, Context};

/// Main crawler configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct CrawlerConfig {
    /// General crawler settings
    pub general: GeneralConfig,
    
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    
    /// Memory optimization settings
    pub memory: MemoryConfig,
    
    /// Content storage settings
    pub storage: StorageConfig,
    
    /// JavaScript rendering settings
    pub javascript: JavaScriptConfig,
    
    /// User agent configuration
    pub user_agents: UserAgentConfig,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Redis configuration
    pub redis: RedisConfig,
    
    /// Webhook configuration
    pub webhooks: WebhookConfig,
    
    /// URL pattern filters
    pub patterns: PatternConfig,
    
    /// Domain-specific configurations
    pub domains: HashMap<String, DomainConfig>,
}

/// General crawler settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Maximum crawl depth
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    
    /// Maximum pages to crawl per job
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    
    /// Number of concurrent crawlers
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    
    /// Request timeout in seconds
    #[serde(with = "humantime_serde", default = "default_request_timeout")]
    pub request_timeout: Duration,
    
    /// Whether to follow redirects
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    
    /// Maximum redirects to follow
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,
    
    /// Whether to respect robots.txt
    #[serde(default = "default_true")]
    pub respect_robots_txt: bool,
    
    /// Default user agent
    #[serde(default = "default_user_agent")]
    pub default_user_agent: String,
    
    /// Whether to extract sitemaps
    #[serde(default = "default_true")]
    pub extract_sitemaps: bool,
    
    /// Whether to detect and parse feeds
    #[serde(default = "default_true")]
    pub detect_feeds: bool,
    
    /// Allowed schemes (http, https, etc.)
    #[serde(default = "default_allowed_schemes")]
    pub allowed_schemes: Vec<String>,
    
    /// DNS resolver configuration
    #[serde(default = "default_dns_resolver")]
    pub dns_resolver: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Default delay between requests in milliseconds
    #[serde(with = "humantime_serde", default = "default_delay")]
    pub default_delay: Duration,
    
    /// Minimum delay between requests
    #[serde(with = "humantime_serde", default = "default_min_delay")]
    pub min_delay: Duration,
    
    /// Maximum delay between requests
    #[serde(with = "humantime_serde", default = "default_max_delay")]
    pub max_delay: Duration,
    
    /// Requests per second limit
    #[serde(default = "default_rps")]
    pub requests_per_second: f64,
    
    /// Burst size for rate limiting
    #[serde(default = "default_burst_size")]
    pub burst_size: usize,
    
    /// Whether to use adaptive rate limiting
    #[serde(default = "default_true")]
    pub adaptive: bool,
    
    /// Whether to respect Crawl-Delay from robots.txt
    #[serde(default = "default_true")]
    pub respect_crawl_delay: bool,
    
    /// Whether to respect Retry-After headers
    #[serde(default = "default_true")]
    pub respect_retry_after: bool,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures to open circuit
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    
    /// Initial backoff duration
    #[serde(with = "humantime_serde", default = "default_initial_backoff")]
    pub initial_backoff: Duration,
    
    /// Maximum backoff duration
    #[serde(with = "humantime_serde", default = "default_max_backoff")]
    pub max_backoff: Duration,
    
    /// Duration to wait in open state
    #[serde(with = "humantime_serde", default = "default_open_duration")]
    pub open_duration: Duration,
    
    /// Successes needed in half-open to close
    #[serde(default = "default_half_open_threshold")]
    pub half_open_success_threshold: u32,
}

/// Memory optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Bloom filter size
    #[serde(default = "default_bloom_filter_size")]
    pub bloom_filter_size: usize,
    
    /// Bloom filter false positive rate
    #[serde(default = "default_bloom_filter_fp_rate")]
    pub bloom_filter_fp_rate: f64,
    
    /// LRU cache size
    #[serde(default = "default_lru_cache_size")]
    pub lru_cache_size: usize,
    
    /// Maximum queue size
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,
    
    /// Enable Redis spillover
    #[serde(default = "default_false")]
    pub enable_redis_spillover: bool,
}

/// Content storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Whether to store page content
    #[serde(default = "default_true")]
    pub store_content: bool,
    
    /// Whether to compress content
    #[serde(default = "default_true")]
    pub compress_content: bool,
    
    /// Whether to deduplicate content
    #[serde(default = "default_true")]
    pub deduplicate: bool,
    
    /// Whether to extract text from PDFs
    #[serde(default = "default_true")]
    pub extract_pdf_text: bool,
    
    /// Whether to extract image metadata
    #[serde(default = "default_true")]
    pub extract_image_metadata: bool,
    
    /// Maximum content size to store (in MB)
    #[serde(default = "default_max_content_size")]
    pub max_content_size_mb: usize,
    
    /// Content types to store
    #[serde(default = "default_stored_content_types")]
    pub stored_content_types: Vec<String>,
}

/// JavaScript rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaScriptConfig {
    /// Whether JavaScript rendering is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// Browser engine to use (chrome, firefox, safari)
    #[serde(default = "default_browser_engine")]
    pub engine: String,
    
    /// Run in headless mode
    #[serde(default = "default_true")]
    pub headless: bool,
    
    /// Page load timeout in seconds
    #[serde(with = "humantime_serde", default = "default_js_timeout")]
    pub timeout: Duration,
    
    /// Wait for network idle
    #[serde(default = "default_true")]
    pub wait_for_network_idle: bool,
    
    /// Maximum concurrent browsers
    #[serde(default = "default_max_browsers")]
    pub max_browsers: usize,
    
    /// Viewport width
    #[serde(default = "default_viewport_width")]
    pub viewport_width: u32,
    
    /// Viewport height
    #[serde(default = "default_viewport_height")]
    pub viewport_height: u32,
    
    /// Resource types to block
    #[serde(default = "default_blocked_resources")]
    pub blocked_resources: Vec<String>,
    
    /// Custom JavaScript to execute
    #[serde(default)]
    pub custom_scripts: Vec<String>,
    
    /// Enable browser cache
    #[serde(default = "default_false")]
    pub enable_cache: bool,
    
    /// Enable cookies
    #[serde(default = "default_true")]
    pub enable_cookies: bool,
}

/// User agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgentConfig {
    /// Rotation strategy (fixed, random, round_robin, per_domain, content_aware)
    #[serde(default = "default_rotation_strategy")]
    pub rotation_strategy: String,
    
    /// Custom user agents
    #[serde(default)]
    pub custom_agents: Vec<String>,
    
    /// Enable desktop agents
    #[serde(default = "default_true")]
    pub enable_desktop: bool,
    
    /// Enable mobile agents
    #[serde(default = "default_false")]
    pub enable_mobile: bool,
    
    /// Enable bot agents
    #[serde(default = "default_false")]
    pub enable_bots: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    #[serde(default = "default_database_url")]
    pub url: String,
    
    /// Maximum connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    #[serde(with = "humantime_serde", default = "default_connection_timeout")]
    pub connection_timeout: Duration,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    #[serde(default = "default_redis_url")]
    pub url: String,
    
    /// Maximum connections
    #[serde(default = "default_redis_max_connections")]
    pub max_connections: u32,
    
    /// Enable for job queue
    #[serde(default = "default_true")]
    pub enable_job_queue: bool,
    
    /// Enable for DNS cache
    #[serde(default = "default_true")]
    pub enable_dns_cache: bool,
    
    /// Enable for URL tracking
    #[serde(default = "default_false")]
    pub enable_url_tracking: bool,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Whether webhooks are enabled
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// Webhook endpoints
    #[serde(default)]
    pub endpoints: Vec<WebhookEndpoint>,
    
    /// Events to send
    #[serde(default = "default_webhook_events")]
    pub events: Vec<String>,
    
    /// Include statistics in payload
    #[serde(default = "default_true")]
    pub include_stats: bool,
    
    /// Retry attempts for failed webhooks
    #[serde(default = "default_webhook_retries")]
    pub retry_attempts: u32,
    
    /// HMAC secret for signing
    #[serde(default)]
    pub hmac_secret: Option<String>,
}

/// Webhook endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    /// Endpoint URL
    pub url: String,
    
    /// Events for this endpoint
    #[serde(default)]
    pub events: Vec<String>,
    
    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// URL pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    /// URL patterns to include
    #[serde(default)]
    pub include_patterns: Vec<String>,
    
    /// URL patterns to exclude
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    
    /// File extensions to include
    #[serde(default = "default_included_extensions")]
    pub include_extensions: Vec<String>,
    
    /// File extensions to exclude
    #[serde(default)]
    pub exclude_extensions: Vec<String>,
    
    /// Maximum pagination depth
    #[serde(default = "default_max_pagination")]
    pub max_pagination_depth: usize,
    
    /// Detect infinite patterns
    #[serde(default = "default_true")]
    pub detect_infinite_patterns: bool,
    
    /// Calendar pattern threshold
    #[serde(default = "default_calendar_threshold")]
    pub calendar_threshold: usize,
}

/// Domain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Override rate limit for this domain
    #[serde(with = "humantime_serde", default)]
    pub rate_limit: Option<Duration>,
    
    /// Override max depth for this domain
    #[serde(default)]
    pub max_depth: Option<usize>,
    
    /// Custom user agent for this domain
    #[serde(default)]
    pub user_agent: Option<String>,
    
    /// Enable JavaScript for this domain
    #[serde(default)]
    pub enable_javascript: Option<bool>,
    
    /// Custom headers for this domain
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// Authentication for this domain
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type (basic, bearer, cookie, oauth)
    pub auth_type: String,
    
    /// Username for basic auth
    #[serde(default)]
    pub username: Option<String>,
    
    /// Password for basic auth
    #[serde(default)]
    pub password: Option<String>,
    
    /// Bearer token
    #[serde(default)]
    pub token: Option<String>,
    
    /// Cookies
    #[serde(default)]
    pub cookies: HashMap<String, String>,
    
    /// OAuth configuration
    #[serde(default)]
    pub oauth: Option<OAuthConfig>,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Client ID
    pub client_id: String,
    
    /// Client secret
    pub client_secret: String,
    
    /// Authorization URL
    pub auth_url: String,
    
    /// Token URL
    pub token_url: String,
    
    /// Scopes
    #[serde(default)]
    pub scopes: Vec<String>,
}

// Default value functions
fn default_max_depth() -> usize { 10 }
fn default_max_pages() -> usize { 10000 }
fn default_concurrency() -> usize { 10 }
fn default_request_timeout() -> Duration { Duration::from_secs(30) }
fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_redirects() -> usize { 5 }
fn default_user_agent() -> String { "Mozilla/5.0 (compatible; SAMBot/1.0)".to_string() }
fn default_allowed_schemes() -> Vec<String> { vec!["http".to_string(), "https".to_string()] }
fn default_dns_resolver() -> String { "8.8.8.8:53".to_string() }
fn default_delay() -> Duration { Duration::from_millis(1000) }
fn default_min_delay() -> Duration { Duration::from_millis(100) }
fn default_max_delay() -> Duration { Duration::from_secs(60) }
fn default_rps() -> f64 { 10.0 }
fn default_burst_size() -> usize { 5 }
fn default_failure_threshold() -> u32 { 5 }
fn default_initial_backoff() -> Duration { Duration::from_secs(60) }
fn default_max_backoff() -> Duration { Duration::from_secs(3600) }
fn default_open_duration() -> Duration { Duration::from_secs(300) }
fn default_half_open_threshold() -> u32 { 3 }
fn default_bloom_filter_size() -> usize { 1000000 }
fn default_bloom_filter_fp_rate() -> f64 { 0.01 }
fn default_lru_cache_size() -> usize { 10000 }
fn default_max_queue_size() -> usize { 100000 }
fn default_max_content_size() -> usize { 50 }
fn default_stored_content_types() -> Vec<String> {
    vec![
        "text/html".to_string(),
        "text/plain".to_string(),
        "application/json".to_string(),
        "application/xml".to_string(),
        "application/pdf".to_string(),
        "image/jpeg".to_string(),
        "image/png".to_string(),
        "image/gif".to_string(),
        "image/webp".to_string(),
        "image/svg+xml".to_string(),
    ]
}
fn default_browser_engine() -> String { "chrome".to_string() }
fn default_js_timeout() -> Duration { Duration::from_secs(30) }
fn default_max_browsers() -> usize { 3 }
fn default_viewport_width() -> u32 { 1920 }
fn default_viewport_height() -> u32 { 1080 }
fn default_blocked_resources() -> Vec<String> {
    vec!["image".to_string(), "font".to_string(), "media".to_string()]
}
fn default_rotation_strategy() -> String { "random".to_string() }
fn default_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| 
        "postgresql://sam:sam@localhost/sam".to_string()
    )
}
fn default_max_connections() -> u32 { 20 }
fn default_connection_timeout() -> Duration { Duration::from_secs(5) }
fn default_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| 
        "redis://localhost:6379".to_string()
    )
}
fn default_redis_max_connections() -> u32 { 10 }
fn default_webhook_events() -> Vec<String> {
    vec!["start".to_string(), "complete".to_string(), "fail".to_string()]
}
fn default_webhook_retries() -> u32 { 3 }
fn default_included_extensions() -> Vec<String> {
    vec![
        "html".to_string(), "htm".to_string(), "php".to_string(),
        "asp".to_string(), "aspx".to_string(), "jsp".to_string(),
    ]
}
fn default_max_pagination() -> usize { 20 }
fn default_calendar_threshold() -> usize { 10 }


impl Default for GeneralConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for JavaScriptConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for UserAgentConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl Default for PatternConfig {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

impl CrawlerConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        let config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
        } else {
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {:?}", path))?
        };
        
        Ok(config)
    }
    
    /// Save configuration to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        let content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self)
                .context("Failed to serialize config to TOML")?
        } else {
            serde_yaml::to_string(self)
                .context("Failed to serialize config to YAML")?
        };
        
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        
        Ok(())
    }
    
    /// Load configuration from default locations
    pub fn load() -> Result<Self> {
        // Try to load from various default locations
        let config_paths = vec![
            PathBuf::from("crawler.yaml"),
            PathBuf::from("crawler.toml"),
            PathBuf::from("config/crawler.yaml"),
            PathBuf::from("config/crawler.toml"),
            PathBuf::from("/etc/sam/crawler.yaml"),
            PathBuf::from("/etc/sam/crawler.toml"),
        ];
        
        for path in config_paths {
            if path.exists() {
                log::info!("Loading crawler config from: {:?}", path);
                return Self::from_file(path);
            }
        }
        
        // No config file found, use defaults
        log::info!("No config file found, using default configuration");
        Ok(Self::default())
    }
    
    /// Merge with another config (other takes precedence)
    pub fn merge(self, other: Self) -> Self {
        // This is simplified - in production you'd want proper merging logic
        other
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate general settings
        if self.general.max_depth == 0 {
            anyhow::bail!("max_depth must be greater than 0");
        }
        
        if self.general.concurrency == 0 {
            anyhow::bail!("concurrency must be greater than 0");
        }
        
        // Validate rate limiting
        if self.rate_limiting.requests_per_second <= 0.0 {
            anyhow::bail!("requests_per_second must be positive");
        }
        
        if self.rate_limiting.min_delay > self.rate_limiting.max_delay {
            anyhow::bail!("min_delay cannot be greater than max_delay");
        }
        
        // Validate circuit breaker
        if self.circuit_breaker.failure_threshold == 0 {
            anyhow::bail!("failure_threshold must be greater than 0");
        }
        
        // Validate memory settings
        if self.memory.bloom_filter_fp_rate <= 0.0 || self.memory.bloom_filter_fp_rate >= 1.0 {
            anyhow::bail!("bloom_filter_fp_rate must be between 0 and 1");
        }
        
        Ok(())
    }
    
    /// Apply this configuration to the crawler
    pub async fn apply(&self) -> Result<()> {
        // Apply rate limiting config
        let rate_config = super::rate_limiter::RateLimitConfig {
            default_delay_ms: self.rate_limiting.default_delay.as_millis() as u64,
            min_delay_ms: self.rate_limiting.min_delay.as_millis() as u64,
            max_delay_ms: self.rate_limiting.max_delay.as_millis() as u64,
            slow_response_factor: 1.5,  // Default factor
            fast_response_factor: 0.9,  // Default factor
            slow_threshold_ms: 2000,  // 2 seconds is slow
            fast_threshold_ms: 500,   // 500ms is fast
            max_concurrent_per_domain: 2,  // Default concurrent requests
        };
        super::rate_limiter::init_rate_limiter(rate_config, Some(self.rate_limiting.requests_per_second as u32));
        
        // Apply JavaScript config if enabled
        if self.javascript.enabled {
            let js_config = super::js_renderer::JsRendererConfig {
                engine: match self.javascript.engine.as_str() {
                    "firefox" => super::js_renderer::BrowserEngine::Firefox,
                    "safari" => super::js_renderer::BrowserEngine::Safari,
                    _ => super::js_renderer::BrowserEngine::Chrome,
                },
                headless: self.javascript.headless,
                timeout: self.javascript.timeout,
                wait_for_network_idle: self.javascript.wait_for_network_idle,
                max_browsers: self.javascript.max_browsers,
                user_agent: None,
                viewport_width: self.javascript.viewport_width,
                viewport_height: self.javascript.viewport_height,
                blocked_resources: self.javascript.blocked_resources.iter()
                    .filter_map(|s| match s.as_str() {
                        "image" => Some(super::js_renderer::ResourceType::Image),
                        "font" => Some(super::js_renderer::ResourceType::Font),
                        "media" => Some(super::js_renderer::ResourceType::Media),
                        "script" => Some(super::js_renderer::ResourceType::Script),
                        "stylesheet" => Some(super::js_renderer::ResourceType::Stylesheet),
                        _ => None,
                    })
                    .collect(),
                custom_scripts: self.javascript.custom_scripts.clone(),
                enable_cache: self.javascript.enable_cache,
                enable_cookies: self.javascript.enable_cookies,
                proxy: None,
            };
            super::js_renderer::initialize_js_renderer(js_config).await?;
        }
        
        log::info!("Crawler configuration applied successfully");
        Ok(())
    }
}

/// Global configuration instance
static GLOBAL_CONFIG: once_cell::sync::Lazy<std::sync::Arc<tokio::sync::RwLock<CrawlerConfig>>> = 
    once_cell::sync::Lazy::new(|| {
        std::sync::Arc::new(tokio::sync::RwLock::new(CrawlerConfig::default()))
    });

/// Get the global configuration
pub async fn get_config() -> CrawlerConfig {
    GLOBAL_CONFIG.read().await.clone()
}

/// Set the global configuration
pub async fn set_config(config: CrawlerConfig) -> Result<()> {
    config.validate()?;
    config.apply().await?;
    *GLOBAL_CONFIG.write().await = config;
    Ok(())
}

/// Load and apply configuration from file
pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = CrawlerConfig::from_file(path)?;
    set_config(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CrawlerConfig::default();
        assert_eq!(config.general.max_depth, 10);
        assert_eq!(config.general.concurrency, 10);
        assert!(config.general.respect_robots_txt);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = CrawlerConfig::default();
        assert!(config.validate().is_ok());
        
        config.general.max_depth = 0;
        assert!(config.validate().is_err());
        
        config.general.max_depth = 10;
        config.rate_limiting.requests_per_second = -1.0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_yaml_serialization() {
        let config = CrawlerConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: CrawlerConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.general.max_depth, config.general.max_depth);
    }
    
    #[test]
    fn test_toml_serialization() {
        let config = CrawlerConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: CrawlerConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.general.max_depth, config.general.max_depth);
    }
}
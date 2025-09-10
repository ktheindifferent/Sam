//! # Crawler Runner Module
//!
//! This module implements the core logic for the distributed web crawler service in the SAM system.
//! It manages crawl jobs, performs concurrent crawling, handles DNS and HTTP lookups, and manages caching using both Redis and file-based fallbacks.
//!
//! ## Features
//! - Distributed, concurrent crawling of web pages and domains
//! - DNS and HTTP(S) probing with caching
//! - Job queueing, status tracking, and retry logic
//! - Robust error handling and logging
//! - Pluggable cache backend (Redis or file)
//! - Token and link extraction from crawled pages
//!
//! ## Design
//! The crawler is designed to be robust and scalable, supporting multiple concurrent workers and fault-tolerant job processing. It uses a combination of static data (common URLs, TLDs, prefixes, etc.) and dynamic job queues to discover and crawl new domains. DNS and HTTP lookups are cached to minimize redundant network requests. The system is designed to recover from errors and persist retry information for failed crawls.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use log::{info, warn};
use once_cell::sync::Lazy;
use rand::distributions::Alphanumeric;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng, SeedableRng};
use reqwest::Url;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;

use crate::sam::services::crawler::job::CrawlJob;
use crate::sam::services::crawler::page::CrawledPage;

// use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};

static REQWEST_CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(super::robots::DEFAULT_USER_AGENT) // Use the centralized User-Agent
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(num_cpus::get())
        .pool_idle_timeout(Some(Duration::from_secs(15)))
        .danger_accept_invalid_certs(false) // Enable proper certificate validation
        .build()
        .expect("Failed to build reqwest client")
});

static COMMON_URLS: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    let bytes = include_bytes!("common_urls.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
});

static COMMON_TOKENS: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    let bytes = include_bytes!("common_tokens.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
});

static COMMON_PREFIXES: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    let bytes = include_bytes!("common_prefixes.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
});

static COMMON_TLDS: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    let bytes = include_bytes!("common_tlds.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
});

static COMMON_WORDS: once_cell::sync::Lazy<Vec<String>> = once_cell::sync::Lazy::new(|| {
    let bytes = include_bytes!("common_words.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
});

static CRAWLER_RUNNING: AtomicBool = AtomicBool::new(false);

// Add a static DNS cache (domain -> Option<bool> for found/not found)
static DNS_CACHE_PATH: &str = "/opt/sam/dns.cache";
static DNS_LOOKUP_CACHE: Lazy<tokio::sync::Mutex<HashMap<String, bool>>> =
    Lazy::new(|| tokio::sync::Mutex::new(HashMap::new()));

// Shared sleep-until timestamp (epoch seconds)
static SLEEP_UNTIL: once_cell::sync::Lazy<AtomicU64> =
    once_cell::sync::Lazy::new(|| AtomicU64::new(0));
static TIMEOUT_COUNT: once_cell::sync::Lazy<std::sync::Mutex<usize>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(0));

// Domain rate limiter to prevent overwhelming servers
static DOMAIN_LAST_ACCESS: once_cell::sync::Lazy<tokio::sync::Mutex<HashMap<String, u64>>> =
    once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(HashMap::new()));

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_POOL: once_cell::sync::Lazy<Pool> = once_cell::sync::Lazy::new(|| {
    let cfg = DeadpoolConfig::from_url(REDIS_URL);

    cfg.create_pool(Some(Runtime::Tokio1)).expect("Failed to create Redis connection pool")
});

/// Loads the DNS cache from Redis or a file, depending on configuration and availability.
///
/// # Arguments
/// * `should_use_redis` - If true, attempts to load from Redis first; falls back to file if unavailable or corrupted.
///
/// # Behavior
/// - If Redis is running and available, attempts to load the DNS cache from Redis.
/// - If Redis is unavailable or the cache is corrupted, falls back to loading from a file on disk.
/// - If the file does not exist, creates an empty cache file.
/// - Updates the global DNS_LOOKUP_CACHE with the loaded data.
///
/// # Async
/// This function is async and returns a boxed future for compatibility with static initializers.
fn load_dns_cache(should_use_redis: bool) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        if crate::sam::services::redis::is_running().await && should_use_redis {
            match tokio::time::timeout(Duration::from_secs(3), REDIS_POOL.get()).await {
                Ok(Ok(mut con)) => {
                    match deadpool_redis::redis::cmd("GET")
                        .arg("sam:dns_cache")
                        .query_async::<Option<Vec<u8>>>(&mut con)
                        .await
                    {
                        Ok(Some(data)) => {
                            if let Ok(map) = serde_json::from_slice::<HashMap<String, bool>>(&data)
                            {
                                {
                                    let mut cache = DNS_LOOKUP_CACHE.lock().await;
                                    *cache = map;
                                    log::info!(
                                        "Loaded DNS cache from Redis with {} entries",
                                        cache.len()
                                    );
                                }
                            } else {
                                log::warn!("Failed to parse DNS cache from Redis");
                                return load_dns_cache(false).await;
                            }
                        }
                        Ok(None) => {
                            log::info!("No DNS cache found in Redis");
                            save_dns_cache().await;
                            return load_dns_cache(true).await;
                        }
                        Err(e) => {
                            log::warn!("Failed to load DNS cache from Redis: {}", e);
                            return load_dns_cache(false).await;
                        }
                    }
                }
                Ok(Err(e)) => {
                    log::warn!("Failed to get Redis connection from pool: {}", e);
                    return load_dns_cache(false).await;
                }
                Err(_) => {
                    log::warn!("Timeout while waiting for Redis connection");
                    return load_dns_cache(false).await;
                }
            }
        } else {
            log::info!("Falling back to file-based DNS cache");
            // Fallback to file
            if !Path::new(DNS_CACHE_PATH).exists() {
                let _ = fs::write(DNS_CACHE_PATH, b"{}").await;
            }
            let path = Path::new(DNS_CACHE_PATH);
            if let Ok(data) = fs::read(path).await {
                if let Ok(map) = serde_json::from_slice::<HashMap<String, bool>>(&data) {
                    {
                        let mut cache = DNS_LOOKUP_CACHE.lock().await;
                        *cache = map;
                        log::info!("Loaded DNS cache from file with {} entries", cache.len());
                    }
                }
            }
        }
    })
}

/// Saves the DNS cache to Redis if available, otherwise falls back to saving to a file.
///
/// # Behavior
/// - Serializes the DNS_LOOKUP_CACHE to JSON.
/// - Attempts to save to Redis if running.
/// - If Redis is unavailable or saving fails, writes the cache to a file on disk.
/// - Logs all errors and fallbacks.
///
/// # Async
/// This function is async and should be awaited.
async fn save_dns_cache() {
    let should_fallback: bool;
    let cache = DNS_LOOKUP_CACHE.lock().await;
    let cache_bytes = match serde_json::to_vec(&*cache) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::warn!("Failed to serialize DNS cache: {}", e);
            return;
        }
    };
    drop(cache);
    if crate::sam::services::redis::is_running().await {
        match REDIS_POOL.get().await {
            Ok(mut con) => {
                match deadpool_redis::redis::cmd("SET")
                    .arg("sam:dns_cache")
                    .arg(cache_bytes.clone())
                    .query_async::<()>(&mut con)
                    .await
                {
                    Ok(_) => {
                        {
                            let cache = DNS_LOOKUP_CACHE.lock().await;
                            log::info!("Saved DNS cache to Redis with {} entries", cache.len());
                        }
                        return;
                    }
                    Err(e) => {
                        should_fallback = true;
                        log::warn!("Failed to save DNS cache to Redis: {}", e);
                    }
                }
            }
            Err(e) => {
                should_fallback = true;
                log::warn!(
                    "Failed to get Redis connection from pool for saving DNS cache: {}",
                    e
                );
            }
        }
    } else {
        should_fallback = true;
    }

    if should_fallback {
        log::info!("Falling back to file-based DNS cache");
        let _ = fs::write(DNS_CACHE_PATH, cache_bytes).await;
    }
}

/// Writes a URL to the retry cache for later reprocessing.
///
/// # Arguments
/// * `url` - The URL string to be retried.
///
/// # Behavior
/// - If Redis is available, appends the URL to a Redis list.
/// - If Redis is unavailable, appends the URL to a local file.
/// - Ensures the retry directory exists before writing.
/// - Logs all errors and fallbacks.
///
/// # Async
/// This function is async and should be awaited.
pub async fn write_url_to_retry_cache(url: &str) {
    let mut should_fallback = false;
    // Use Redis if available, otherwise fallback to file
    if crate::sam::services::redis::is_running().await {
        match REDIS_POOL.get().await {
            Ok(mut con) => {
                if let Err(e) = deadpool_redis::redis::cmd("RPUSH")
                    .arg("sam:crawl_retry")
                    .arg(url)
                    .query_async::<i32>(&mut con)
                    .await
                {
                    should_fallback = true;
                    log::warn!("Failed to write retry URL to Redis: {}", e);
                }
            }
            Err(e) => {
                should_fallback = true;
                log::warn!(
                    "Failed to get Redis connection from pool for retry cache: {}",
                    e
                );
            }
        }
    } else {
        should_fallback = true;
    }

    if should_fallback {
        // Fallback to file
        let retry_path = "/opt/sam/tmp/crawl_retry.dmp";
        if let Err(e) = fs::create_dir_all("/opt/sam/tmp").await {
            log::warn!("Failed to create retry dir: {}", e);
            return;
        }
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(retry_path)
            .await
        {
            if let Err(e) = file.write_all(format!("{url}\n").as_bytes()).await {
                log::warn!("Failed to write timed out URL to retry file: {}", e);
            }
        } else {
            log::warn!("Failed to open retry file for writing");
        }
    }
}

/// Checks if a string is a valid absolute URL with a scheme and host.
///
/// # Arguments
/// * `s` - The string to validate as a URL.
///
/// # Returns
/// * `true` if the string is a valid absolute URL with a host and scheme, `false` otherwise.
pub fn is_valid_url(s: &str) -> bool {
    match Url::parse(s) {
        Ok(url) => url.has_host() && url.scheme() != "",
        Err(_) => false,
    }
}

/// Internal function to crawl a single URL, extract tokens and links, and optionally recurse.
///
/// # Arguments
/// * `job_oid` - The unique identifier for the crawl job.
/// * `url` - The URL to crawl.
/// * `depth` - The current recursion depth.
/// * `client` - The shared HTTP client for making requests.
///
/// # Returns
/// * `Result<Vec<CrawledPage>>` - A vector of crawled page results or an error.
///
/// # Behavior
/// - Checks for global sleep and throttling.
/// - Validates the URL and skips known search endpoints.
/// - Checks for existing crawled data in the database.
/// - Performs HTTP GET requests with retries and timeout handling.
/// - Extracts tokens and links from HTML content using a blocking task.
/// - Filters and deduplicates tokens and links.
/// - Handles error cases, including timeouts and retry logic.
/// - Returns all successfully crawled pages for the given URL.
///
/// # Async
/// This function is async and should be awaited.
async fn crawl_url_inner(
    job_oid: String,
    url: String,
    _depth: usize,
    client: std::sync::Arc<reqwest::Client>,
) -> crate::sam::memory::Result<Vec<CrawledPage>> {
    // Validate URL for security (SSRF protection)
    let parsed_url = match crate::sam::security::validate_url(&url) {
        Ok(valid_url) => valid_url,
        Err(e) => {
            log::warn!("URL validation failed for {}: {}", url, e);
            return Err(crate::sam::memory::Error::Other(
                format!("URL validation failed: {}", e),
            )
            .into());
        }
    };
    
    // Extract domain from URL for various checks
    let domain = parsed_url.host_str().unwrap_or_default().to_string();

    // Get the user agent for this URL (for consistency)
    let user_agent = super::user_agents::get_user_agent_for_url(&url).await;
    
    // Check if URL was previously rejected (optimization to avoid repeated checks)
    // Only check if database is available
    match super::CrawlRejected::is_rejected(&url, &user_agent).await {
        Ok(Some(previous_rejection)) => {
            log::debug!("URL previously rejected ({} times): {} - reason: {:?}", 
                       previous_rejection.rejection_count, url, previous_rejection.reason);
            
            // For robots.txt rejections, we still need to check as rules may have changed
            // For other rejections, we can skip if they're recent (within last hour)
            let one_hour_ago = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64 - 3600;
                
            if previous_rejection.reason != super::RejectionReason::RobotsTxt 
                && previous_rejection.rejected_at > one_hour_ago {
                return Err(crate::sam::memory::Error::Other(
                    format!("URL previously rejected: {:?}", previous_rejection.reason),
                ).into());
            }
        }
        Ok(None) => {
            // URL not previously rejected
        }
        Err(e) => {
            // Database not available, continue without check
            log::debug!("Could not check rejection history (database unavailable): {}", e);
        }
    }

    // Check robots.txt compliance
    if !super::robots::is_url_allowed(&url).await {
        log::info!("URL blocked by robots.txt: {}", url);
        super::metrics::record_robots_block(&domain, &url).await;
        
        // Record the rejection to database for analysis and to avoid repeated checks
        let mut rejection = super::CrawlRejected::robots_blocked(
            url.clone(),
            Some("Disallowed by robots.txt".to_string()), // TODO: Get specific rule if available
            user_agent.clone(),
            Some(job_oid.clone()),
        );
        
        if let Err(e) = rejection.save().await {
            log::warn!("Failed to save robots.txt rejection record: {}", e);
        }
        
        return Err(crate::sam::memory::Error::Other(
            "URL blocked by robots.txt".to_string(),
        )
        .into());
    }

    // Check circuit breaker
    if !super::circuit_breaker::is_domain_allowed(&domain).await {
        log::info!("Domain blocked by circuit breaker: {}", domain);
        super::metrics::record_circuit_breaker_block(&domain).await;
        
        // Record circuit breaker rejection
        let mut rejection = super::CrawlRejected::new(
            url.clone(),
            super::RejectionReason::CircuitBreaker,
            user_agent.clone(),
            Some(job_oid.clone()),
        );
        
        if let Err(e) = rejection.save().await {
            log::warn!("Failed to save circuit breaker rejection record: {}", e);
        }
        
        return Err(crate::sam::memory::Error::Other(
            "Domain blocked by circuit breaker".to_string(),
        )
        .into());
    }

    // Apply rate limiting and validation
    apply_global_rate_limit().await;
    validate_url(&url)?;
    
    // Apply crawl delay from robots.txt if specified
    if let Some(delay) = super::robots::get_crawl_delay(&domain).await {
        log::debug!("Applying crawl delay of {:?} for {}", delay, domain);
        tokio::time::sleep(delay).await;
    }
    
    apply_domain_rate_limit(&url).await;

    if is_search_url(&url.to_ascii_lowercase()) {
        return Err(crate::sam::memory::Error::Other(
            "URL appears to be a search endpoint, skipping".to_string(),
        )
        .into());
    }

    // let mut pg_query = crate::sam::memory::PostgresQueries::default();
    // pg_query.queries.push(crate::sam::memory::PGCol::String(format!("{}",url.clone())));
    // pg_query.query_columns.push("url =".to_string());
    // let existing = match CrawledPage::select_async(Some(1), None, None, Some(pg_query).clone()).await {
    //     Ok(pages) => pages,
    //     Err(e) => {
    //         log::debug!("Failed to query existing CrawledPage: {}", e);
    //         Vec::new()
    //     }
    // };
    // if !existing.is_empty() {
    //     return Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(
    //         format!("CrawledPage already exists for URL: {}", url),
    //     )));
    // }

    let mut page = create_crawled_page(&job_oid, &url);
    let (file_mime, mut mime_tokens) = process_mime_type(&url, &mut page).await?;

    // Use the user agent we already fetched
    log::debug!("Using user agent for {}: {}", url, user_agent);
    
    // Check if domain has authentication configured
    let config = super::config::get_config().await;
    let auth_method = if let Some(domain_config) = config.domains.get(&domain) {
        if let Some(auth_config) = &domain_config.auth {
            super::auth::auth_from_config(auth_config)
        } else {
            super::auth::AuthMethod::None
        }
    } else {
        super::auth::AuthMethod::None
    };

    let mut resp = None;
    let mut last_err = None;
    for attempt in 0..3 {
        let mut headers = reqwest::header::HeaderMap::new();
        
        // Apply authentication if configured
        if let Err(e) = super::auth::apply_auth_to_request(&mut headers, &domain, &auth_method).await {
            log::warn!("Failed to apply authentication for {}: {}", domain, e);
        }
        
        let mut request = client.get(&url)
            .header("User-Agent", &user_agent);
        
        // Add auth headers
        for (name, value) in headers.iter() {
            request = request.header(name, value);
        }
        
        match tokio::time::timeout(Duration::from_secs(60), request.send()).await {
            Ok(Ok(r)) => {
                resp = Some(r);
                break;
            }
            Ok(Err(e)) => {
                last_err = Some(e.to_string());
                log::debug!(
                    "HTTP request error (attempt {}): {} for {}",
                    attempt + 1,
                    last_err.as_ref().expect("last_err should be Some"),
                    url
                );
            }
            Err(_) => {
                last_err = Some("Request timed out".to_string());
                log::error!("HTTP request timed out (attempt {}): {}", attempt + 1, url);
            }
        }
        // Optional: small delay between retries
        sleep(Duration::from_millis(100)).await;
    }
    let resp: Result<reqwest::Response, crate::sam::memory::Error> = match resp {
        Some(r) => Ok(r),
        None => Err(crate::sam::memory::Error::Other(format!(
            "Request failed after retries: {}",
            last_err.unwrap_or_else(|| "unknown".to_string())
        ))),
    };

    let mut all_pages = Vec::new();

    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();

            if status == 200 {
                // Extract headers before consuming resp
                let headers = resp.headers().clone();
                let url_clone = url.clone();
                let headers_clone = headers.clone();
                let mime_from_header = extract_mime_from_headers(&headers_clone);
                
                // Detect content type from headers
                let content_type_str = headers.get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("text/html");
                
                // Get response body based on content type
                let (mut html, raw_bytes) = if content_type_str.starts_with("image/") || 
                                              content_type_str.starts_with("application/pdf") ||
                                              content_type_str.starts_with("application/octet-stream") {
                    // For binary content, get bytes
                    match resp.bytes().await {
                        Ok(bytes) => {
                            log::debug!("Got {} bytes of binary content for {}", bytes.len(), url);
                            (String::new(), Some(bytes.to_vec()))
                        }
                        Err(e) => {
                            log::warn!("Failed to get bytes for {}: {}", url, e);
                            (String::new(), None)
                        }
                    }
                } else {
                    // For text content (including JS/CSS), get text
                    match resp.text().await {
                        Ok(text) => {
                            // For JS/CSS, we might want to extract useful information
                            if content_type_str.contains("javascript") || content_type_str.contains("css") {
                                log::debug!("Got {} characters of {} content for {}", 
                                    text.len(), 
                                    if content_type_str.contains("javascript") { "JavaScript" } else { "CSS" },
                                    url);
                            }
                            (text, None)
                        },
                        Err(e) => {
                            log::warn!("Failed to get text for {}: {}", url, e);
                            (String::new(), None)
                        }
                    }
                };
                
                // Check if the page needs JavaScript rendering
                // (e.g., minimal HTML, SPA indicators, or specific domains)
                let needs_js = super::js_renderer::is_js_rendering_available().await && {
                    // Check for SPA indicators
                    let is_spa = html.len() < 5000 && // Small initial HTML
                        (html.contains("window.__INITIAL_STATE__") ||
                         html.contains("window.__PRELOADED_STATE__") ||
                         html.contains("React.createElement") ||
                         html.contains("angular.module") ||
                         html.contains("new Vue") ||
                         html.contains("_app") ||
                         html.contains("__NEXT_DATA__"));
                    
                    // Check for specific domains known to be SPAs
                    let spa_domain = url.contains("twitter.com") ||
                        url.contains("facebook.com") ||
                        url.contains("instagram.com") ||
                        url.contains("linkedin.com") ||
                        url.contains("github.com");
                    
                    is_spa || spa_domain
                };
                
                if needs_js {
                    log::info!("Page appears to be an SPA, attempting JavaScript rendering for: {}", url);
                    match super::js_renderer::render_with_javascript(&url).await {
                        Ok(render_result) => {
                            log::info!("Successfully rendered with JavaScript: {} ({}ms, {} links found)", 
                                url, render_result.render_time.as_millis(), render_result.links.len());
                            
                            // Use the rendered HTML
                            html = render_result.html;
                            
                            // Log any detected frameworks
                            if !render_result.frameworks.is_empty() {
                                log::debug!("Detected frameworks: {:?}", render_result.frameworks);
                            }
                            
                            // Log any JavaScript errors
                            if !render_result.js_errors.is_empty() {
                                log::debug!("JavaScript errors encountered: {:?}", render_result.js_errors);
                            }
                        }
                        Err(e) => {
                            log::warn!("JavaScript rendering failed for {}: {}, using original HTML", url, e);
                            // Continue with original HTML
                        }
                    }
                }
                
                let content_type = super::content_types::ContentType::from_mime(content_type_str);
                
                // Process content based on type - use raw bytes if available, otherwise HTML
                let content_bytes = if let Some(bytes) = &raw_bytes {
                    bytes.as_slice()
                } else {
                    html.as_bytes()
                };
                
                let processed_content = match super::content_types::ContentProcessor::process(
                    content_bytes,
                    &content_type,
                    &url
                ).await {
                    Ok(content) => content,
                    Err(e) => {
                        log::warn!("Failed to process content for {}: {}", url, e);
                        // Create basic extracted content
                        super::content_types::ExtractedContent {
                            content_type: content_type.clone(),
                            text: Some(html.clone()),
                            metadata: std::collections::HashMap::new(),
                            links: Vec::new(),
                            size_bytes: html.len(),
                            hash: String::new(),
                            thumbnail: None,
                        }
                    }
                };
                
                // Save the processed content to CrawledContent
                // For images and binary content, we might not have text but still want to save metadata
                let should_save = processed_content.text.is_some() || 
                                 matches!(content_type, super::content_types::ContentType::Image(_) | 
                                                       super::content_types::ContentType::Pdf);
                
                if should_save {
                    // Use extracted text or metadata for searchable content
                    let searchable_text = if let Some(text) = &processed_content.text {
                        text.clone()
                    } else if matches!(content_type, super::content_types::ContentType::Image(_)) {
                        format!("Image: {} Size: {} bytes", url, processed_content.size_bytes)
                    } else {
                        String::new()
                    };
                    
                    // Store original content - for binary, store the raw bytes
                    let original_content = if raw_bytes.is_some() {
                        // For binary content, we could store a reference or skip storing
                        None // Don't store large binary data in text fields
                    } else {
                        Some(html.as_str())
                    };
                    
                    let content_storage = super::CrawledContent::new(
                        url.clone(),
                        &searchable_text,
                        original_content,
                        status
                    );
                    
                    // Extract title and description from HTML (if HTML)
                    let mut content_with_metadata = content_storage;
                    if !html.is_empty() {
                        content_with_metadata.title = super::CrawledContent::extract_title(&html);
                        content_with_metadata.description = super::CrawledContent::extract_description(&html);
                        content_with_metadata.language = super::CrawledContent::detect_language(&html);
                    } else {
                        // For non-HTML content, use metadata from processed content
                        content_with_metadata.title = processed_content.metadata.get("title")
                            .map(|s| s.to_string());
                        content_with_metadata.description = Some(format!("{:?} - {} bytes", content_type, processed_content.size_bytes));
                    }
                    content_with_metadata.content_type = mime_from_header.clone();
                    
                    // Convert headers to JSON
                    let headers_json = serde_json::json!({
                        "headers": headers.iter()
                            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
                            .collect::<Vec<_>>()
                    });
                    content_with_metadata.headers = headers_json;
                    
                    // Save content asynchronously (don't block on it)
                    match content_with_metadata.save().await {
                        Ok(was_new) => {
                            if was_new {
                                log::debug!("Saved new content for URL: {}", url);
                            } else {
                                log::debug!("Content already exists (deduplicated) for URL: {}", url);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to save CrawledContent for {}: {}", url, e);
                        }
                    }
                }
                
                // Pass headers and html into the closure
                // Instead of spawn_blocking, do parsing directly (async context)
                let mut tokens = Vec::new();
                let mut links = Vec::new();
                
                // Add links from processed content (for PDFs, XML, JSON, etc.)
                if !processed_content.links.is_empty() {
                    links.extend(processed_content.links.clone());
                }

                // Prefer MIME type from header, then file extension, then default
                if let Some(mimeh) = mime_from_header {
                    mime_tokens.push(mimeh);
                } else if let Some(mime) = file_mime {
                    mime_tokens.push(mime.to_string());
                } else {
                    mime_tokens.push("application/octet-stream".to_string());
                }

                // Treat .php, .asp, .aspx, .jsp, .jspx, .htm, .html, .xhtml, .shtml, .cgi, .pl, .cfm, .rb, .py, .xml, .json, .md, .txt, etc. as "document" types that may contain links
                let doc_exts = [
                    ".html", ".htm", ".xhtml", ".shtml", ".php", ".asp", ".aspx", ".jsp", ".jspx",
                    ".cgi", ".pl", ".cfm", ".rb", ".py", ".xml", ".json", ".md", ".txt", "/",
                ];
                let is_document = mime_tokens
                    .iter()
                    .any(|m| m.starts_with("text/") || m.starts_with("application/"))
                    || doc_exts.iter().any(|ext| url.ends_with(ext));

                // Extract links for HTML documents, or if we have links from processed content
                if (is_document && mime_tokens.iter().any(|m| m.starts_with("text/html"))) || !processed_content.links.is_empty() {
                    let document = scraper::Html::parse_document(&html);

                    let contains_replacement_char = html.contains('�')
                        || document.root_element().text().any(|t| t.contains('�'));
                    if contains_replacement_char {
                        // skip parsing
                        // (mime_tokens, tokens, links)
                    } else {
                        let body_selector = match scraper::Selector::parse("body") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'body': {}", e);
                                all_pages.push(page);
                                return Ok(all_pages);
                                // return (mime_tokens, tokens, links);
                            }
                        };
                        let skip_tags = [
                            "script", "style", "noscript", "svg", "canvas", "iframe", "template",
                        ];
                        let skip_selector = skip_tags
                            .iter()
                            .filter_map(|tag| scraper::Selector::parse(tag).ok())
                            .collect::<Vec<_>>();

                        for body in document.select(&body_selector) {
                            extract_text(&body, &skip_selector, &mut tokens);
                        }

                        extract_links_from_document(&document, &url_clone, &mut links);

                        // (mime_tokens, tokens, links)
                    }
                } else {
                    log::debug!("Skipping non-document file: {}", url_clone.clone());
                    // (mime_tokens, tokens, links)
                }

                // let (mut mime_tokens, mut tokens, mut links) = match result {
                //     Ok((mime_tokens, tokens, links)) => (mime_tokens, tokens, links),
                //     Err(e) => {
                //         log::warn!("Failed to parse HTML for {}: {}", url, e);
                //         (Vec::new(), Vec::new(), Vec::new())
                //     }
                // };

                tokens.sort();
                tokens.dedup();
                links.sort();
                links.dedup();

                filter_tokens(&mut tokens, &url);

                let mut all_tokens = mime_tokens.clone();
                all_tokens.extend(tokens);
                tokens = all_tokens;

                page.tokens = tokens;

                // Filter links: keep only those that start with "http://" or "https://", and do not start with "data:"
                links.retain(|link| {
                    let link_lc = link.to_ascii_lowercase();
                    (link_lc.starts_with("http://") || link_lc.starts_with("https://"))
                        && !link_lc.starts_with("data:")
                });

                page.links = links;

                // Record successful crawl metrics
                let response_time = Duration::from_millis(100); // Estimate based on typical response
                super::metrics::record_crawl_success(
                    &domain,
                    &url,
                    html.len() as u64,
                    response_time,
                    status,
                    mime_tokens.first().cloned(),
                ).await;
                super::circuit_breaker::record_domain_success(&domain).await;

                all_pages.push(page.clone());
            } else {
                // Record failed crawl (non-200 status)
                super::metrics::record_crawl_failure(&domain, &url, &format!("HTTP status: {}", status)).await;
                super::circuit_breaker::record_domain_failure(&domain).await;
                
                tokio::spawn({
                    let url = url.clone();
                    async move {
                        write_url_to_retry_cache(&url).await;
                    }
                });
            }
        }
        Err(e) => {
            log::warn!("Error fetching URL {}: {}", url, e);
            
            // Record failed crawl metrics
            super::metrics::record_crawl_failure(&domain, &url, &e.to_string()).await;
            super::circuit_breaker::record_domain_failure(&domain).await;

            tokio::spawn({
                let url = url.clone();
                async move {
                    write_url_to_retry_cache(&url).await;
                }
            });

            // If the error is a timeout, increment a static counter and occasionally sleep all threads

            let err_str = e.to_string().to_ascii_lowercase();
            if err_str.contains("timed out") || err_str.contains("timeout") {
                let mut count = TIMEOUT_COUNT.lock().expect("Failed to acquire timeout count lock");
                *count += 1;
                if (*count % 10) == 0 {
                    // Set global sleep for all threads for a random duration between 10 and 120 seconds
                    let mut rng = rand::thread_rng();
                    let sleep_secs = rng.gen_range(10..=120);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs();
                    let until = now + sleep_secs;
                    SLEEP_UNTIL.store(until, Ordering::SeqCst);
                    log::warn!("Timeout detected {} times, sleeping ALL threads for {} seconds to avoid ban", *count, sleep_secs);
                }
            }
        }
    }

    Ok(all_pages)
}

/// Boxed async function for recursion compatibility.
///
/// # Arguments
/// * See `crawl_url_inner`.
///
/// # Returns
/// * Boxed future for async recursion.
fn crawl_url_boxed<'a>(
    job_oid: String,
    url: String,
    depth: usize,
    client: std::sync::Arc<reqwest::Client>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = crate::sam::memory::Result<Vec<CrawledPage>>> + Send + 'a>,
> {
    Box::pin(async move {
        crawl_url_inner(job_oid, url, depth, client).await
    })
}

/// Public entry point for crawling a URL (non-recursive).
///
/// # Arguments
/// * See `crawl_url_inner`.
///
/// # Returns
/// * `Result<Vec<CrawledPage>>` - A vector of crawled page results or an error.
///
/// # Async
/// This function is async and should be awaited.
pub async fn crawl_url(
    job_oid: String,
    url: String,
    client: std::sync::Arc<reqwest::Client>,
) -> crate::sam::memory::Result<Vec<CrawledPage>> {
    // Add a timeout to prevent hanging
    match tokio::time::timeout(
        Duration::from_secs(30),
        crawl_url_boxed(job_oid, url.clone(), 0, client)
    ).await {
        Ok(result) => result,
        Err(_) => {
            log::error!("Timeout crawling URL after 30s: {}", url);
            Err(crate::sam::memory::Error::Other(
                format!("Timeout crawling URL: {}", url)
            ).into())
        }
    }
}

/// Start the crawler service synchronously
/// 
/// This function blocks the current thread and starts the crawler service.
/// For async contexts, use `start_service_async` instead.
pub fn start_service() {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        start_service_async().await;
    });
}

/// Wrapper for run_crawler_service to simplify spawning
async fn run_crawler_wrapper() {
    log::info!("Crawler wrapper started - entry point");
    
    // Add a small delay to ensure logging works
    tokio::time::sleep(Duration::from_millis(10)).await;
    log::info!("Crawler wrapper - after initial sleep");
    
    match run_crawler_service().await {
        Ok(_) => {
            log::info!("Crawler service completed normally");
        }
        Err(e) => {
            log::error!("Error in crawler service: {:?}", e);
            CRAWLER_RUNNING.store(false, Ordering::SeqCst);
        }
    }
    
    log::info!("Crawler wrapper finished");
}

/// Starts the crawler service asynchronously, spawning worker tasks for each CPU core.
///
/// # Behavior
/// - Ensures the service is only started once.
/// - Spawns a worker for each CPU core to process crawl jobs concurrently.
/// - Sets the global running flag.
/// - Logs service start.
///
/// # Async
/// This function is async and should be awaited.
pub async fn start_service_async() {
    // Check if already running
    if CRAWLER_RUNNING.load(Ordering::SeqCst) {
        log::info!("Crawler service already running");
        return;
    }
    
    log::info!("Crawler service starting...");
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
    
    // Try running directly first to bypass spawn issues
    log::info!("Running crawler directly without spawn");
    
    // Use tokio::spawn but immediately await it
    let result = tokio::spawn(async {
        log::info!("Inside spawn block - starting");
        
        // Try a very simple test first
        let test_url = "http://127.0.0.1:8000";
        log::info!("Testing basic connectivity to {}", test_url);
        
        // Use blocking thread for network operations to bypass async issues
        let blocking_result = tokio::task::spawn_blocking(move || {
            log::info!("In blocking thread - testing connection");
            
            // Try raw TCP connection
            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:8000".parse::<std::net::SocketAddr>().unwrap_or_else(|_| {
                    log::error!("Failed to parse socket address");
                    "127.0.0.1:8000".parse().unwrap()
                }),
                std::time::Duration::from_secs(5)
            ) {
                Ok(stream) => {
                    log::info!("✓ Blocking TCP connection successful to 127.0.0.1:8000");
                    drop(stream);
                    true
                }
                Err(e) => {
                    log::error!("✗ Blocking TCP connection failed: {}", e);
                    false
                }
            }
        }).await;
        
        match blocking_result {
            Ok(success) => {
                if success {
                    log::info!("Connection test passed, starting crawler");
                    // Now try the actual crawler
                    match run_crawler_service().await {
                        Ok(_) => log::info!("Crawler service completed"),
                        Err(e) => log::error!("Crawler service error: {:?}", e),
                    }
                } else {
                    log::error!("Connection test failed, not starting crawler");
                }
            }
            Err(e) => {
                log::error!("Blocking task join error: {}", e);
            }
        }
        
        log::info!("Spawn block completed");
    }).await;
    
    match result {
        Ok(_) => log::info!("Spawn completed successfully"),
        Err(e) => log::error!("Spawn join error: {}", e),
    }
    
    // Keep the service marked as running
    log::info!("Crawler service spawn sequence completed");
}

/// Stops the crawler service and sets the running flag to false.
///
/// # Behavior
/// - Sets the global running flag to false.
/// - Logs service stop.
pub fn stop_service() {
    info!("Crawler service stopping...");
    CRAWLER_RUNNING.store(false, Ordering::SeqCst);
    
    // Shutdown JavaScript renderer if running
    let rt = tokio::runtime::Runtime::new();
    if let Ok(runtime) = rt {
        runtime.block_on(async {
            if super::js_renderer::is_js_rendering_available().await {
                match super::js_renderer::shutdown_js_renderer().await {
                    Ok(_) => info!("JavaScript renderer shut down successfully"),
                    Err(e) => warn!("Failed to shutdown JavaScript renderer: {}", e),
                }
            }
        });
    }
    
    info!("Crawler service stopped.");
}

/// Get current crawler metrics report
///
/// Returns a formatted string containing comprehensive crawler metrics.
pub async fn get_metrics_report() -> String {
    super::metrics::generate_metrics_report().await
}

/// Get circuit breaker status for all domains
///
/// Returns a map of domain names to their circuit breaker statistics.
pub async fn get_circuit_breaker_status() -> HashMap<String, super::circuit_breaker::DomainStats> {
    super::circuit_breaker::get_all_domain_stats().await
}

/// Returns the current status of the crawler service as a string.
///
/// # Returns
/// * `"running"` if the service is active, `"stopped"` otherwise.
pub fn service_status() -> &'static str {
    if CRAWLER_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

/// Main crawler loop that finds pending jobs, crawls URLs, and updates job status.
///
/// # Behavior
/// - Continuously polls for pending crawl jobs.
/// - For each job, marks as running, crawls the start URL and discovered links using BFS up to a maximum depth.
/// - Uses concurrency to crawl multiple URLs in parallel.
/// - Saves crawled pages in batches to the database.
/// - Handles retry logic for failed URLs.
/// - If no jobs are found, generates new jobs from common URLs and discovered domains using DNS and HTTP probing.
/// - Periodically sleeps between iterations.
///
/// # Async
/// This function is async and should be awaited.
pub async fn run_crawler_service() -> crate::sam::memory::Result<()> {
    log::info!("run_crawler_service: Starting crawler service loop");
    
    // Load configuration
    log::info!("run_crawler_service: Loading crawler configuration");
    match super::config::CrawlerConfig::load() {
        Ok(config) => {
            log::info!("run_crawler_service: Configuration loaded successfully");
            if let Err(e) = super::config::set_config(config).await {
                log::warn!("run_crawler_service: Failed to apply configuration: {}", e);
            }
        }
        Err(e) => {
            log::warn!("run_crawler_service: Failed to load configuration: {}, using defaults", e);
        }
    }
    
    // Initialize database pool for the crawler
    log::info!("run_crawler_service: Initializing database connection pool");
    match super::initialize_db_pool().await {
        Ok(_) => log::info!("run_crawler_service: Database pool initialized successfully"),
        Err(e) => {
            log::error!("run_crawler_service: Failed to initialize database pool: {}", e);
            // Continue anyway - some operations may work without database
            log::warn!("run_crawler_service: Continuing without database - some features will be limited");
        }
    }
    
    log::info!("run_crawler_service: Creating HTTP client");
    let client = match std::panic::catch_unwind(|| Arc::new(REQWEST_CLIENT.clone())) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to create HTTP client: {:?}", e);
            return Err(anyhow::anyhow!("Failed to create HTTP client"));
        }
    };
    log::info!("run_crawler_service: HTTP client created");
    
    // Initialize JavaScript renderer for SPA support (optional)
    log::info!("run_crawler_service: Initializing JavaScript renderer");
    let js_config = super::js_renderer::JsRendererConfig {
        headless: true,
        max_browsers: 2, // Keep it low to save resources
        timeout: std::time::Duration::from_secs(30),
        wait_for_network_idle: true,
        viewport_width: 1920,
        viewport_height: 1080,
        blocked_resources: vec![
            super::js_renderer::ResourceType::Image,
            super::js_renderer::ResourceType::Font,
            super::js_renderer::ResourceType::Media,
        ],
        ..Default::default()
    };
    
    match super::js_renderer::initialize_js_renderer(js_config).await {
        Ok(_) => log::info!("run_crawler_service: JavaScript renderer initialized successfully"),
        Err(e) => {
            log::warn!("run_crawler_service: Failed to initialize JavaScript renderer: {}", e);
            log::warn!("run_crawler_service: Continuing without JavaScript rendering support");
        }
    }
    
    let all_crawled_pages = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    // Set up logging
    // log::set_max_level(LevelFilter::Info);

    // Load common URLs, tokens, TLDs, prefixes, and words
    // let tlds = COMMON_TLDS.clone();
    // let prefixes = COMMON_PREFIXES.clone();
    // let words = COMMON_WORDS.clone();

    // DNS resolver setup - use system default for now
    log::info!("run_crawler_service: Setting up DNS resolver");
    
    let resolver = if false && (std::env::var("ATLAS_DNS_SERVER").is_ok() || std::env::var("CAPROVER").is_ok()) {
        // Use Atlas DNS server
        let atlas_addr = std::env::var("ATLAS_DNS_SERVER")
            .unwrap_or_else(|_| {
                if std::env::var("CAPROVER").is_ok() {
                    // For CapRover, we need to resolve the service name first or use IP directly
                    "172.16.0.15:53".to_string()  // Use localhost for now in CapRover
                } else {
                    "172.16.0.15:53".to_string()  // Local Atlas
                }
            });
        
        log::info!("Using Atlas DNS server at: {}", atlas_addr);
        
        let mut config = ResolverConfig::new();
        
        // Parse address - handle both IP:port and hostname:port formats
        let socket_addr = if let Ok(addr) = atlas_addr.parse::<std::net::SocketAddr>() {
            Some(addr)
        } else if atlas_addr.contains(':') {
            // Try to parse as hostname:port
            let parts: Vec<&str> = atlas_addr.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let Ok(port) = parts[1].parse::<u16>() {
                    // For now, use localhost with the specified port
                    Some(std::net::SocketAddr::from(([172, 16, 0, 15], port)))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(socket_addr) = socket_addr {
            config.add_name_server(trust_dns_resolver::config::NameServerConfig {
                socket_addr,
                protocol: trust_dns_resolver::config::Protocol::Udp,
                tls_dns_name: None,
                trust_negative_responses: true,
                bind_addr: None,
                tls_config: None,
            });
        } else {
            log::warn!("Failed to parse Atlas DNS address, using default resolver");
            config = ResolverConfig::default();
        }
        
        // Add fallback DNS servers
        config.add_name_server(trust_dns_resolver::config::NameServerConfig {
            socket_addr: "8.8.8.8:53".parse().unwrap(),
            protocol: trust_dns_resolver::config::Protocol::Udp,
            tls_dns_name: None,
            trust_negative_responses: true,
            bind_addr: None,
            tls_config: None,
        });
        
        TokioAsyncResolver::tokio(config, ResolverOpts::default())
    } else {
        log::info!("Using default system DNS resolver");
        TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
    };

    // Load DNS cache from redis or file
    log::info!("run_crawler_service: Loading DNS cache");
    load_dns_cache(true).await;
    log::info!("run_crawler_service: DNS cache loaded");

    // Track when we last saved DNS cache
    let mut last_dns_save = std::time::Instant::now();
    
    loop {
        log::debug!("run_crawler_service: Main loop iteration");
        if !CRAWLER_RUNNING.load(Ordering::SeqCst) {
            log::debug!("run_crawler_service: Crawler not running, sleeping");
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        log::debug!("run_crawler_service: Crawler is running, proceeding");

        // Periodically save DNS cache (every 5 minutes)
        if last_dns_save.elapsed() > Duration::from_secs(300) {
            tokio::spawn(async {
                save_dns_cache().await;
            });
            last_dns_save = std::time::Instant::now();
        }

        // Find a pending job
        log::debug!("Checking for pending crawl jobs...");
        let mut jobs = match tokio::time::timeout(
            Duration::from_secs(5),
            CrawlJob::select_async(Some(5000), None, None, None)
        ).await {
            Ok(Ok(jobs)) => {
                log::debug!("Found {} total crawl jobs", jobs.len());
                jobs
                    .into_iter()
                    .filter(|j| j.status == "pending")
                    .collect::<Vec<_>>()
            },
            Ok(Err(e)) => {
                log::error!("Failed to query crawl jobs from database: {}", e);
                vec![]
            },
            Err(_) => {
                log::warn!("Database query timed out after 5 seconds, proceeding without jobs");
                vec![]
            },
        };

        // If no jobs found, create some initial ones
        // if jobs.is_empty() {
        //     log::info!("No pending jobs found, generating initial URLs to crawl");
            
        //     // Mix of popular sites and some randomness
        //     let base_domains = vec![
        //         "example.com",
        //         "wikipedia.org", 
        //         "github.com",
        //         "stackoverflow.com",
        //         "reddit.com",
        //         "news.ycombinator.com",
        //         "techcrunch.com",
        //         "medium.com",
        //         "dev.to",
        //         "hackernews.com",
        //     ];
            
        //     // Pick 3 random domains to crawl
        //     let selected: Vec<_> = {
        //         let mut rng = rand::thread_rng();
        //         base_domains.choose_multiple(&mut rng, 3).cloned().collect()
        //     };
            
        //     for domain in selected {
        //         let url = format!("https://{}/", domain);
        //         log::info!("Creating new crawl job for: {}", url);
                
        //         let oid: String = rand::thread_rng()
        //             .sample_iter(&Alphanumeric)
        //             .take(15)
        //             .map(char::from)
        //             .collect();
                    
        //         let mut job = CrawlJob::new();
        //         job.oid = oid;
        //         job.start_url = url;
        //         job.status = "pending".to_string();
        //         job.created_at = std::time::SystemTime::now()
        //             .duration_since(std::time::UNIX_EPOCH)
        //             .map(|d| d.as_secs() as i64)
        //             .unwrap_or(0);
        //         job.updated_at = job.created_at;
                
        //         // Try to save the job, but continue even if it fails
        //         match job.save_async().await {
        //             Ok(_) => {
        //                 log::info!("Created crawl job: {}", job.oid);
        //                 jobs.push(job);
        //             }
        //             Err(e) => {
        //                 log::warn!("Failed to save crawl job to database: {}, using in-memory", e);
        //                 jobs.push(job);
        //             }
        //         }
        //     }
        // }
        
        jobs.shuffle(&mut rand::thread_rng());
        jobs.truncate(1);

        if let Some(mut job) = jobs.into_iter().next() {
            let job_oid = job.oid.clone();
            info!("Starting crawl job: oid={} url={}", job.oid, job.start_url);
            // Mark as running
            job.status = "running".to_string();
            job.updated_at =
                match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => {
                        log::debug!("SystemTime before UNIX EPOCH: {:?}", e);
                        0
                    }
                };
            // Try to save job status, but don't block on it
            match tokio::time::timeout(Duration::from_secs(2), job.save_async()).await {
                Ok(Ok(_)) => log::debug!("Job status saved to database"),
                Ok(Err(e)) => log::warn!("Failed to save job status: {}", e),
                Err(_) => log::warn!("Timeout saving job status, continuing anyway"),
            }

            // Crawl start_url and discovered links (BFS, depth 2)
            let max_depth = 10;
            // Initialize visited set with URLs from all CrawlJob entries in Postgres
            let mut visited_urls = HashSet::new();
            
            log::info!("Loading existing crawled pages from database...");
            match tokio::time::timeout(
                Duration::from_secs(3),
                CrawledPage::select_async(None, None, None, None)
            ).await {
                Ok(Ok(crawled_pages)) => {
                    log::info!("Loaded {} existing crawled pages", crawled_pages.len());
                    for page in crawled_pages {
                        visited_urls.insert(page.url);
                    }
                }
                Ok(Err(e)) => log::warn!("Failed to load crawled pages: {}, starting fresh", e),
                Err(_) => log::warn!("Timeout loading crawled pages, starting fresh"),
            }

            let mut job_urls = HashSet::new();
            log::info!("Loading existing jobs from database...");
            match tokio::time::timeout(
                Duration::from_secs(3),
                CrawlJob::select_async(None, None, None, None)
            ).await {
                Ok(Ok(all_jobs)) => {
                    log::info!("Loaded {} existing jobs", all_jobs.len());
                    for job in all_jobs {
                        job_urls.insert(job.start_url);
                    }
                }
                Ok(Err(e)) => log::warn!("Failed to load jobs: {}, continuing", e),
                Err(_) => log::warn!("Timeout loading jobs, continuing"),
            }

            let visited = Arc::new(tokio::sync::Mutex::new(visited_urls));
            let all_job_urls = Arc::new(tokio::sync::Mutex::new(job_urls));
            let queue = Arc::new(tokio::sync::Mutex::new(VecDeque::from([(
                job.start_url.clone(),
                0,
            )])));

            let concurrency = std::env::var("SAM_WORKER_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                let cores = num_cpus::get();
                if cores >= 32 {
                    cores * 2  // High-core systems: 2x multiplier (96 for you)
                } else if cores >= 16 {
                    (cores as f64 * 1.5) as usize  // Mid-range: 1.5x multiplier
                } else {
                    cores + 4  // Low-core: cores + 4
                }
            });
            log::info!("Starting crawl loop with concurrency={}, max_depth={}", concurrency, max_depth);
            
            let mut iteration = 0;
            loop {
                iteration += 1;
                log::debug!("Crawl iteration {}", iteration);
                
                // Collect all URLs at the current minimum depth
                let (batch, current_depth) = {
                    let mut q = queue.lock().await;
                    let queue_size = q.len();
                    log::debug!("Queue has {} URLs", queue_size);
                    
                    let mut batch = Vec::new();
                    let mut min_depth: Option<usize> = None;
                    // Find the minimum depth in the queue
                    for &(_, d) in q.iter() {
                        min_depth = match min_depth {
                            Some(md) => Some(md.min(d)),
                            None => Some(d),
                        };
                    }
                    let min_depth = match min_depth {
                        Some(d) => d,
                        None => {
                            log::info!("Queue is empty, finishing crawl");
                            break;
                        }
                    };
                    
                    // Check depth limit
                    if min_depth >= max_depth {
                        log::info!("Reached max depth {}, stopping crawl", max_depth);
                        break;
                    }
                    
                    // Drain all URLs at this depth
                    let mut i = 0;
                    while i < q.len() {
                        if q[i].1 == min_depth {
                            let (url, depth) = q.remove(i).expect("Queue index should be valid");
                            batch.push((url, depth));
                        } else {
                            i += 1;
                        }
                    }
                    log::info!("Processing {} URLs at depth {}", batch.len(), min_depth);
                    (batch, min_depth)
                };
                if batch.is_empty() {
                    log::info!("No URLs to process, exiting crawl loop");
                    break;
                }
                // Mark all as visited
                {
                    let mut v = visited.lock().await;
                    for (url, _) in &batch {
                        v.insert(url.clone());
                    }
                }

                // Crawl all URLs at this depth concurrently
                log::info!("Starting concurrent crawl of {} URLs", batch.len());
                use futures::stream;
                let results = stream::iter(batch.into_iter())
                    .map(|(url, depth)| {
                        let job_oid = job_oid.clone();
                        let client = client.clone();

                        async move {
                            log::debug!("Crawling URL: {}", url);
                            let start = tokio::time::Instant::now();
                            let result = crawl_url(job_oid.clone(), url.clone(), client).await;
                            let elapsed = start.elapsed();
                            
                            match &result {
                                Ok(pages) => log::info!("✓ Crawled {} in {:.2}s, found {} pages", 
                                    url, elapsed.as_secs_f32(), pages.len()),
                                Err(e) => log::warn!("✗ Failed to crawl {} in {:.2}s: {}", 
                                    url, elapsed.as_secs_f32(), e),
                            }
                            
                            (url, depth, result)
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect::<Vec<_>>()
                    .await;
                
                log::info!("Completed crawling batch, processing {} results", results.len());

                // Process results
                let mut new_links = Vec::new();
                for (url, depth, result) in results {
                    match result {
                        Ok(pages) => {
                            for page in &pages {
                                for link in &page.links {
                                    let should_add = {
                                        let v = visited.lock().await;
                                        !v.contains(link)
                                    };
                                    if should_add {
                                        if (new_links.len() < 1000)
                                            || !mime_type_from_url(link).contains("text/")
                                        {
                                            let url_lc = link.clone();
                                            if is_search_url(&url_lc) {
                                                // Skip search endpoints
                                                log::debug!("Skipping search endpoint: {}", link);
                                            } else {
                                                // Add to new links for further crawling
                                                new_links.push((link.clone(), depth + 1));
                                            }
                                        } else {
                                            // Spawn a new thread to create and save the CrawlJob for this link
                                            // Collect jobs in a batch and save them together for efficiency
                                            static JOB_BATCH: once_cell::sync::Lazy<
                                                tokio::sync::Mutex<
                                                    Vec<crate::sam::memory::cache::WebCrawl>,
                                                >,
                                            > = once_cell::sync::Lazy::new(|| {
                                                tokio::sync::Mutex::new(Vec::new())
                                            });
                                            {
                                                let mut batch = JOB_BATCH.lock().await;
                                                let cache_job =
                                                    crate::sam::memory::cache::WebCrawl::new(
                                                        link.clone(),
                                                    );

                                                let url_lc = link.clone();
                                                if is_search_url(&url_lc) {
                                                    // Skip search endpoints
                                                    log::debug!(
                                                        "Skipping search endpoint: {}",
                                                        link
                                                    );
                                                } else {
                                                    let res = {
                                                        let v = visited.lock().await;
                                                        let all_jobs = all_job_urls.lock().await;
                                                        !v.contains(&cache_job.url)
                                                            && !all_jobs.contains(&cache_job.url)
                                                    };
                                                    if res {
                                                        batch.push(cache_job);
                                                        let mut v = all_job_urls.lock().await;
                                                        for cache_job in batch.iter() {
                                                            v.insert(cache_job.url.clone());
                                                        }
                                                    }
                                                }

                                                if batch.len() >= 1000 {
                                                    let jobs_to_save = batch.split_off(0);
                                                    drop(batch); // Release lock before await
                                                    if let Err(e) =
                                                        crate::sam::memory::cache::WebCrawl::save_batch_async(jobs_to_save)
                                                            .await
                                                    {
                                                        log::warn!(
                                                            "Failed to save batch crawl jobs: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Stream saves to reduce memory usage - save immediately if we have pages
                            if !pages.is_empty() {
                                let mut all = all_crawled_pages.lock().await;
                                all.extend(pages.into_iter());

                                // Save more frequently to reduce memory usage (every 100 pages instead of 1000)
                                if all.len() >= 100 {
                                    log::info!("C: Saving {} crawled pages", all.len());
                                    let pages_to_save = all.drain(..).collect::<Vec<_>>();
                                    drop(all); // Release lock before I/O

                                    for chunk in pages_to_save.chunks(50) {
                                        if let Err(e) = CrawledPage::save_async_batch(chunk).await {
                                            log::warn!("Failed to save batch of pages: {}", e);
                                            for p in chunk {
                                                write_url_to_retry_cache(&p.url).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            info!("Crawler error: {}", e);
                            log::error!("Crawler error: {}", e);
                            write_url_to_retry_cache(&url).await;
                        }
                    }
                }
                // Add new links to queue in one lock
                if !new_links.is_empty() {
                    let mut q = queue.lock().await;
                    for (link, d) in new_links {
                        if d <= max_depth {
                            q.push_back((link, d));
                        }
                    }
                }
            }

            let mut all = all_crawled_pages.lock().await;

            // Batch save all crawled pages in chunks of 500
            log::info!("B: Saving {} crawled pages", all.len());
            for chunk in all.chunks(10) {
                if let Err(e) = CrawledPage::save_async_batch(chunk).await {
                    log::warn!("Failed to save batch of pages: {}", e);
                    for p in chunk {
                        write_url_to_retry_cache(&p.url).await;
                    }
                }
            }
            all.clear();

            drop(all);
            // drop(all_crawled_pages);

            // Mark job as done
            job.status = "done".to_string();
            job.updated_at =
                match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => {
                        log::warn!("SystemTime before UNIX EPOCH: {:?}", e);
                        0
                    }
                };
            crate::sam::services::crawler::job::CrawlJob::destroy_async(job.oid.clone())
                .await
                .unwrap_or_else(|_| {
                    log::warn!("Failed to destroy crawl job: oid={}", job.oid);
                    false
                });

            drop(visited);

            let _ = job.save_async().await;
            info!("Finished crawl job: oid={}", job.oid);
        } else {
            // No jobs: scan common URLs and/or use DNS queries to find domains
            info!("No pending crawl jobs found. Crawling common URLs.");
            let mut urls_to_try: Vec<String> = COMMON_URLS.iter().map(|s| s.to_string()).collect();
            

            // Load retry URLs from the retry file and remove the file after loading
            let retry_path = "/opt/sam/tmp/crawl_retry.dmp";
            if let Ok(data) = fs::read_to_string(retry_path).await {
                let retry_urls: Vec<String> = data
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect();
                if !retry_urls.is_empty() {
                    log::info!("Loaded {} retry URLs from {}", retry_urls.len(), retry_path);
                    urls_to_try.extend(retry_urls);
                }
                // Remove the retry file after loading
                fs::remove_file(retry_path).await.unwrap_or_else(|_| {
                    log::warn!("Failed to remove retry file: {}", retry_path);
                });
            }

            // Metrics: log time to generate domain list
            let domain_gen_start = tokio::time::Instant::now();

            let tlds = COMMON_TLDS.clone();
            let prefixes = COMMON_PREFIXES.clone();
            let mut words = COMMON_WORDS.clone();
            let tokens = COMMON_TOKENS.clone();

            // Use most common token list to extend the words list and dedupe
            words.extend(tokens.clone());
            words.sort();
            words.dedup();

            // Sample words and prefixes to generate domains
            let _domains: Vec<String> = Vec::new();
            use rayon::prelude::*;

            let mut rng = SmallRng::from_entropy();
            let mut sampled_words = words.clone();
            sampled_words.shuffle(&mut rng);
            // Use rayon's par_iter to efficiently take the first 1,000 elements in parallel
            let sampled_words: Vec<_> = sampled_words.par_iter().take(1000).cloned().collect(); // Reduced for faster startup

            let domain_gen_duration = domain_gen_start.elapsed();
            log::info!("Domain generation took {:?}", domain_gen_duration);

            let mut domains: Vec<String> = tlds
                .par_iter()
                .flat_map_iter(|tld| {
                    let mut local_domains = Vec::with_capacity(
                        sampled_words.len()
                            * (1 + prefixes.len() + sampled_words.len() * prefixes.len())
                            + prefixes.len()
                            + sampled_words.len(),
                    );

                    // word.tld and prefix.word.tld and prefix.word2.word.tld
                    for word in &sampled_words {
                        local_domains.push(format!("{word}.{tld}"));
                        for prefix in &prefixes {
                            local_domains.push(format!("{prefix}.{word}.{tld}"));
                            for word2 in &sampled_words {
                                local_domains.push(format!("{prefix}.{word2}.{word}.{tld}"));
                            }
                        }
                    }
                    // prefix.tld
                    for prefix in &prefixes {
                        local_domains.push(format!("{prefix}.{tld}"));
                    }
                    // word.tld (again, but dedup later)
                    for word in &sampled_words {
                        local_domains.push(format!("{word}.{tld}"));
                    }
                    local_domains
                })
                .collect();
            let mut rng = SmallRng::from_entropy();
            domains.sort();
            domains.dedup();
            domains.shuffle(&mut rng);

            let max_domains = 1000;
            let domains = &domains[..std::cmp::min(domains.len(), max_domains)];

            let mut urls_found = Vec::new();

            // Use higher concurrency for DNS lookups (DNS is lightweight)
            let concurrency = (num_cpus::get() * 8).max(32).min(128);
            log::info!(
                "Starting DNS lookups for {} domains with concurrency {}",
                domains.len(),
                concurrency
            );
            let dns_start = tokio::time::Instant::now();
            let total_domains = domains.len();
            let processed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let found_domains = tokio_stream::iter(domains.iter().cloned().enumerate())
                .map(|(idx, domain)| {
                    let resolver = resolver.clone();
                    let client_clone = client.clone();
                    let processed = processed.clone();
                    let total = total_domains;
                    async move {
                        if idx == 0 {
                            log::info!("Starting first DNS lookup for domain: {}", domain);
                        }
                        let lookup_start = tokio::time::Instant::now();
                        let found = lookup_domain(&resolver, &domain, client_clone).await;
                        let lookup_duration = lookup_start.elapsed();
                        
                        // Update progress counter
                        let count = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if count % 10 == 0 || count == total {
                            log::info!(
                                "DNS lookup progress: {}/{} domains processed ({:.1}%)",
                                count, total, (count as f64 / total as f64) * 100.0
                            );
                        }
                        
                        log::debug!(
                            "DNS+HTTP lookup for domain {} took {:?} (found={})",
                            domain,
                            lookup_duration,
                            found
                        );
                        if found {
                            Some(domain)
                        } else {
                            None
                        }
                    }
                })
                .buffer_unordered(concurrency)
                .filter_map(|opt| async move { opt })
                .collect::<Vec<String>>()
                .await;
            let dns_duration = dns_start.elapsed();
            log::info!(
                "DNS+HTTP lookups completed: {} domains processed in {:?}, {} domains found",
                domains.len(),
                dns_duration,
                found_domains.len()
            );

            for domain in found_domains {
                urls_found.push(format!("https://{domain}/"));
                urls_found.push(format!("http://{domain}/"));
            }
            urls_to_try.extend(urls_found);
            urls_to_try.sort();
            urls_to_try.dedup();

            log::info!("Found {} URLs to crawl", urls_to_try.len());

            let mut rng = SmallRng::from_entropy();

            let mut urls: Vec<String> = urls_to_try.into_iter().collect();

            urls.shuffle(&mut rng);
            
            // Actually crawl the URLs since database isn't working
            log::info!("Starting to crawl {} URLs directly", urls.len());
            // Still try to create jobs for tracking (even if saves fail)
            for url in &urls {
                let job_create_start = tokio::time::Instant::now();
                let oid: String = thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(15)
                    .map(char::from)
                    .collect();
                let mut job = CrawlJob::new();
                job.start_url = url.clone();
                job.status = "pending".to_string();
                job.created_at =
                    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                        Ok(duration) => duration.as_secs() as i64,
                        Err(e) => {
                            log::warn!("SystemTime before UNIX EPOCH: {:?}", e);
                            0
                        }
                    };
                job.updated_at = job.created_at;
                job.oid = oid;
                job.save_async().await.unwrap_or_else(|_| {
                    log::warn!("Failed to save crawl job for URL: {}", url);
                    job
                });
                let job_create_duration = job_create_start.elapsed();
                log::debug!(
                    "Created crawl job for URL: {} in {:?}",
                    url,
                    job_create_duration
                );
            }
        }
        sleep(Duration::from_secs(10)).await;
    }
}

/// Performs a DNS lookup for a domain, with caching and HTTP(S) probing.
///
/// # Arguments
/// * `resolver` - The DNS resolver to use.
/// * `domain` - The domain name to look up.
/// * `client` - The shared HTTP client for probing.
///
/// # Returns
/// * `true` if the domain resolves and responds to HTTP(S), `false` otherwise.
///
/// # Async
/// This function is async and should be awaited.
async fn lookup_domain(
    resolver: &TokioAsyncResolver,
    domain: &str,
    client: std::sync::Arc<reqwest::Client>,
) -> bool {
    log::debug!("lookup_domain: Starting lookup for domain: {}", domain);
    
    // Check cache first
    {
        let cache = DNS_LOOKUP_CACHE.lock().await;
        if let Some(found) = cache.get(domain) {
            log::debug!("lookup_domain: Cache hit for {}: {}", domain, found);
            return *found;
        }
    }
    log::debug!("lookup_domain: Cache miss for {}, doing DNS lookup", domain);
    
    // Not in cache, do DNS lookup
    let mut found = false;
    for attempt in 0..3 {
        log::debug!("lookup_domain: DNS lookup attempt {} for {}", attempt + 1, domain);
        let dns_start = tokio::time::Instant::now();
        
        let result = match tokio::time::timeout(
            Duration::from_secs(3), // Reduced for faster processing
            resolver.lookup_ip(domain),
        )
        .await
        {
            Ok(Ok(lookup)) if lookup.iter().next().is_some() => {
                log::debug!("lookup_domain: DNS resolved {} in {:?}", domain, dns_start.elapsed());
                // DNS exists, now check HTTP/HTTPS HEAD
                let http_url = format!("http://{domain}/");
                let https_url = format!("https://{domain}/");

                let mut http_ok = false;
                let https_ok = false;
                for http_attempt in 0..3 {
                    let http_fut = client.head(&http_url).send();
                    let https_fut = client.head(&https_url).send();
                    let result = tokio::time::timeout(
                            Duration::from_secs(5),
                            async {
                                tokio::select! {
                                    resp = http_fut => resp.ok().map(|r| r.status().is_success() || r.status().is_redirection()),
                                    resp = https_fut => resp.ok().map(|r| r.status().is_success() || r.status().is_redirection()),
                                }
                            }
                        ).await;
                    match result {
                        Ok(Some(true)) => {
                            http_ok = true;
                            break;
                        }
                        Ok(Some(false)) | Ok(None) | Err(_) => {
                            log::warn!(
                                "HEAD request timed out or failed (attempt {}): {}",
                                http_attempt + 1,
                                domain
                            );
                        }
                    }
                    sleep(Duration::from_millis(300)).await;
                }
                if http_ok || https_ok {
                    found = true;
                    break;
                }

                false
            }
            Ok(Ok(_)) => {
                log::debug!("lookup_domain: DNS resolved but no IPs found for {} in {:?}", domain, dns_start.elapsed());
                false
            }
            Ok(Err(e)) => {
                log::debug!("lookup_domain: DNS lookup error for {} in {:?}: {}", domain, dns_start.elapsed(), e);
                false
            }
            Err(_) => {
                log::warn!("lookup_domain: DNS lookup timeout (3s) for {} on attempt {}", domain, attempt + 1);
                false
            }
        };
        if result {
            found = true;
            break;
        }
        sleep(Duration::from_millis(300)).await;
    }
    // Update cache (but don't save to disk here)
    {
        let mut cache = DNS_LOOKUP_CACHE.lock().await;
        cache.insert(domain.to_string(), found);
    }
    log::debug!("lookup_domain: Final result for {}: found={}", domain, found);
    found
}

fn is_search_url(url: &str) -> bool {
    // Use lazy static regex for better performance
    static SEARCH_PATTERNS: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(
        || {
            regex::Regex::new(r"(?i)(/search[/?]|/query[/?]|/find[/?]|/lookup[/?]|/results[/?]|/explore[/?]|/filter[/?]|/discover[/?]|/browse[/?]|/list[/?]|/websearch\?|/search_history\?|\?q=|&q=|search=|query=|lookup=|results=|explore=|filter=|discover=|browse=|\bu=|url=|\bid=|redirect|backurl=|text=|searchterm|search_term|return_to|https?%3A%2F%2F)").expect("Failed to compile search pattern regex")
        },
    );

    let url_lc = url.to_ascii_lowercase();

    // Check for multiple URLs in one (redirect chains)
    if url_lc.matches("https://").count() >= 2
        || url_lc.matches("http://").count() >= 2
        || (url_lc.contains("https://") && url_lc.contains("http://"))
    {
        return true;
    }

    SEARCH_PATTERNS.is_match(&url_lc)
}

/// Recursively extracts text tokens from an HTML element, skipping specified tags.
///
/// # Arguments
/// * `element` - The current HTML element to process.
/// * `skip_selector` - A list of selectors to skip (e.g., script, style).
/// * `tokens` - The mutable vector to collect tokens into.
fn extract_text(
    element: &scraper::ElementRef,
    skip_selector: &[scraper::Selector],
    tokens: &mut Vec<String>,
) {
    for sel in skip_selector {
        if sel.matches(element) {
            return;
        }
    }
    for child in element.children() {
        match child.value() {
            scraper::node::Node::Text(t) => {
                for word in t.text.split_whitespace() {
                    let w = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if !w.is_empty() {
                        tokens.push(w.to_lowercase());
                    }
                }
            }
            scraper::node::Node::Element(_) => {
                if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                    extract_text(&child_elem, skip_selector, tokens);
                }
            }
            _ => {}
        }
    }
}

/// Returns the MIME type for a given URL string based on its file extension.
/// Falls back to "application/octet-stream" if unknown.
///
/// # Arguments
/// * `url` - The URL string to analyze.
///
/// # Returns
/// * A string slice representing the MIME type.
pub fn mime_type_from_url(url: &str) -> &'static str {
    let url_lc = url.to_ascii_lowercase();
    let url_no_query = url_lc.split(&['?', '#'][..]).next().unwrap_or("");
    let path = std::path::Path::new(url_no_query);
    if let Some(segment) = path.file_name().and_then(|s| s.to_str()) {
        if segment.contains('.') {
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                // Check if the extension is a known TLD (like .com, .net, etc.)
                if COMMON_TLDS.contains(&ext.to_ascii_lowercase()) {
                    // It's a TLD, not a file extension, so treat as HTML
                    return "text/html";
                }
                for (map_ext, mime) in crate::sam::tools::MIME_MAP.iter() {
                    if ext.eq_ignore_ascii_case(map_ext.trim_start_matches('.')) {
                        return mime;
                    }
                }
            }
        }
    }
    "text/unknown"
}

/// Extracts MIME type from HTTP headers
fn extract_mime_from_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("Content-Type")
        .or_else(|| headers.get("content-type"))
        .and_then(|mimeh| mimeh.to_str().ok())
        .map(|mime_str| {
            mime_str
                .split(';')
                .next()
                .unwrap_or(mime_str)
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|mime| !mime.is_empty())
}

/// Extracts links from an HTML document
fn extract_links_from_document(document: &scraper::Html, base_url: &str, links: &mut Vec<String>) {
    const LINK_SELECTORS: &[(&str, &str)] = &[
        ("a[href]", "href"),
        ("img[src]", "src"),
        ("audio[src]", "src"),
        ("video[src]", "src"),
        ("audio source[src], video source[src]", "src"),
        ("link[rel=\"stylesheet\"]", "href"),
        ("script[src]", "src"),
    ];

    for (selector_str, attr_name) in LINK_SELECTORS {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            for element in document.select(&selector) {
                if let Some(attr_value) = element.value().attr(attr_name) {
                    if let Ok(abs_url) = resolve_url(base_url, attr_value) {
                        links.push(abs_url);
                    }
                }
            }
        }
    }
}

/// Resolves a potentially relative URL to an absolute URL
fn resolve_url(base_url: &str, url: &str) -> Result<String, url::ParseError> {
    Url::parse(url)
        .or_else(|_| Url::parse(base_url).and_then(|base| base.join(url)))
        .map(|u| u.to_string())
}

/// Filters tokens based on various criteria
fn filter_tokens(tokens: &mut Vec<String>, url: &str) {
    // Create date regex patterns
    let date_patterns = create_date_regex_patterns();

    // Filter out common tokens unless they match date patterns
    tokens.retain(|token| is_date_token(token, &date_patterns) || !COMMON_TOKENS.contains(token));

    // Filter by length
    tokens.retain(|token| token.len() > 2 && token.len() < 50);

    // Remove tokens that are part of the URL
    remove_url_tokens(tokens, url);

    // Remove tokens that are part of the domain
    remove_domain_tokens(tokens, url);
}

/// Creates regex patterns for various date formats
fn create_date_regex_patterns() -> Vec<regex::Regex> {
    vec![
        regex::Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{8}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{4}\.\d{1,2}\.\d{1,2}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$").expect("Failed to compile date regex pattern"),
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?(Z|([+-]\d{2}:\d{2}))?)?$")
            .expect("Failed to compile ISO date regex pattern"),
    ]
}

/// Checks if a token matches any date pattern
fn is_date_token(token: &str, patterns: &[regex::Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(token))
}

/// Removes tokens that are part of the URL path
fn remove_url_tokens(tokens: &mut Vec<String>, url: &str) {
    let url_tokens: HashSet<_> = url.split('/').map(|s| s.to_lowercase()).collect();
    tokens.retain(|token| !url_tokens.contains(&token.to_lowercase()));
}

/// Removes tokens that are part of the domain name
fn remove_domain_tokens(tokens: &mut Vec<String>, url: &str) {
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(domain) = parsed_url.domain() {
            let domain_tokens: HashSet<_> = domain.split('.').map(|s| s.to_lowercase()).collect();
            tokens.retain(|token| !domain_tokens.contains(&token.to_lowercase()));
        }
    }
}

/// Applies global rate limiting based on sleep-until timestamp
async fn apply_global_rate_limit() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let sleep_until = SLEEP_UNTIL.load(Ordering::SeqCst);

    if now < sleep_until {
        let sleep_secs = sleep_until - now;
        log::debug!(
            "Global sleep in effect, sleeping for {} seconds",
            sleep_secs
        );
        tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
    }
}

/// Validates that the URL is properly formed
fn validate_url(url: &str) -> crate::sam::memory::Result<()> {
    if !is_valid_url(url) {
        Err(crate::sam::memory::Error::Other("Invalid URL".to_string()).into())
    } else {
        Ok(())
    }
}

/// Applies domain-specific rate limiting
async fn apply_domain_rate_limit(url: &str) {
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(domain) = parsed_url.domain() {
            let domain_str = domain.to_string();
            let mut last_access_map = DOMAIN_LAST_ACCESS.lock().await;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64;

            if let Some(&last_access) = last_access_map.get(&domain_str) {
                let elapsed = now_ms.saturating_sub(last_access);
                if elapsed < 1000 {
                    let sleep_ms = 1000 - elapsed;
                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                }
            }

            last_access_map.insert(domain_str, now_ms);
        }
    }
}

/// Creates a new CrawledPage instance
fn create_crawled_page(job_oid: &str, url: &str) -> CrawledPage {
    let mut page = CrawledPage::new();
    page.crawl_job_oid = job_oid.to_string();
    page.url = url.to_string();
    page
}

/// Processes MIME type for the URL
async fn process_mime_type(
    url: &str,
    page: &mut CrawledPage,
) -> crate::sam::memory::Result<(Option<&'static str>, Vec<String>)> {
    let mut mime_tokens = Vec::new();
    let mime_ext = mime_type_from_url(url);

    let file_mime = if !mime_ext.contains("unknown") {
        Some(mime_ext)
    } else {
        None
    };

    if let Some(mime) = file_mime {
        // Check if this is a supported content type
        let is_supported = mime.starts_with("text/") ||
                          mime.starts_with("image/") ||
                          mime.starts_with("application/json") ||
                          mime.starts_with("application/xml") ||
                          mime.starts_with("application/pdf") ||
                          mime.starts_with("application/javascript") ||
                          mime.starts_with("text/javascript") ||
                          mime.starts_with("text/css") ||
                          mime.starts_with("application/x-javascript") ||
                          mime == "application/octet-stream"; // Generic binary, let content-type detection handle it
        
        if is_supported {
            mime_tokens.push(mime.to_string());
            log::debug!("Processing URL with MIME type: {}", mime);
        } else {
            // For other MIME types, check if they're explicitly blocked
            // Common blocked types (large media files)
            let blocked_types = ["video/", "audio/", "application/zip", "application/x-rar", "application/x-tar"];
            let is_blocked = blocked_types.iter().any(|bt| mime.starts_with(bt));
            
            if is_blocked {
                log::debug!("Skipping URL with blocked MIME type: {}", mime);
                page.tokens = vec![mime.to_string()];
                
                // Store in CrawlRejected table for analysis
                let mut rejection = super::CrawlRejected {
                    id: 0, // Will be set by database
                    url: url.to_string(),
                    domain: url.split('/').nth(2).unwrap_or("").to_string(),
                    path: url.to_string(),
                    reason: super::RejectionReason::UnsupportedContentType,
                    robots_rule: None,
                    user_agent: "".to_string(), // Will be set later if available
                    crawl_job_oid: None,
                    rejected_at: chrono::Utc::now().timestamp(),
                    retry_after: None,
                    rejection_count: 1,
                };
                
                // Try to save rejection (don't fail if database is unavailable)
                if let Err(e) = rejection.save().await {
                    log::debug!("Failed to save rejection record: {}", e);
                }
                
                return Err(crate::sam::memory::Error::Other(format!("Blocked MIME type: {}", mime)).into());
            }
            
            // Otherwise, allow it through and let content processor handle it
            log::debug!("Allowing URL with MIME type: {}", mime);
            mime_tokens.push(mime.to_string());
        }
    }

    Ok((file_mime, mime_tokens))
}

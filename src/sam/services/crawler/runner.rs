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
use log::info;
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
use url::ParseError;

use crate::sam::services::crawler::job::CrawlJob;
use crate::sam::services::crawler::page::CrawledPage;

// use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};

static REQWEST_CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(super::robots::DEFAULT_USER_AGENT) // Use the centralized User-Agent
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(8)
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

    // Check robots.txt compliance
    if !super::robots::is_url_allowed(&url).await {
        log::info!("URL blocked by robots.txt: {}", url);
        super::metrics::record_robots_block(&domain, &url).await;
        return Err(crate::sam::memory::Error::Other(
            "URL blocked by robots.txt".to_string(),
        )
        .into());
    }

    // Check circuit breaker
    if !super::circuit_breaker::is_domain_allowed(&domain).await {
        log::info!("Domain blocked by circuit breaker: {}", domain);
        super::metrics::record_circuit_breaker_block(&domain).await;
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
    let (file_mime, mut mime_tokens) = process_mime_type(&url, &mut page)?;

    let mut resp = None;
    let mut last_err = None;
    for attempt in 0..3 {
        match tokio::time::timeout(Duration::from_secs(60), client.get(&url).send()).await {
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
        ))
        .into()),
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
                // Await the response text here, outside spawn_blocking
                let html = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::warn!("Failed to get text for {}: {}", url, e);
                        String::new()
                    }
                };
                // Pass headers and html into the closure
                // Instead of spawn_blocking, do parsing directly (async context)
                let mut tokens = Vec::new();
                let mut links = Vec::new();

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

                if is_document && mime_tokens.iter().any(|m| m.starts_with("text/html")) {
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
    crawl_url_boxed(job_oid, url, 0, client).await
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
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);

        tokio::spawn(async {
            if let Err(e) = run_crawler_service().await {
                log::error!("Error in crawler service: {}", e);
            }
        });
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
}

/// Stops the crawler service and sets the running flag to false.
///
/// # Behavior
/// - Sets the global running flag to false.
/// - Logs service stop.
pub fn stop_service() {
    info!("Crawler service stopping...");
    CRAWLER_RUNNING.store(false, Ordering::SeqCst);
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
    let client = Arc::new(REQWEST_CLIENT.clone());
    let all_crawled_pages = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    // Set up logging
    // log::set_max_level(LevelFilter::Info);

    // Load common URLs, tokens, TLDs, prefixes, and words
    // let tlds = COMMON_TLDS.clone();
    // let prefixes = COMMON_PREFIXES.clone();
    // let words = COMMON_WORDS.clone();

    // DNS resolver setup
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    // Load DNS cache from redis or file
    load_dns_cache(true).await;

    // Track when we last saved DNS cache
    let mut last_dns_save = std::time::Instant::now();

    loop {
        if !CRAWLER_RUNNING.load(Ordering::SeqCst) {
            sleep(Duration::from_secs(1)).await;
            continue;
        }

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
            let _ = job.save_async().await;

            // Crawl start_url and discovered links (BFS, depth 2)
            let max_depth = 10;
            // Initialize visited set with URLs from all CrawlJob entries in Postgres
            let mut visited_urls = HashSet::new();
            if let Ok(crawled_pages) = CrawledPage::select_async(None, None, None, None).await {
                for page in crawled_pages {
                    visited_urls.insert(page.url);
                }
            }

            let mut job_urls = HashSet::new();
            if let Ok(all_jobs) = CrawlJob::select_async(None, None, None, None).await {
                for job in all_jobs {
                    job_urls.insert(job.start_url);
                }
            }

            let visited = Arc::new(tokio::sync::Mutex::new(visited_urls));
            let all_job_urls = Arc::new(tokio::sync::Mutex::new(job_urls));
            let queue = Arc::new(tokio::sync::Mutex::new(VecDeque::from([(
                job.start_url.clone(),
                0,
            )])));

            let concurrency = (num_cpus::get() * 2).max(8).min(32); // Better concurrency with limits
            loop {
                // Collect all URLs at the current minimum depth
                let (batch, _current_depth) = {
                    let mut q = queue.lock().await;
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
                        None => break,
                    };
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
                    (batch, min_depth)
                };
                if batch.is_empty() {
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
                use futures::stream;
                let results = stream::iter(batch.into_iter())
                    .map(|(url, depth)| {
                        let job_oid = job_oid.clone();

                        let client = client.clone();

                        async move {
                            // Crawl the URL
                            (url.clone(), depth, crawl_url(job_oid, url, client).await)
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect::<Vec<_>>()
                    .await;

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
                                                let mut cache_job =
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
            let sampled_words: Vec<_> = sampled_words.par_iter().take(5).cloned().collect(); // Reduced for faster startup

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

            let max_domains = 50; // Reduced from 1000 for faster processing
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
fn process_mime_type(
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
        if mime.starts_with("text/") {
            mime_tokens.push(mime.to_string());
        } else {
            page.tokens = vec![mime.to_string()];
            return Err(crate::sam::memory::Error::Other("Non-text MIME type".to_string()).into());
        }
    }

    Ok((file_mime, mime_tokens))
}

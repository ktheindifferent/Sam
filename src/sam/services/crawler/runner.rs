use crate::sam::services::crawler::job::CrawlJob;
use crate::sam::services::crawler::page::CrawledPage;
use std::collections::{HashSet, VecDeque, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU64};
use tokio::time::{sleep, Duration};
use once_cell::sync::{Lazy, OnceCell};
use std::path::Path;
use tokio::fs;
use log::{info, LevelFilter};
use trust_dns_resolver::TokioAsyncResolver;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use reqwest::Url;
// use futures::stream::FuturesUnordered;
use futures::StreamExt;
use num_cpus;
use rand::seq::SliceRandom;
use tokio::io::AsyncWriteExt;
use trust_dns_resolver::config::*;
use url::ParseError; // <-- Add this import
use std::pin::Pin;
use std::future::Future;
// use tokio_stream::StreamExt;

static REQWEST_CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(300)
        .pool_idle_timeout(Some(Duration::from_secs(60)))
        .danger_accept_invalid_certs(true)
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

// use rand::{seq::SliceRandom, thread_rng};

static CRAWLER_RUNNING: AtomicBool = AtomicBool::new(false);

// Add a static DNS cache (domain -> Option<bool> for found/not found)
static DNS_CACHE_PATH: &str = "/opt/sam/dns.cache";
static DNS_LOOKUP_CACHE: Lazy<tokio::sync::Mutex<HashMap<String, bool>>> = Lazy::new(|| tokio::sync::Mutex::new(HashMap::new()));

// Shared sleep-until timestamp (epoch seconds)
static SLEEP_UNTIL: once_cell::sync::Lazy<AtomicU64> = once_cell::sync::Lazy::new(|| AtomicU64::new(0));
static TIMEOUT_COUNT: once_cell::sync::Lazy<std::sync::Mutex<usize>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(0));

static REDIS_URL: &str = "redis://127.0.0.1/";

// Load DNS cache from disk or Redis at startup
fn load_dns_cache(should_use_redis: bool) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        use redis::AsyncCommands;
        if crate::sam::services::redis::is_running() && should_use_redis {
            match redis::Client::open(REDIS_URL) {
                Ok(client) => {
                    match client.get_multiplexed_async_connection().await {
                        Ok(mut con) => {
                            match redis::cmd("GET").arg("sam:dns_cache").query_async::<Option<Vec<u8>>>(&mut con).await {
                                Ok(Some(data)) => {
                                    if let Ok(map) = serde_json::from_slice::<HashMap<String, bool>>(&data) {
                                        let mut cache = DNS_LOOKUP_CACHE.lock().await;
                                        *cache = map;
                                        log::info!("Loaded DNS cache from Redis with {} entries", cache.len());
                                    } else {
                                        log::warn!("Failed to parse DNS cache from Redis");
                                        // TODO: Corupt so destroy and reload
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
                        Err(e) => {
                            log::warn!("Failed to connect to Redis: {}", e);
                            return load_dns_cache(false).await;
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to create Redis client: {}", e);
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
                    let mut cache = DNS_LOOKUP_CACHE.lock().await;
                    *cache = map;
                    log::info!("Loaded DNS cache from file with {} entries", cache.len());
                }
            }
        }
    })
}

// Save DNS cache to disk or Redis
async fn save_dns_cache() {
    let mut should_fallback = false;
    let cache = DNS_LOOKUP_CACHE.lock().await;
    let cache_bytes = match serde_json::to_vec(&*cache) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::warn!("Failed to serialize DNS cache: {}", e);
            return;
        }
    };
    if crate::sam::services::redis::is_running() {
        if let Ok(client) = redis::Client::open(REDIS_URL) {
            if let Ok(mut con) = client.get_multiplexed_async_connection().await {
                match redis::cmd("SET").arg("sam:dns_cache").arg(cache_bytes.clone()).query_async::<()>(&mut con).await {
                    Ok(_) => {
                        log::info!("Saved DNS cache to Redis with {} entries", cache.len());
                        return;
                    }
                    Err(e) => {
                        should_fallback = true;
                        log::warn!("Failed to save DNS cache to Redis: {}", e);
                    }
                }
            } else {
                should_fallback = true;
                log::warn!("Failed to connect to Redis for saving DNS cache");
            }
        } else {
            should_fallback = true;
            log::warn!("Failed to create Redis client for saving DNS cache");
        }
    } else {
        should_fallback = true;
    }

    if should_fallback {
        log::info!("Falling back to file-based DNS cache");
        let _ = fs::write(DNS_CACHE_PATH, cache_bytes).await;
    }
}

/// Writes a URL to the retry file (for failed/timed out crawls)
pub async fn write_url_to_retry_cache(url: &str) {
    let mut should_fallback = false;
    // Use Redis if available, otherwise fallback to file
    if crate::sam::services::redis::is_running() {
        match redis::Client::open(REDIS_URL) {
            Ok(client) => {
                match client.get_multiplexed_async_connection().await {
                    Ok(mut con) => {
                        if let Err(e) = redis::cmd("RPUSH")
                            .arg("sam:crawl_retry")
                            .arg(url)
                            .query_async::<()>(&mut con)
                            .await
                        {
                            should_fallback = true;
                            log::warn!("Failed to write retry URL to Redis: {}", e);
                        }
                    }
                    Err(e) => {
                        should_fallback = true;
                        log::warn!("Failed to connect to Redis for retry cache: {}", e);
                    }
                }
            }
            Err(e) => {
                should_fallback = true;
                log::warn!("Failed to create Redis client for retry cache: {}", e);
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
            if let Err(e) = file.write_all(format!("{}\n", url).as_bytes()).await {
                log::warn!("Failed to write timed out URL to retry file: {}", e);
            }
        } else {
            log::warn!("Failed to open retry file for writing");
        }
    }
}

/// Returns true if the input string is a valid URL (absolute, with scheme and host)
pub fn is_valid_url(s: &str) -> bool {
    match Url::parse(s) {
        Ok(url) => url.has_host() && url.scheme() != "",
        Err(_) => false,
    }
}


// Internal boxed async fn for recursion
// Prevent printing crawled webdata to terminal by not using println!, dbg!, eprintln!, or any direct output anywhere in this function or its parsing logic
// Returns all crawled pages (including recursive)
async fn crawl_url_inner(
    job_oid: String,
    url: String,
    depth: usize,
    established_clients: Vec<std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>>,
    client: std::sync::Arc<reqwest::Client>,
) -> crate::sam::memory::Result<Vec<CrawledPage>> {

    // Shared sleep logic: check if we should sleep before making a request
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let sleep_until = SLEEP_UNTIL.load(Ordering::SeqCst);
    if now < sleep_until {
        let sleep_secs = sleep_until - now;
        log::debug!("Global sleep in effect, sleeping for {} seconds", sleep_secs);
        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }

    // Bugfix: Check if the URL is valid before proceeding
    if !is_valid_url(&url) {
        return Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(format!("Invalid URL"))));
    }

    // Return early if the URL looks like a search endpoint
    let url_lc = url.to_ascii_lowercase();
    if url_lc.contains("/search/")
        || url_lc.contains("search=")
        || url_lc.contains("q=")
        || url_lc.contains("/find/")
        || url_lc.contains("/query/")
        || url_lc.contains("query=")
        || url_lc.contains("/lookup/")
        || url_lc.contains("lookup=")
        || url_lc.contains("/results/")
        || url_lc.contains("results=")
        || url_lc.contains("/explore/")
        || url_lc.contains("explore=")
        || url_lc.contains("/filter/")
        || url_lc.contains("filter=")
        || url_lc.contains("/discover/")
        || url_lc.contains("discover=")
        || url_lc.contains("/browse/")
        || url_lc.contains("browse=")
        || url_lc.contains("u=")
        || url_lc.contains("url=")
        || url_lc.contains("id=")
        || url_lc.contains("backURL=")
        || url_lc.contains("text=")
        || url_lc.contains("searchterm=")
        || url_lc.contains("/list/")

    {
        return Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(
            "URL appears to be a search endpoint, skipping".to_string(),
        )));
    }


    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String(format!("%{}%",url.clone())));
    pg_query.query_columns.push("url ilike".to_string());
    let existing = match CrawledPage::select_async(None, None, None, Some(pg_query), established_clients.clone()).await {
        Ok(pages) => pages,
        Err(e) => {
            log::debug!("Failed to query existing CrawledPage: {}", e);
            Vec::new()
        }
    };
    if !existing.is_empty() {
        return Ok(existing);
    }

    let mut page = CrawledPage::new();
    page.crawl_job_oid = job_oid.clone();
    page.url = url.clone();

  
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
                log::debug!("HTTP request error (attempt {}): {} for {}", attempt + 1, last_err.as_ref().unwrap(), url);
            }
            Err(_) => {
                last_err = Some("Request timed out".to_string());
                log::error!("HTTP request timed out (attempt {}): {}", attempt + 1, url);
            }
        }
        // Optional: small delay between retries
        sleep(Duration::from_millis(300)).await;
    }
    let resp = match resp {
        Some(r) => Ok(r),
        None => Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(
            format!("Request failed after retries: {}", last_err.unwrap_or_else(|| "unknown".to_string()))
        )))
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
                // Try to extract the MIME type from the Content-Type header, ignoring parameters like charset
                let mut mime_from_header: Option<String> = None;
                if let Some(mimeh) = headers_clone.get("Content-Type").or_else(|| headers_clone.get("content-type")) {
                    if let Ok(mime_str) = mimeh.to_str() {
                        // Only take the part before ';' (ignore charset, etc.), trim, and lowercase
                        let mime_main = mime_str.split(';').next().unwrap_or(mime_str).trim().to_ascii_lowercase();
                        if !mime_main.is_empty() {
                            mime_from_header = Some(mime_main);
                        }
                    }
                }
                // Await the response text here, outside spawn_blocking
                let html = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::warn!("Failed to get text for {}: {}", url, e);
                        String::new()
                    }
                };
                // Pass headers and html into the closure
                let result = tokio::task::spawn_blocking(move || {
                    let mut tokens = Vec::new();
                    let mut links = Vec::new();
                    let mut mime_tokens = Vec::new();
                    let url_lc = url_clone.to_ascii_lowercase();
                    let mut file_mime: Option<&str> = None;
                    // ...existing code...
                    // Bugfix: Check for special replacement character (�) in the HTML body or any tag text
                    let document = scraper::Html::parse_document(&html);
                    let contains_replacement_char = html.contains('�')
                        || document.root_element().text().any(|t| t.contains('�'));
                    if contains_replacement_char {
                        return (mime_tokens, tokens, links);
                    }
                    // ...existing code...
                    let file_ext = {
                        let url_no_query = url_lc.split(&['?', '#'][..]).next().unwrap_or("");
                        let path = std::path::Path::new(url_no_query);
                        // Only treat as file if the last segment contains a dot (.) and is not a known TLD
                        if let Some(segment) = path.file_name().and_then(|s| s.to_str()) {
                            if segment.contains('.') {
                                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                                    // List of common TLDs to exclude
                                    // List of all known TLDs (as of 2024-06, from IANA root zone database)
                                    // Source: https://data.iana.org/TLD/tlds-alpha-by-domain.txt
                                    let tlds = COMMON_TLDS.clone();
                                    if !tlds.contains(&ext.to_string()) {
                                        Some(format!(".{}", ext))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(ref ext) = file_ext {
                        for (map_ext, mime) in crate::sam::tools::MIME_MAP.iter() {
                            if ext.eq_ignore_ascii_case(map_ext) {
                                file_mime = Some(*mime);
                                break;
                            }
                        }
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
                        ".html", ".htm", ".xhtml", ".shtml", ".php", ".asp", ".aspx", ".jsp", ".jspx", ".cgi", ".pl", ".cfm", ".rb", ".py", ".xml", ".json", ".md", ".txt", "/"
                    ];
                    // If no extension, treat as document; otherwise, check if extension is in doc_exts
                    let is_document = match &file_ext {
                        Some(ext) => doc_exts.iter().any(|d| ext.eq_ignore_ascii_case(d)),
                        None => true,
                    };
                    
                    if is_document && mime_tokens.iter().any(|m| m.starts_with("text/html")) {

                        // Bugfix: Check for special replacement character (�) in the HTML body or any tag text
                        let document = scraper::Html::parse_document(&html);
                        let contains_replacement_char = html.contains('�')
                            || document.root_element().text().any(|t| t.contains('�'));
                        if contains_replacement_char {
                            return (mime_tokens, tokens, links);
                        }
                
                        let body_selector = match scraper::Selector::parse("body") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'body': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        let skip_tags = ["script", "style", "noscript", "svg", "canvas", "iframe", "template"];
                        let skip_selector = skip_tags
                            .iter()
                            .filter_map(|tag| match scraper::Selector::parse(tag) {
                                Ok(selector) => Some(selector),
                                Err(e) => {
                                    log::warn!("Failed to parse selector '{}': {}", tag, e);
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
    

                        for body in document.select(&body_selector) {
                            extract_text(&body, &skip_selector, &mut tokens);
                        }

                        let a_selector = match scraper::Selector::parse("a[href]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'a[href]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&a_selector) {
                            if let Some(link) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(link)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(link)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let img_selector = match scraper::Selector::parse("img[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'img[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&img_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let audio_selector = match scraper::Selector::parse("audio[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'audio[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&audio_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let source_selector = match scraper::Selector::parse("audio source[src], video source[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'audio source[src], video source[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&source_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let video_selector = match scraper::Selector::parse("video[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'video[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&video_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let link_selector = match scraper::Selector::parse("link[rel=\"stylesheet\"]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'link[rel=\"stylesheet\"]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&link_selector) {
                            if let Some(href) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(href)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(href)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let script_selector = match scraper::Selector::parse("script[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'script[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&script_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src).or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }
                    } else {
                        log::debug!("Skipping non-document file: {}", url_clone.clone());
                        // println!("{:?}: {}", file_ext, is_document);
                
                    }

                    
                    
                    (mime_tokens, tokens, links)
                }).await;

                let (mut mime_tokens, mut tokens, mut links) = match result {
                    Ok((mime_tokens, tokens, links)) => (mime_tokens, tokens, links),
                    Err(e) => {
                        log::warn!("Failed to parse HTML for {}: {}", url, e);
                        (Vec::new(), Vec::new(), Vec::new())
                    }
                };

                tokens.sort();
                tokens.dedup();
                links.sort();
                links.dedup();

    
                let date_regex = regex::Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}$");
                let date2_regex = regex::Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$");
                let date3_regex = regex::Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$");
                let date4_regex = regex::Regex::new(r"^\d{8}$");
                let date5_regex = regex::Regex::new(r"^\d{4}\.\d{1,2}\.\d{1,2}$");
                let date6_regex = regex::Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$");
                let date7_regex = regex::Regex::new(r"^\d{4}-\d{2}-\2}(T\d{2}:\d{2}(:\d{2})?(Z|([+-]\d{2}:\d{2}))?)?$");

                tokens.retain(|token| {
                    !COMMON_TOKENS.contains(token)
                        || date_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date2_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date3_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date4_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date5_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date6_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date7_regex.as_ref().map_or(false, |re| re.is_match(token))
                });
                tokens.retain(|token| token.len() > 2 && token.len() < 50);
                let url_tokens: HashSet<_> = url.split('/').map(|s| s.to_lowercase()).collect();
                tokens.retain(|token| !url_tokens.contains(&token.to_lowercase()));
                if let Ok(domain) = Url::parse(&url).and_then(|u| {
                    u.domain()
                        .map(|d| d.to_string())
                        .ok_or_else(|| ParseError::EmptyHost)
                }) {
                    let domain_tokens: HashSet<_> = domain.split('.').map(|s| s.to_lowercase()).collect();
                    tokens.retain(|token| !domain_tokens.contains(&token.to_lowercase()));
                }

                let mut all_tokens = mime_tokens;
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
                
                
                all_pages.push(page.clone());
                
                
            } else {
                write_url_to_retry_cache(&url).await;
            }
        }
        Err(e) => {
            log::warn!("Error fetching URL {}: {}", url, e);

            write_url_to_retry_cache(&url).await;

            // If the error is a timeout, increment a static counter and occasionally sleep all threads
            
            let err_str = e.to_string().to_ascii_lowercase();
            if err_str.contains("timed out") || err_str.contains("timeout") {
                let mut count = TIMEOUT_COUNT.lock().unwrap();
                *count += 1;
                if (*count % 10) == 0 {
                    // Set global sleep for all threads for a random duration between 10 and 120 seconds
                    let mut rng = rand::thread_rng();
                    let sleep_secs = rng.gen_range(10..=120);
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let until = now + sleep_secs;
                    SLEEP_UNTIL.store(until, Ordering::SeqCst);
                    log::warn!("Timeout detected {} times, sleeping ALL threads for {} seconds to avoid ban", *count, sleep_secs);
                }
            }
        }
    }

    Ok(all_pages)
}

// Boxed async fn for recursion
fn crawl_url_boxed<'a>(job_oid: String, url: String, depth: usize, established_clients: Vec<std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>>, client: std::sync::Arc<reqwest::Client>) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::sam::memory::Result<Vec<CrawledPage>>> + Send + 'a>> {
    Box::pin(crawl_url_inner(job_oid, url, depth, established_clients, client))
}

// Public entry point (non-recursive, just calls boxed version)
pub async fn crawl_url(job_oid: String, url: String, established_clients: Vec<std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>>, client: std::sync::Arc<reqwest::Client>) -> crate::sam::memory::Result<Vec<CrawledPage>> {
    crawl_url_boxed(job_oid, url, 0, established_clients, client).await
}

/// Async-friendly version for use from async contexts (e.g., ratatui CLI)
pub async fn start_service_async() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);
   
        tokio::spawn(async {
            let cpu_cores = num_cpus::get();
            for _ in 0..cpu_cores {
                tokio::spawn(async {
                    if let Err(e) = run_crawler_service().await {
                        log::error!("Error in crawler service: {}", e);
                    }
                });
            }
        });
            
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
}

pub fn stop_service() {
    info!("Crawler service stopping...");
    CRAWLER_RUNNING.store(false, Ordering::SeqCst);
    info!("Crawler service stopped.");
}

pub fn service_status() -> &'static str {
    if CRAWLER_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

/// Main crawler loop: finds pending jobs, crawls, updates status
// TODO: Make postgres connection pool size configurable or higher level
pub async fn run_crawler_service() -> crate::sam::memory::Result<()> {
    let client = Arc::new(REQWEST_CLIENT.clone());

    // Set up logging
    log::set_max_level(LevelFilter::Info);

    // Establish an async connection pool for PostgreSQL using tokio_postgres
    let mut established_clients = Vec::new();
    for i in 0..2 {
        let pgclient = Arc::new(tokio::sync::Mutex::new(crate::sam::memory::Config::client_async().await.unwrap()));
        established_clients.push(pgclient);
    }

    // Load common URLs, tokens, TLDs, prefixes, and words
    let tlds = COMMON_TLDS.clone();
    let prefixes = COMMON_PREFIXES.clone();
    let words = COMMON_WORDS.clone();
   
    // DNS resolver setup
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
        .expect("Failed to create DNS resolver");

    // Load DNS cache from redis or file
    load_dns_cache(true).await;

    loop {
        if (!CRAWLER_RUNNING.load(Ordering::SeqCst)) {
            sleep(Duration::from_secs(1)).await;
            continue;
        }
    

        // Find a pending job
        let mut jobs = match CrawlJob::select_async(Some(100), None, None, None).await {
            Ok(jobs) => jobs.into_iter().filter(|j| j.status == "pending").collect::<Vec<_>>(),
            Err(_) => vec![],
        };

        jobs.shuffle(&mut rand::thread_rng());

        if let Some(mut job) = jobs.into_iter().next() {
            let job_oid = job.oid.clone();
            info!("Starting crawl job: oid={} url={}", job.oid, job.start_url);
            // Mark as running
            job.status = "running".to_string();
            job.updated_at = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    log::debug!("SystemTime before UNIX EPOCH: {:?}", e);
                    0
                }
            };
            let _ = job.save_async().await;

            // Crawl start_url and discovered links (BFS, depth 2)
            let max_depth = 10;
            let visited = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
            let queue = Arc::new(tokio::sync::Mutex::new(VecDeque::from([(job.start_url.clone(), 0)])));
            let all_crawled_pages = Arc::new(tokio::sync::Mutex::new(Vec::new()));

            let concurrency = num_cpus::get().max(4); // At least 4 concurrent tasks
            loop {
                // Collect all URLs at the current minimum depth
                let (batch, current_depth) = {
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
                            let (url, depth) = q.remove(i).unwrap();
                            batch.push((url, depth));
                        } else {
                            i += 1;
                        }
                    }
                    (batch, min_depth)
                };
                if batch.is_empty() { break; }
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
                        let established_clients = established_clients.clone();
                        let client = client.clone();
                        async move {
                            (url.clone(), depth, crawl_url(job_oid, url, established_clients, client).await)
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect::<Vec<_>>()
                    .await;
                // Process results
                let mut new_links = Vec::new();
                for (url, depth, result) in results {
                    match result {
                        Ok(mut pages) => {
                            for page in &pages {
                                for link in &page.links {
                                    let should_add = {
                                        let v = visited.lock().await;
                                        !v.contains(link)
                                    };
                                    if should_add {
                                        new_links.push((link.clone(), depth + 1));
                                    }
                                }
                            }
                            // Add pages to all_crawled_pages in one lock
                            {
                                let mut all = all_crawled_pages.lock().await;
                                all.append(&mut pages);
                                if all.len() > 500 {
                                    // Batch save all crawled pages in chunks of 500
                                    for chunk in all.chunks(500) {
                                        if let Err(e) = CrawledPage::save_async_batch(chunk, established_clients.clone()).await {
                                            log::warn!("Failed to save batch of pages: {}", e);
                                            for p in chunk {
                                                write_url_to_retry_cache(&p.url).await;
                                            }
                                        }
                                    }
                                    all.clear();
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

            let all_crawled_pages = match Arc::try_unwrap(all_crawled_pages) {
                Ok(mutex) => mutex.into_inner(),
                Err(arc) => arc.blocking_lock().clone(),
            };
            // Batch save all crawled pages in chunks of 500
            for chunk in all_crawled_pages.chunks(500) {
                if let Err(e) = CrawledPage::save_async_batch(chunk, established_clients.clone()).await {
                    log::warn!("Failed to save batch of pages: {}", e);
                    for p in chunk {
                        write_url_to_retry_cache(&p.url).await;
                    }
                }
            }
            // Mark job as done
            job.status = "done".to_string();
            job.updated_at = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    log::warn!("SystemTime before UNIX EPOCH: {:?}", e);
                    0
                }
            };
            crate::sam::services::crawler::job::CrawlJob::destroy_async(job.oid.clone()).await.unwrap_or_else(|_| {
                log::warn!("Failed to destroy crawl job: oid={}", job.oid);
                false
            });
            // let _ = job.save_async().await;
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
                let _ = fs::remove_file(retry_path).await.unwrap_or_else(|_| {
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
            let mut domains: Vec<String> = Vec::new();
            use rayon::prelude::*;

            let mut rng = SmallRng::from_entropy();
            let mut sampled_words = words.clone();
            sampled_words.shuffle(&mut rng);
            // Use rayon's par_iter to efficiently take the first 1,000 elements in parallel
            let sampled_words: Vec<_> = sampled_words.par_iter().take(10).cloned().collect();

            let domain_gen_duration = domain_gen_start.elapsed();
            log::info!("Domain generation took {:?}", domain_gen_duration);

            let mut domains: Vec<String> = tlds.par_iter().flat_map_iter(|tld| {
             

                let mut local_domains = Vec::with_capacity(
                    sampled_words.len() * (1 + prefixes.len() + sampled_words.len() * prefixes.len()) + prefixes.len() + sampled_words.len()
                );

                // word.tld and prefix.word.tld and prefix.word2.word.tld
                for word in &sampled_words {
                    local_domains.push(format!("{}.{}", word, tld));
                    for prefix in &prefixes {
                        local_domains.push(format!("{}.{}.{}", prefix, word, tld));
                        for word2 in &sampled_words {
                            local_domains.push(format!("{}.{}.{}.{}", prefix, word2, word, tld));
                        }
                    }
                }
                // prefix.tld
                for prefix in &prefixes {
                    local_domains.push(format!("{}.{}", prefix, tld));
                }
                // word.tld (again, but dedup later)
                for word in &sampled_words {
                    local_domains.push(format!("{}.{}", word, tld));
                }
                local_domains
            }).collect();
            let mut rng = SmallRng::from_entropy();
            domains.sort();
            domains.dedup();
            domains.shuffle(&mut rng);

            let max_domains = 1000;
            let domains = &domains[..std::cmp::min(domains.len(), max_domains)];

            let mut urls_found = Vec::new();

            // Use concurrency to speed up DNS lookups
            let concurrency = num_cpus::get();
            log::info!("Starting DNS lookups for {} domains with concurrency {}", domains.len(), concurrency);
            let dns_start = tokio::time::Instant::now();

            let found_domains = tokio_stream::iter(domains.iter().cloned())
                .map(|domain| {
                    let resolver = resolver.clone();
                    let client_clone = client.clone();
                    async move {
                        let lookup_start = tokio::time::Instant::now();
                        let found = lookup_domain(&resolver, &domain, client_clone).await;
                        let lookup_duration = lookup_start.elapsed();
                        log::debug!("DNS+HTTP lookup for domain {} took {:?} (found={})", domain, lookup_duration, found);
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
            log::info!("DNS+HTTP lookups for {} domains took {:?}", domains.len(), dns_duration);

            for domain in found_domains {
                urls_found.push(format!("https://{}/", domain));
                urls_found.push(format!("http://{}/", domain));
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
                let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
                let mut job = CrawlJob::new();
                job.start_url = url.clone();
                job.status = "pending".to_string();
                job.created_at = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
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
                log::debug!("Created crawl job for URL: {} in {:?}", url, job_create_duration);
            }
        }
        sleep(Duration::from_secs(10)).await;
    }
}


// Helper function to perform a single DNS lookup with cache
async fn lookup_domain(
    resolver: &TokioAsyncResolver,
    domain: &str,
    client: std::sync::Arc<reqwest::Client>
) -> bool {
    // Check cache first
    {
        let cache = DNS_LOOKUP_CACHE.lock().await;
        if let Some(found) = cache.get(domain) {
            return *found;
        }
    }
    // Not in cache, do DNS lookup
    let mut found = false;
    for attempt in 0..3 {
        let result = match tokio::time::timeout(
            Duration::from_secs(15), // Increased from 10 to 15
            resolver.lookup_ip(domain)
        ).await {
            Ok(Ok(lookup)) if lookup.iter().next().is_some() => {
                // DNS exists, now check HTTP/HTTPS HEAD
                let http_url = format!("http://{}/", domain);
                let https_url = format!("https://{}/", domain);

              
                    let mut http_ok = false;
                    let mut https_ok = false;
                    for http_attempt in 0..3 {
                        let http_fut = client.head(&http_url).send();
                        let https_fut = client.head(&https_url).send();
                        let result = tokio::time::timeout(
                            Duration::from_secs(15),
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
                                log::warn!("HEAD request timed out or failed (attempt {}): {}", http_attempt + 1, domain);
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
            Ok(_) | Err(_) => {
                log::warn!("DNS lookup timed out or failed (attempt {}): {}", attempt + 1, domain);
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
    found
}

fn extract_text(element: &scraper::ElementRef, skip_selector: &[scraper::Selector], tokens: &mut Vec<String>) {
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
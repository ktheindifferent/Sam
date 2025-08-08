//! # Sitemap Parser Module
//!
//! This module implements XML sitemap parsing for the web crawler.
//! It discovers and extracts URLs from sitemap.xml files to improve crawl coverage.
//!
//! ## Features
//! - Parse standard sitemap.xml format
//! - Support for sitemap index files
//! - Extract URLs with priority and change frequency
//! - Handle compressed sitemaps (gzip)
//! - Recursive sitemap discovery

use std::collections::HashSet;
use std::time::Duration;
use reqwest::Url;
use scraper::{Html, Selector};
use log::{debug, warn, error};
use serde::Deserialize;

use super::robots::DEFAULT_USER_AGENT;

/// Represents a URL entry from a sitemap
#[derive(Debug, Clone)]
pub struct SitemapEntry {
    pub url: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<String>,
    pub priority: Option<f64>,
}

/// Represents a sitemap index entry
#[derive(Debug, Clone)]
pub struct SitemapIndex {
    pub loc: String,
    pub lastmod: Option<String>,
}

/// Maximum number of URLs to extract from a single sitemap
const MAX_URLS_PER_SITEMAP: usize = 10000;

/// Maximum depth for recursive sitemap fetching
const MAX_SITEMAP_DEPTH: usize = 3;

/// Fetch and parse a sitemap from a URL
pub async fn fetch_sitemap(url: &str) -> Result<Vec<SitemapEntry>, Box<dyn std::error::Error>> {
    debug!("Fetching sitemap from: {}", url);
    
    let client = reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(false)
        .build()?;
    
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to fetch sitemap: HTTP {}", response.status()).into());
    }
    
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let bytes = response.bytes().await?;
    
    // Handle gzip compressed sitemaps
    let content = if url.ends_with(".gz") || content_type.contains("gzip") {
        decompress_gzip(&bytes)?
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    
    // Check if this is a sitemap index
    if content.contains("<sitemapindex") {
        parse_sitemap_index(&content).await
    } else {
        parse_sitemap(&content)
    }
}

/// Decompress gzip content
fn decompress_gzip(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;
    Ok(content)
}

/// Parse a standard sitemap.xml
fn parse_sitemap(content: &str) -> Result<Vec<SitemapEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    
    // Try to parse as XML
    let doc = Html::parse_document(content);
    
    // Create selectors for sitemap elements
    let url_selector = Selector::parse("url").unwrap();
    let loc_selector = Selector::parse("loc").unwrap();
    let lastmod_selector = Selector::parse("lastmod").unwrap();
    let changefreq_selector = Selector::parse("changefreq").unwrap();
    let priority_selector = Selector::parse("priority").unwrap();
    
    for url_element in doc.select(&url_selector).take(MAX_URLS_PER_SITEMAP) {
        let loc = url_element
            .select(&loc_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string());
        
        if let Some(url) = loc {
            if url.is_empty() {
                continue;
            }
            
            let lastmod = url_element
                .select(&lastmod_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());
            
            let changefreq = url_element
                .select(&changefreq_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());
            
            let priority = url_element
                .select(&priority_selector)
                .next()
                .and_then(|e| e.text().collect::<String>().trim().parse::<f64>().ok());
            
            entries.push(SitemapEntry {
                url,
                lastmod,
                changefreq,
                priority,
            });
        }
    }
    
    // Fallback to regex parsing if XML parsing fails or returns empty
    if entries.is_empty() {
        entries = parse_sitemap_regex(content);
    }
    
    debug!("Parsed {} URLs from sitemap", entries.len());
    Ok(entries)
}

/// Parse sitemap using regex as fallback
fn parse_sitemap_regex(content: &str) -> Vec<SitemapEntry> {
    let mut entries = Vec::new();
    let url_regex = regex::Regex::new(r"<loc>\s*([^<]+)\s*</loc>").unwrap();
    
    for cap in url_regex.captures_iter(content).take(MAX_URLS_PER_SITEMAP) {
        if let Some(url) = cap.get(1) {
            let url_str = url.as_str().trim();
            if !url_str.is_empty() {
                entries.push(SitemapEntry {
                    url: url_str.to_string(),
                    lastmod: None,
                    changefreq: None,
                    priority: None,
                });
            }
        }
    }
    
    entries
}

/// Parse a sitemap index file and recursively fetch referenced sitemaps
async fn parse_sitemap_index(content: &str) -> Result<Vec<SitemapEntry>, Box<dyn std::error::Error>> {
    let mut all_entries = Vec::new();
    let mut sitemap_urls = Vec::new();
    
    // Parse sitemap index
    let doc = Html::parse_document(content);
    let sitemap_selector = Selector::parse("sitemap").unwrap();
    let loc_selector = Selector::parse("loc").unwrap();
    
    for sitemap_element in doc.select(&sitemap_selector) {
        if let Some(loc) = sitemap_element
            .select(&loc_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
        {
            if !loc.is_empty() {
                sitemap_urls.push(loc);
            }
        }
    }
    
    // Fallback to regex if XML parsing fails
    if sitemap_urls.is_empty() {
        let sitemap_regex = regex::Regex::new(r"<sitemap>.*?<loc>\s*([^<]+)\s*</loc>.*?</sitemap>").unwrap();
        for cap in sitemap_regex.captures_iter(content) {
            if let Some(url) = cap.get(1) {
                sitemap_urls.push(url.as_str().trim().to_string());
            }
        }
    }
    
    debug!("Found {} sitemaps in index", sitemap_urls.len());
    
    // Fetch each sitemap (with concurrency limit)
    let futures: Vec<_> = sitemap_urls
        .into_iter()
        .take(10) // Limit to 10 sitemaps to avoid overwhelming
        .map(|url| fetch_sitemap_recursive(&url, 1))
        .collect();
    
    for future in futures {
        match future.await {
            Ok(entries) => all_entries.extend(entries),
            Err(e) => warn!("Failed to fetch sitemap: {}", e),
        }
    }
    
    Ok(all_entries)
}

/// Recursively fetch sitemaps with depth limit
async fn fetch_sitemap_recursive(url: &str, depth: usize) -> Result<Vec<SitemapEntry>, Box<dyn std::error::Error>> {
    if depth > MAX_SITEMAP_DEPTH {
        return Ok(vec![]);
    }
    
    fetch_sitemap(url).await
}

/// Discover sitemap URLs for a domain
pub async fn discover_sitemaps(domain: &str) -> Vec<String> {
    let mut sitemap_urls = Vec::new();
    
    // Common sitemap locations
    let common_paths = vec![
        "/sitemap.xml",
        "/sitemap_index.xml",
        "/sitemap-index.xml",
        "/sitemaps/sitemap.xml",
        "/sitemap.xml.gz",
        "/sitemap1.xml",
        "/post-sitemap.xml",
        "/page-sitemap.xml",
        "/product-sitemap.xml",
        "/category-sitemap.xml",
    ];
    
    let base_url = if domain.starts_with("http") {
        domain.to_string()
    } else {
        format!("https://{}", domain)
    };
    
    for path in common_paths {
        sitemap_urls.push(format!("{}{}", base_url, path));
    }
    
    // Also check robots.txt for sitemap references
    if let Ok(robots_sitemaps) = super::robots::get_sitemaps(domain).await.into_iter().collect::<Result<Vec<_>, _>>() {
        sitemap_urls.extend(robots_sitemaps);
    }
    
    sitemap_urls
}

/// Extract all URLs from sitemaps for a domain
pub async fn extract_urls_from_sitemaps(domain: &str) -> HashSet<String> {
    let mut all_urls = HashSet::new();
    let sitemap_urls = discover_sitemaps(domain).await;
    
    for sitemap_url in sitemap_urls {
        match fetch_sitemap(&sitemap_url).await {
            Ok(entries) => {
                for entry in entries {
                    // Validate URL before adding
                    if let Ok(_) = Url::parse(&entry.url) {
                        all_urls.insert(entry.url);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to fetch sitemap {}: {}", sitemap_url, e);
            }
        }
    }
    
    debug!("Extracted {} URLs from sitemaps for {}", all_urls.len(), domain);
    all_urls
}

/// Filter URLs by priority
pub fn filter_by_priority(entries: Vec<SitemapEntry>, min_priority: f64) -> Vec<SitemapEntry> {
    entries.into_iter()
        .filter(|entry| entry.priority.unwrap_or(0.5) >= min_priority)
        .collect()
}

/// Filter URLs by change frequency
pub fn filter_by_changefreq(entries: Vec<SitemapEntry>, allowed_freqs: &[&str]) -> Vec<SitemapEntry> {
    entries.into_iter()
        .filter(|entry| {
            if let Some(ref freq) = entry.changefreq {
                allowed_freqs.iter().any(|&f| freq.eq_ignore_ascii_case(f))
            } else {
                true // Include if no changefreq specified
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sitemap_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <url>
        <loc>https://example.com/page1</loc>
        <lastmod>2024-01-01</lastmod>
        <changefreq>daily</changefreq>
        <priority>0.8</priority>
    </url>
    <url>
        <loc>https://example.com/page2</loc>
        <priority>0.5</priority>
    </url>
</urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "https://example.com/page1");
        assert_eq!(entries[0].priority, Some(0.8));
        assert_eq!(entries[1].url, "https://example.com/page2");
    }

    #[test]
    fn test_parse_sitemap_regex_fallback() {
        let xml = r#"<urlset>
    <url><loc>https://example.com/test1</loc></url>
    <url><loc>https://example.com/test2</loc></url>
</urlset>"#;

        let entries = parse_sitemap_regex(xml);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "https://example.com/test1");
        assert_eq!(entries[1].url, "https://example.com/test2");
    }

    #[test]
    fn test_filter_by_priority() {
        let entries = vec![
            SitemapEntry {
                url: "url1".to_string(),
                priority: Some(0.8),
                lastmod: None,
                changefreq: None,
            },
            SitemapEntry {
                url: "url2".to_string(),
                priority: Some(0.3),
                lastmod: None,
                changefreq: None,
            },
            SitemapEntry {
                url: "url3".to_string(),
                priority: None,
                lastmod: None,
                changefreq: None,
            },
        ];

        let filtered = filter_by_priority(entries, 0.5);
        assert_eq!(filtered.len(), 2); // url1 and url3 (default 0.5)
    }
}
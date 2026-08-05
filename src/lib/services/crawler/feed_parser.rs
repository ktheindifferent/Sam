//! RSS and Atom feed detection and parsing
//!
//! This module provides functionality to detect, parse, and extract URLs
//! from RSS and Atom feeds for improved crawl coverage.

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Represents a feed item/entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub title: Option<String>,
    pub link: String,
    pub description: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub guid: Option<String>,
    pub author: Option<String>,
    pub categories: Vec<String>,
    pub enclosures: Vec<String>,
}

/// Represents a parsed feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub title: Option<String>,
    pub link: Option<String>,
    pub description: Option<String>,
    pub feed_type: FeedType,
    pub items: Vec<FeedItem>,
    pub last_build_date: Option<DateTime<Utc>>,
}

/// Type of feed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FeedType {
    RSS,
    Atom,
    Unknown,
}

/// Detect feed links in HTML content
pub fn detect_feed_links(html: &str) -> Vec<String> {
    let mut feed_urls = Vec::new();

    // Look for link tags with RSS/Atom types
    let feed_patterns = [
        r#"<link[^>]*type=["']application/rss\+xml["'][^>]*href=["']([^"']+)["']"#,
        r#"<link[^>]*href=["']([^"']+)["'][^>]*type=["']application/rss\+xml["']"#,
        r#"<link[^>]*type=["']application/atom\+xml["'][^>]*href=["']([^"']+)["']"#,
        r#"<link[^>]*href=["']([^"']+)["'][^>]*type=["']application/atom\+xml["']"#,
    ];

    for pattern in &feed_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            for cap in regex.captures_iter(html) {
                if let Some(url) = cap.get(1) {
                    feed_urls.push(url.as_str().to_string());
                }
            }
        }
    }

    // Also look for common feed URLs
    let doc = scraper::Html::parse_document(html);
    let link_selector = scraper::Selector::parse("a").unwrap();

    for element in doc.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            let href_lower = href.to_lowercase();
            if href_lower.contains("/feed")
                || href_lower.contains("/rss")
                || href_lower.contains("/atom")
                || href_lower.ends_with(".rss")
                || href_lower.ends_with(".atom")
                || href_lower.ends_with(".xml")
                    && (href_lower.contains("feed") || href_lower.contains("rss"))
            {
                feed_urls.push(href.to_string());
            }
        }
    }

    // Deduplicate
    let unique: HashSet<String> = feed_urls.into_iter().collect();
    unique.into_iter().collect()
}

/// Check if a URL might be a feed
pub fn is_feed_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    url_lower.contains("/feed")
        || url_lower.contains("/rss")
        || url_lower.contains("/atom")
        || url_lower.ends_with(".rss")
        || url_lower.ends_with(".atom")
        || url_lower.ends_with("/feed.xml")
        || url_lower.ends_with("/rss.xml")
        || url_lower.ends_with("/atom.xml")
}

/// Fetch and parse a feed from URL
pub async fn fetch_and_parse_feed(url: &str) -> Result<Feed> {
    debug!("Fetching feed from: {}", url);

    let client = reqwest::Client::builder()
        .user_agent(super::robots::DEFAULT_USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch feed: HTTP {}",
            response.status()
        ));
    }

    // Check content type
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let is_feed = content_type.contains("rss")
        || content_type.contains("atom")
        || content_type.contains("xml")
        || content_type.contains("text/plain");

    if !is_feed && !content_type.is_empty() && !content_type.contains("html") {
        warn!("Unexpected content type for feed: {}", content_type);
    }

    let content = response.text().await?;
    parse_feed(&content)
}

/// Parse feed content
pub fn parse_feed(content: &str) -> Result<Feed> {
    // Detect feed type
    let feed_type = detect_feed_type(content);

    match feed_type {
        FeedType::RSS => parse_rss(content),
        FeedType::Atom => parse_atom(content),
        FeedType::Unknown => {
            // Try both parsers
            parse_rss(content).or_else(|_| parse_atom(content))
        }
    }
}

/// Detect the type of feed from content
fn detect_feed_type(content: &str) -> FeedType {
    let content_lower = content.to_ascii_lowercase();
    if content_lower.contains("<rss") || content_lower.contains("<channel>") {
        FeedType::RSS
    } else if content_lower.contains("<feed")
        && content_lower.contains("xmlns")
        && content_lower.contains("atom")
    {
        FeedType::Atom
    } else {
        FeedType::Unknown
    }
}

/// Parse RSS feed
fn parse_rss(content: &str) -> Result<Feed> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut feed = Feed {
        title: None,
        link: None,
        description: None,
        feed_type: FeedType::RSS,
        items: Vec::new(),
        last_build_date: None,
    };

    let mut current_item: Option<FeedItem> = None;
    let mut current_element = String::new();
    let mut in_channel = false;
    let mut in_item = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                match name.as_str() {
                    "channel" => in_channel = true,
                    "item" => {
                        in_item = true;
                        current_item = Some(FeedItem {
                            title: None,
                            link: String::new(),
                            description: None,
                            pub_date: None,
                            guid: None,
                            author: None,
                            categories: Vec::new(),
                            enclosures: Vec::new(),
                        });
                    }
                    "enclosure" => {
                        if let Some(ref mut item) = current_item {
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    if attr.key.as_ref() == b"url" {
                                        let url = String::from_utf8_lossy(&attr.value).to_string();
                                        item.enclosures.push(url);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if name == "link" {
                    let mut href = String::new();
                    let mut rel = String::new();

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"href" => href = String::from_utf8_lossy(&attr.value).to_string(),
                            b"rel" => rel = String::from_utf8_lossy(&attr.value).to_string(),
                            _ => {}
                        }
                    }

                    if in_item {
                        if let Some(ref mut item) = current_item {
                            if rel.is_empty() || rel == "alternate" {
                                item.link = href;
                            } else if rel == "enclosure" {
                                item.enclosures.push(href);
                            }
                        }
                    } else if rel.is_empty() || rel == "alternate" {
                        feed.link = Some(href);
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();

                if in_item {
                    if let Some(ref mut item) = current_item {
                        match current_element.as_str() {
                            "title" => item.title = Some(text),
                            "link" => item.link = text,
                            "description" | "content:encoded" => item.description = Some(text),
                            "pubDate" => {
                                item.pub_date = parse_date(&text);
                            }
                            "guid" => item.guid = Some(text),
                            "author" | "dc:creator" => item.author = Some(text),
                            "category" => item.categories.push(text),
                            _ => {}
                        }
                    }
                } else if in_channel {
                    match current_element.as_str() {
                        "title" => feed.title = Some(text),
                        "link" => feed.link = Some(text),
                        "description" => feed.description = Some(text),
                        "lastBuildDate" => {
                            feed.last_build_date = parse_date(&text);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match name.as_str() {
                    "item" => {
                        if let Some(item) = current_item.take() {
                            if !item.link.is_empty() {
                                feed.items.push(item);
                            }
                        }
                        in_item = false;
                    }
                    "channel" => in_channel = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing RSS: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(feed)
}

/// Parse Atom feed
fn parse_atom(content: &str) -> Result<Feed> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut feed = Feed {
        title: None,
        link: None,
        description: None,
        feed_type: FeedType::Atom,
        items: Vec::new(),
        last_build_date: None,
    };

    let mut current_item: Option<FeedItem> = None;
    let mut current_element = String::new();
    let mut in_entry = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                match name.as_str() {
                    "entry" => {
                        in_entry = true;
                        current_item = Some(FeedItem {
                            title: None,
                            link: String::new(),
                            description: None,
                            pub_date: None,
                            guid: None,
                            author: None,
                            categories: Vec::new(),
                            enclosures: Vec::new(),
                        });
                    }
                    "link" => {
                        let mut href = String::new();
                        let mut rel = String::new();

                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                match attr.key.as_ref() {
                                    b"href" => {
                                        href = String::from_utf8_lossy(&attr.value).to_string()
                                    }
                                    b"rel" => {
                                        rel = String::from_utf8_lossy(&attr.value).to_string()
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if in_entry {
                            if let Some(ref mut item) = current_item {
                                if rel.is_empty() || rel == "alternate" {
                                    item.link = href;
                                } else if rel == "enclosure" {
                                    item.enclosures.push(href);
                                }
                            }
                        } else if rel.is_empty() || rel == "alternate" {
                            feed.link = Some(href);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();

                if in_entry {
                    if let Some(ref mut item) = current_item {
                        match current_element.as_str() {
                            "title" => item.title = Some(text),
                            "summary" | "content" => item.description = Some(text),
                            "published" | "updated" => {
                                item.pub_date = parse_iso_date(&text);
                            }
                            "id" => item.guid = Some(text),
                            "author" | "name" => item.author = Some(text),
                            _ => {}
                        }
                    }
                } else {
                    match current_element.as_str() {
                        "title" => feed.title = Some(text),
                        "subtitle" => feed.description = Some(text),
                        "updated" => {
                            feed.last_build_date = parse_iso_date(&text);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if name == "entry" {
                    if let Some(item) = current_item.take() {
                        if !item.link.is_empty() {
                            feed.items.push(item);
                        }
                    }
                    in_entry = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing Atom: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(feed)
}

/// Parse RFC 2822 date
fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc2822(date_str)
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(date_str))
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Parse ISO 8601 date
fn parse_iso_date(date_str: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(date_str)
        .or_else(|_| chrono::DateTime::parse_from_rfc2822(date_str))
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Extract all URLs from a feed
pub fn extract_urls_from_feed(feed: &Feed) -> Vec<String> {
    let mut urls = Vec::new();

    // Add main feed link
    if let Some(link) = &feed.link {
        urls.push(link.clone());
    }

    // Add all item links
    for item in &feed.items {
        if !item.link.is_empty() {
            urls.push(item.link.clone());
        }

        // Add enclosures
        urls.extend(item.enclosures.clone());
    }

    // Deduplicate
    let unique: HashSet<String> = urls.into_iter().collect();
    unique.into_iter().collect()
}

/// Discover feeds from a website
pub async fn discover_feeds(website_url: &str) -> Result<Vec<String>> {
    let mut discovered_feeds = Vec::new();

    // Fetch the main page
    let client = reqwest::Client::builder()
        .user_agent(super::robots::DEFAULT_USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client.get(website_url).send().await?;

    if !response.status().is_success() {
        return Ok(discovered_feeds);
    }

    let html = response.text().await?;

    // Detect feed links in HTML
    let mut feed_links = detect_feed_links(&html);

    // Convert relative URLs to absolute
    if let Ok(base_url) = url::Url::parse(website_url) {
        feed_links = feed_links
            .into_iter()
            .map(|link| {
                if link.starts_with("http://") || link.starts_with("https://") {
                    link
                } else if link.starts_with('/') {
                    format!(
                        "{}://{}{}",
                        base_url.scheme(),
                        base_url.host_str().unwrap_or(""),
                        link
                    )
                } else {
                    format!("{}/{}", website_url.trim_end_matches('/'), link)
                }
            })
            .collect();
    }

    // Also try common feed paths
    let common_paths = [
        "/feed",
        "/rss",
        "/atom",
        "/feed.xml",
        "/rss.xml",
        "/atom.xml",
        "/feeds",
    ];

    for path in &common_paths {
        let feed_url = format!("{}{}", website_url.trim_end_matches('/'), path);

        // Quick check if feed exists
        match client.head(&feed_url).send().await {
            Ok(response) if response.status().is_success() => {
                feed_links.push(feed_url);
            }
            _ => {}
        }
    }

    // Deduplicate
    let unique: HashSet<String> = feed_links.into_iter().collect();
    discovered_feeds.extend(unique);

    info!(
        "Discovered {} feeds from {}",
        discovered_feeds.len(),
        website_url
    );
    Ok(discovered_feeds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_url_detection() {
        assert!(is_feed_url("https://example.com/feed"));
        assert!(is_feed_url("https://example.com/rss.xml"));
        assert!(is_feed_url("https://example.com/atom.xml"));
        assert!(!is_feed_url("https://example.com/page.html"));
    }

    #[test]
    fn test_feed_link_detection() {
        let html = r#"
            <html>
            <head>
                <link rel="alternate" type="application/rss+xml" href="/feed.xml">
                <link rel="alternate" type="application/atom+xml" href="/atom.xml">
            </head>
            <body>
                <a href="/rss">RSS Feed</a>
            </body>
            </html>
        "#;

        let links = detect_feed_links(html);
        assert!(links.contains(&"/feed.xml".to_string()));
        assert!(links.contains(&"/atom.xml".to_string()));
        assert!(links.contains(&"/rss".to_string()));
    }

    #[test]
    fn test_rss_parsing() {
        let rss = r#"<?xml version="1.0"?>
            <rss version="2.0">
                <channel>
                    <title>Test Feed</title>
                    <link>https://example.com</link>
                    <description>Test Description</description>
                    <item>
                        <title>Test Item</title>
                        <link>https://example.com/item1</link>
                        <description>Item Description</description>
                    </item>
                </channel>
            </rss>
        "#;

        let feed = parse_feed(rss).unwrap();
        assert_eq!(feed.feed_type, FeedType::RSS);
        assert_eq!(feed.title, Some("Test Feed".to_string()));
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].link, "https://example.com/item1");
    }
}

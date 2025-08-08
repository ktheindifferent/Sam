//! # Robots.txt Parser Module
//!
//! This module implements robots.txt parsing and compliance checking for the web crawler.
//! It ensures the crawler respects website crawling policies and maintains ethical crawling practices.
//!
//! ## Features
//! - Parse and interpret robots.txt files according to the standard
//! - Cache robots.txt rules for efficiency
//! - Check URL permissions before crawling
//! - Handle crawl delays and rate limiting
//! - Support for user-agent specific rules

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use reqwest::Url;
use tokio::sync::RwLock;
use std::sync::Arc;
use log::{debug, warn};
use once_cell::sync::Lazy;

/// Default user agent for the SAM crawler
pub const DEFAULT_USER_AGENT: &str = "SAM-Crawler/0.0.2 (+https://github.com/OSF/sam)";

/// Represents parsed robots.txt rules for a specific domain
#[derive(Debug, Clone)]
pub struct RobotsRules {
    /// Rules specific to our user agent
    user_agent_rules: Vec<RuleEntry>,
    /// Default rules for all agents
    default_rules: Vec<RuleEntry>,
    /// Crawl delay in seconds (if specified)
    crawl_delay: Option<f64>,
    /// Sitemap URLs found in robots.txt
    sitemaps: Vec<String>,
    /// Timestamp when these rules were fetched
    fetched_at: SystemTime,
}

#[derive(Debug, Clone)]
struct RuleEntry {
    pattern: String,
    is_allow: bool,
}

impl RobotsRules {
    /// Check if a URL is allowed to be crawled according to the rules
    pub fn is_allowed(&self, url: &str) -> bool {
        let path = if let Ok(parsed_url) = Url::parse(url) {
            parsed_url.path().to_string()
        } else {
            return false;
        };

        // Check user-agent specific rules first
        for rule in &self.user_agent_rules {
            if self.matches_pattern(&rule.pattern, &path) {
                return rule.is_allow;
            }
        }

        // Fall back to default rules
        for rule in &self.default_rules {
            if self.matches_pattern(&rule.pattern, &path) {
                return rule.is_allow;
            }
        }

        // Default to allow if no matching rule
        true
    }

    /// Check if a pattern matches a path
    fn matches_pattern(&self, pattern: &str, path: &str) -> bool {
        if pattern == "/" {
            return true;
        }
        
        // Handle wildcard patterns
        if pattern.contains('*') {
            let regex_pattern = pattern
                .replace("*", ".*")
                .replace("?", ".");
            if let Ok(re) = regex::Regex::new(&format!("^{}", regex_pattern)) {
                return re.is_match(path);
            }
        }
        
        // Simple prefix matching
        path.starts_with(pattern)
    }

    /// Get the crawl delay if specified
    pub fn get_crawl_delay(&self) -> Option<Duration> {
        self.crawl_delay.map(|seconds| Duration::from_secs_f64(seconds))
    }

    /// Get sitemap URLs from robots.txt
    pub fn get_sitemaps(&self) -> &[String] {
        &self.sitemaps
    }

    /// Check if the rules are stale (older than 24 hours)
    pub fn is_stale(&self) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.fetched_at) {
            elapsed > Duration::from_secs(86400) // 24 hours
        } else {
            true
        }
    }
}

/// Cache for robots.txt rules
static ROBOTS_CACHE: Lazy<Arc<RwLock<HashMap<String, RobotsRules>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Fetch and parse robots.txt for a given domain
pub async fn fetch_robots_txt(domain: &str) -> Result<RobotsRules, Box<dyn std::error::Error>> {
    let robots_url = format!("{}://{}/robots.txt", 
        if domain.starts_with("https") { "https" } else { "http" },
        domain.replace("http://", "").replace("https://", ""));
    
    debug!("Fetching robots.txt from: {}", robots_url);
    
    let client = reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(false) // Fixed: Enable proper certificate validation
        .build()?;
    
    let response = client.get(&robots_url).send().await?;
    
    if !response.status().is_success() {
        // If robots.txt doesn't exist, allow all crawling
        return Ok(RobotsRules {
            user_agent_rules: vec![],
            default_rules: vec![],
            crawl_delay: None,
            sitemaps: vec![],
            fetched_at: SystemTime::now(),
        });
    }
    
    let content = response.text().await?;
    parse_robots_txt(&content)
}

/// Parse robots.txt content
fn parse_robots_txt(content: &str) -> Result<RobotsRules, Box<dyn std::error::Error>> {
    let mut user_agent_rules = Vec::new();
    let mut default_rules = Vec::new();
    let mut crawl_delay = None;
    let mut sitemaps = Vec::new();
    
    let mut current_user_agent = String::new();
    let mut is_our_agent = false;
    
    for line in content.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        
        let directive = parts[0].trim().to_lowercase();
        let value = parts[1].trim();
        
        match directive.as_str() {
            "user-agent" => {
                current_user_agent = value.to_lowercase();
                is_our_agent = current_user_agent == "*" || 
                              current_user_agent.contains("sam") ||
                              current_user_agent.contains("crawler");
            }
            "disallow" if !value.is_empty() => {
                let rule = RuleEntry {
                    pattern: value.to_string(),
                    is_allow: false,
                };
                
                if is_our_agent && current_user_agent != "*" {
                    user_agent_rules.push(rule);
                } else if current_user_agent == "*" {
                    default_rules.push(rule);
                }
            }
            "allow" if !value.is_empty() => {
                let rule = RuleEntry {
                    pattern: value.to_string(),
                    is_allow: true,
                };
                
                if is_our_agent && current_user_agent != "*" {
                    user_agent_rules.push(rule);
                } else if current_user_agent == "*" {
                    default_rules.push(rule);
                }
            }
            "crawl-delay" if is_our_agent || current_user_agent == "*" => {
                if let Ok(delay) = value.parse::<f64>() {
                    crawl_delay = Some(delay);
                }
            }
            "sitemap" => {
                sitemaps.push(value.to_string());
            }
            _ => {}
        }
    }
    
    Ok(RobotsRules {
        user_agent_rules,
        default_rules,
        crawl_delay,
        sitemaps,
        fetched_at: SystemTime::now(),
    })
}

/// Check if a URL is allowed to be crawled, using cached rules when possible
pub async fn is_url_allowed(url: &str) -> bool {
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return false,
    };
    
    let domain = match parsed_url.host_str() {
        Some(h) => h.to_string(),
        None => return false,
    };
    
    // Check cache first
    {
        let cache = ROBOTS_CACHE.read().await;
        if let Some(rules) = cache.get(&domain) {
            if !rules.is_stale() {
                return rules.is_allowed(url);
            }
        }
    }
    
    // Fetch and cache new rules
    match fetch_robots_txt(&domain).await {
        Ok(rules) => {
            let is_allowed = rules.is_allowed(url);
            
            // Update cache
            let mut cache = ROBOTS_CACHE.write().await;
            cache.insert(domain, rules);
            
            is_allowed
        }
        Err(e) => {
            warn!("Failed to fetch robots.txt for {}: {}. Allowing crawl by default.", domain, e);
            true // Default to allow if we can't fetch robots.txt
        }
    }
}

/// Get crawl delay for a domain from cached rules
pub async fn get_crawl_delay(domain: &str) -> Option<Duration> {
    let cache = ROBOTS_CACHE.read().await;
    cache.get(domain).and_then(|rules| rules.get_crawl_delay())
}

/// Get sitemap URLs for a domain from cached rules
pub async fn get_sitemaps(domain: &str) -> Vec<String> {
    let cache = ROBOTS_CACHE.read().await;
    cache.get(domain)
        .map(|rules| rules.get_sitemaps().to_vec())
        .unwrap_or_default()
}

/// Clear the robots.txt cache
pub async fn clear_cache() {
    let mut cache = ROBOTS_CACHE.write().await;
    cache.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let rules = RobotsRules {
            user_agent_rules: vec![],
            default_rules: vec![
                RuleEntry { pattern: "/admin".to_string(), is_allow: false },
                RuleEntry { pattern: "/api/*".to_string(), is_allow: false },
                RuleEntry { pattern: "/public".to_string(), is_allow: true },
            ],
            crawl_delay: None,
            sitemaps: vec![],
            fetched_at: SystemTime::now(),
        };

        assert!(!rules.is_allowed("http://example.com/admin"));
        assert!(!rules.is_allowed("http://example.com/admin/page"));
        assert!(!rules.is_allowed("http://example.com/api/v1"));
        assert!(rules.is_allowed("http://example.com/public"));
        assert!(rules.is_allowed("http://example.com/other"));
    }

    #[test]
    fn test_robots_txt_parsing() {
        let content = r#"
User-agent: *
Disallow: /admin
Allow: /admin/public
Crawl-delay: 1.5

User-agent: sam-crawler
Disallow: /private
Crawl-delay: 2.0

Sitemap: http://example.com/sitemap.xml
        "#;

        let rules = parse_robots_txt(content).unwrap();
        assert_eq!(rules.default_rules.len(), 2);
        assert_eq!(rules.user_agent_rules.len(), 1);
        assert_eq!(rules.crawl_delay, Some(2.0));
        assert_eq!(rules.sitemaps.len(), 1);
    }
}
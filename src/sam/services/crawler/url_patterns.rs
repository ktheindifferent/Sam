//! URL pattern detection for avoiding infinite crawl loops
//! 
//! This module detects and handles potentially infinite URL patterns
//! such as calendars, pagination, and dynamically generated URLs.

use anyhow::{Result, Context};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use url::Url;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

/// URL pattern analyzer for detecting infinite patterns
#[derive(Debug, Clone)]
pub struct UrlPatternAnalyzer {
    /// Patterns we've seen and their frequency
    pattern_frequency: HashMap<String, usize>,
    
    /// Known infinite pattern detectors
    infinite_patterns: Vec<InfinitePatternDetector>,
    
    /// Maximum variations of similar URLs before considering it infinite
    max_variations: usize,
    
    /// Similarity threshold for considering URLs as variations
    similarity_threshold: f64,
}

/// Detector for specific types of infinite patterns
#[derive(Debug, Clone)]
struct InfinitePatternDetector {
    name: String,
    pattern: Regex,
    max_allowed: usize,
}

impl Default for UrlPatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlPatternAnalyzer {
    /// Create a new URL pattern analyzer
    pub fn new() -> Self {
        let infinite_patterns = vec![
            // Calendar patterns
            InfinitePatternDetector {
                name: "calendar_date".to_string(),
                pattern: Regex::new(r"/(\d{4})[/-](\d{1,2})[/-](\d{1,2})").unwrap(),
                max_allowed: 30, // Allow 30 different dates
            },
            InfinitePatternDetector {
                name: "calendar_month".to_string(),
                pattern: Regex::new(r"/(\d{4})[/-](\d{1,2})/?$").unwrap(),
                max_allowed: 12, // Allow 12 months
            },
            InfinitePatternDetector {
                name: "calendar_year".to_string(),
                pattern: Regex::new(r"/(\d{4})/?$").unwrap(),
                max_allowed: 5, // Allow 5 years
            },
            
            // Pagination patterns
            InfinitePatternDetector {
                name: "page_number".to_string(),
                pattern: Regex::new(r"[?&]page=(\d+)").unwrap(),
                max_allowed: 100, // Allow 100 pages
            },
            InfinitePatternDetector {
                name: "offset".to_string(),
                pattern: Regex::new(r"[?&]offset=(\d+)").unwrap(),
                max_allowed: 50,
            },
            InfinitePatternDetector {
                name: "page_path".to_string(),
                pattern: Regex::new(r"/page/(\d+)").unwrap(),
                max_allowed: 100,
            },
            
            // Session/tracking parameters
            InfinitePatternDetector {
                name: "session_id".to_string(),
                pattern: Regex::new(r"[?&](session|sid|sess_id|PHPSESSID)=([a-zA-Z0-9]+)").unwrap(),
                max_allowed: 1, // Only allow 1 session ID
            },
            InfinitePatternDetector {
                name: "tracking_id".to_string(),
                pattern: Regex::new(r"[?&](utm_[a-z]+|fbclid|gclid)=([^&]+)").unwrap(),
                max_allowed: 5,
            },
            
            // API versioning
            InfinitePatternDetector {
                name: "api_version".to_string(),
                pattern: Regex::new(r"/v(\d+)/").unwrap(),
                max_allowed: 5,
            },
            
            // User profiles
            InfinitePatternDetector {
                name: "user_id".to_string(),
                pattern: Regex::new(r"/(user|profile|member)/(\d+)").unwrap(),
                max_allowed: 100,
            },
            
            // Search results
            InfinitePatternDetector {
                name: "search_query".to_string(),
                pattern: Regex::new(r"[?&](q|query|search|s)=([^&]+)").unwrap(),
                max_allowed: 50,
            },
            
            // Sort/filter parameters
            InfinitePatternDetector {
                name: "sort_filter".to_string(),
                pattern: Regex::new(r"[?&](sort|order|filter)=([^&]+)").unwrap(),
                max_allowed: 20,
            },
        ];
        
        Self {
            pattern_frequency: HashMap::new(),
            infinite_patterns,
            max_variations: 100,
            similarity_threshold: 0.8,
        }
    }
    
    /// Check if a URL might lead to infinite crawling
    pub fn is_potentially_infinite(&mut self, url: &str) -> bool {
        // Check against known infinite patterns
        for detector in &self.infinite_patterns {
            if let Some(captures) = detector.pattern.captures(url) {
                let pattern_key = format!("{}:{}", detector.name, captures.get(0).unwrap().as_str());
                
                let count = self.pattern_frequency.entry(pattern_key.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                
                if *count > detector.max_allowed {
                    warn!("URL matches infinite pattern '{}': {} (limit: {})", 
                          detector.name, url, detector.max_allowed);
                    return true;
                }
            }
        }
        
        // Check for URL similarity (too many variations of the same base URL)
        if self.check_url_variations(url) {
            warn!("Too many variations of similar URL: {}", url);
            return true;
        }
        
        false
    }
    
    /// Check if we have too many variations of similar URLs
    fn check_url_variations(&mut self, url: &str) -> bool {
        let normalized = match self.normalize_url(url) {
            Ok(n) => n,
            Err(_) => return false,
        };
        
        let count = self.pattern_frequency.entry(normalized.clone())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        
        *count > self.max_variations
    }
    
    /// Normalize URL by removing variable parts
    fn normalize_url(&self, url: &str) -> Result<String> {
        let parsed = Url::parse(url)?;
        
        // Get base without query parameters
        let base = format!("{}://{}{}", 
                          parsed.scheme(), 
                          parsed.host_str().unwrap_or(""),
                          parsed.path());
        
        // Replace numbers with placeholders
        let normalized = Regex::new(r"\d+")
            .unwrap()
            .replace_all(&base, "#")
            .to_string();
        
        Ok(normalized)
    }
    
    /// Get statistics about detected patterns
    pub fn get_stats(&self) -> PatternStats {
        let total_patterns = self.pattern_frequency.len();
        let infinite_detected: Vec<String> = self.pattern_frequency
            .iter()
            .filter(|(k, v)| {
                // Check if any pattern exceeded its limit
                for detector in &self.infinite_patterns {
                    if k.starts_with(&format!("{}:", detector.name)) && **v > detector.max_allowed {
                        return true;
                    }
                }
                false
            })
            .map(|(k, _)| k.clone())
            .collect();
        
        PatternStats {
            total_patterns,
            infinite_patterns_detected: infinite_detected.len(),
            top_patterns: self.get_top_patterns(10),
        }
    }
    
    /// Get top N most frequent patterns
    fn get_top_patterns(&self, n: usize) -> Vec<(String, usize)> {
        let mut patterns: Vec<_> = self.pattern_frequency.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns.truncate(n);
        patterns
    }
    
    /// Reset pattern tracking
    pub fn reset(&mut self) {
        self.pattern_frequency.clear();
    }
}

/// Statistics about detected patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStats {
    pub total_patterns: usize,
    pub infinite_patterns_detected: usize,
    pub top_patterns: Vec<(String, usize)>,
}

/// URL deduplication with normalization
#[derive(Debug, Clone)]
pub struct UrlDeduplicator {
    seen_urls: HashSet<String>,
    seen_normalized: HashSet<String>,
}

impl Default for UrlDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlDeduplicator {
    pub fn new() -> Self {
        Self {
            seen_urls: HashSet::new(),
            seen_normalized: HashSet::new(),
        }
    }
    
    /// Check if URL is duplicate (considering normalization)
    pub fn is_duplicate(&mut self, url: &str) -> bool {
        // Check exact match first
        if self.seen_urls.contains(url) {
            return true;
        }
        
        // Normalize and check
        if let Ok(normalized) = self.normalize_url(url) {
            if self.seen_normalized.contains(&normalized) {
                debug!("URL is duplicate after normalization: {}", url);
                return true;
            }
            
            // Add to seen sets
            self.seen_urls.insert(url.to_string());
            self.seen_normalized.insert(normalized);
            false
        } else {
            // If normalization fails, just use exact matching
            self.seen_urls.insert(url.to_string());
            false
        }
    }
    
    /// Normalize URL for deduplication
    fn normalize_url(&self, url: &str) -> Result<String> {
        let mut parsed = Url::parse(url)?;
        
        // Remove fragment
        parsed.set_fragment(None);
        
        // Sort query parameters
        if let Some(query) = parsed.query() {
            let mut params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            
            // Remove tracking parameters
            params.retain(|(k, _)| {
                !k.starts_with("utm_") && 
                !k.starts_with("fb") && 
                !k.starts_with("gclid") &&
                k != "ref" &&
                k != "source"
            });
            
            // Sort parameters
            params.sort_by(|a, b| a.0.cmp(&b.0));
            
            // Rebuild query string
            if params.is_empty() {
                parsed.set_query(None);
            } else {
                let query_string = params.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&");
                parsed.set_query(Some(&query_string));
            }
        }
        
        // Convert to lowercase
        Ok(parsed.to_string().to_lowercase())
    }
    
    /// Get number of unique URLs seen
    pub fn count(&self) -> usize {
        self.seen_urls.len()
    }
    
    /// Clear all seen URLs
    pub fn clear(&mut self) {
        self.seen_urls.clear();
        self.seen_normalized.clear();
    }
}

/// Check if URL is a common infinite pattern
pub fn is_infinite_pattern(url: &str) -> bool {
    // Quick checks for common infinite patterns
    let patterns = [
        // Calendars
        r"/calendar/",
        r"/events/\d{4}",
        r"/archive/\d{4}",
        
        // Pagination
        r"[?&]page=\d{3,}",  // Page > 100
        r"[?&]offset=\d{4,}", // Large offset
        r"/page/\d{3,}",
        
        // Print versions
        r"/print/",
        r"[?&]print=",
        
        // Downloads
        r"/download/",
        r"[?&]download=",
        
        // Different formats
        r"\.pdf$",
        r"\.doc[x]?$",
        r"\.zip$",
    ];
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(url) {
                return true;
            }
        }
    }
    
    false
}

/// Clean URL by removing unnecessary parameters
pub fn clean_url(url: &str) -> Result<String> {
    let mut parsed = Url::parse(url)?;
    
    // Remove fragment
    parsed.set_fragment(None);
    
    // Remove tracking and session parameters
    if let Some(query) = parsed.query() {
        let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
            .filter(|(k, _)| {
                let key = k.to_lowercase();
                !key.starts_with("utm_") &&
                !key.starts_with("fb") &&
                !key.contains("session") &&
                !key.contains("sid") &&
                key != "gclid" &&
                key != "ref" &&
                key != "source" &&
                key != "phpsessid"
            })
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        
        if params.is_empty() {
            parsed.set_query(None);
        } else {
            let query_string = params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            parsed.set_query(Some(&query_string));
        }
    }
    
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_infinite_pattern_detection() {
        let mut analyzer = UrlPatternAnalyzer::new();
        
        // Test calendar pattern
        for i in 1..150 {
            let url = format!("https://example.com/2024/01/{:02}", i);
            if i > 30 {
                assert!(analyzer.is_potentially_infinite(&url));
                break;
            }
        }
        
        // Test pagination pattern
        analyzer.reset();
        for i in 1..150 {
            let url = format!("https://example.com/posts?page={}", i);
            if i > 100 {
                assert!(analyzer.is_potentially_infinite(&url));
                break;
            }
        }
    }
    
    #[test]
    fn test_url_deduplication() {
        let mut dedup = UrlDeduplicator::new();
        
        // Same URL with different tracking parameters
        assert!(!dedup.is_duplicate("https://example.com/page?utm_source=test"));
        assert!(dedup.is_duplicate("https://example.com/page?utm_source=other"));
        
        // Same URL with different order of parameters
        dedup.clear();
        assert!(!dedup.is_duplicate("https://example.com/page?a=1&b=2"));
        assert!(dedup.is_duplicate("https://example.com/page?b=2&a=1"));
    }
    
    #[test]
    fn test_url_cleaning() {
        let url = "https://example.com/page?utm_source=test&id=123&PHPSESSID=abc#section";
        let cleaned = clean_url(url).unwrap();
        
        assert!(cleaned.contains("id=123"));
        assert!(!cleaned.contains("utm_source"));
        assert!(!cleaned.contains("PHPSESSID"));
        assert!(!cleaned.contains("#section"));
    }
}
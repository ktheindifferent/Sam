//! URL processing and validation utilities
//!
//! This module handles URL validation, filtering, and processing tasks
//! including search URL detection, MIME type detection, and link extraction.

use std::collections::HashSet;
use reqwest::Url;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector, ElementRef};

/// Static data for common resources
static COMMON_TLDS: Lazy<Vec<String>> = Lazy::new(|| {
    let bytes = include_bytes!("common_tlds.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
});

static COMMON_TOKENS: Lazy<Vec<String>> = Lazy::new(|| {
    let bytes = include_bytes!("common_tokens.txt").to_vec();
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
});

/// Search URL detection regex
static SEARCH_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(/search[/?]|/query[/?]|/find[/?]|/lookup[/?]|/results[/?]|/explore[/?]|/filter[/?]|/discover[/?]|/browse[/?]|/list[/?]|/websearch\?|/search_history\?|\?q=|&q=|search=|query=|lookup=|results=|explore=|filter=|discover=|browse=|\bu=|url=|\bid=|redirect|backurl=|text=|searchterm|search_term|return_to|https?%3A%2F%2F)")
        .unwrap_or_else(|e| {
            log::error!("Failed to compile search pattern regex: {}", e);
            sentry::capture_message(&format!("Search regex compilation failed: {}", e), sentry::Level::Error);
            Regex::new(r"search").expect("Fallback regex should compile")
        })
});

/// Date pattern regexes for token filtering
static DATE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}$").unwrap_or_else(|e| {
            log::error!("Failed to compile date regex pattern: {}", e);
            sentry::capture_message(&format!("Date regex compilation failed: {}", e), sentry::Level::Warning);
            Regex::new(r".*").expect("Fallback regex should compile")
        }),
        Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$").expect("Failed to compile date regex pattern"),
        Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$").expect("Failed to compile date regex pattern"),
        Regex::new(r"^\d{8}$").expect("Failed to compile date regex pattern"),
        Regex::new(r"^\d{4}\.\d{1,2}\.\d{1,2}$").expect("Failed to compile date regex pattern"),
        Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$").expect("Failed to compile date regex pattern"),
        Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?(Z|([+-]\d{2}:\d{2}))?)?$")
            .expect("Failed to compile ISO date regex pattern"),
    ]
});

/// URL processor handles all URL-related operations
pub struct UrlProcessor;

impl UrlProcessor {
    /// Check if a string is a valid absolute URL with a scheme and host
    pub fn is_valid_url(s: &str) -> bool {
        match Url::parse(s) {
            Ok(url) => url.has_host() && !url.scheme().is_empty(),
            Err(_) => false,
        }
    }

    /// Check if URL appears to be a search endpoint
    pub fn is_search_url(url: &str) -> bool {
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

    /// Get MIME type from URL based on file extension
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
                    
                    // Look up MIME type by extension
                    for (map_ext, mime) in crate::tools::MIME_MAP.iter() {
                        if ext.eq_ignore_ascii_case(map_ext.trim_start_matches('.')) {
                            return mime;
                        }
                    }
                }
            }
        }
        "text/unknown"
    }

    /// Extract MIME type from HTTP headers
    pub fn extract_mime_from_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
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

    /// Resolve a potentially relative URL to an absolute URL
    pub fn resolve_url(base_url: &str, url: &str) -> Result<String, url::ParseError> {
        Url::parse(url)
            .or_else(|_| Url::parse(base_url).and_then(|base| base.join(url)))
            .map(|u| u.to_string())
    }

    /// Extract links from an HTML document
    pub fn extract_links_from_document(document: &Html, base_url: &str) -> Vec<String> {
        let mut links = Vec::new();
        
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
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(attr_value) = element.value().attr(attr_name) {
                        if let Ok(abs_url) = Self::resolve_url(base_url, attr_value) {
                            links.push(abs_url);
                        }
                    }
                }
            }
        }

        // Filter and deduplicate links
        links.retain(|link| {
            let link_lc = link.to_ascii_lowercase();
            (link_lc.starts_with("http://") || link_lc.starts_with("https://"))
                && !link_lc.starts_with("data:")
        });

        links.sort();
        links.dedup();
        links
    }

    /// Extract text tokens from HTML content
    pub fn extract_text_tokens(document: &Html) -> Vec<String> {
        let mut tokens = Vec::new();

        let body_selector = match Selector::parse("body") {
            Ok(sel) => sel,
            Err(e) => {
                log::warn!("Failed to parse selector 'body': {}", e);
                return tokens;
            }
        };

        let skip_tags = ["script", "style", "noscript", "svg", "canvas", "iframe", "template"];
        let skip_selectors: Vec<Selector> = skip_tags
            .iter()
            .filter_map(|tag| Selector::parse(tag).ok())
            .collect();

        for body in document.select(&body_selector) {
            Self::extract_text_recursive(&body, &skip_selectors, &mut tokens);
        }

        tokens.sort();
        tokens.dedup();
        tokens
    }

    /// Recursively extract text from HTML elements
    fn extract_text_recursive(
        element: &ElementRef,
        skip_selectors: &[Selector],
        tokens: &mut Vec<String>,
    ) {
        // Skip certain elements
        for selector in skip_selectors {
            if selector.matches(element) {
                return;
            }
        }

        for child in element.children() {
            match child.value() {
                scraper::node::Node::Text(text) => {
                    for word in text.text.split_whitespace() {
                        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
                        if !clean_word.is_empty() {
                            tokens.push(clean_word.to_lowercase());
                        }
                    }
                }
                scraper::node::Node::Element(_) => {
                    if let Some(child_elem) = ElementRef::wrap(child) {
                        Self::extract_text_recursive(&child_elem, skip_selectors, tokens);
                    }
                }
                _ => {}
            }
        }
    }

    /// Filter tokens based on various criteria
    pub fn filter_tokens(tokens: &mut Vec<String>, url: &str) {
        // Filter out common tokens unless they match date patterns
        tokens.retain(|token| Self::is_date_token(token) || !COMMON_TOKENS.contains(token));

        // Filter by length
        tokens.retain(|token| token.len() > 2 && token.len() < 50);

        // Remove tokens that are part of the URL
        Self::remove_url_tokens(tokens, url);

        // Remove tokens that are part of the domain
        Self::remove_domain_tokens(tokens, url);
    }

    /// Check if a token matches any date pattern
    fn is_date_token(token: &str) -> bool {
        DATE_PATTERNS.iter().any(|re| re.is_match(token))
    }

    /// Remove tokens that are part of the URL path
    fn remove_url_tokens(tokens: &mut Vec<String>, url: &str) {
        let url_tokens: HashSet<_> = url.split('/').map(|s| s.to_lowercase()).collect();
        tokens.retain(|token| !url_tokens.contains(&token.to_lowercase()));
    }

    /// Remove tokens that are part of the domain name
    fn remove_domain_tokens(tokens: &mut Vec<String>, url: &str) {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(domain) = parsed_url.domain() {
                let domain_tokens: HashSet<_> = domain.split('.').map(|s| s.to_lowercase()).collect();
                tokens.retain(|token| !domain_tokens.contains(&token.to_lowercase()));
            }
        }
    }

    /// Check if URL is supported based on content type
    pub fn is_supported_content_type(mime_type: &str) -> bool {
        mime_type.starts_with("text/") ||
        mime_type.starts_with("image/") ||
        mime_type.starts_with("application/json") ||
        mime_type.starts_with("application/xml") ||
        mime_type.starts_with("application/pdf") ||
        mime_type.starts_with("application/javascript") ||
        mime_type.starts_with("text/javascript") ||
        mime_type.starts_with("text/css") ||
        mime_type.starts_with("application/x-javascript") ||
        mime_type == "application/octet-stream"
    }

    /// Check if content type is blocked
    pub fn is_blocked_content_type(mime_type: &str) -> bool {
        let blocked_types = ["video/", "audio/", "application/zip", "application/x-rar", "application/x-tar"];
        blocked_types.iter().any(|bt| mime_type.starts_with(bt))
    }

    /// Check if URL represents a document that may contain links
    pub fn is_document_url(url: &str, mime_tokens: &[String]) -> bool {
        let doc_exts = [
            ".html", ".htm", ".xhtml", ".shtml", ".php", ".asp", ".aspx", ".jsp", ".jspx",
            ".cgi", ".pl", ".cfm", ".rb", ".py", ".xml", ".json", ".md", ".txt", "/",
        ];

        mime_tokens
            .iter()
            .any(|m| m.starts_with("text/") || m.starts_with("application/"))
            || doc_exts.iter().any(|ext| url.ends_with(ext))
    }

    /// Get domain from URL
    pub fn get_domain(url: &str) -> Option<String> {
        Url::parse(url).ok()?.domain().map(|d| d.to_string())
    }

    /// Normalize URL for comparison
    pub fn normalize_url(url: &str) -> Option<String> {
        let mut parsed = Url::parse(url).ok()?;
        
        // Remove fragment
        parsed.set_fragment(None);
        
        // Sort query parameters
        let mut query_pairs: Vec<_> = parsed.query_pairs().collect();
        query_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        
        // Rebuild query string
        let query_string = if query_pairs.is_empty() {
            None
        } else {
            Some(query_pairs
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&"))
        };
        
        parsed.set_query(query_string.as_deref());
        
        Some(parsed.to_string())
    }
}

/// URL validation result
#[derive(Debug, Clone, PartialEq)]
pub enum UrlValidationResult {
    Valid,
    Invalid(String),
    Blocked(String),
    SearchEndpoint,
    UnsupportedContentType(String),
}

/// Comprehensive URL validator
pub struct UrlValidator {
    allowed_schemes: HashSet<String>,
    blocked_domains: HashSet<String>,
    max_url_length: usize,
}

impl Default for UrlValidator {
    fn default() -> Self {
        Self {
            allowed_schemes: ["http", "https"].iter().map(|s| s.to_string()).collect(),
            blocked_domains: HashSet::new(),
            max_url_length: 2048,
        }
    }
}

impl UrlValidator {
    /// Create a new URL validator with custom settings
    pub fn new(allowed_schemes: Vec<String>, blocked_domains: Vec<String>, max_url_length: usize) -> Self {
        Self {
            allowed_schemes: allowed_schemes.into_iter().collect(),
            blocked_domains: blocked_domains.into_iter().collect(),
            max_url_length,
        }
    }

    /// Validate a URL comprehensively
    pub fn validate(&self, url: &str) -> UrlValidationResult {
        // Check length
        if url.len() > self.max_url_length {
            return UrlValidationResult::Invalid(format!("URL too long: {} characters", url.len()));
        }

        // Check if it's a valid URL
        if !UrlProcessor::is_valid_url(url) {
            return UrlValidationResult::Invalid("Invalid URL format".to_string());
        }

        // Parse URL for further checks
        let parsed_url = match Url::parse(url) {
            Ok(url) => url,
            Err(e) => return UrlValidationResult::Invalid(format!("URL parse error: {}", e)),
        };

        // Check scheme
        if !self.allowed_schemes.contains(parsed_url.scheme()) {
            return UrlValidationResult::Invalid(format!("Unsupported scheme: {}", parsed_url.scheme()));
        }

        // Check for blocked domains
        if let Some(domain) = parsed_url.domain() {
            if self.blocked_domains.contains(domain) {
                return UrlValidationResult::Blocked(format!("Blocked domain: {}", domain));
            }
        }

        // Check for search endpoints
        if UrlProcessor::is_search_url(url) {
            return UrlValidationResult::SearchEndpoint;
        }

        UrlValidationResult::Valid
    }

    /// Add a domain to the blocked list
    pub fn block_domain(&mut self, domain: &str) {
        self.blocked_domains.insert(domain.to_string());
    }

    /// Remove a domain from the blocked list
    pub fn unblock_domain(&mut self, domain: &str) {
        self.blocked_domains.remove(domain);
    }

    /// Check if a domain is blocked
    pub fn is_domain_blocked(&self, domain: &str) -> bool {
        self.blocked_domains.contains(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        assert!(UrlProcessor::is_valid_url("https://example.com"));
        assert!(UrlProcessor::is_valid_url("http://subdomain.example.com/path"));
        assert!(!UrlProcessor::is_valid_url("not-a-url"));
        assert!(!UrlProcessor::is_valid_url("mailto:test@example.com"));
    }

    #[test]
    fn test_search_url_detection() {
        assert!(UrlProcessor::is_search_url("https://example.com/search?q=test"));
        assert!(UrlProcessor::is_search_url("https://example.com/query"));
        assert!(UrlProcessor::is_search_url("https://example.com/?search=test"));
        assert!(!UrlProcessor::is_search_url("https://example.com/about"));
        assert!(!UrlProcessor::is_search_url("https://example.com/contact"));
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(UrlProcessor::mime_type_from_url("https://example.com/page.html"), "text/html");
        assert_eq!(UrlProcessor::mime_type_from_url("https://example.com/image.jpg"), "image/jpeg");
        assert_eq!(UrlProcessor::mime_type_from_url("https://example.com/document.pdf"), "application/pdf");
        assert_eq!(UrlProcessor::mime_type_from_url("https://example.com/"), "text/unknown");
    }

    #[test]
    fn test_url_resolver() {
        let base = "https://example.com/path/";
        assert_eq!(
            UrlProcessor::resolve_url(base, "relative.html").unwrap(),
            "https://example.com/path/relative.html"
        );
        assert_eq!(
            UrlProcessor::resolve_url(base, "/absolute.html").unwrap(),
            "https://example.com/absolute.html"
        );
        assert_eq!(
            UrlProcessor::resolve_url(base, "https://other.com/page.html").unwrap(),
            "https://other.com/page.html"
        );
    }

    #[test]
    fn test_url_validator() {
        let validator = UrlValidator::default();
        
        assert_eq!(validator.validate("https://example.com"), UrlValidationResult::Valid);
        assert!(matches!(
            validator.validate("ftp://example.com"),
            UrlValidationResult::Invalid(_)
        ));
        assert_eq!(
            validator.validate("https://example.com/search?q=test"),
            UrlValidationResult::SearchEndpoint
        );
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(UrlProcessor::get_domain("https://example.com/path"), Some("example.com".to_string()));
        assert_eq!(UrlProcessor::get_domain("http://sub.example.com"), Some("sub.example.com".to_string()));
        assert_eq!(UrlProcessor::get_domain("not-a-url"), None);
    }

    #[test]
    fn test_url_normalization() {
        assert_eq!(
            UrlProcessor::normalize_url("https://example.com/path?b=2&a=1#fragment"),
            Some("https://example.com/path?a=1&b=2".to_string())
        );
        assert_eq!(
            UrlProcessor::normalize_url("https://example.com/path#fragment"),
            Some("https://example.com/path".to_string())
        );
    }
}

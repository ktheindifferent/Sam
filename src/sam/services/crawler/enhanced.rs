use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use url::Url;
use scraper::{Html, Selector};
use reqwest::Client;

/// Enhanced crawl result with additional metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedCrawlResult {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub summary: Option<String>,
    pub links: Vec<LinkInfo>,
    pub open_ports: Vec<u16>,
    pub server_info: Option<ServerInfo>,
    pub meta_tags: HashMap<String, String>,
    pub social_media: SocialMediaInfo,
    pub security_headers: SecurityHeadersInfo,
}

/// Information about a link
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkInfo {
    pub url: String,
    pub text: String,
    pub rel: Option<String>,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LinkType {
    Internal,
    External,
    Email,
    Phone,
    Download,
    Social,
    Unknown,
}

/// Server information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerInfo {
    pub ip_address: Option<String>,
    pub server_header: Option<String>,
    pub powered_by: Option<String>,
    pub response_time_ms: u64,
}

/// Social media information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SocialMediaInfo {
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub twitter_card: Option<String>,
    pub twitter_site: Option<String>,
}

/// Security headers information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityHeadersInfo {
    pub has_csp: bool,
    pub has_hsts: bool,
    pub has_x_frame_options: bool,
    pub has_x_content_type_options: bool,
    pub has_x_xss_protection: bool,
    pub security_score: u8, // 0-100
}

use std::collections::HashMap;

/// Common ports to scan
const COMMON_PORTS: &[u16] = &[
    21,    // FTP
    22,    // SSH
    23,    // Telnet
    25,    // SMTP
    53,    // DNS
    80,    // HTTP
    110,   // POP3
    143,   // IMAP
    443,   // HTTPS
    445,   // SMB
    587,   // SMTP (submission)
    993,   // IMAPS
    995,   // POP3S
    1433,  // MSSQL
    3306,  // MySQL
    3389,  // RDP
    5432,  // PostgreSQL
    5900,  // VNC
    6379,  // Redis
    8080,  // HTTP Alternate
    8443,  // HTTPS Alternate
    27017, // MongoDB
];

/// Enhanced crawler with advanced features
pub struct EnhancedCrawler {
    client: Client,
    scan_ports: bool,
    generate_summaries: bool,
    max_summary_length: usize,
}

impl EnhancedCrawler {
    /// Create a new enhanced crawler
    pub fn new(scan_ports: bool, generate_summaries: bool) -> Self {
        let client = Client::builder()
            .user_agent(super::super::crawler::DEFAULT_USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        
        EnhancedCrawler {
            client,
            scan_ports,
            generate_summaries,
            max_summary_length: 500,
        }
    }
    
    /// Crawl a URL with enhanced features
    pub async fn crawl_enhanced(&self, url: &str) -> Result<EnhancedCrawlResult, String> {
        // Validate URL
        let parsed_url = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        
        // Fetch the page
        let start_time = std::time::Instant::now();
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        let response_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Extract headers
        let server_header = response.headers()
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        let powered_by = response.headers()
            .get("x-powered-by")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        // Check security headers
        let security_headers = SecurityHeadersInfo {
            has_csp: response.headers().contains_key("content-security-policy"),
            has_hsts: response.headers().contains_key("strict-transport-security"),
            has_x_frame_options: response.headers().contains_key("x-frame-options"),
            has_x_content_type_options: response.headers().contains_key("x-content-type-options"),
            has_x_xss_protection: response.headers().contains_key("x-xss-protection"),
            security_score: calculate_security_score(&response.headers()),
        };
        
        // Get HTML content
        let html = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
        let document = Html::parse_document(&html);
        
        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document.select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string());
        
        // Extract meta tags
        let mut meta_tags = HashMap::new();
        let mut description = None;
        let mut keywords = Vec::new();
        
        let meta_selector = Selector::parse("meta").unwrap();
        for element in document.select(&meta_selector) {
            if let Some(name) = element.value().attr("name") {
                if let Some(content) = element.value().attr("content") {
                    meta_tags.insert(name.to_string(), content.to_string());
                    
                    if name == "description" {
                        description = Some(content.to_string());
                    } else if name == "keywords" {
                        keywords = content.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
            
            // Open Graph tags
            if let Some(property) = element.value().attr("property") {
                if let Some(content) = element.value().attr("content") {
                    meta_tags.insert(property.to_string(), content.to_string());
                }
            }
        }
        
        // Extract social media info
        let social_media = SocialMediaInfo {
            og_title: meta_tags.get("og:title").cloned(),
            og_description: meta_tags.get("og:description").cloned(),
            og_image: meta_tags.get("og:image").cloned(),
            twitter_card: meta_tags.get("twitter:card").cloned(),
            twitter_site: meta_tags.get("twitter:site").cloned(),
        };
        
        // Extract links
        let links = self.extract_links(&document, &parsed_url);
        
        // Generate summary if requested
        let summary = if self.generate_summaries {
            Some(self.generate_summary(&document))
        } else {
            None
        };
        
        // Scan ports if requested
        let open_ports = if self.scan_ports {
            if let Some(host) = parsed_url.host_str() {
                self.scan_ports(host).await
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        // Get IP address
        let ip_address = if let Some(host) = parsed_url.host_str() {
            resolve_ip(host).await
        } else {
            None
        };
        
        Ok(EnhancedCrawlResult {
            url: url.to_string(),
            title,
            description,
            keywords,
            summary,
            links,
            open_ports,
            server_info: Some(ServerInfo {
                ip_address,
                server_header,
                powered_by,
                response_time_ms,
            }),
            meta_tags,
            social_media,
            security_headers,
        })
    }
    
    /// Extract and categorize links
    fn extract_links(&self, document: &Html, base_url: &Url) -> Vec<LinkInfo> {
        let mut links = Vec::new();
        let link_selector = Selector::parse("a").unwrap();
        
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let text = element.text().collect::<String>().trim().to_string();
                let rel = element.value().attr("rel").map(|s| s.to_string());
                
                // Determine link type
                let link_type = if href.starts_with("mailto:") {
                    LinkType::Email
                } else if href.starts_with("tel:") {
                    LinkType::Phone
                } else if is_download_link(href) {
                    LinkType::Download
                } else if is_social_media_link(href) {
                    LinkType::Social
                } else if let Ok(link_url) = base_url.join(href) {
                    if link_url.host() == base_url.host() {
                        LinkType::Internal
                    } else {
                        LinkType::External
                    }
                } else {
                    LinkType::Unknown
                };
                
                links.push(LinkInfo {
                    url: href.to_string(),
                    text,
                    rel,
                    link_type,
                });
            }
        }
        
        links
    }
    
    /// Generate a summary of the page content
    fn generate_summary(&self, document: &Html) -> String {
        let mut text_content = String::new();
        
        // Extract text from paragraphs
        let p_selector = Selector::parse("p").unwrap();
        for element in document.select(&p_selector) {
            let text = element.text().collect::<String>();
            let cleaned = text.trim().replace('\n', " ").replace("  ", " ");
            if !cleaned.is_empty() {
                text_content.push_str(&cleaned);
                text_content.push(' ');
            }
            
            if text_content.len() > self.max_summary_length * 2 {
                break;
            }
        }
        
        // Truncate to max length
        if text_content.len() > self.max_summary_length {
            let mut summary = text_content.chars()
                .take(self.max_summary_length)
                .collect::<String>();
            
            // Try to end at a sentence
            if let Some(pos) = summary.rfind(". ") {
                summary.truncate(pos + 1);
            } else if let Some(pos) = summary.rfind(' ') {
                summary.truncate(pos);
                summary.push_str("...");
            }
            
            summary
        } else {
            text_content
        }
    }
    
    /// Scan common ports on a host
    async fn scan_ports(&self, host: &str) -> Vec<u16> {
        let mut open_ports = Vec::new();
        
        for &port in COMMON_PORTS {
            if is_port_open(host, port).await {
                open_ports.push(port);
            }
        }
        
        open_ports
    }
}

/// Check if a port is open
async fn is_port_open(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    
    match timeout(Duration::from_millis(500), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

/// Resolve hostname to IP address
async fn resolve_ip(host: &str) -> Option<String> {
    let addr = format!("{}:80", host);
    addr.to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .map(|socket_addr| socket_addr.ip().to_string())
}

/// Check if a link is a download link
fn is_download_link(href: &str) -> bool {
    let download_extensions = [
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
        ".zip", ".rar", ".tar", ".gz", ".7z",
        ".exe", ".dmg", ".pkg", ".deb", ".rpm",
        ".mp3", ".mp4", ".avi", ".mkv", ".mov",
        ".jpg", ".jpeg", ".png", ".gif", ".svg",
    ];
    
    let lower = href.to_lowercase();
    download_extensions.iter().any(|ext| lower.ends_with(ext))
}

/// Check if a link is to social media
fn is_social_media_link(href: &str) -> bool {
    let social_domains = [
        "facebook.com", "twitter.com", "x.com", "instagram.com",
        "linkedin.com", "youtube.com", "tiktok.com", "pinterest.com",
        "reddit.com", "tumblr.com", "snapchat.com", "whatsapp.com",
        "telegram.org", "discord.com", "github.com", "gitlab.com",
    ];
    
    social_domains.iter().any(|domain| href.contains(domain))
}

/// Calculate security score based on headers
fn calculate_security_score(headers: &reqwest::header::HeaderMap) -> u8 {
    let mut score = 0u8;
    
    if headers.contains_key("strict-transport-security") {
        score += 20;
    }
    if headers.contains_key("content-security-policy") {
        score += 20;
    }
    if headers.contains_key("x-frame-options") {
        score += 15;
    }
    if headers.contains_key("x-content-type-options") {
        score += 15;
    }
    if headers.contains_key("x-xss-protection") {
        score += 10;
    }
    if headers.contains_key("referrer-policy") {
        score += 10;
    }
    if headers.contains_key("permissions-policy") {
        score += 10;
    }
    
    score.min(100)
}
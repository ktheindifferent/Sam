//! JavaScript rendering support for SPA sites using headless browser
//!
//! This module provides JavaScript rendering capabilities for crawling
//! Single Page Applications (SPAs) and JavaScript-heavy websites.

use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use log::{info, error, debug};
use url::Url;
use once_cell::sync::Lazy;

/// Browser engine to use for rendering
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BrowserEngine {
    /// Chrome/Chromium via Chrome DevTools Protocol
    Chrome,
    /// Firefox via WebDriver
    Firefox,
    /// Safari via WebDriver (macOS only)
    Safari,
}

/// JavaScript rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsRendererConfig {
    /// Browser engine to use
    pub engine: BrowserEngine,
    /// Enable headless mode (no UI)
    pub headless: bool,
    /// Page load timeout in seconds
    pub timeout: Duration,
    /// Wait for network idle before considering page loaded
    pub wait_for_network_idle: bool,
    /// Maximum concurrent browser instances
    pub max_browsers: usize,
    /// User agent string (if different from crawler's)
    pub user_agent: Option<String>,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Block certain resource types (images, fonts, etc.)
    pub blocked_resources: Vec<ResourceType>,
    /// Execute custom JavaScript after page load
    pub custom_scripts: Vec<String>,
    /// Enable browser cache
    pub enable_cache: bool,
    /// Enable cookies
    pub enable_cookies: bool,
    /// Proxy settings
    pub proxy: Option<String>,
}

impl Default for JsRendererConfig {
    fn default() -> Self {
        Self {
            engine: BrowserEngine::Chrome,
            headless: true,
            timeout: Duration::from_secs(30),
            wait_for_network_idle: true,
            max_browsers: 3,
            user_agent: None,
            viewport_width: 1920,
            viewport_height: 1080,
            blocked_resources: vec![
                ResourceType::Image,
                ResourceType::Font,
                ResourceType::Media,
            ],
            custom_scripts: vec![],
            enable_cache: false,
            enable_cookies: true,
            proxy: None,
        }
    }
}

/// Resource types that can be blocked
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    TextTrack,
    Xhr,
    Fetch,
    EventSource,
    WebSocket,
    Manifest,
    Other,
}

/// Result of JavaScript rendering
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// Rendered HTML content
    pub html: String,
    /// Final URL after redirects
    pub final_url: String,
    /// Page title
    pub title: Option<String>,
    /// Meta description
    pub description: Option<String>,
    /// Discovered links
    pub links: Vec<String>,
    /// JavaScript errors encountered
    pub js_errors: Vec<String>,
    /// Network requests made
    pub network_requests: Vec<NetworkRequest>,
    /// Screenshots if captured
    pub screenshot: Option<Vec<u8>>,
    /// Rendering time
    pub render_time: Duration,
    /// Whether JavaScript was detected
    pub has_javascript: bool,
    /// Detected frameworks (React, Angular, Vue, etc.)
    pub frameworks: Vec<String>,
}

/// Network request information
#[derive(Debug, Clone)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub resource_type: ResourceType,
    pub size: Option<usize>,
}

/// Browser instance wrapper
struct BrowserInstance {
    id: String,
    engine: BrowserEngine,
    in_use: Arc<RwLock<bool>>,
    created_at: std::time::Instant,
    render_count: Arc<RwLock<usize>>,
}

/// JavaScript renderer for SPA sites
pub struct JsRenderer {
    config: JsRendererConfig,
    browser_pool: Arc<RwLock<Vec<Arc<BrowserInstance>>>>,
    semaphore: Arc<Semaphore>,
    stats: Arc<RwLock<RenderStats>>,
}

/// Rendering statistics
#[derive(Debug, Default)]
struct RenderStats {
    total_renders: u64,
    successful_renders: u64,
    failed_renders: u64,
    total_render_time: Duration,
    js_errors_count: u64,
    frameworks_detected: HashMap<String, u64>,
}

impl JsRenderer {
    /// Create a new JavaScript renderer
    pub fn new(config: JsRendererConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_browsers));
        
        Self {
            config,
            browser_pool: Arc::new(RwLock::new(Vec::new())),
            semaphore,
            stats: Arc::new(RwLock::new(RenderStats::default())),
        }
    }

    /// Initialize the renderer (start browser instances)
    pub async fn initialize(&self) -> Result<(), JsRenderError> {
        info!("Initializing JavaScript renderer with {} browser instances", self.config.max_browsers);
        
        // Pre-warm browser pool
        for i in 0..self.config.max_browsers.min(2) {
            let instance = self.create_browser_instance(i).await?;
            let mut pool = self.browser_pool.write().await;
            pool.push(Arc::new(instance));
        }
        
        Ok(())
    }

    /// Render a URL with JavaScript
    pub async fn render(&self, url: &str) -> Result<RenderResult, JsRenderError> {
        let start_time = std::time::Instant::now();
        
        // Validate URL
        let parsed_url = Url::parse(url)
            .map_err(|e| JsRenderError::InvalidUrl(url.to_string(), e.to_string()))?;
        
        // Check if JavaScript rendering is needed
        if !self.needs_js_rendering(&parsed_url).await {
            return Err(JsRenderError::NotRequired(url.to_string()));
        }
        
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await
            .map_err(|e| JsRenderError::BrowserPoolExhausted(e.to_string()))?;
        
        // Get or create browser instance
        let browser = self.get_browser_instance().await?;
        
        // Mark browser as in use
        {
            let mut in_use = browser.in_use.write().await;
            *in_use = true;
        }
        
        // Perform rendering
        let result = match browser.engine {
            BrowserEngine::Chrome => self.render_with_chrome(&browser, url).await,
            BrowserEngine::Firefox => self.render_with_firefox(&browser, url).await,
            BrowserEngine::Safari => self.render_with_safari(&browser, url).await,
        };
        
        // Mark browser as available
        {
            let mut in_use = browser.in_use.write().await;
            *in_use = false;
            
            let mut count = browser.render_count.write().await;
            *count += 1;
        }
        
        // Update statistics
        match &result {
            Ok(render_result) => {
                let mut stats = self.stats.write().await;
                stats.total_renders += 1;
                stats.successful_renders += 1;
                stats.total_render_time += render_result.render_time;
                stats.js_errors_count += render_result.js_errors.len() as u64;
                
                for framework in &render_result.frameworks {
                    *stats.frameworks_detected.entry(framework.clone()).or_insert(0) += 1;
                }
            }
            Err(_) => {
                let mut stats = self.stats.write().await;
                stats.total_renders += 1;
                stats.failed_renders += 1;
            }
        }
        
        result
    }

    /// Check if a URL needs JavaScript rendering
    async fn needs_js_rendering(&self, url: &Url) -> bool {
        // Check for common SPA patterns
        let path = url.path();
        
        // Common SPA indicators
        if path.contains("/app") || 
           path.contains("/#/") ||
           path.ends_with("/dashboard") ||
           path.ends_with("/admin") {
            return true;
        }
        
        // Check domain-specific rules
        let domain = url.domain().unwrap_or("");
        let spa_domains = [
            "twitter.com",
            "facebook.com",
            "instagram.com",
            "linkedin.com",
            "github.com",
            "gmail.com",
            "youtube.com",
            "netflix.com",
            "spotify.com",
        ];
        
        if spa_domains.iter().any(|d| domain.contains(d)) {
            return true;
        }
        
        // Could make a HEAD request to check Content-Type
        // or look for specific headers that indicate SPA
        
        false
    }

    /// Get an available browser instance
    async fn get_browser_instance(&self) -> Result<Arc<BrowserInstance>, JsRenderError> {
        let mut pool = self.browser_pool.write().await;
        
        // Find an available instance
        for browser in pool.iter() {
            let in_use = browser.in_use.read().await;
            if !*in_use {
                return Ok(Arc::clone(browser));
            }
        }
        
        // Create a new instance if pool not full
        if pool.len() < self.config.max_browsers {
            let instance = self.create_browser_instance(pool.len()).await?;
            let arc_instance = Arc::new(instance);
            pool.push(Arc::clone(&arc_instance));
            return Ok(arc_instance);
        }
        
        Err(JsRenderError::BrowserPoolExhausted("All browsers in use".to_string()))
    }

    /// Create a new browser instance
    async fn create_browser_instance(&self, id: usize) -> Result<BrowserInstance, JsRenderError> {
        info!("Creating browser instance {} with engine {:?}", id, self.config.engine);
        
        // In a real implementation, this would launch actual browser process
        // For now, we'll create a mock instance
        Ok(BrowserInstance {
            id: format!("browser-{}", id),
            engine: self.config.engine,
            in_use: Arc::new(RwLock::new(false)),
            created_at: std::time::Instant::now(),
            render_count: Arc::new(RwLock::new(0)),
        })
    }

    /// Render with Chrome/Chromium
    async fn render_with_chrome(&self, browser: &BrowserInstance, url: &str) -> Result<RenderResult, JsRenderError> {
        debug!("Rendering {} with Chrome browser {}", url, browser.id);
        
        // This is a placeholder implementation
        // In production, this would use Chrome DevTools Protocol (CDP)
        // via libraries like chromiumoxide or headless_chrome
        
        // Simulate rendering
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Mock result
        Ok(RenderResult {
            html: format!("<html><body>Rendered content for {}</body></html>", url),
            final_url: url.to_string(),
            title: Some("Rendered Page".to_string()),
            description: Some("JavaScript rendered page".to_string()),
            links: vec![],
            js_errors: vec![],
            network_requests: vec![],
            screenshot: None,
            render_time: Duration::from_millis(500),
            has_javascript: true,
            frameworks: vec!["React".to_string()],
        })
    }

    /// Render with Firefox
    async fn render_with_firefox(&self, browser: &BrowserInstance, url: &str) -> Result<RenderResult, JsRenderError> {
        debug!("Rendering {} with Firefox browser {}", url, browser.id);
        
        // Placeholder for Firefox/Gecko implementation
        // Would use WebDriver or similar
        
        Err(JsRenderError::NotImplemented("Firefox rendering not yet implemented".to_string()))
    }

    /// Render with Safari
    async fn render_with_safari(&self, browser: &BrowserInstance, url: &str) -> Result<RenderResult, JsRenderError> {
        debug!("Rendering {} with Safari browser {}", url, browser.id);
        
        // Placeholder for Safari/WebKit implementation
        
        Err(JsRenderError::NotImplemented("Safari rendering not yet implemented".to_string()))
    }

    /// Detect JavaScript frameworks
    pub async fn detect_frameworks(&self, html: &str) -> Vec<String> {
        let mut frameworks = Vec::new();
        
        // React
        if html.contains("react") || html.contains("_reactRoot") || html.contains("__REACT") {
            frameworks.push("React".to_string());
        }
        
        // Angular
        if html.contains("ng-app") || html.contains("ng-controller") || html.contains("angular") {
            frameworks.push("Angular".to_string());
        }
        
        // Vue
        if html.contains("v-app") || html.contains("vue") || html.contains("__vue__") {
            frameworks.push("Vue".to_string());
        }
        
        // Svelte
        if html.contains("svelte") || html.contains("__svelte") {
            frameworks.push("Svelte".to_string());
        }
        
        // Next.js
        if html.contains("__NEXT_DATA__") || html.contains("_next") {
            frameworks.push("Next.js".to_string());
        }
        
        // Gatsby
        if html.contains("gatsby") || html.contains("___gatsby") {
            frameworks.push("Gatsby".to_string());
        }
        
        frameworks
    }

    /// Get rendering statistics
    pub async fn get_stats(&self) -> RenderStats {
        let stats = self.stats.read().await;
        RenderStats {
            total_renders: stats.total_renders,
            successful_renders: stats.successful_renders,
            failed_renders: stats.failed_renders,
            total_render_time: stats.total_render_time,
            js_errors_count: stats.js_errors_count,
            frameworks_detected: stats.frameworks_detected.clone(),
        }
    }

    /// Shutdown the renderer
    pub async fn shutdown(&self) -> Result<(), JsRenderError> {
        info!("Shutting down JavaScript renderer");
        
        let pool = self.browser_pool.write().await;
        
        // In a real implementation, this would close browser processes
        for browser in pool.iter() {
            debug!("Closing browser instance {}", browser.id);
        }
        
        Ok(())
    }
}

/// Error types for JavaScript rendering
#[derive(Debug, thiserror::Error)]
pub enum JsRenderError {
    #[error("Invalid URL: {0} - {1}")]
    InvalidUrl(String, String),
    
    #[error("JavaScript rendering not required for: {0}")]
    NotRequired(String),
    
    #[error("Browser pool exhausted: {0}")]
    BrowserPoolExhausted(String),
    
    #[error("Browser launch failed: {0}")]
    BrowserLaunchFailed(String),
    
    #[error("Page load timeout for: {0}")]
    PageLoadTimeout(String),
    
    #[error("JavaScript execution error: {0}")]
    JavaScriptError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    #[error("Render failed: {0}")]
    RenderFailed(String),
}

/// Global JavaScript renderer instance
static GLOBAL_JS_RENDERER: Lazy<Arc<RwLock<Option<JsRenderer>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(None))
});

/// Initialize the global JavaScript renderer
pub async fn initialize_js_renderer(config: JsRendererConfig) -> Result<(), JsRenderError> {
    let renderer = JsRenderer::new(config);
    renderer.initialize().await?;
    
    let mut global = GLOBAL_JS_RENDERER.write().await;
    *global = Some(renderer);
    
    Ok(())
}

/// Render a URL using the global renderer
pub async fn render_with_javascript(url: &str) -> Result<RenderResult, JsRenderError> {
    let global = GLOBAL_JS_RENDERER.read().await;
    
    match global.as_ref() {
        Some(renderer) => renderer.render(url).await,
        None => Err(JsRenderError::NotImplemented("JavaScript renderer not initialized".to_string())),
    }
}

/// Check if JavaScript rendering is available
pub async fn is_js_rendering_available() -> bool {
    let global = GLOBAL_JS_RENDERER.read().await;
    global.is_some()
}

/// Shutdown the global renderer
pub async fn shutdown_js_renderer() -> Result<(), JsRenderError> {
    let mut global = GLOBAL_JS_RENDERER.write().await;
    
    if let Some(renderer) = global.as_ref() {
        renderer.shutdown().await?;
    }
    
    *global = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_framework_detection() {
        let renderer = JsRenderer::new(JsRendererConfig::default());
        
        let react_html = "<div id='root' data-reactroot></div>";
        let frameworks = renderer.detect_frameworks(react_html).await;
        assert!(frameworks.contains(&"React".to_string()));
        
        let angular_html = "<div ng-app='myApp'></div>";
        let frameworks = renderer.detect_frameworks(angular_html).await;
        assert!(frameworks.contains(&"Angular".to_string()));
        
        let vue_html = "<div id='app' v-app></div>";
        let frameworks = renderer.detect_frameworks(vue_html).await;
        assert!(frameworks.contains(&"Vue".to_string()));
    }

    #[tokio::test]
    async fn test_needs_js_rendering() {
        let renderer = JsRenderer::new(JsRendererConfig::default());
        
        let spa_url = Url::parse("https://example.com/app/dashboard").unwrap();
        assert!(renderer.needs_js_rendering(&spa_url).await);
        
        let twitter_url = Url::parse("https://twitter.com/user").unwrap();
        assert!(renderer.needs_js_rendering(&twitter_url).await);
        
        let static_url = Url::parse("https://example.com/about.html").unwrap();
        assert!(!renderer.needs_js_rendering(&static_url).await);
    }
}
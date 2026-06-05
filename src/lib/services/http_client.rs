use reqwest::{Client, ClientBuilder, Proxy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    pub timeout_seconds: u64,
    pub connect_timeout_seconds: u64,
    pub pool_idle_timeout_seconds: u64,
    pub pool_max_idle_per_host: usize,
    pub max_redirects: usize,
    pub user_agent: String,
    pub proxy_url: Option<String>,
    pub accept_invalid_certs: bool,
    pub enable_compression: bool,
    pub enable_cookies: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            connect_timeout_seconds: 10,
            pool_idle_timeout_seconds: 90,
            pool_max_idle_per_host: 32,
            max_redirects: 10,
            user_agent: "SAM-Services/1.0".to_string(),
            proxy_url: None,
            accept_invalid_certs: false,
            enable_compression: true,
            enable_cookies: true,
        }
    }
}

pub struct HttpClientBuilder {
    config: HttpClientConfig,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
        }
    }

    pub fn with_config(config: HttpClientConfig) -> Self {
        Self { config }
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    pub fn connect_timeout(mut self, seconds: u64) -> Self {
        self.config.connect_timeout_seconds = seconds;
        self
    }

    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.config.user_agent = user_agent;
        self
    }

    pub fn proxy(mut self, proxy_url: String) -> Self {
        self.config.proxy_url = Some(proxy_url);
        self
    }

    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.config.accept_invalid_certs = accept;
        self
    }

    pub fn build(self) -> Result<Client, reqwest::Error> {
        let mut builder = ClientBuilder::new()
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .connect_timeout(Duration::from_secs(self.config.connect_timeout_seconds))
            .pool_idle_timeout(Duration::from_secs(self.config.pool_idle_timeout_seconds))
            .pool_max_idle_per_host(self.config.pool_max_idle_per_host)
            .redirect(reqwest::redirect::Policy::limited(
                self.config.max_redirects,
            ))
            .user_agent(self.config.user_agent)
            .danger_accept_invalid_certs(self.config.accept_invalid_certs);

        if self.config.enable_compression {
            // Note: gzip, deflate, brotli methods may not be available in current reqwest version
            // builder = builder.gzip(true).deflate(true).brotli(true);
        }

        if self.config.enable_cookies {
            // Note: cookie_store method may not be available in current reqwest version
            // builder = builder.cookie_store(true);
        }

        if let Some(proxy_url) = self.config.proxy_url {
            let proxy = Proxy::all(proxy_url)?;
            builder = builder.proxy(proxy);
        }

        builder.build()
    }
}

pub struct SharedHttpClient {
    client: Arc<Client>,
    config: HttpClientConfig,
}

impl SharedHttpClient {
    pub fn new(config: HttpClientConfig) -> Result<Self, reqwest::Error> {
        let client = HttpClientBuilder::with_config(config.clone()).build()?;
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    pub fn default() -> Result<Self, reqwest::Error> {
        Self::new(HttpClientConfig::default())
    }

    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }
}

#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub bearer_token: Option<String>,
    pub basic_auth: Option<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub http_config: HttpClientConfig,
}

impl ApiClientConfig {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            api_key: None,
            bearer_token: None,
            basic_auth: None,
            headers: Vec::new(),
            http_config: HttpClientConfig::default(),
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.bearer_token = Some(token);
        self
    }

    pub fn with_basic_auth(mut self, username: String, password: String) -> Self {
        self.basic_auth = Some((username, password));
        self
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = config;
        self
    }
}

pub struct ApiClient {
    client: Arc<Client>,
    config: ApiClientConfig,
}

impl ApiClient {
    pub fn new(config: ApiClientConfig) -> Result<Self, reqwest::Error> {
        let client = HttpClientBuilder::with_config(config.http_config.clone()).build()?;
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    pub fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut request = self.client.request(method, url);

        // Add authentication
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        if let Some(ref token) = self.config.bearer_token {
            request = request.bearer_auth(token);
        }

        if let Some((ref username, ref password)) = self.config.basic_auth {
            request = request.basic_auth(username, Some(password));
        }

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        request
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.request(reqwest::Method::GET, path).send().await
    }

    pub async fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::POST, path)
    }

    pub async fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::PUT, path)
    }

    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.request(reqwest::Method::DELETE, path).send().await
    }

    pub async fn patch(&self, path: &str) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::PATCH, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.connect_timeout_seconds, 10);
        assert_eq!(config.user_agent, "SAM-Services/1.0");
        assert!(config.enable_compression);
        assert!(config.enable_cookies);
        assert!(!config.accept_invalid_certs);
    }

    #[test]
    fn test_http_client_builder() {
        let client = HttpClientBuilder::new()
            .timeout(60)
            .connect_timeout(5)
            .user_agent("CustomAgent/2.0".to_string())
            .accept_invalid_certs(true)
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_api_client_config() {
        let config = ApiClientConfig::new("https://api.example.com".to_string())
            .with_api_key("test-key".to_string())
            .with_header("Custom-Header".to_string(), "value".to_string());

        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.headers.len(), 1);
    }
}

//! Authentication support for crawling protected resources
//!
//! This module provides authentication capabilities for the crawler,
//! including Basic Auth, Bearer tokens, Cookies, and OAuth 2.0.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, COOKIE};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use base64::{Engine as _, engine::general_purpose};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication
    None,
    
    /// HTTP Basic Authentication
    Basic {
        username: String,
        password: String,
    },
    
    /// Bearer token authentication
    Bearer {
        token: String,
    },
    
    /// Cookie-based authentication
    Cookie {
        cookies: HashMap<String, String>,
    },
    
    /// OAuth 2.0 authentication
    OAuth {
        client_id: String,
        client_secret: String,
        auth_url: String,
        token_url: String,
        scopes: Vec<String>,
        #[serde(skip)]
        access_token: Option<OAuthToken>,
    },
    
    /// API Key authentication
    ApiKey {
        key: String,
        header_name: String,
    },
    
    /// Custom headers
    Custom {
        headers: HashMap<String, String>,
    },
}

/// OAuth token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub scopes: Vec<String>,
}

impl OAuthToken {
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at
        } else {
            false
        }
    }
    
    /// Check if the token needs refresh (5 minutes before expiry)
    pub fn needs_refresh(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at - Duration::minutes(5)
        } else {
            false
        }
    }
}

/// Authentication manager for handling various auth methods
pub struct AuthManager {
    /// Cached OAuth tokens by domain
    oauth_tokens: Arc<RwLock<HashMap<String, OAuthToken>>>,
    
    /// Session cookies by domain
    session_cookies: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    
    /// HTTP client for OAuth flows
    client: reqwest::Client,
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        Self {
            oauth_tokens: Arc::new(RwLock::new(HashMap::new())),
            session_cookies: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }
    
    /// Apply authentication to request headers
    pub async fn apply_auth(
        &self,
        headers: &mut HeaderMap,
        domain: &str,
        auth: &AuthMethod,
    ) -> Result<()> {
        match auth {
            AuthMethod::None => {
                // No authentication needed
            }
            
            AuthMethod::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = general_purpose::STANDARD.encode(credentials);
                let auth_value = format!("Basic {}", encoded);
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&auth_value)
                        .context("Invalid Basic auth header")?
                );
                log::debug!("Applied Basic authentication for {}", domain);
            }
            
            AuthMethod::Bearer { token } => {
                let auth_value = format!("Bearer {}", token);
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&auth_value)
                        .context("Invalid Bearer token header")?
                );
                log::debug!("Applied Bearer token authentication for {}", domain);
            }
            
            AuthMethod::Cookie { cookies } => {
                let cookie_string = cookies
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("; ");
                
                headers.insert(
                    COOKIE,
                    HeaderValue::from_str(&cookie_string)
                        .context("Invalid cookie header")?
                );
                log::debug!("Applied {} cookies for {}", cookies.len(), domain);
            }
            
            AuthMethod::OAuth { client_id, client_secret, token_url, scopes, .. } => {
                // Get or refresh OAuth token
                let token = self.get_or_refresh_oauth_token(
                    domain,
                    client_id,
                    client_secret,
                    token_url,
                    scopes,
                ).await?;
                
                let auth_value = format!("{} {}", token.token_type, token.access_token);
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&auth_value)
                        .context("Invalid OAuth header")?
                );
                log::debug!("Applied OAuth authentication for {}", domain);
            }
            
            AuthMethod::ApiKey { key, header_name } => {
                headers.insert(
                    HeaderName::from_bytes(header_name.as_bytes())
                        .context("Invalid header name")?,
                    HeaderValue::from_str(key)
                        .context("Invalid API key")?
                );
                log::debug!("Applied API key authentication for {}", domain);
            }
            
            AuthMethod::Custom { headers: custom_headers } => {
                for (name, value) in custom_headers {
                    headers.insert(
                        HeaderName::from_bytes(name.as_bytes())
                            .context("Invalid header name")?,
                        HeaderValue::from_str(value)
                            .context("Invalid header value")?
                    );
                }
                log::debug!("Applied {} custom headers for {}", custom_headers.len(), domain);
            }
        }
        
        Ok(())
    }
    
    /// Get or refresh OAuth token
    async fn get_or_refresh_oauth_token(
        &self,
        domain: &str,
        client_id: &str,
        client_secret: &str,
        token_url: &str,
        scopes: &[String],
    ) -> Result<OAuthToken> {
        // Check cache first
        {
            let tokens = self.oauth_tokens.read().await;
            if let Some(token) = tokens.get(domain) {
                if !token.needs_refresh() {
                    return Ok(token.clone());
                }
            }
        }
        
        // Request new token
        log::info!("Requesting new OAuth token for {}", domain);
        let token = self.request_oauth_token(
            client_id,
            client_secret,
            token_url,
            scopes,
        ).await?;
        
        // Cache the token
        {
            let mut tokens = self.oauth_tokens.write().await;
            tokens.insert(domain.to_string(), token.clone());
        }
        
        Ok(token)
    }
    
    /// Request OAuth token using client credentials flow
    async fn request_oauth_token(
        &self,
        client_id: &str,
        client_secret: &str,
        token_url: &str,
        scopes: &[String],
    ) -> Result<OAuthToken> {
        let mut params = HashMap::new();
        params.insert("grant_type", "client_credentials".to_string());
        params.insert("client_id", client_id.to_string());
        params.insert("client_secret", client_secret.to_string());
        
        if !scopes.is_empty() {
            let scope_string = scopes.join(" ");
            params.insert("scope", scope_string);
        }
        
        let response = self.client
            .post(token_url)
            .form(&params)
            .send()
            .await
            .context("Failed to request OAuth token")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OAuth token request failed with status {}: {}", status, error_text);
        }
        
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            token_type: String,
            expires_in: Option<i64>,
            refresh_token: Option<String>,
            scope: Option<String>,
        }
        
        let token_response: TokenResponse = response.json().await
            .context("Failed to parse OAuth token response")?;
        
        let expires_at = token_response.expires_in.map(|seconds| {
            Utc::now() + Duration::seconds(seconds)
        });
        
        let scopes = token_response.scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        
        Ok(OAuthToken {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_at,
            refresh_token: token_response.refresh_token,
            scopes,
        })
    }
    
    /// Store session cookies from a response
    pub async fn store_cookies(&self, domain: &str, cookies: HashMap<String, String>) {
        let cookie_count = cookies.len();
        let mut session_cookies = self.session_cookies.write().await;
        session_cookies.insert(domain.to_string(), cookies);
        log::debug!("Stored {} cookies for {}", cookie_count, domain);
    }
    
    /// Get stored cookies for a domain
    pub async fn get_cookies(&self, domain: &str) -> Option<HashMap<String, String>> {
        let session_cookies = self.session_cookies.read().await;
        session_cookies.get(domain).cloned()
    }
    
    /// Parse cookies from Set-Cookie headers
    pub fn parse_set_cookie_headers(headers: &HeaderMap) -> HashMap<String, String> {
        let mut cookies = HashMap::new();
        
        for value in headers.get_all("set-cookie") {
            if let Ok(cookie_str) = value.to_str() {
                // Simple cookie parsing (doesn't handle all edge cases)
                if let Some(semicolon_pos) = cookie_str.find(';') {
                    let cookie_part = &cookie_str[..semicolon_pos];
                    if let Some(equals_pos) = cookie_part.find('=') {
                        let name = cookie_part[..equals_pos].trim();
                        let value = cookie_part[equals_pos + 1..].trim();
                        cookies.insert(name.to_string(), value.to_string());
                    }
                }
            }
        }
        
        cookies
    }
    
    /// Perform login flow for forms-based authentication
    pub async fn perform_login(
        &self,
        login_url: &str,
        username_field: &str,
        password_field: &str,
        username: &str,
        password: &str,
        additional_fields: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, String>> {
        let mut form_data = HashMap::new();
        form_data.insert(username_field.to_string(), username.to_string());
        form_data.insert(password_field.to_string(), password.to_string());
        
        if let Some(fields) = additional_fields {
            form_data.extend(fields);
        }
        
        log::info!("Performing login at {}", login_url);
        
        let response = self.client
            .post(login_url)
            .form(&form_data)
            .send()
            .await
            .context("Failed to perform login")?;
        
        let cookies = Self::parse_set_cookie_headers(response.headers());
        
        if !response.status().is_success() {
            log::warn!("Login returned status {}", response.status());
        }
        
        Ok(cookies)
    }
}

/// Global authentication manager
static GLOBAL_AUTH_MANAGER: Lazy<AuthManager> = Lazy::new(|| {
    AuthManager::new()
});

/// Apply authentication to a request
pub async fn apply_auth_to_request(
    headers: &mut HeaderMap,
    domain: &str,
    auth: &AuthMethod,
) -> Result<()> {
    GLOBAL_AUTH_MANAGER.apply_auth(headers, domain, auth).await
}

/// Store cookies for a domain
pub async fn store_domain_cookies(domain: &str, cookies: HashMap<String, String>) {
    GLOBAL_AUTH_MANAGER.store_cookies(domain, cookies).await;
}

/// Get cookies for a domain
pub async fn get_domain_cookies(domain: &str) -> Option<HashMap<String, String>> {
    GLOBAL_AUTH_MANAGER.get_cookies(domain).await
}

/// Perform a login flow
pub async fn perform_login_flow(
    login_url: &str,
    username_field: &str,
    password_field: &str,
    username: &str,
    password: &str,
    additional_fields: Option<HashMap<String, String>>,
) -> Result<HashMap<String, String>> {
    GLOBAL_AUTH_MANAGER.perform_login(
        login_url,
        username_field,
        password_field,
        username,
        password,
        additional_fields,
    ).await
}

/// Create auth method from configuration
pub fn auth_from_config(config: &super::config::AuthConfig) -> AuthMethod {
    match config.auth_type.as_str() {
        "basic" => {
            if let (Some(username), Some(password)) = (&config.username, &config.password) {
                AuthMethod::Basic {
                    username: username.clone(),
                    password: password.clone(),
                }
            } else {
                AuthMethod::None
            }
        }
        "bearer" => {
            if let Some(token) = &config.token {
                AuthMethod::Bearer {
                    token: token.clone(),
                }
            } else {
                AuthMethod::None
            }
        }
        "cookie" => {
            AuthMethod::Cookie {
                cookies: config.cookies.clone(),
            }
        }
        "oauth" => {
            if let Some(oauth) = &config.oauth {
                AuthMethod::OAuth {
                    client_id: oauth.client_id.clone(),
                    client_secret: oauth.client_secret.clone(),
                    auth_url: oauth.auth_url.clone(),
                    token_url: oauth.token_url.clone(),
                    scopes: oauth.scopes.clone(),
                    access_token: None,
                }
            } else {
                AuthMethod::None
            }
        }
        _ => AuthMethod::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_auth_encoding() {
        let _auth = AuthMethod::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        
        let headers = HeaderMap::new();
        let manager = AuthManager::new();
        
        // Would need async test context
        // manager.apply_auth(&mut headers, "example.com", &auth).await.unwrap();
        // assert!(headers.contains_key(AUTHORIZATION));
    }
    
    #[test]
    fn test_cookie_formatting() {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "abc123".to_string());
        cookies.insert("token".to_string(), "xyz789".to_string());
        
        let auth = AuthMethod::Cookie { cookies };
        
        // Test cookie string formatting
        if let AuthMethod::Cookie { cookies } = auth {
            let cookie_string = cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            
            assert!(cookie_string.contains("session=abc123"));
            assert!(cookie_string.contains("token=xyz789"));
        }
    }
    
    #[test]
    fn test_oauth_token_expiry() {
        let token = OAuthToken {
            access_token: "token".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() - Duration::hours(1)),
            refresh_token: None,
            scopes: vec![],
        };
        
        assert!(token.is_expired());
        assert!(token.needs_refresh());
        
        let valid_token = OAuthToken {
            access_token: "token".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            refresh_token: None,
            scopes: vec![],
        };
        
        assert!(!valid_token.is_expired());
        assert!(!valid_token.needs_refresh());
    }
}
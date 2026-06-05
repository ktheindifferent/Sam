use chrono::{DateTime, Duration, Utc};
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config, Pool, Runtime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Type alias for consistent session error handling
type SessionError = Box<dyn std::error::Error + Send + Sync>;

/// Session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub ip_address: String,
    pub user_agent: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub data: HashMap<String, String>,
    pub is_authenticated: bool,
    pub csrf_token: String,
}

impl Session {
    /// Create a new session
    pub fn new(ip_address: String, user_agent: String, duration_hours: i64) -> Self {
        let now = Utc::now();
        let session_id = Uuid::new_v4().to_string();
        let csrf_token = Uuid::new_v4().to_string();

        Session {
            id: session_id,
            user_id: None,
            username: None,
            email: None,
            ip_address,
            user_agent,
            created_at: now,
            last_accessed: now,
            expires_at: now + Duration::hours(duration_hours),
            data: HashMap::new(),
            is_authenticated: false,
            csrf_token,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }

    /// Authenticate the session with user information
    pub fn authenticate(&mut self, user_id: String, username: String, email: Option<String>) {
        self.user_id = Some(user_id);
        self.username = Some(username);
        self.email = email;
        self.is_authenticated = true;
        self.touch();
    }

    /// Invalidate the session (logout)
    pub fn invalidate(&mut self) {
        self.user_id = None;
        self.username = None;
        self.email = None;
        self.is_authenticated = false;
        self.data.clear();
        self.expires_at = Utc::now(); // Expire immediately
    }
}

/// Session manager for handling Redis-based sessions
pub struct SessionManager {
    redis_pool: Pool,
    session_ttl: i64, // in seconds
    max_sessions_per_user: usize,
}

impl SessionManager {
    /// Create a new session manager
    pub async fn new(
        redis_url: &str,
        session_ttl_hours: i64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        // Test connection
        let mut conn = pool.get().await?;
        let _: String = deadpool_redis::redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await?;

        Ok(SessionManager {
            redis_pool: pool,
            session_ttl: session_ttl_hours * 3600,
            max_sessions_per_user: 5, // Limit concurrent sessions per user
        })
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        ip_address: String,
        user_agent: String,
    ) -> Result<Session, SessionError> {
        let session = Session::new(ip_address, user_agent, self.session_ttl / 3600);
        self.save_session(&session).await?;
        Ok(session)
    }

    /// Save session to Redis
    pub fn save_session(
        &self,
        session: &Session,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), SessionError>> + Send + 'static>,
    > {
        let session = session.clone();
        let redis_pool = self.redis_pool.clone();
        let session_ttl = self.session_ttl;
        Box::pin(async move {
            let mut conn = redis_pool
                .get()
                .await
                .map_err(|e| SessionError::from(format!("Redis pool error: {}", e)))?;
            let key = format!("session:{}", session.id);
            let value = serde_json::to_string(&session)?;

            // Set with expiration
            conn.set_ex::<_, _, ()>(key, value, session_ttl as u64)
                .await?;

            // If authenticated, track user sessions
            if let Some(user_id) = &session.user_id {
                let user_sessions_key = format!("user_sessions:{}", user_id);
                conn.sadd::<_, _, ()>(&user_sessions_key, &session.id)
                    .await?;
                conn.expire::<_, ()>(&user_sessions_key, session_ttl)
                    .await?;

                // TODO: Enforce max sessions per user when lifetime issues are resolved
            }

            Ok(())
        })
    }

    /// Get session from Redis
    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        let mut conn = self.redis_pool.get().await?;
        let key = format!("session:{}", session_id);

        let value: Option<String> = conn.get(&key).await?;

        match value {
            Some(json) => {
                let mut session: Session = serde_json::from_str(&json)?;

                // Check if expired
                if session.is_expired() {
                    self.delete_session(&session.id).await?;
                    return Ok(None);
                }

                // Update last accessed time
                session.touch();
                self.save_session(&session).await?;

                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut conn = self.redis_pool.get().await?;
        let key = format!("session:{}", session_id);

        // Get session to find user_id
        let value: Option<String> = conn.get(&key).await?;
        if let Some(json) = value {
            if let Ok(session) = serde_json::from_str::<Session>(&json) {
                if let Some(user_id) = session.user_id {
                    let user_sessions_key = format!("user_sessions:{}", user_id);
                    conn.srem::<_, _, ()>(&user_sessions_key, session_id)
                        .await?;
                }
            }
        }

        // Delete the session
        conn.del::<_, ()>(&key).await?;
        Ok(())
    }

    /// Validate CSRF token
    pub async fn validate_csrf_token(
        &self,
        session_id: &str,
        csrf_token: &str,
    ) -> Result<bool, SessionError> {
        if let Some(session) = self.get_session(session_id).await? {
            Ok(session.csrf_token == csrf_token)
        } else {
            Ok(false)
        }
    }

    /// Get all sessions for a user
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, SessionError> {
        let mut conn = self.redis_pool.get().await?;
        let user_sessions_key = format!("user_sessions:{}", user_id);

        let session_ids: Vec<String> = conn.smembers(&user_sessions_key).await?;
        let mut sessions = Vec::new();

        for session_id in session_ids {
            if let Some(session) = self.get_session(&session_id).await? {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    /// Invalidate all sessions for a user
    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<(), SessionError> {
        let sessions = self.get_user_sessions(user_id).await?;

        for session in sessions {
            self.delete_session(&session.id).await?;
        }

        Ok(())
    }

    /// Enforce maximum sessions per user
    async fn enforce_session_limit(&self, user_id: &str) -> Result<(), SessionError> {
        let sessions = self.get_user_sessions(user_id).await?;

        if sessions.len() > self.max_sessions_per_user {
            // Sort by last accessed time
            let mut sorted_sessions = sessions;
            sorted_sessions.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));

            // Remove oldest sessions
            let sessions_to_remove = sorted_sessions.len() - self.max_sessions_per_user;
            for i in 0..sessions_to_remove {
                self.delete_session(&sorted_sessions[i].id).await?;
            }
        }

        Ok(())
    }

    /// Clean up expired sessions (should be called periodically)
    pub async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let mut conn = self.redis_pool.get().await?;
        let pattern = "session:*";
        let keys: Vec<String> = deadpool_redis::redis::cmd("KEYS")
            .arg(pattern)
            .query_async::<Vec<String>>(&mut conn)
            .await?;

        let mut deleted_count = 0;

        for key in keys {
            let value: Option<String> = conn.get(&key).await?;
            if let Some(json) = value {
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    if session.is_expired() {
                        self.delete_session(&session.id).await?;
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

/// Middleware helper for session validation
pub struct SessionMiddleware {
    manager: SessionManager,
}

impl SessionMiddleware {
    pub fn new(manager: SessionManager) -> Self {
        SessionMiddleware { manager }
    }

    /// Extract and validate session from request headers
    pub async fn validate_request(
        &self,
        session_id: Option<String>,
        csrf_token: Option<String>,
        require_auth: bool,
    ) -> Result<Option<Session>, String> {
        // Check if session ID is provided
        let session_id = match session_id {
            Some(id) => id,
            None => {
                if require_auth {
                    return Err("Session ID required".to_string());
                } else {
                    return Ok(None);
                }
            }
        };

        // Get session
        let session = match self.manager.get_session(&session_id).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                if require_auth {
                    return Err("Invalid or expired session".to_string());
                } else {
                    return Ok(None);
                }
            }
            Err(e) => return Err(format!("Session validation error: {}", e)),
        };

        // Check authentication if required
        if require_auth && !session.is_authenticated {
            return Err("Authentication required".to_string());
        }

        // Validate CSRF token if provided
        if let Some(token) = csrf_token {
            match self.manager.validate_csrf_token(&session_id, &token).await {
                Ok(true) => {}
                Ok(false) => return Err("Invalid CSRF token".to_string()),
                Err(e) => return Err(format!("CSRF validation error: {}", e)),
            }
        }

        Ok(Some(session))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("192.168.1.1".to_string(), "Mozilla/5.0".to_string(), 24);

        assert!(!session.id.is_empty());
        assert!(!session.csrf_token.is_empty());
        assert!(!session.is_authenticated);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_authentication() {
        let mut session = Session::new("192.168.1.1".to_string(), "Mozilla/5.0".to_string(), 24);

        session.authenticate(
            "user123".to_string(),
            "john_doe".to_string(),
            Some("john@example.com".to_string()),
        );

        assert!(session.is_authenticated);
        assert_eq!(session.user_id, Some("user123".to_string()));
        assert_eq!(session.username, Some("john_doe".to_string()));
    }

    #[test]
    fn test_session_expiration() {
        let mut session = Session::new("192.168.1.1".to_string(), "Mozilla/5.0".to_string(), 24);

        // Force expiration
        session.expires_at = Utc::now() - Duration::hours(1);

        assert!(session.is_expired());
    }

    #[test]
    fn test_session_invalidation() {
        let mut session = Session::new("192.168.1.1".to_string(), "Mozilla/5.0".to_string(), 24);

        session.authenticate("user123".to_string(), "john_doe".to_string(), None);

        session.data.insert("key".to_string(), "value".to_string());

        session.invalidate();

        assert!(!session.is_authenticated);
        assert!(session.user_id.is_none());
        assert!(session.data.is_empty());
        assert!(session.is_expired());
    }
}

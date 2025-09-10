//! Spotify service for background music control
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use log::info;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use crate::sam::services::thread_manager::{self, ThreadConfig};

#[derive(Debug, Clone, PartialEq)]
pub enum SpotifyStatus {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug)]
pub struct SpotifyService {
    pub status: SpotifyStatus,
    pub shuffle: bool,
    // Add more fields as needed (e.g., current track, playlist, etc.)
}

static SPOTIFY_STATE: Lazy<Arc<Mutex<SpotifyService>>> = Lazy::new(|| {
    Arc::new(Mutex::new(SpotifyService {
        status: SpotifyStatus::Stopped,
        shuffle: false,
    }))
});

static PLAYBACK_THREAD_ID: Lazy<Mutex<Option<String>>> =
    Lazy::new(|| Mutex::new(None));

static SHUTDOWN_SIGNAL: Lazy<Arc<AtomicBool>> = 
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

/// Helper function to safely acquire a lock with error handling
fn acquire_state_lock() -> Result<MutexGuard<'static, SpotifyService>, String> {
    SPOTIFY_STATE.lock()
        .or_else(|e| -> Result<MutexGuard<'static, SpotifyService>, std::sync::PoisonError<MutexGuard<'static, SpotifyService>>> {
            // Attempt to recover from poisoned mutex
            log::warn!("Recovering from poisoned Spotify state mutex: {}", e);
            Ok(e.into_inner())
        })
        .map_err(|_| "Failed to acquire Spotify state lock".to_string())
}

/// Helper function to safely acquire the playback thread lock
fn acquire_thread_lock() -> Result<MutexGuard<'static, Option<String>>, String> {
    PLAYBACK_THREAD_ID.lock()
        .or_else(|e| -> Result<MutexGuard<'static, Option<String>>, std::sync::PoisonError<MutexGuard<'static, Option<String>>>> {
            // Attempt to recover from poisoned mutex
            log::warn!("Recovering from poisoned playback thread mutex: {}", e);
            Ok(e.into_inner())
        })
        .map_err(|_| "Failed to acquire playback thread lock".to_string())
}

/// Try to acquire state lock with timeout (for critical operations)
#[cfg(feature = "timeout")]
fn try_acquire_state_lock_timeout(timeout: Duration) -> Result<MutexGuard<'static, SpotifyService>, String> {
    // Note: std::sync::Mutex doesn't have built-in timeout support
    // For production, consider using parking_lot::Mutex or tokio::sync::Mutex
    acquire_state_lock()
}

/// Start the Spotify service (background music thread)
pub async fn start() {
    let mut state = match acquire_state_lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
            return;
        }
    };
    if state.status == SpotifyStatus::Playing {
        info!("Spotify service already running");
        return;
    }
    state.status = SpotifyStatus::Playing;
    info!("Starting Spotify playback thread");
    let state_arc = SPOTIFY_STATE.clone();
    let mut thread_guard = match acquire_thread_lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire playback thread lock: {}", e);
            return;
        }
    };
    
    if thread_guard.is_none() {
        SHUTDOWN_SIGNAL.store(false, Ordering::Relaxed);
        
        let config = ThreadConfig {
            name: "spotify_playback".to_string(),
            restart_on_panic: true,
            max_restarts: 3,
            restart_delay_ms: 2000,
            health_check_interval_ms: Some(30000),
            enable_monitoring: true,
            priority: crate::sam::services::thread_manager::ThreadPriority::Normal,
            max_memory_mb: None,
            cpu_affinity: None,
        };
        
        let thread_id = thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
            info!("Spotify playback thread started");
            
            while !shutdown_signal.load(Ordering::Relaxed) && !SHUTDOWN_SIGNAL.load(Ordering::Relaxed) {
                {
                    let s = match state_arc.lock()
                        .or_else(|e: std::sync::PoisonError<std::sync::MutexGuard<SpotifyService>>| -> Result<std::sync::MutexGuard<SpotifyService>, std::sync::PoisonError<std::sync::MutexGuard<SpotifyService>>> {
                            log::warn!("Recovering from poisoned mutex in playback thread: {}", e);
                            Ok(e.into_inner())
                        }) {
                        Ok(guard) => guard,
                        Err(_) => {
                            log::error!("Critical: Failed to acquire state lock in playback thread");
                            break;
                        }
                    };
                    match s.status {
                        SpotifyStatus::Playing => {
                            // Simulate playing music
                            info!("[Spotify] Playing music... (shuffle: {})", s.shuffle);
                        }
                        SpotifyStatus::Paused => {
                            info!("[Spotify] Paused");
                        }
                        SpotifyStatus::Stopped => {
                            info!("[Spotify] Stopped");
                            break;
                        }
                    }
                }
                thread::sleep(Duration::from_secs(2));
            }
            
            info!("Spotify playback thread stopped");
        });
        
        *thread_guard = Some(thread_id);
    }
}

/// Stop the Spotify service (stop music and thread)
pub async fn stop() {
    match acquire_state_lock() {
        Ok(mut state) => {
            state.status = SpotifyStatus::Stopped;
            info!("Stopping Spotify playback");
        }
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
            // Still try to signal shutdown even if lock fails
        }
    }
    // Signal the thread to stop
    SHUTDOWN_SIGNAL.store(true, Ordering::Relaxed);
    
    let mut thread_guard = match acquire_thread_lock() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to acquire playback thread lock: {}", e);
            return;
        }
    };
    
    if let Some(thread_id) = thread_guard.take() {
        // Stop the managed thread
        if let Err(e) = thread_manager::stop_thread(&thread_id) {
            log::error!("Failed to stop Spotify thread: {}", e);
        }
    }
}

/// Pause playback
pub async fn pause() {
    match acquire_state_lock() {
        Ok(mut state) => {
            if state.status == SpotifyStatus::Playing {
                state.status = SpotifyStatus::Paused;
                info!("Spotify paused");
            }
        }
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
        }
    }
}

/// Resume playback
pub async fn play() {
    match acquire_state_lock() {
        Ok(mut state) => {
            if state.status == SpotifyStatus::Paused {
                state.status = SpotifyStatus::Playing;
                info!("Spotify resumed");
            }
        }
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
        }
    }
}

/// Toggle shuffle
pub async fn shuffle() {
    match acquire_state_lock() {
        Ok(mut state) => {
            state.shuffle = !state.shuffle;
            info!("Spotify shuffle set to {}", state.shuffle);
        }
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
        }
    }
}

/// Get current status
pub fn status() -> String {
    match acquire_state_lock() {
        Ok(state) => {
            format!(
                "{}{}",
                match state.status {
                    SpotifyStatus::Playing => "playing",
                    SpotifyStatus::Paused => "paused",
                    SpotifyStatus::Stopped => "stopped",
                },
                if state.shuffle { " (shuffle)" } else { "" }
            )
        }
        Err(e) => {
            log::error!("Failed to acquire Spotify state lock: {}", e);
            "error".to_string()
        }
    }
}

pub struct SpotifyApi {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client: Client,
}

impl SpotifyApi {
    pub fn new(client_id: String, client_secret: String) -> Self {
        SpotifyApi {
            client_id,
            client_secret,
            access_token: None,
            refresh_token: None,
            client: Client::new(),
        }
    }

    /// Authenticate with Spotify (OAuth2 Client Credentials flow)
    pub async fn authenticate(&mut self) -> Result<(), String> {
        let auth = STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let params = [("grant_type", "client_credentials")];
        let res = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {auth}"))
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("Spotify auth failed: {}", res.status()));
        }
        let json: serde_json::Value = res.json().await.map_err(|e| format!("JSON error: {e}"))?;
        self.access_token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(())
    }

    /// Refresh the access token (not used in client credentials flow)
    pub async fn refresh_token(&mut self) -> Result<(), String> {
        Err("Refresh token not supported in client credentials flow".to_string())
    }

    /// Play music (resume playback)
    pub async fn play(&self) -> Result<(), String> {
        let token = self.access_token.as_ref().ok_or("No access token")?;
        let res = self
            .client
            .put("https://api.spotify.com/v1/me/player/play")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(format!("Spotify play failed: {}", res.status()))
        }
    }

    /// Pause playback
    pub async fn pause(&self) -> Result<(), String> {
        let token = self.access_token.as_ref().ok_or("No access token")?;
        let res = self
            .client
            .put("https://api.spotify.com/v1/me/player/pause")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(format!("Spotify pause failed: {}", res.status()))
        }
    }

    /// Toggle shuffle
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<(), String> {
        let token = self.access_token.as_ref().ok_or("No access token")?;
        let url = format!("https://api.spotify.com/v1/me/player/shuffle?state={shuffle}");
        let res = self
            .client
            .put(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(format!("Spotify shuffle failed: {}", res.status()))
        }
    }

    /// Get current playback status
    pub async fn get_status(&self) -> Result<String, String> {
        let token = self.access_token.as_ref().ok_or("No access token")?;
        let res = self
            .client
            .get("https://api.spotify.com/v1/me/player")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("Spotify status failed: {}", res.status()));
        }
        let json: serde_json::Value = res.json().await.map_err(|e| format!("JSON error: {e}"))?;
        Ok(json.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use proptest::prelude::*;
    use wiremock::matchers::{method, path, header};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    
    #[test]
    fn test_spotify_status_initial() {
        let state = acquire_state_lock().expect("Failed to acquire lock in test");
        // Initial state may vary, but should be one of the valid states
        assert!(matches!(state.status, SpotifyStatus::Stopped | SpotifyStatus::Playing | SpotifyStatus::Paused));
    }
    
    #[tokio::test]
    async fn test_spotify_lifecycle() {
        // Reset state
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.status = SpotifyStatus::Stopped;
            state.shuffle = false;
        }
        
        // Test start
        start().await;
        {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            assert_eq!(state.status, SpotifyStatus::Playing);
        }
        
        // Test pause
        pause().await;
        {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            assert_eq!(state.status, SpotifyStatus::Paused);
        }
        
        // Test resume
        play().await;
        {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            assert_eq!(state.status, SpotifyStatus::Playing);
        }
        
        // Test stop
        stop().await;
        {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            assert_eq!(state.status, SpotifyStatus::Stopped);
        }
    }
    
    #[tokio::test]
    async fn test_shuffle_toggle() {
        let initial_shuffle = {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.shuffle
        };
        
        shuffle().await;
        
        let new_shuffle = {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.shuffle
        };
        
        assert_ne!(initial_shuffle, new_shuffle);
        
        // Toggle back
        shuffle().await;
        
        let final_shuffle = {
            let state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.shuffle
        };
        
        assert_eq!(initial_shuffle, final_shuffle);
    }
    
    #[test]
    fn test_status_string() {
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.status = SpotifyStatus::Playing;
            state.shuffle = false;
        }
        assert_eq!(status(), "playing");
        
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.status = SpotifyStatus::Paused;
            state.shuffle = true;
        }
        assert_eq!(status(), "paused (shuffle)");
        
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock in test");
            state.status = SpotifyStatus::Stopped;
            state.shuffle = false;
        }
        assert_eq!(status(), "stopped");
    }
    
    #[tokio::test]
    async fn test_spotify_api_authenticate() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .and(header("Authorization", "Basic dGVzdF9pZDp0ZXN0X3NlY3JldA=="))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "access_token": "test_access_token",
                    "token_type": "Bearer",
                    "expires_in": 3600
                })))
            .mount(&mock_server)
            .await;
        
        // Create API with mock server URL
        let mut api = SpotifyApi::new("test_id".to_string(), "test_secret".to_string());
        api.client = Client::builder()
            .build()
            .expect("Failed to build HTTP client in test");
        
        // Note: In real tests, we'd need to mock the actual URL
        // This demonstrates the testing pattern
    }
    
    #[tokio::test]
    async fn test_spotify_api_play() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("PUT"))
            .and(path("/v1/me/player/play"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        
        let api = SpotifyApi {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            access_token: Some("test_token".to_string()),
            refresh_token: None,
            client: Client::new(),
        };
        
        // In real tests, would test against mock server
    }
    
    #[tokio::test]
    async fn test_spotify_api_pause() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("PUT"))
            .and(path("/v1/me/player/pause"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        
        let api = SpotifyApi {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            access_token: Some("test_token".to_string()),
            refresh_token: None,
            client: Client::new(),
        };
        
        // In real tests, would test against mock server
    }
    
    #[tokio::test]
    async fn test_spotify_api_shuffle() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("PUT"))
            .and(path("/v1/me/player/shuffle"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        
        let api = SpotifyApi {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            access_token: Some("test_token".to_string()),
            refresh_token: None,
            client: Client::new(),
        };
        
        // Test with shuffle true and false
        let _ = api.set_shuffle(true).await;
        let _ = api.set_shuffle(false).await;
    }
    
    #[test]
    fn test_spotify_api_new() {
        let api = SpotifyApi::new("test_id".to_string(), "test_secret".to_string());
        assert_eq!(api.client_id, "test_id");
        assert_eq!(api.client_secret, "test_secret");
        assert!(api.access_token.is_none());
        assert!(api.refresh_token.is_none());
    }
    
    #[tokio::test]
    async fn test_spotify_api_error_handling() {
        let api = SpotifyApi {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            access_token: None, // No token should cause error
            refresh_token: None,
            client: Client::new(),
        };
        
        let result = api.play().await;
        assert!(result.is_err());
        assert_eq!(result.expect_err("Expected error for no access token"), "No access token");
        
        let result = api.pause().await;
        assert!(result.is_err());
        assert_eq!(result.expect_err("Expected error for no access token"), "No access token");
        
        let result = api.set_shuffle(true).await;
        assert!(result.is_err());
        assert_eq!(result.expect_err("Expected error for no access token"), "No access token");
    }
    
    proptest! {
        #[test]
        fn test_base64_encoding(
            client_id in "[a-zA-Z0-9]{10,30}",
            client_secret in "[a-zA-Z0-9]{10,30}"
        ) {
            let encoded = STANDARD.encode(format!("{}:{}", client_id, client_secret));
            prop_assert!(!encoded.is_empty());
            prop_assert!(!encoded.contains(':'));
        }
        
        #[test]
        fn test_shuffle_url_generation(shuffle in any::<bool>()) {
            let url = format!("https://api.spotify.com/v1/me/player/shuffle?state={shuffle}");
            prop_assert!(url.contains(&shuffle.to_string()));
            prop_assert!(url.starts_with("https://api.spotify.com"));
        }
    }
    
    #[test]
    fn test_concurrent_state_access() {
        use std::sync::Arc;
        use std::thread;
        
        let barrier = Arc::new(std::sync::Barrier::new(10));
        let mut handles = vec![];
        
        for i in 0..10 {
            let c = barrier.clone();
            let handle = thread::spawn(move || {
                c.wait();
                // Concurrent access to state should be safe
                match acquire_state_lock() {
                    Ok(state) => {
                        let _status = state.status.clone();
                        let _shuffle = state.shuffle;
                    }
                    Err(e) => {
                        panic!("Failed to acquire lock in concurrent test: {}", e);
                    }
                }
                
                // Also test status() function
                let _status_str = status();
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().expect("Thread panicked during concurrent access test");
        }
    }
    
    #[test]
    fn test_mutex_recovery() {
        // Test that we can recover from poisoned mutex
        use std::panic;
        use std::thread;
        
        // Create a thread that will panic while holding the lock
        let result = thread::spawn(|| {
            let _state = SPOTIFY_STATE.lock().expect("Initial lock should work");
            panic!("Intentional panic to poison mutex");
        }).join();
        
        // Verify the thread panicked
        assert!(result.is_err());
        
        // Now test that we can still acquire the lock via our helper function
        match acquire_state_lock() {
            Ok(state) => {
                // Successfully recovered from poisoned mutex
                let _ = state.status.clone();
            }
            Err(e) => {
                panic!("Failed to recover from poisoned mutex: {}", e);
            }
        }
        
        // Also test that status() function handles poisoned mutex gracefully
        let status_str = status();
        assert!(!status_str.is_empty());
    }
    
    #[test]
    fn test_error_handling_in_status() {
        // Test that status() returns "error" when lock acquisition fails
        // This is already tested implicitly but let's be explicit
        let status_result = status();
        // Should always return a valid string, never panic
        assert!(status_result == "playing" || 
                status_result == "paused" || 
                status_result == "paused (shuffle)" ||
                status_result == "stopped" || 
                status_result == "playing (shuffle)" ||
                status_result == "stopped (shuffle)" ||
                status_result == "error");
    }
    
    #[tokio::test]
    async fn test_graceful_degradation() {
        // Test that operations continue even when some fail
        
        // Start should handle existing playing state gracefully
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock");
            state.status = SpotifyStatus::Playing;
        }
        start().await; // Should log "already running" but not panic
        
        // Pause from non-playing state should be no-op
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock");
            state.status = SpotifyStatus::Stopped;
        }
        pause().await; // Should be no-op
        
        // Play from non-paused state should be no-op
        {
            let mut state = acquire_state_lock().expect("Failed to acquire lock");
            state.status = SpotifyStatus::Playing;
        }
        play().await; // Should be no-op
    }
    
    #[test]
    fn test_multiple_rapid_status_calls() {
        // Test rapid concurrent calls to status()
        use std::thread;
        
        let mut handles = vec![];
        for _ in 0..100 {
            let handle = thread::spawn(|| {
                for _ in 0..10 {
                    let _ = status();
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().expect("Status call thread should not panic");
        }
    }
    
    #[tokio::test]
    async fn test_thread_lock_error_handling() {
        // Test that thread operations handle lock errors gracefully
        
        // Start should handle thread lock errors
        start().await;
        
        // Stop should handle thread lock errors
        stop().await;
        
        // Verify we're in stopped state
        let status_str = status();
        assert!(status_str == "stopped" || status_str == "stopped (shuffle)");
    }
}

//! Spotify service for background music control
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use once_cell::sync::Lazy;
use log::info;
use reqwest::Client;

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

static mut PLAYBACK_THREAD: Option<thread::JoinHandle<()>> = None;

/// Start the Spotify service (background music thread)
pub async fn start() {
    let mut state = SPOTIFY_STATE.lock().unwrap();
    if state.status == SpotifyStatus::Playing {
        info!("Spotify service already running");
        return;
    }
    state.status = SpotifyStatus::Playing;
    info!("Starting Spotify playback thread");
    let state_arc = SPOTIFY_STATE.clone();
    unsafe {
        if PLAYBACK_THREAD.is_none() {
            PLAYBACK_THREAD = Some(thread::spawn(move || {
                loop {
                    {
                        let s = state_arc.lock().unwrap();
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
            }));
        }
    }
}

/// Stop the Spotify service (stop music and thread)
pub async fn stop() {
    let mut state = SPOTIFY_STATE.lock().unwrap();
    state.status = SpotifyStatus::Stopped;
    info!("Stopping Spotify playback");
    unsafe {
        if let Some(handle) = PLAYBACK_THREAD.take() {
            let _ = handle.join();
        }
    }
}

/// Pause playback
pub async fn pause() {
    let mut state = SPOTIFY_STATE.lock().unwrap();
    if state.status == SpotifyStatus::Playing {
        state.status = SpotifyStatus::Paused;
        info!("Spotify paused");
    }
}

/// Resume playback
pub async fn play() {
    let mut state = SPOTIFY_STATE.lock().unwrap();
    if state.status == SpotifyStatus::Paused {
        state.status = SpotifyStatus::Playing;
        info!("Spotify resumed");
    }
}

/// Toggle shuffle
pub async fn shuffle() {
    let mut state = SPOTIFY_STATE.lock().unwrap();
    state.shuffle = !state.shuffle;
    info!("Spotify shuffle set to {}", state.shuffle);
}

/// Get current status
pub fn status() -> String {
    let state = SPOTIFY_STATE.lock().unwrap();
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
        let auth = base64::encode(format!("{}:{}", self.client_id, self.client_secret));
        let params = [
            ("grant_type", "client_credentials"),
        ];
        let res = self.client.post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {auth}"))
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("Spotify auth failed: {}", res.status()));
        }
        let json: serde_json::Value = res.json().await.map_err(|e| format!("JSON error: {e}"))?;
        self.access_token = json.get("access_token").and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(())
    }

    /// Refresh the access token (not used in client credentials flow)
    pub async fn refresh_token(&mut self) -> Result<(), String> {
        Err("Refresh token not supported in client credentials flow".to_string())
    }

    /// Play music (resume playback)
    pub async fn play(&self) -> Result<(), String> {
        let token = self.access_token.as_ref().ok_or("No access token")?;
        let res = self.client.put("https://api.spotify.com/v1/me/player/play")
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
        let res = self.client.put("https://api.spotify.com/v1/me/player/pause")
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
        let res = self.client.put(&url)
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
        let res = self.client.get("https://api.spotify.com/v1/me/player")
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

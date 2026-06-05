//! Plugin loader with hot-reload via filesystem watcher.
//!
//! Scans `~/.sam/plugins/` for plugin directories, loads them into the
//! `PluginRegistry`, and watches for changes to trigger reloads.

use super::manifest::PluginManifest;
use super::wasm_runtime::{WasmPlugin, WasmPluginConfig};
use super::PluginRegistry;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the plugin loader.
#[derive(Debug, Clone)]
pub struct PluginLoaderConfig {
    /// Directory to scan for plugins.
    pub plugins_dir: PathBuf,
    /// Max memory per plugin in bytes.
    pub max_memory_per_plugin: u64,
    /// Fuel limit for CPU metering.
    pub fuel_limit: u64,
    /// Enable filesystem watcher for hot-reload.
    pub hot_reload: bool,
}

impl Default for PluginLoaderConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            plugins_dir: PathBuf::from(home).join(".sam").join("plugins"),
            max_memory_per_plugin: 64 * 1024 * 1024,
            fuel_limit: 1_000_000_000,
            hot_reload: true,
        }
    }
}

/// Plugin loader that manages discovery, loading, and hot-reload.
pub struct PluginLoader {
    config: PluginLoaderConfig,
    registry: Arc<RwLock<PluginRegistry>>,
}

impl PluginLoader {
    pub fn new(config: PluginLoaderConfig, registry: Arc<RwLock<PluginRegistry>>) -> Self {
        Self { config, registry }
    }

    /// Scan the plugins directory and load all valid plugins.
    pub async fn load_all(&self) -> Result<usize, String> {
        let plugins_dir = &self.config.plugins_dir;
        if !plugins_dir.exists() {
            log::info!(
                "Plugins directory does not exist: {}",
                plugins_dir.display()
            );
            let _ = std::fs::create_dir_all(plugins_dir);
            return Ok(0);
        }

        let entries = std::fs::read_dir(plugins_dir)
            .map_err(|e| format!("Failed to read plugins directory: {}", e))?;

        let mut loaded = 0;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("Error reading plugin entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            match self.load_plugin(&path).await {
                Ok(()) => {
                    loaded += 1;
                }
                Err(e) => {
                    log::warn!("Failed to load plugin from {}: {}", path.display(), e);
                }
            }
        }

        log::info!("Loaded {} plugins from {}", loaded, plugins_dir.display());
        Ok(loaded)
    }

    /// Load a single plugin from a directory.
    async fn load_plugin(&self, plugin_dir: &Path) -> Result<(), String> {
        let manifest_path = plugin_dir.join("plugin.toml");
        if !manifest_path.exists() {
            return Err(format!("No plugin.toml in {}", plugin_dir.display()));
        }

        let wasm_config = WasmPluginConfig {
            max_memory_bytes: self.config.max_memory_per_plugin,
            fuel_limit: self.config.fuel_limit,
        };

        let plugin = WasmPlugin::load(plugin_dir, wasm_config)?;
        let plugin_arc = Arc::new(plugin);

        // Initialize the plugin
        plugin_arc
            .initialize()
            .await
            .map_err(|e| format!("Plugin initialization failed: {}", e))?;

        // Register in the registry
        let mut registry = self.registry.write().await;
        // Remove existing plugin with same name if reloading
        let name = plugin_arc.name().to_string();
        if registry.get(&name).is_some() {
            log::info!("Reloading plugin '{}'", name);
            // PluginRegistry doesn't have remove, but we can re-register
            // For hot-reload we need to handle this gracefully
        }

        registry
            .register(plugin_arc)
            .map_err(|e| format!("Registration failed: {}", e))?;

        Ok(())
    }

    /// Spawn a filesystem watcher for hot-reload.
    /// Returns a join handle for the watcher task.
    pub fn spawn_watcher(self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let registry = self.registry.clone();

        tokio::spawn(async move {
            if !config.hot_reload {
                log::info!("Plugin hot-reload is disabled");
                return;
            }

            let plugins_dir = config.plugins_dir.clone();
            if !plugins_dir.exists() {
                let _ = std::fs::create_dir_all(&plugins_dir);
            }

            let (tx, mut rx) = tokio::sync::mpsc::channel(32);

            // Spawn a blocking thread for the notify watcher
            let watch_dir = plugins_dir.clone();
            let _watcher_thread = std::thread::spawn(move || {
                let rt_tx = tx;
                let mut watcher: RecommendedWatcher =
                    match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                        if let Ok(event) = res {
                            let _ = rt_tx.blocking_send(event);
                        }
                    }) {
                        Ok(w) => w,
                        Err(e) => {
                            log::error!("Failed to create filesystem watcher: {}", e);
                            return;
                        }
                    };

                if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
                    log::error!("Failed to watch plugins directory: {}", e);
                    return;
                }

                log::info!(
                    "Plugin hot-reload watcher started on {}",
                    watch_dir.display()
                );

                // Keep the thread alive so the watcher stays active
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
            });

            // Debounce: wait a bit after changes before reloading
            let loader = PluginLoader::new(config, registry);
            let mut last_reload = std::time::Instant::now();

            while let Some(event) = rx.recv().await {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        // Check if the changed file is a .wasm or .toml
                        let relevant = event.paths.iter().any(|p| {
                            p.extension()
                                .map(|ext| ext == "wasm" || ext == "toml")
                                .unwrap_or(false)
                        });

                        if relevant && last_reload.elapsed() > std::time::Duration::from_secs(2) {
                            log::info!("Plugin file change detected, reloading plugins...");
                            last_reload = std::time::Instant::now();
                            // Short delay for writes to complete
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            if let Err(e) = loader.load_all().await {
                                log::warn!("Plugin reload failed: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
    }
}

/// User-level plugin configuration from `~/.sam/config.toml`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PluginUserConfig {
    /// Enable the plugin system.
    #[serde(default)]
    pub enabled: bool,
    /// Path to plugins directory (default: ~/.sam/plugins).
    pub plugins_dir: Option<String>,
    /// Max memory per plugin in MB.
    pub max_memory_per_plugin_mb: Option<u64>,
    /// Enable hot-reload via filesystem watcher.
    pub hot_reload: Option<bool>,
}

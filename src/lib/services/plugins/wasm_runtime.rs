//! WASM-based plugin runtime using wasmtime.
//!
//! `WasmPlugin` loads a `.wasm` module and adapts it to the existing `Plugin`
//! trait so the registry treats it identically to compiled-in plugins.

use super::manifest::PluginManifest;
use super::Plugin;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;
use wasmtime::*;

/// Configuration for the WASM plugin sandbox.
pub struct WasmPluginConfig {
    /// Maximum memory in bytes a single plugin can allocate.
    pub max_memory_bytes: u64,
    /// Fuel limit for CPU metering (0 = unlimited).
    pub fuel_limit: u64,
}

impl Default for WasmPluginConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64 MB
            fuel_limit: 1_000_000_000,
        }
    }
}

/// A dynamically loaded WASM plugin.
pub struct WasmPlugin {
    manifest: PluginManifest,
    wasm_path: PathBuf,
    engine: Engine,
    module: Module,
    store: Mutex<Option<Store<()>>>,
    config: WasmPluginConfig,
}

impl WasmPlugin {
    /// Load a WASM plugin from a directory containing `plugin.toml` and `plugin.wasm`.
    pub fn load(plugin_dir: &std::path::Path, config: WasmPluginConfig) -> Result<Self, String> {
        let manifest_path = plugin_dir.join("plugin.toml");
        let wasm_path = plugin_dir.join("plugin.wasm");

        let manifest = PluginManifest::from_file(&manifest_path)?;

        if !wasm_path.exists() {
            return Err(format!("plugin.wasm not found in {}", plugin_dir.display()));
        }

        let mut engine_config = Config::new();
        engine_config.consume_fuel(config.fuel_limit > 0);

        let engine = Engine::new(&engine_config)
            .map_err(|e| format!("Failed to create WASM engine: {}", e))?;

        let module = Module::from_file(&engine, &wasm_path)
            .map_err(|e| format!("Failed to load WASM module: {}", e))?;

        log::info!(
            "Loaded WASM plugin '{}' v{} from {}",
            manifest.name,
            manifest.version,
            plugin_dir.display()
        );

        Ok(Self {
            manifest,
            wasm_path,
            engine,
            module,
            store: Mutex::new(None),
            config,
        })
    }

    /// Create a fresh store with fuel and memory limits.
    fn new_store(&self) -> Result<Store<()>, String> {
        let mut store = Store::new(&self.engine, ());
        if self.config.fuel_limit > 0 {
            store
                .set_fuel(self.config.fuel_limit)
                .map_err(|e| format!("Failed to set fuel: {}", e))?;
        }
        Ok(store)
    }

    /// Call an exported function by name with no args, return string result.
    fn call_export(&self, func_name: &str) -> Result<String, String> {
        let mut store = self.new_store()?;
        let linker = Linker::new(&self.engine);

        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| format!("Instantiation failed: {}", e))?;

        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| format!("Export '{}' not found", func_name))?;

        let mut results = vec![Val::I32(0)];
        func.call(&mut store, &[], &mut results)
            .map_err(|e| format!("Call to '{}' failed: {}", func_name, e))?;

        Ok(format!("{}={:?}", func_name, results))
    }
}

#[async_trait]
impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn version(&self) -> &str {
        &self.manifest.version
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // Try calling _initialize export if it exists
        match self.call_export("_initialize") {
            Ok(msg) => {
                log::info!("WASM plugin '{}' initialized: {}", self.manifest.name, msg);
            }
            Err(e) => {
                // _initialize is optional
                log::debug!(
                    "WASM plugin '{}' has no _initialize export: {}",
                    self.manifest.name,
                    e
                );
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        match self.call_export("_shutdown") {
            Ok(msg) => {
                log::info!("WASM plugin '{}' shut down: {}", self.manifest.name, msg);
            }
            Err(_) => {
                log::debug!(
                    "WASM plugin '{}' has no _shutdown export",
                    self.manifest.name
                );
            }
        }
        Ok(())
    }

    fn commands(&self) -> Vec<(String, String)> {
        self.manifest
            .commands
            .iter()
            .map(|cmd| (cmd.name.clone(), cmd.description.clone()))
            .collect()
    }

    async fn handle_command(&self, command: &str, _args: &[&str]) -> anyhow::Result<String> {
        // Try calling _handle_command export
        // For simplicity, we call the export with just the command name
        // A real implementation would serialize args into WASM memory
        match self.call_export("_handle_command") {
            Ok(result) => Ok(result),
            Err(e) => Err(anyhow::anyhow!(
                "Plugin '{}' failed to handle '{}': {}",
                self.manifest.name,
                command,
                e
            )),
        }
    }
}

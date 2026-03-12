//! Plugin system for SAM.
//!
//! Provides a `Plugin` trait and `PluginRegistry` for compiled-in plugins.
//! When the `plugins` feature is enabled, also provides WASM-based dynamic
//! plugin loading with hot-reload via filesystem watcher.

pub mod manifest;

#[cfg(feature = "plugins")]
pub mod wasm_runtime;
#[cfg(feature = "plugins")]
pub mod loader;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// A compiled-in plugin that extends SAM's functionality.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique plugin name.
    fn name(&self) -> &str;

    /// Semantic version string.
    fn version(&self) -> &str;

    /// Called once when SAM starts. Perform setup here.
    async fn initialize(&self) -> anyhow::Result<()>;

    /// Called on SAM shutdown. Perform cleanup here.
    async fn shutdown(&self) -> anyhow::Result<()>;

    /// Return commands this plugin provides (name → description).
    fn commands(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Handle a command routed to this plugin.
    async fn handle_command(&self, _command: &str, _args: &[&str]) -> anyhow::Result<String> {
        Err(anyhow::anyhow!("Command not supported"))
    }
}

/// Registry of all compiled-in plugins.
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin. Returns error if name conflicts.
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) -> anyhow::Result<()> {
        let name = plugin.name().to_string();
        if self.plugins.contains_key(&name) {
            return Err(anyhow::anyhow!("Plugin '{}' already registered", name));
        }
        log::info!("Registered plugin: {} v{}", plugin.name(), plugin.version());
        self.plugins.insert(name, plugin);
        Ok(())
    }

    /// Initialize all registered plugins.
    pub async fn initialize_all(&self) -> anyhow::Result<()> {
        for (name, plugin) in &self.plugins {
            log::info!("Initializing plugin: {}", name);
            plugin.initialize().await?;
        }
        Ok(())
    }

    /// Shutdown all registered plugins.
    pub async fn shutdown_all(&self) -> anyhow::Result<()> {
        for (name, plugin) in &self.plugins {
            log::info!("Shutting down plugin: {}", name);
            if let Err(e) = plugin.shutdown().await {
                log::warn!("Error shutting down plugin '{}': {}", name, e);
            }
        }
        Ok(())
    }

    /// Unregister a plugin by name (for hot-reload).
    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Plugin>> {
        let removed = self.plugins.remove(name);
        if removed.is_some() {
            log::info!("Unregistered plugin: {}", name);
        }
        removed
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Plugin>> {
        self.plugins.get(name)
    }

    /// List all registered plugin names.
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Collect all commands from all plugins.
    pub fn all_commands(&self) -> Vec<(String, String, String)> {
        let mut commands = Vec::new();
        for (_, plugin) in &self.plugins {
            for (cmd, desc) in plugin.commands() {
                commands.push((plugin.name().to_string(), cmd, desc));
            }
        }
        commands
    }

    /// Route a command to the appropriate plugin.
    pub async fn handle_command(&self, plugin_name: &str, command: &str, args: &[&str]) -> anyhow::Result<String> {
        match self.plugins.get(plugin_name) {
            Some(plugin) => plugin.handle_command(command, args).await,
            None => Err(anyhow::anyhow!("Plugin '{}' not found", plugin_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    #[async_trait]
    impl Plugin for TestPlugin {
        fn name(&self) -> &str { "test" }
        fn version(&self) -> &str { "0.1.0" }
        async fn initialize(&self) -> anyhow::Result<()> { Ok(()) }
        async fn shutdown(&self) -> anyhow::Result<()> { Ok(()) }
        fn commands(&self) -> Vec<(String, String)> {
            vec![("greet".to_string(), "Say hello".to_string())]
        }
        async fn handle_command(&self, command: &str, _args: &[&str]) -> anyhow::Result<String> {
            match command {
                "greet" => Ok("Hello from test plugin!".to_string()),
                _ => Err(anyhow::anyhow!("Unknown command")),
            }
        }
    }

    #[tokio::test]
    async fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(TestPlugin)).unwrap();

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("test").is_some());

        let commands = registry.all_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].1, "greet");
    }

    #[tokio::test]
    async fn test_plugin_command_routing() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(TestPlugin)).unwrap();

        let result = registry.handle_command("test", "greet", &[]).await.unwrap();
        assert_eq!(result, "Hello from test plugin!");
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(TestPlugin)).unwrap();
        assert!(registry.register(Arc::new(TestPlugin)).is_err());
    }
}

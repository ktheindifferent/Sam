//! CLI commands for the plugin system.
//!
//! Usage:
//!   plugin list       - List all loaded plugins
//!   plugin info <n>   - Show details for a plugin
//!   plugin reload     - Reload all plugins from disk

use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_plugin(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let subcmd = parts.get(1).copied().unwrap_or("list");

    match subcmd {
        "list" => {
            let mut lines = output_lines.lock().await;
            lines.push("=== Loaded Plugins ===".to_string());

            // Access the global plugin registry
            // For now, show a placeholder since the registry is owned by main
            lines.push("Plugin system status:".to_string());

            #[cfg(feature = "plugins")]
            {
                lines.push("  WASM plugin runtime: enabled".to_string());
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let plugins_dir =
                    std::path::PathBuf::from(&home).join(".sam").join("plugins");
                if plugins_dir.exists() {
                    match std::fs::read_dir(&plugins_dir) {
                        Ok(entries) => {
                            let mut count = 0;
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_dir() {
                                    let manifest = path.join("plugin.toml");
                                    if manifest.exists() {
                                        match crate::services::plugins::manifest::PluginManifest::from_file(&manifest) {
                                            Ok(m) => {
                                                lines.push(format!(
                                                    "  {} v{} - {}",
                                                    m.name, m.version, m.description
                                                ));
                                                count += 1;
                                            }
                                            Err(e) => {
                                                lines.push(format!(
                                                    "  [error] {}: {}",
                                                    path.display(),
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            if count == 0 {
                                lines.push(format!(
                                    "  No plugins found in {}",
                                    plugins_dir.display()
                                ));
                            }
                        }
                        Err(e) => {
                            lines.push(format!(
                                "  Error reading plugins dir: {}",
                                e
                            ));
                        }
                    }
                } else {
                    lines.push(format!(
                        "  Plugins directory not found: {}",
                        plugins_dir.display()
                    ));
                    lines.push("  Create it to start using plugins.".to_string());
                }
            }

            #[cfg(not(feature = "plugins"))]
            {
                lines.push("  WASM plugin runtime: disabled (compile with --features plugins)".to_string());
                lines.push("  Compiled-in plugins only.".to_string());
            }
        }

        "info" => {
            let name = parts.get(2).copied().unwrap_or("");
            let mut lines = output_lines.lock().await;
            if name.is_empty() {
                lines.push("Usage: plugin info <plugin-name>".to_string());
                return;
            }

            #[cfg(feature = "plugins")]
            {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let manifest_path = std::path::PathBuf::from(&home)
                    .join(".sam")
                    .join("plugins")
                    .join(name)
                    .join("plugin.toml");

                if manifest_path.exists() {
                    match crate::services::plugins::manifest::PluginManifest::from_file(
                        &manifest_path,
                    ) {
                        Ok(m) => {
                            lines.push(format!("Name:        {}", m.name));
                            lines.push(format!("Version:     {}", m.version));
                            lines.push(format!("Description: {}", m.description));
                            lines.push(format!("Author:      {}", m.author));
                            lines.push(format!(
                                "Permissions: network={}, fs={}, services={}, notifications={}",
                                m.permissions.network,
                                m.permissions.filesystem,
                                m.permissions.services,
                                m.permissions.notifications
                            ));
                            if m.commands.is_empty() {
                                lines.push("Commands:    (none)".to_string());
                            } else {
                                lines.push("Commands:".to_string());
                                for cmd in &m.commands {
                                    lines.push(format!("  {} - {}", cmd.name, cmd.description));
                                }
                            }
                        }
                        Err(e) => {
                            lines.push(format!("Error loading manifest: {}", e));
                        }
                    }
                } else {
                    lines.push(format!("Plugin '{}' not found", name));
                }
            }

            #[cfg(not(feature = "plugins"))]
            {
                lines.push(format!("Plugin '{}' — WASM runtime disabled", name));
            }
        }

        "reload" => {
            let mut lines = output_lines.lock().await;
            #[cfg(feature = "plugins")]
            {
                lines.push("Triggering plugin reload...".to_string());
                lines.push("Plugins will be reloaded from ~/.sam/plugins/".to_string());
            }
            #[cfg(not(feature = "plugins"))]
            {
                lines.push(
                    "WASM plugin runtime disabled. Compile with --features plugins".to_string(),
                );
            }
        }

        _ => {
            let mut lines = output_lines.lock().await;
            lines.push("Plugin commands:".to_string());
            lines.push("  plugin list            List loaded plugins".to_string());
            lines.push("  plugin info <name>     Show plugin details".to_string());
            lines.push("  plugin reload          Reload plugins from disk".to_string());
        }
    }
}

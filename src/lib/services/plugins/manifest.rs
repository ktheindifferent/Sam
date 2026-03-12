//! Plugin manifest parsed from `plugin.toml`.
//!
//! Each plugin directory under `~/.sam/plugins/<name>/` must contain a
//! `plugin.toml` that describes the plugin's metadata, permissions, and
//! commands.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Manifest loaded from `plugin.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Permissions requested by this plugin.
    #[serde(default)]
    pub permissions: PluginPermissions,
    /// Commands this plugin provides.
    #[serde(default)]
    pub commands: Vec<PluginCommandDef>,
}

/// Permissions a plugin can request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    /// Can the plugin make outbound network requests?
    #[serde(default)]
    pub network: bool,
    /// Can the plugin access the filesystem?
    #[serde(default)]
    pub filesystem: bool,
    /// Can the plugin query service status?
    #[serde(default)]
    pub services: bool,
    /// Can the plugin emit notifications?
    #[serde(default)]
    pub notifications: bool,
}

/// A command definition exported by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandDef {
    pub name: String,
    pub description: String,
}

impl PluginManifest {
    /// Load a manifest from a `plugin.toml` file.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let toml_str = r#"
name = "hello-plugin"
version = "1.0.0"
description = "A test plugin"
author = "Test Author"

[permissions]
network = false
services = true

[[commands]]
name = "hello"
description = "Say hello"
"#;
        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "hello-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.permissions.services);
        assert!(!manifest.permissions.network);
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.commands[0].name, "hello");
    }

    #[test]
    fn test_default_permissions() {
        let toml_str = r#"
name = "minimal"
version = "0.1.0"
"#;
        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert!(!manifest.permissions.network);
        assert!(!manifest.permissions.filesystem);
        assert!(!manifest.permissions.services);
        assert!(!manifest.permissions.notifications);
        assert!(manifest.commands.is_empty());
    }
}

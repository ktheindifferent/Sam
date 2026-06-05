use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{error, info, warn};
use std::env;

/// Environment configuration for CapRover and external services
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub is_caprover: bool,
    pub redis_url: Option<String>,
    pub postgres_url: Option<String>,
    pub tts_url: Option<String>,
    pub stt_url: Option<String>,
    pub use_docker: bool,
}

impl EnvironmentConfig {
    /// Create a new environment configuration based on environment variables
    pub fn from_env() -> Result<Self> {
        let is_caprover = env::var("CAPROVER").unwrap_or_default().to_lowercase() == "true";

        let config = if is_caprover {
            info!("Running in CapRover environment - using external services");
            Self {
                is_caprover: true,
                redis_url: env::var("REDIS_URL").ok(),
                postgres_url: env::var("DATABASE_URL")
                    .or_else(|_| env::var("POSTGRES_URL"))
                    .ok(),
                tts_url: env::var("TTS_URL").ok(),
                stt_url: env::var("STT_URL").ok(),
                use_docker: false, // Never use Docker in CapRover
            }
        } else {
            info!("Running in standard environment - using local/Docker services");
            Self {
                is_caprover: false,
                redis_url: env::var("REDIS_URL")
                    .ok()
                    .or_else(|| Some("redis://127.0.0.1:6379".to_string())),
                postgres_url: env::var("DATABASE_URL")
                    .or_else(|_| env::var("POSTGRES_URL"))
                    .ok(),
                tts_url: env::var("TTS_URL").ok(),
                stt_url: env::var("STT_URL").ok(),
                use_docker: true, // Use Docker for local services
            }
        };

        // Validate required services in CapRover mode
        if config.is_caprover {
            config.validate_caprover_config()?;
        }

        Ok(config)
    }

    /// Validate that required external services are configured in CapRover mode
    fn validate_caprover_config(&self) -> Result<()> {
        let mut missing = Vec::new();

        if self.redis_url.is_none() {
            warn!("REDIS_URL not set in CapRover mode - Redis features will be disabled");
        }

        if self.postgres_url.is_none() {
            // PostgreSQL might be optional if using SQLite
            let db_engine = env::var("DATABASE_ENGINE").unwrap_or_else(|_| "postgres".to_string());
            if db_engine == "postgres" {
                missing.push("DATABASE_URL or POSTGRES_URL");
            }
        }

        if !missing.is_empty() {
            return Err(anyhow::anyhow!(
                "Missing required environment variables for CapRover mode: {}",
                missing.join(", ")
            ));
        }

        Ok(())
    }

    /// Check if Redis should be used (available and configured)
    pub fn should_use_redis(&self) -> bool {
        self.redis_url.is_some()
    }

    /// Check if PostgreSQL should be used
    pub fn should_use_postgres(&self) -> bool {
        self.postgres_url.is_some()
            && env::var("DATABASE_ENGINE").unwrap_or_else(|_| "postgres".to_string()) == "postgres"
    }

    /// Check if Docker services should be managed
    pub fn should_manage_docker(&self) -> bool {
        !self.is_caprover && self.use_docker
    }

    /// Get the Redis connection URL
    pub fn get_redis_url(&self) -> String {
        self.redis_url
            .clone()
            .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string())
    }

    /// Get the PostgreSQL connection URL
    pub fn get_postgres_url(&self) -> Result<String> {
        self.postgres_url
            .clone()
            .context("PostgreSQL URL not configured")
    }
}

/// Global environment configuration instance using lazy_static for thread safety

lazy_static! {
    static ref ENV_CONFIG: EnvironmentConfig = {
        match EnvironmentConfig::from_env() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to initialize environment configuration: {}", e);
                // Return a default configuration on error
                EnvironmentConfig {
                    is_caprover: false,
                    redis_url: None,
                    postgres_url: std::env::var("DATABASE_URL").ok(),
                    tts_url: None,
                    stt_url: None,
                    use_docker: std::env::var("USE_DOCKER")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                }
            }
        }
    };
}

/// Get the environment configuration
pub fn get_env_config() -> &'static EnvironmentConfig {
    &ENV_CONFIG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caprover_detection() {
        // Test with CAPROVER=true
        env::set_var("CAPROVER", "true");
        let config = EnvironmentConfig::from_env().unwrap();
        assert!(config.is_caprover);
        assert!(!config.use_docker);

        // Test with CAPROVER=false
        env::set_var("CAPROVER", "false");
        let config = EnvironmentConfig::from_env().unwrap();
        assert!(!config.is_caprover);
        assert!(config.use_docker);

        // Cleanup
        env::remove_var("CAPROVER");
    }

    #[test]
    fn test_service_urls() {
        env::set_var("REDIS_URL", "redis://external:6379");
        env::set_var("DATABASE_URL", "postgres://user:pass@host/db");

        let config = EnvironmentConfig::from_env().unwrap();
        assert_eq!(config.get_redis_url(), "redis://external:6379");
        assert_eq!(
            config.get_postgres_url().unwrap(),
            "postgres://user:pass@host/db"
        );

        // Cleanup
        env::remove_var("REDIS_URL");
        env::remove_var("DATABASE_URL");
    }
}

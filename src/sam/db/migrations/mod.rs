use anyhow::{Result, Context};
use async_trait::async_trait;
use deadpool_postgres::{Pool, Transaction};
use std::sync::Arc;
use tokio_postgres::Row;
use log::{info, warn, error};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use sha2::{Sha256, Digest};

#[async_trait]
pub trait Migration: Send + Sync {
    fn version(&self) -> i64;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn up(&self, tx: &Transaction<'_>) -> Result<()>;
    async fn down(&self, tx: &Transaction<'_>) -> Result<()>;
    
    fn checksum(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.version().to_string());
        hasher.update(self.name());
        hasher.update(self.description());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone)]
pub struct AppliedMigration {
    pub version: i64,
    pub name: String,
    pub checksum: String,
    pub applied_at: DateTime<Utc>,
    pub execution_time_ms: i64,
}

pub struct MigrationRunner {
    pool: Arc<Pool>,
    migrations: Vec<Box<dyn Migration>>,
    dry_run: bool,
    auto_backup: bool,
}

impl MigrationRunner {
    pub fn new(pool: Arc<Pool>) -> Self {
        Self {
            pool,
            migrations: Vec::new(),
            dry_run: false,
            auto_backup: true,
        }
    }
    
    pub fn with_migrations(mut self, migrations: Vec<Box<dyn Migration>>) -> Self {
        self.migrations = migrations;
        self.migrations.sort_by_key(|m| m.version());
        self
    }
    
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }
    
    pub fn auto_backup(mut self, enabled: bool) -> Self {
        self.auto_backup = enabled;
        self
    }
    
    pub async fn run(&self) -> Result<()> {
        info!("Starting migration runner (dry_run: {}, auto_backup: {})", 
              self.dry_run, self.auto_backup);
        
        self.ensure_migration_table().await?;
        let applied = self.get_applied_migrations().await?;
        let applied_versions: HashSet<i64> = applied.iter()
            .map(|m| m.version)
            .collect();
        
        let pending_migrations: Vec<&Box<dyn Migration>> = self.migrations.iter()
            .filter(|m| !applied_versions.contains(&m.version()))
            .collect();
        
        if pending_migrations.is_empty() {
            info!("No pending migrations to apply");
            return Ok(());
        }
        
        info!("Found {} pending migration(s)", pending_migrations.len());
        
        if self.auto_backup && !self.dry_run {
            self.create_backup().await?;
        }
        
        for migration in pending_migrations {
            self.apply_migration(migration.as_ref()).await?;
        }
        
        Ok(())
    }
    
    pub async fn rollback(&self, target_version: Option<i64>) -> Result<()> {
        info!("Starting migration rollback (target: {:?})", target_version);
        
        let applied = self.get_applied_migrations().await?;
        if applied.is_empty() {
            info!("No migrations to rollback");
            return Ok(());
        }
        
        let target = target_version.unwrap_or_else(|| {
            applied.iter().map(|m| m.version).max().unwrap_or(0) - 1
        });
        
        if self.auto_backup && !self.dry_run {
            self.create_backup().await?;
        }
        
        let migrations_to_rollback: Vec<AppliedMigration> = applied.into_iter()
            .filter(|m| m.version > target)
            .collect();
        
        for applied_migration in migrations_to_rollback.iter().rev() {
            if let Some(migration) = self.migrations.iter()
                .find(|m| m.version() == applied_migration.version) {
                self.rollback_migration(migration.as_ref()).await?;
            } else {
                warn!("Migration {} not found in codebase, skipping rollback", 
                      applied_migration.version);
            }
        }
        
        Ok(())
    }
    
    pub async fn status(&self) -> Result<MigrationStatus> {
        let applied = self.get_applied_migrations().await?;
        let applied_versions: HashSet<i64> = applied.iter()
            .map(|m| m.version)
            .collect();
        
        let mut status = MigrationStatus {
            applied: Vec::new(),
            pending: Vec::new(),
            conflicts: Vec::new(),
        };
        
        for migration in &applied {
            status.applied.push(MigrationInfo {
                version: migration.version,
                name: migration.name.clone(),
                checksum: migration.checksum.clone(),
                applied_at: Some(migration.applied_at),
            });
        }
        
        for migration in &self.migrations {
            if !applied_versions.contains(&migration.version()) {
                status.pending.push(MigrationInfo {
                    version: migration.version(),
                    name: migration.name().to_string(),
                    checksum: migration.checksum(),
                    applied_at: None,
                });
            } else {
                let applied_migration = applied.iter()
                    .find(|m| m.version == migration.version())
                    .unwrap();
                
                if applied_migration.checksum != migration.checksum() {
                    status.conflicts.push(MigrationConflict {
                        version: migration.version(),
                        name: migration.name().to_string(),
                        expected_checksum: migration.checksum(),
                        actual_checksum: applied_migration.checksum.clone(),
                    });
                }
            }
        }
        
        Ok(status)
    }
    
    async fn ensure_migration_table(&self) -> Result<()> {
        let client = self.pool.get().await?;
        
        let query = r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version BIGINT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                checksum VARCHAR(64) NOT NULL,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                execution_time_ms BIGINT NOT NULL DEFAULT 0,
                rolled_back_at TIMESTAMPTZ
            );
            
            CREATE INDEX IF NOT EXISTS idx_migrations_applied_at 
            ON schema_migrations(applied_at);
            
            CREATE INDEX IF NOT EXISTS idx_migrations_checksum 
            ON schema_migrations(checksum);
        "#;
        
        client.batch_execute(query).await
            .context("Failed to create migration table")?;
        
        info!("Migration table ensured");
        Ok(())
    }
    
    async fn get_applied_migrations(&self) -> Result<Vec<AppliedMigration>> {
        let client = self.pool.get().await?;
        
        let rows = client.query(
            "SELECT version, name, checksum, applied_at, execution_time_ms 
             FROM schema_migrations 
             WHERE rolled_back_at IS NULL
             ORDER BY version ASC",
            &[]
        ).await?;
        
        let mut migrations = Vec::new();
        for row in rows {
            migrations.push(AppliedMigration {
                version: row.get(0),
                name: row.get(1),
                checksum: row.get(2),
                applied_at: row.get(3),
                execution_time_ms: row.get(4),
            });
        }
        
        Ok(migrations)
    }
    
    async fn apply_migration(&self, migration: &dyn Migration) -> Result<()> {
        info!("Applying migration {}: {}", migration.version(), migration.name());
        
        if self.dry_run {
            info!("[DRY RUN] Would apply migration {}", migration.version());
            return Ok(());
        }
        
        let start = std::time::Instant::now();
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        
        migration.up(&tx).await
            .context(format!("Failed to apply migration {}", migration.version()))?;
        
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        tx.execute(
            "INSERT INTO schema_migrations (version, name, checksum, execution_time_ms) 
             VALUES ($1, $2, $3, $4)",
            &[&migration.version(), &migration.name(), &migration.checksum(), &execution_time_ms]
        ).await?;
        
        tx.commit().await?;
        
        info!("Migration {} applied successfully in {}ms", 
              migration.version(), execution_time_ms);
        Ok(())
    }
    
    async fn rollback_migration(&self, migration: &dyn Migration) -> Result<()> {
        info!("Rolling back migration {}: {}", migration.version(), migration.name());
        
        if self.dry_run {
            info!("[DRY RUN] Would rollback migration {}", migration.version());
            return Ok(());
        }
        
        let start = std::time::Instant::now();
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        
        migration.down(&tx).await
            .context(format!("Failed to rollback migration {}", migration.version()))?;
        
        tx.execute(
            "UPDATE schema_migrations 
             SET rolled_back_at = NOW() 
             WHERE version = $1",
            &[&migration.version()]
        ).await?;
        
        tx.commit().await?;
        
        info!("Migration {} rolled back successfully in {}ms", 
              migration.version(), start.elapsed().as_millis());
        Ok(())
    }
    
    async fn create_backup(&self) -> Result<()> {
        info!("Creating database backup before migration");
        
        let backup_name = format!("migration_backup_{}", 
                                  chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        
        let client = self.pool.get().await?;
        
        client.execute(
            "INSERT INTO backups (name, type, status, started_at) 
             VALUES ($1, 'migration', 'in_progress', NOW())",
            &[&backup_name]
        ).await?;
        
        info!("Backup {} initiated", backup_name);
        Ok(())
    }
    
    pub async fn validate_checksums(&self) -> Result<Vec<MigrationConflict>> {
        let applied = self.get_applied_migrations().await?;
        let mut conflicts = Vec::new();
        
        for applied_migration in applied {
            if let Some(migration) = self.migrations.iter()
                .find(|m| m.version() == applied_migration.version) {
                if applied_migration.checksum != migration.checksum() {
                    conflicts.push(MigrationConflict {
                        version: applied_migration.version,
                        name: applied_migration.name.clone(),
                        expected_checksum: migration.checksum(),
                        actual_checksum: applied_migration.checksum,
                    });
                }
            }
        }
        
        if !conflicts.is_empty() {
            error!("Found {} migration checksum conflict(s)", conflicts.len());
        }
        
        Ok(conflicts)
    }
}

#[derive(Debug)]
pub struct MigrationStatus {
    pub applied: Vec<MigrationInfo>,
    pub pending: Vec<MigrationInfo>,
    pub conflicts: Vec<MigrationConflict>,
}

#[derive(Debug)]
pub struct MigrationInfo {
    pub version: i64,
    pub name: String,
    pub checksum: String,
    pub applied_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct MigrationConflict {
    pub version: i64,
    pub name: String,
    pub expected_checksum: String,
    pub actual_checksum: String,
}

pub fn load_migrations() -> Vec<Box<dyn Migration>> {
    let mut migrations: Vec<Box<dyn Migration>> = Vec::new();
    
    migrations.push(Box::new(v001_initial_schema::Migration));
    migrations.push(Box::new(v002_add_indexes::Migration));
    migrations.push(Box::new(v003_add_security_tables::Migration));
    
    migrations.sort_by_key(|m| m.version());
    migrations
}

pub mod v001_initial_schema;
pub mod v002_add_indexes;
pub mod v003_add_security_tables;

#[cfg(test)]
mod tests;
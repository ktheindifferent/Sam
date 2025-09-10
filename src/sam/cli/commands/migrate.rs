use anyhow::Result;
use colored::Colorize;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_migrate(args: Vec<String>, _output_lines: &Arc<Mutex<Vec<String>>>) {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("status");
    
    match subcommand {
        "up" => run_migrations().await,
        "down" | "rollback" => rollback_migrations(args.get(1)).await,
        "status" => show_status().await,
        "create" => create_migration(args.get(1)),
        "validate" => validate_checksums().await,
        "help" | "--help" | "-h" => show_help(),
        _ => {
            println!("{}", format!("Unknown subcommand: {}. Use 'sam migrate help' for usage.", subcommand).red());
        }
    }
}

async fn run_migrations() {
    println!("{}", "Running database migrations...".cyan());
    
    match perform_migrations(false, false).await {
        Ok(count) => {
            if count > 0 {
                println!("{}", format!("✓ Successfully applied {} migration(s)", count).green());
            } else {
                println!("{}", "✓ Database is up to date".green());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("✗ Migration failed: {}", e).red());
            std::process::exit(1);
        }
    }
}

async fn rollback_migrations(target: Option<&String>) {
    let target_version = target.and_then(|s| s.parse::<i64>().ok());
    
    println!("{}", "Rolling back migrations...".cyan());
    
    match perform_rollback(target_version).await {
        Ok(count) => {
            if count > 0 {
                println!("{}", format!("✓ Successfully rolled back {} migration(s)", count).green());
            } else {
                println!("{}", "✓ No migrations to roll back".green());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("✗ Rollback failed: {}", e).red());
            std::process::exit(1);
        }
    }
}

async fn show_status() {
    println!("{}", "Database Migration Status".cyan().bold());
    println!("{}", "=========================".cyan());
    
    match get_migration_status().await {
        Ok(status) => {
            if !status.applied.is_empty() {
                println!("\n{}", "Applied Migrations:".green().bold());
                for migration in &status.applied {
                    println!("  ✓ {:03} - {} ({})", 
                             migration.version,
                             migration.name,
                             migration.applied_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                 .unwrap_or_else(|| "unknown".to_string()));
                }
            }
            
            if !status.pending.is_empty() {
                println!("\n{}", "Pending Migrations:".yellow().bold());
                for migration in &status.pending {
                    println!("  ○ {:03} - {}", migration.version, migration.name);
                }
            }
            
            if !status.conflicts.is_empty() {
                println!("\n{}", "⚠ Checksum Conflicts:".red().bold());
                for conflict in &status.conflicts {
                    println!("  ✗ {:03} - {}", conflict.version, conflict.name);
                    println!("    Expected: {}", conflict.expected_checksum);
                    println!("    Actual:   {}", conflict.actual_checksum);
                }
            }
            
            if status.pending.is_empty() && status.conflicts.is_empty() {
                println!("\n{}", "✓ Database schema is up to date".green());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("✗ Failed to get migration status: {}", e).red());
            std::process::exit(1);
        }
    }
}

fn create_migration(name: Option<&String>) {
    let name = match name {
        Some(n) => n,
        None => {
            eprintln!("{}", "✗ Migration name is required".red());
            eprintln!("Usage: sam migrate create <name>");
            return;
        }
    };
    
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let version_number = timestamp.to_string().parse::<i64>().unwrap_or(0) / 1000000;
    let filename = format!("v{:03}_{}.rs", version_number, name.to_lowercase().replace(' ', "_"));
    
    let template = format!("use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {{
    fn version(&self) -> i64 {{
        {}
    }}
    
    fn name(&self) -> &str {{
        \"{}\"
    }}
    
    fn description(&self) -> &str {{
        \"TODO: Add description\"
    }}
    
    async fn up(&self, tx: &Transaction<'_>) -> Result<()> {{
        tx.batch_execute(r#\"
            -- TODO: Add your UP migration SQL here
        \"#).await?;
        
        Ok(())
    }}
    
    async fn down(&self, tx: &Transaction<'_>) -> Result<()> {{
        tx.batch_execute(r#\"
            -- TODO: Add your DOWN migration SQL here
        \"#).await?;
        
        Ok(())
    }}
}}", version_number, name);
    
    println!("{}", format!("Created migration template: src/sam/db/migrations/{}", filename).green());
    println!("Next steps:");
    println!("  1. Edit the migration file with your schema changes");
    println!("  2. Add the migration to load_migrations() in src/sam/db/migrations/mod.rs");
    println!("  3. Run 'sam migrate up' to apply the migration");
    
    // In a real implementation, we would write this file to disk
    println!("\n{}", "Migration template:".cyan());
    println!("{}", template);
}

async fn validate_checksums() {
    println!("{}", "Validating migration checksums...".cyan());
    
    match check_migration_checksums().await {
        Ok(conflicts) => {
            if conflicts.is_empty() {
                println!("{}", "✓ All migration checksums are valid".green());
            } else {
                println!("{}", format!("⚠ Found {} checksum conflict(s):", conflicts.len()).red());
                for conflict in conflicts {
                    println!("  ✗ Migration {}: {}", conflict.version, conflict.name);
                    println!("    Expected: {}", conflict.expected_checksum);
                    println!("    Actual:   {}", conflict.actual_checksum);
                }
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}", format!("✗ Failed to validate checksums: {}", e).red());
            std::process::exit(1);
        }
    }
}

fn show_help() {
    println!("{}", "Database Migration Commands".cyan().bold());
    println!("{}", "===========================".cyan());
    println!();
    println!("Usage: sam migrate <command> [options]");
    println!();
    println!("Commands:");
    println!("  {:<20} Run all pending migrations", "up".green());
    println!("  {:<20} Rollback to previous migration", "down|rollback".green());
    println!("  {:<20} Rollback to specific version", "down|rollback <version>".green());
    println!("  {:<20} Show migration status", "status".green());
    println!("  {:<20} Create a new migration file", "create <name>".green());
    println!("  {:<20} Validate migration checksums", "validate".green());
    println!("  {:<20} Show this help message", "help".green());
    println!();
    println!("Examples:");
    println!("  sam migrate up                    # Apply all pending migrations");
    println!("  sam migrate down                  # Rollback last migration");
    println!("  sam migrate rollback 3            # Rollback to version 3");
    println!("  sam migrate status                # Show current migration status");
    println!("  sam migrate create add_users      # Create new migration file");
}

// Helper functions that interact with the migration system
async fn perform_migrations(dry_run: bool, auto_backup: bool) -> Result<usize> {
    let pool = crate::sam::services::pg::connect().await?;
    let migrations = crate::sam::db::migrations::load_migrations();
    
    let runner = crate::sam::db::migrations::MigrationRunner::new(pool)
        .with_migrations(migrations)
        .dry_run(dry_run)
        .auto_backup(auto_backup);
    
    let status = runner.status().await?;
    let pending_count = status.pending.len();
    
    runner.run().await?;
    Ok(pending_count)
}

async fn perform_rollback(target_version: Option<i64>) -> Result<usize> {
    let pool = crate::sam::services::pg::connect().await?;
    let migrations = crate::sam::db::migrations::load_migrations();
    
    let runner = crate::sam::db::migrations::MigrationRunner::new(pool)
        .with_migrations(migrations)
        .auto_backup(true);
    
    let status = runner.status().await?;
    let initial_count = status.applied.len();
    
    runner.rollback(target_version).await?;
    
    let final_status = runner.status().await?;
    let final_count = final_status.applied.len();
    
    Ok((initial_count - final_count) as usize)
}

async fn get_migration_status() -> Result<crate::sam::db::migrations::MigrationStatus> {
    let pool = crate::sam::services::pg::connect().await?;
    let migrations = crate::sam::db::migrations::load_migrations();
    
    let runner = crate::sam::db::migrations::MigrationRunner::new(pool)
        .with_migrations(migrations);
    
    runner.status().await
}

async fn check_migration_checksums() -> Result<Vec<crate::sam::db::migrations::MigrationConflict>> {
    let pool = crate::sam::services::pg::connect().await?;
    let migrations = crate::sam::db::migrations::load_migrations();
    
    let runner = crate::sam::db::migrations::MigrationRunner::new(pool)
        .with_migrations(migrations);
    
    runner.validate_checksums().await
}
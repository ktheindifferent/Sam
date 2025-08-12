# Database Migration System

## Overview

A comprehensive database migration system has been implemented to manage schema versioning and evolution. This replaces the previous hardcoded schema approach with a robust, versioned migration framework.

## Features

### Core Capabilities
- **Version Control**: Each migration has a unique version number for proper ordering
- **Bidirectional Migrations**: Support for both `up` (apply) and `down` (rollback) operations
- **Checksum Validation**: SHA-256 checksums ensure migration integrity
- **Transaction Safety**: All migrations run within database transactions
- **Dry Run Mode**: Test migrations without applying changes
- **Auto-backup**: Optional automatic backups before migrations

### Safety Features
1. **Checksum Detection**: Detects if applied migrations have been modified
2. **Atomic Operations**: Migrations either fully succeed or fully rollback
3. **Migration History**: Complete audit trail of applied migrations
4. **Conflict Detection**: Identifies checksum mismatches between code and database

## Usage

### CLI Commands

```bash
# Apply all pending migrations
sam migrate up

# Rollback the last migration
sam migrate down

# Rollback to a specific version
sam migrate rollback 2

# Check migration status
sam migrate status

# Create a new migration template
sam migrate create add_user_table

# Validate migration checksums
sam migrate validate
```

### Migration Status Output

The `sam migrate status` command provides detailed information:
- **Applied Migrations**: Shows version, name, and timestamp
- **Pending Migrations**: Lists migrations waiting to be applied
- **Checksum Conflicts**: Highlights any modified migrations

## Architecture

### File Structure

```
src/sam/db/migrations/
├── mod.rs                      # Core migration framework
├── v001_initial_schema.rs      # Initial database schema
├── v002_add_indexes.rs         # Performance indexes
├── v003_add_security_tables.rs # Security audit tables
└── tests.rs                    # Migration system tests
```

### Key Components

1. **Migration Trait**: Defines the interface for all migrations
   - `version()`: Unique version identifier
   - `name()`: Human-readable name
   - `description()`: Migration purpose
   - `up()`: Apply migration
   - `down()`: Rollback migration
   - `checksum()`: SHA-256 hash for integrity

2. **MigrationRunner**: Orchestrates migration execution
   - Manages migration ordering
   - Handles transactions
   - Tracks applied migrations
   - Provides dry-run capability

3. **Migration Table**: `schema_migrations` table tracks:
   - Version number
   - Migration name
   - Checksum
   - Applied timestamp
   - Execution time
   - Rollback status

## Creating New Migrations

### Step 1: Generate Migration File

```bash
sam migrate create your_migration_name
```

This creates a template with:
- Auto-generated version number (timestamp-based)
- Boilerplate structure
- TODO placeholders for SQL

### Step 2: Implement Migration

```rust
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
    fn version(&self) -> i64 {
        4  // Unique version number
    }
    
    fn name(&self) -> &str {
        "add_user_roles"
    }
    
    fn description(&self) -> &str {
        "Add user roles and permissions tables"
    }
    
    async fn up(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(r#"
            CREATE TABLE user_roles (
                id SERIAL PRIMARY KEY,
                name VARCHAR(50) UNIQUE NOT NULL,
                permissions JSONB
            );
            
            CREATE INDEX idx_user_roles_name ON user_roles(name);
        "#).await?;
        Ok(())
    }
    
    async fn down(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(r#"
            DROP TABLE IF EXISTS user_roles CASCADE;
        "#).await?;
        Ok(())
    }
}
```

### Step 3: Register Migration

Add to `load_migrations()` in `src/sam/db/migrations/mod.rs`:

```rust
pub fn load_migrations() -> Vec<Box<dyn Migration>> {
    let mut migrations: Vec<Box<dyn Migration>> = Vec::new();
    
    migrations.push(Box::new(v001_initial_schema::Migration));
    migrations.push(Box::new(v002_add_indexes::Migration));
    migrations.push(Box::new(v003_add_security_tables::Migration));
    migrations.push(Box::new(v004_add_user_roles::Migration)); // New migration
    
    migrations.sort_by_key(|m| m.version());
    migrations
}
```

### Step 4: Apply Migration

```bash
sam migrate up
```

## Best Practices

### Migration Guidelines

1. **Keep Migrations Small**: Each migration should do one thing
2. **Make Migrations Idempotent**: Use `IF NOT EXISTS` and `IF EXISTS`
3. **Test Rollbacks**: Ensure `down()` properly reverses `up()`
4. **Never Modify Applied Migrations**: Create new migrations for changes
5. **Document Complex Logic**: Add comments for non-obvious SQL

### Version Numbering

- Use sequential integers (1, 2, 3, ...)
- Or timestamp-based versions (20240112153045)
- Keep versions unique and ordered

### Testing

Always test migrations in development:
1. Apply migration: `sam migrate up`
2. Verify schema changes
3. Test rollback: `sam migrate down`
4. Verify rollback worked
5. Re-apply: `sam migrate up`

## Troubleshooting

### Common Issues

1. **Checksum Conflicts**
   - Cause: Migration file modified after application
   - Solution: Create a new migration with the changes

2. **Failed Migration**
   - Cause: SQL error or constraint violation
   - Solution: Fix the migration and retry
   - The transaction ensures no partial changes

3. **Missing Dependencies**
   - Cause: Migration depends on non-existent table/column
   - Solution: Check migration ordering

### Recovery

If a migration fails:
1. The transaction automatically rolls back
2. Fix the issue in the migration code
3. Run `sam migrate up` again

For corrupted migration state:
1. Check `schema_migrations` table
2. Manually correct if necessary
3. Run `sam migrate validate` to verify

## Integration

The migration system integrates with:
- **Database Initialization**: `initialize_schema()` now uses migrations
- **Health Checks**: Validates migration state
- **Backup System**: Can trigger backups before migrations
- **Monitoring**: Tracks migration metrics

## Migration History

### Current Migrations

1. **v001_initial_schema**: Core tables for crawling, files, backups, sessions
2. **v002_add_indexes**: Performance indexes for all tables
3. **v003_add_security_tables**: Security audit, rate limiting, API keys

## Future Enhancements

Potential improvements:
- Migration squashing for old migrations
- Parallel migration execution (where safe)
- Migration dependency graph
- Automatic migration generation from models
- Migration performance profiling
- Schema documentation generation
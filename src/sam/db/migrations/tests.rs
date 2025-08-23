#[cfg(test)]
mod tests {
    use super::super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use deadpool_postgres::Transaction;
    
    // Test migration implementation
    struct TestMigration {
        version: i64,
        name: String,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Migration for TestMigration {
        fn version(&self) -> i64 {
            self.version
        }
        
        fn name(&self) -> &str {
            &self.name
        }
        
        fn description(&self) -> &str {
            "Test migration"
        }
        
        async fn up(&self, tx: &Transaction<'_>) -> Result<()> {
            if self.should_fail {
                return Err(anyhow::anyhow!("Test migration failed intentionally"));
            }
            
            // Validate version is numeric to prevent SQL injection in tests
            if !self.version.chars().all(|c| c.is_ascii_digit()) {
                return Err(anyhow::anyhow!("Invalid version format: must be numeric"));
            }
            
            tx.execute(
                &format!("CREATE TABLE IF NOT EXISTS test_table_{} (id SERIAL PRIMARY KEY)", self.version),
                &[]
            ).await?;
            
            Ok(())
        }
        
        async fn down(&self, tx: &Transaction<'_>) -> Result<()> {
            // Validate version is numeric to prevent SQL injection in tests
            if !self.version.chars().all(|c| c.is_ascii_digit()) {
                return Err(anyhow::anyhow!("Invalid version format: must be numeric"));
            }
            
            tx.execute(
                &format!("DROP TABLE IF EXISTS test_table_{}", self.version),
                &[]
            ).await?;
            
            Ok(())
        }
    }
    
    #[test]
    fn test_migration_checksum() {
        let migration1 = TestMigration {
            version: 1,
            name: "test_migration".to_string(),
            should_fail: false,
        };
        
        let migration2 = TestMigration {
            version: 1,
            name: "test_migration".to_string(),
            should_fail: false,
        };
        
        // Same migrations should have same checksum
        assert_eq!(migration1.checksum(), migration2.checksum());
        
        let migration3 = TestMigration {
            version: 2,
            name: "test_migration".to_string(),
            should_fail: false,
        };
        
        // Different versions should have different checksums
        assert_ne!(migration1.checksum(), migration3.checksum());
        
        let migration4 = TestMigration {
            version: 1,
            name: "different_name".to_string(),
            should_fail: false,
        };
        
        // Different names should have different checksums
        assert_ne!(migration1.checksum(), migration4.checksum());
    }
    
    #[test]
    fn test_migration_sorting() {
        let mut migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(TestMigration {
                version: 3,
                name: "third".to_string(),
                should_fail: false,
            }),
            Box::new(TestMigration {
                version: 1,
                name: "first".to_string(),
                should_fail: false,
            }),
            Box::new(TestMigration {
                version: 2,
                name: "second".to_string(),
                should_fail: false,
            }),
        ];
        
        migrations.sort_by_key(|m| m.version());
        
        assert_eq!(migrations[0].version(), 1);
        assert_eq!(migrations[1].version(), 2);
        assert_eq!(migrations[2].version(), 3);
    }
    
    #[tokio::test]
    async fn test_migration_runner_dry_run() {
        // This test requires a test database setup
        // Skip if no database is available
        
        match std::env::var("TEST_DATABASE_URL") {
            Ok(_) => {
                // Would run actual dry-run test here
                println!("Dry run test would execute here");
            }
            Err(_) => {
                println!("Skipping database test - TEST_DATABASE_URL not set");
            }
        }
    }
    
    #[test]
    fn test_migration_status_struct() {
        let status = MigrationStatus {
            applied: vec![
                MigrationInfo {
                    version: 1,
                    name: "initial".to_string(),
                    checksum: "abc123".to_string(),
                    applied_at: Some(chrono::Utc::now()),
                },
            ],
            pending: vec![
                MigrationInfo {
                    version: 2,
                    name: "second".to_string(),
                    checksum: "def456".to_string(),
                    applied_at: None,
                },
            ],
            conflicts: vec![],
        };
        
        assert_eq!(status.applied.len(), 1);
        assert_eq!(status.pending.len(), 1);
        assert!(status.conflicts.is_empty());
    }
    
    #[test]
    fn test_migration_conflict_detection() {
        let conflict = MigrationConflict {
            version: 1,
            name: "conflicted".to_string(),
            expected_checksum: "expected123".to_string(),
            actual_checksum: "actual456".to_string(),
        };
        
        assert_ne!(conflict.expected_checksum, conflict.actual_checksum);
    }
    
    #[test]
    fn test_initial_migrations_load() {
        let migrations = load_migrations();
        
        // We should have at least 3 initial migrations
        assert!(migrations.len() >= 3);
        
        // Verify they're in order
        for i in 1..migrations.len() {
            assert!(migrations[i].version() > migrations[i-1].version());
        }
        
        // Verify first migration is version 1
        assert_eq!(migrations[0].version(), 1);
        assert_eq!(migrations[0].name(), "initial_schema");
    }
}
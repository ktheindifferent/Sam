use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
    fn version(&self) -> i64 {
        1
    }

    fn name(&self) -> &str {
        "initial_schema"
    }

    fn description(&self) -> &str {
        "Create initial database schema with core tables"
    }

    async fn up(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            -- Crawler tables
            CREATE TABLE IF NOT EXISTS crawl_jobs (
                id SERIAL PRIMARY KEY,
                url TEXT NOT NULL,
                max_depth INTEGER DEFAULT 2,
                status TEXT DEFAULT 'pending',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS crawl_pages (
                id SERIAL PRIMARY KEY,
                job_id INTEGER REFERENCES crawl_jobs(id) ON DELETE CASCADE,
                url TEXT NOT NULL,
                title TEXT,
                content TEXT,
                status_code INTEGER,
                error TEXT,
                crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(job_id, url)
            );
            
            -- File storage tables
            CREATE TABLE IF NOT EXISTS files (
                id SERIAL PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                size BIGINT NOT NULL,
                mime_type TEXT,
                checksum TEXT,
                metadata JSONB,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS file_versions (
                id SERIAL PRIMARY KEY,
                file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                version_number INTEGER NOT NULL,
                size BIGINT NOT NULL,
                checksum TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Backup tables
            CREATE TABLE IF NOT EXISTS backups (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                size BIGINT,
                path TEXT,
                error TEXT,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Session tables
            CREATE TABLE IF NOT EXISTS user_sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                csrf_token TEXT NOT NULL,
                data JSONB,
                expires_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Service health tables
            CREATE TABLE IF NOT EXISTS service_health (
                id SERIAL PRIMARY KEY,
                service_name TEXT NOT NULL,
                status TEXT NOT NULL,
                error_count INTEGER DEFAULT 0,
                restart_count INTEGER DEFAULT 0,
                memory_usage BIGINT,
                cpu_usage REAL,
                custom_metrics JSONB,
                checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            DROP TABLE IF EXISTS service_health CASCADE;
            DROP TABLE IF EXISTS user_sessions CASCADE;
            DROP TABLE IF EXISTS backups CASCADE;
            DROP TABLE IF EXISTS file_versions CASCADE;
            DROP TABLE IF EXISTS files CASCADE;
            DROP TABLE IF EXISTS crawl_pages CASCADE;
            DROP TABLE IF EXISTS crawl_jobs CASCADE;
        "#,
        )
        .await?;

        Ok(())
    }
}

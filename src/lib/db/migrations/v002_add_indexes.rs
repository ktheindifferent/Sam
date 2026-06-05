use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
    fn version(&self) -> i64 {
        2
    }

    fn name(&self) -> &str {
        "add_indexes"
    }

    fn description(&self) -> &str {
        "Add performance indexes to core tables"
    }

    async fn up(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            -- Crawler indexes
            CREATE INDEX IF NOT EXISTS idx_crawl_pages_job_id 
                ON crawl_pages(job_id);
            CREATE INDEX IF NOT EXISTS idx_crawl_pages_url 
                ON crawl_pages(url);
            CREATE INDEX IF NOT EXISTS idx_crawl_jobs_status 
                ON crawl_jobs(status);
            CREATE INDEX IF NOT EXISTS idx_crawl_jobs_created_at 
                ON crawl_jobs(created_at DESC);
            
            -- File storage indexes
            CREATE INDEX IF NOT EXISTS idx_files_path 
                ON files(path);
            CREATE INDEX IF NOT EXISTS idx_files_checksum 
                ON files(checksum);
            CREATE INDEX IF NOT EXISTS idx_files_mime_type 
                ON files(mime_type);
            CREATE INDEX IF NOT EXISTS idx_file_versions_file_id 
                ON file_versions(file_id);
            
            -- Backup indexes
            CREATE INDEX IF NOT EXISTS idx_backups_status 
                ON backups(status);
            CREATE INDEX IF NOT EXISTS idx_backups_type 
                ON backups(type);
            CREATE INDEX IF NOT EXISTS idx_backups_created_at 
                ON backups(created_at DESC);
            
            -- Session indexes
            CREATE INDEX IF NOT EXISTS idx_sessions_expires 
                ON user_sessions(expires_at);
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id 
                ON user_sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_csrf_token 
                ON user_sessions(csrf_token);
            
            -- Service health indexes
            CREATE INDEX IF NOT EXISTS idx_service_health_name 
                ON service_health(service_name);
            CREATE INDEX IF NOT EXISTS idx_service_health_status 
                ON service_health(status);
            CREATE INDEX IF NOT EXISTS idx_service_health_checked_at 
                ON service_health(checked_at DESC);
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            -- Drop all indexes created in up()
            DROP INDEX IF EXISTS idx_crawl_pages_job_id;
            DROP INDEX IF EXISTS idx_crawl_pages_url;
            DROP INDEX IF EXISTS idx_crawl_jobs_status;
            DROP INDEX IF EXISTS idx_crawl_jobs_created_at;
            
            DROP INDEX IF EXISTS idx_files_path;
            DROP INDEX IF EXISTS idx_files_checksum;
            DROP INDEX IF EXISTS idx_files_mime_type;
            DROP INDEX IF EXISTS idx_file_versions_file_id;
            
            DROP INDEX IF EXISTS idx_backups_status;
            DROP INDEX IF EXISTS idx_backups_type;
            DROP INDEX IF EXISTS idx_backups_created_at;
            
            DROP INDEX IF EXISTS idx_sessions_expires;
            DROP INDEX IF EXISTS idx_sessions_user_id;
            DROP INDEX IF EXISTS idx_sessions_csrf_token;
            
            DROP INDEX IF EXISTS idx_service_health_name;
            DROP INDEX IF EXISTS idx_service_health_status;
            DROP INDEX IF EXISTS idx_service_health_checked_at;
        "#,
        )
        .await?;

        Ok(())
    }
}

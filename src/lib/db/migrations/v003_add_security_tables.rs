use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
    fn version(&self) -> i64 {
        3
    }

    fn name(&self) -> &str {
        "add_security_tables"
    }

    fn description(&self) -> &str {
        "Add security audit and rate limiting tables"
    }

    async fn up(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            -- Security audit log table
            CREATE TABLE IF NOT EXISTS security_audit_log (
                id SERIAL PRIMARY KEY,
                event_type TEXT NOT NULL,
                user_id TEXT,
                ip_address INET,
                user_agent TEXT,
                resource TEXT,
                action TEXT NOT NULL,
                result TEXT NOT NULL,
                details JSONB,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Rate limiting table
            CREATE TABLE IF NOT EXISTS rate_limit_records (
                id SERIAL PRIMARY KEY,
                identifier TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                request_count INTEGER DEFAULT 1,
                window_start TIMESTAMP NOT NULL,
                window_end TIMESTAMP NOT NULL,
                blocked BOOLEAN DEFAULT FALSE,
                UNIQUE(identifier, endpoint, window_start)
            );
            
            -- Failed login attempts
            CREATE TABLE IF NOT EXISTS failed_login_attempts (
                id SERIAL PRIMARY KEY,
                username TEXT,
                ip_address INET NOT NULL,
                user_agent TEXT,
                attempt_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                error_message TEXT
            );
            
            -- API keys table
            CREATE TABLE IF NOT EXISTS api_keys (
                id SERIAL PRIMARY KEY,
                key_hash TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                user_id TEXT,
                permissions JSONB,
                last_used_at TIMESTAMP,
                expires_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                revoked_at TIMESTAMP
            );
            
            -- Add indexes for security tables
            CREATE INDEX IF NOT EXISTS idx_audit_log_event_type 
                ON security_audit_log(event_type);
            CREATE INDEX IF NOT EXISTS idx_audit_log_user_id 
                ON security_audit_log(user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_log_created_at 
                ON security_audit_log(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_log_ip_address 
                ON security_audit_log(ip_address);
            
            CREATE INDEX IF NOT EXISTS idx_rate_limit_identifier 
                ON rate_limit_records(identifier);
            CREATE INDEX IF NOT EXISTS idx_rate_limit_window 
                ON rate_limit_records(window_start, window_end);
            
            CREATE INDEX IF NOT EXISTS idx_failed_login_ip 
                ON failed_login_attempts(ip_address);
            CREATE INDEX IF NOT EXISTS idx_failed_login_time 
                ON failed_login_attempts(attempt_time DESC);
            
            CREATE INDEX IF NOT EXISTS idx_api_keys_hash 
                ON api_keys(key_hash);
            CREATE INDEX IF NOT EXISTS idx_api_keys_expires 
                ON api_keys(expires_at);
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, tx: &Transaction<'_>) -> Result<()> {
        tx.batch_execute(
            r#"
            DROP TABLE IF EXISTS api_keys CASCADE;
            DROP TABLE IF EXISTS failed_login_attempts CASCADE;
            DROP TABLE IF EXISTS rate_limit_records CASCADE;
            DROP TABLE IF EXISTS security_audit_log CASCADE;
        "#,
        )
        .await?;

        Ok(())
    }
}

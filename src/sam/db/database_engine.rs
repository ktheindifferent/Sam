use anyhow::{Result, Context};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum DatabaseEngine {
    #[default]
    SQLite,
    PostgreSQL,
    MySQL,
    MariaDB,
}


impl DatabaseEngine {
    pub fn from_env() -> Self {
        match std::env::var("DATABASE_ENGINE").as_deref() {
            Ok("postgresql") | Ok("postgres") => DatabaseEngine::PostgreSQL,
            Ok("mysql") => DatabaseEngine::MySQL,
            Ok("mariadb") => DatabaseEngine::MariaDB,
            Ok("sqlite") | _ => DatabaseEngine::SQLite,
        }
    }
    
    pub fn connection_string(&self) -> String {
        match self {
            DatabaseEngine::SQLite => {
                let path = std::env::var("SQLITE_PATH")
                    .unwrap_or_else(|_| "./data/sam.db".to_string());
                format!("sqlite://{}", path)
            }
            DatabaseEngine::PostgreSQL => {
                let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
                let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
                let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "sam".to_string());
                let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
                let pass = std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "sampassword".to_string());
                format!("postgresql://{}:{}@{}:{}/{}", user, pass, host, port, db)
            }
            DatabaseEngine::MySQL | DatabaseEngine::MariaDB => {
                let host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
                let port = std::env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string());
                let db = std::env::var("MYSQL_DB").unwrap_or_else(|_| "sam".to_string());
                let user = std::env::var("MYSQL_USER").unwrap_or_else(|_| "root".to_string());
                let pass = std::env::var("MYSQL_PASSWORD").unwrap_or_else(|_| "sampassword".to_string());
                format!("mysql://{}:{}@{}:{}/{}", user, pass, host, port, db)
            }
        }
    }
}

#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64>;
    async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>>;
    async fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Box<dyn DatabaseTransaction + '_>) -> futures::future::BoxFuture<'_, Result<R>> + Send,
        R: Send;
    async fn health_check(&self) -> Result<()>;
    async fn close(&self) -> Result<()>;
}

#[async_trait]
pub trait DatabaseTransaction: Send + Sync {
    async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64>;
    async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>>;
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Timestamp(chrono::DateTime<chrono::Utc>),
}

#[derive(Debug, Clone)]
pub struct Row {
    columns: Vec<String>,
    values: Vec<Value>,
}

impl Row {
    pub fn new(columns: Vec<String>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }
    
    pub fn get<T: FromValue>(&self, index: usize) -> Result<T> {
        self.values.get(index)
            .ok_or_else(|| anyhow::anyhow!("Column index {} out of bounds", index))
            .and_then(|v| T::from_value(v.clone()))
    }
    
    pub fn get_by_name<T: FromValue>(&self, name: &str) -> Result<T> {
        self.columns.iter()
            .position(|c| c == name)
            .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", name))
            .and_then(|idx| self.get(idx))
    }
    
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
    
    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self>;
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Text(s) => Ok(s),
            Value::Null => Ok(String::new()),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to String", value)),
        }
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Int32(i) => Ok(i),
            Value::Int64(i) => Ok(i as i32),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to i32", value)),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Int64(i) => Ok(i),
            Value::Int32(i) => Ok(i as i64),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to i64", value)),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bool(b) => Ok(b),
            Value::Int32(i) => Ok(i != 0),
            Value::Int64(i) => Ok(i != 0),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to bool", value)),
        }
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Float(f) => Ok(f),
            Value::Double(d) => Ok(d as f32),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to f32", value)),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Double(d) => Ok(d),
            Value::Float(f) => Ok(f as f64),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to f64", value)),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Bytes(b) => Ok(b),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to Vec<u8>", value)),
        }
    }
}

impl FromValue for serde_json::Value {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Json(j) => Ok(j),
            Value::Text(s) => serde_json::from_str(&s).context("Failed to parse JSON from text"),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to JSON", value)),
        }
    }
}

impl FromValue for chrono::DateTime<chrono::Utc> {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Timestamp(t) => Ok(t),
            Value::Text(s) => chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .context("Failed to parse timestamp from text"),
            _ => Err(anyhow::anyhow!("Cannot convert {:?} to DateTime", value)),
        }
    }
}

pub enum DatabaseConnectionWrapper {
    SQLite(SqliteConnection),
    Postgres(PostgresConnection),
}

impl DatabaseConnectionWrapper {
    pub async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64> {
        match self {
            DatabaseConnectionWrapper::SQLite(conn) => conn.execute(query, params).await,
            DatabaseConnectionWrapper::Postgres(conn) => conn.execute(query, params).await,
        }
    }

    pub async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        match self {
            DatabaseConnectionWrapper::SQLite(conn) => conn.query(query, params).await,
            DatabaseConnectionWrapper::Postgres(conn) => conn.query(query, params).await,
        }
    }

    pub async fn health_check(&self) -> Result<()> {
        match self {
            DatabaseConnectionWrapper::SQLite(conn) => conn.health_check().await,
            DatabaseConnectionWrapper::Postgres(conn) => conn.health_check().await,
        }
    }

    pub async fn close(&self) -> Result<()> {
        match self {
            DatabaseConnectionWrapper::SQLite(conn) => conn.close().await,
            DatabaseConnectionWrapper::Postgres(conn) => conn.close().await,
        }
    }
}

pub struct DatabasePool {
    engine: DatabaseEngine,
    connection: Arc<DatabaseConnectionWrapper>,
}

impl DatabasePool {
    pub async fn new(engine: DatabaseEngine) -> Result<Self> {
        let connection = match engine {
            DatabaseEngine::SQLite => {
                let conn = SqliteConnection::new(&engine.connection_string()).await?;
                Arc::new(DatabaseConnectionWrapper::SQLite(conn))
            }
            DatabaseEngine::PostgreSQL => {
                let conn = PostgresConnection::new(&engine.connection_string()).await?;
                Arc::new(DatabaseConnectionWrapper::Postgres(conn))
            }
            _ => {
                return Err(anyhow::anyhow!("Database engine {:?} not yet implemented", engine));
            }
        };
        
        Ok(Self {
            engine,
            connection,
        })
    }
    
    pub fn engine(&self) -> &DatabaseEngine {
        &self.engine
    }
    
    pub fn connection(&self) -> Arc<DatabaseConnectionWrapper> {
        self.connection.clone()
    }
    
    pub async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64> {
        self.connection.execute(query, params).await
    }
    
    pub async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        self.connection.query(query, params).await
    }
    
    pub async fn health_check(&self) -> Result<()> {
        self.connection.health_check().await
    }
}

use rusqlite::Connection as RusqliteConnection;
use tokio::sync::Mutex;

pub struct SqliteConnection {
    conn: Arc<Mutex<RusqliteConnection>>,
}

impl SqliteConnection {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let path = connection_string.strip_prefix("sqlite://")
            .unwrap_or(connection_string);
        
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(".")))?;
        
        let conn = RusqliteConnection::open(path)
            .context("Failed to open SQLite database")?;
        
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 30000000000;"
        ).context("Failed to set SQLite pragmas")?;
        
        info!("SQLite connection established at {}", path);
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl DatabaseConnection for SqliteConnection {
    async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64> {
        let conn = self.conn.lock().await;
        let params = convert_values_to_sqlite(&params);
        let affected = conn.execute(query, rusqlite::params_from_iter(params.iter()))
            .context("Failed to execute SQLite query")?;
        Ok(affected as u64)
    }
    
    async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        let conn = self.conn.lock().await;
        let params = convert_values_to_sqlite(&params);
        let mut stmt = conn.prepare(query)
            .context("Failed to prepare SQLite statement")?;
        
        let column_names: Vec<String> = stmt.column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let mut values = Vec::new();
            for i in 0..column_names.len() {
                values.push(sqlite_value_to_value(row.get_ref(i)?));
            }
            Ok(Row::new(column_names.clone(), values))
        }).context("Failed to query SQLite")?;
        
        let mut result = Vec::new();
        for row in rows {
            result.push(row.context("Failed to read SQLite row")?);
        }
        
        Ok(result)
    }
    
    async fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Box<dyn DatabaseTransaction + '_>) -> futures::future::BoxFuture<'_, Result<R>> + Send,
        R: Send,
    {
        Err(anyhow::anyhow!("SQLite transactions not yet implemented"))
    }
    
    async fn health_check(&self) -> Result<()> {
        self.execute("SELECT 1", vec![]).await?;
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

fn convert_values_to_sqlite(values: &[Value]) -> Vec<rusqlite::types::Value> {
    values.iter().map(|v| match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
        Value::Int32(i) => rusqlite::types::Value::Integer(*i as i64),
        Value::Int64(i) => rusqlite::types::Value::Integer(*i),
        Value::Float(f) => rusqlite::types::Value::Real(*f as f64),
        Value::Double(d) => rusqlite::types::Value::Real(*d),
        Value::Text(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Bytes(b) => rusqlite::types::Value::Blob(b.clone()),
        Value::Json(j) => rusqlite::types::Value::Text(j.to_string()),
        Value::Timestamp(t) => rusqlite::types::Value::Text(t.to_rfc3339()),
    }).collect()
}

fn sqlite_value_to_value(value: rusqlite::types::ValueRef) -> Value {
    use rusqlite::types::ValueRef as V;
    match value {
        V::Null => Value::Null,
        V::Integer(i) => Value::Int64(i),
        V::Real(f) => Value::Double(f),
        V::Text(s) => Value::Text(String::from_utf8_lossy(s).to_string()),
        V::Blob(b) => Value::Bytes(b.to_vec()),
    }
}

use deadpool_postgres::{Pool, Config, Manager, ManagerConfig, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

pub struct PostgresConnection {
    pool: Arc<Pool>,
}

impl PostgresConnection {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let mut cfg = Config::new();
        
        if let Ok(url) = url::Url::parse(connection_string) {
            cfg.host = Some(url.host_str().unwrap_or("localhost").to_string());
            cfg.port = Some(url.port().unwrap_or(5432));
            cfg.dbname = Some(url.path().trim_start_matches('/').to_string());
            cfg.user = Some(url.username().to_string());
            cfg.password = url.password().map(|p| p.to_string());
        }
        
        cfg.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 32,
            timeouts: deadpool_postgres::Timeouts {
                wait: Some(Duration::from_secs(5)),
                create: Some(Duration::from_secs(5)),
                recycle: Some(Duration::from_secs(5)),
            },
            queue_mode: deadpool::managed::QueueMode::Fifo,
        });
        
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        
        // Convert deadpool config to tokio_postgres config
        let mut tokio_cfg = tokio_postgres::Config::new();
        if let Some(host) = &cfg.host {
            tokio_cfg.host(host);
        }
        if let Some(port) = cfg.port {
            tokio_cfg.port(port);
        }
        if let Some(dbname) = &cfg.dbname {
            tokio_cfg.dbname(dbname);
        }
        if let Some(user) = &cfg.user {
            tokio_cfg.user(user);
        }
        if let Some(password) = &cfg.password {
            tokio_cfg.password(password);
        }
        
        let mgr = Manager::from_config(tokio_cfg, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(32)
            .runtime(Runtime::Tokio1)
            .build()
            .context("Failed to create PostgreSQL connection pool")?;
        
        info!("PostgreSQL connection pool created");
        
        Ok(Self {
            pool: Arc::new(pool),
        })
    }
}

#[async_trait]
impl DatabaseConnection for PostgresConnection {
    async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64> {
        let client = self.pool.get().await
            .context("Failed to get PostgreSQL client from pool")?;
        
        let pg_params = convert_values_to_postgres(&params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        
        let affected = client.execute(query, &pg_params_refs).await
            .context("Failed to execute PostgreSQL query")?;
        
        Ok(affected)
    }
    
    async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        let client = self.pool.get().await
            .context("Failed to get PostgreSQL client from pool")?;
        
        let pg_params = convert_values_to_postgres(&params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        
        let rows = client.query(query, &pg_params_refs).await
            .context("Failed to query PostgreSQL")?;
        
        let mut result = Vec::new();
        for row in rows {
            let columns: Vec<String> = row.columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
            
            let mut values = Vec::new();
            for i in 0..columns.len() {
                values.push(postgres_value_to_value(&row, i)?);
            }
            
            result.push(Row::new(columns, values));
        }
        
        Ok(result)
    }
    
    async fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Box<dyn DatabaseTransaction + '_>) -> futures::future::BoxFuture<'_, Result<R>> + Send,
        R: Send,
    {
        let mut client = self.pool.get().await?;
        let transaction = client.transaction().await?;
        f(Box::new(PostgresTransaction { tx: transaction })).await
    }
    
    async fn health_check(&self) -> Result<()> {
        let client = self.pool.get().await
            .context("Failed to get PostgreSQL client for health check")?;
        client.simple_query("SELECT 1").await
            .context("PostgreSQL health check failed")?;
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

struct PostgresTransaction<'a> {
    tx: deadpool_postgres::Transaction<'a>,
}

#[async_trait]
impl DatabaseTransaction for PostgresTransaction<'_> {
    async fn execute(&self, query: &str, params: Vec<Value>) -> Result<u64> {
        let pg_params = convert_values_to_postgres(&params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        
        let affected = self.tx.execute(query, &pg_params_refs).await
            .context("Failed to execute PostgreSQL transaction query")?;
        Ok(affected)
    }
    
    async fn query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        let pg_params = convert_values_to_postgres(&params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        
        let rows = self.tx.query(query, &pg_params_refs).await
            .context("Failed to query PostgreSQL transaction")?;
        
        let mut result = Vec::new();
        for row in rows {
            let columns: Vec<String> = row.columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
            
            let mut values = Vec::new();
            for i in 0..columns.len() {
                values.push(postgres_value_to_value(&row, i)?);
            }
            
            result.push(Row::new(columns, values));
        }
        
        Ok(result)
    }
    
    async fn commit(self: Box<Self>) -> Result<()> {
        self.tx.commit().await.context("Failed to commit PostgreSQL transaction")
    }
    
    async fn rollback(self: Box<Self>) -> Result<()> {
        self.tx.rollback().await.context("Failed to rollback PostgreSQL transaction")
    }
}

fn convert_values_to_postgres(values: &[Value]) -> Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> {
    values.iter().map(|v| {
        let boxed: Box<dyn tokio_postgres::types::ToSql + Sync + Send> = match v {
            Value::Null => Box::new(Option::<i32>::None),
            Value::Bool(b) => Box::new(*b),
            Value::Int32(i) => Box::new(*i),
            Value::Int64(i) => Box::new(*i),
            Value::Float(f) => Box::new(*f),
            Value::Double(d) => Box::new(*d),
            Value::Text(s) => Box::new(s.clone()),
            Value::Bytes(b) => Box::new(b.clone()),
            Value::Json(j) => Box::new(j.clone()),
            Value::Timestamp(t) => Box::new(*t),
        };
        boxed
    }).collect()
}

fn postgres_value_to_value(row: &tokio_postgres::Row, index: usize) -> Result<Value> {
    use tokio_postgres::types::Type;
    
    let column = &row.columns()[index];
    let ty = column.type_();
    
    if row.try_get::<_, Option<i32>>(index)?.is_none() {
        return Ok(Value::Null);
    }
    
    match *ty {
        Type::BOOL => Ok(Value::Bool(row.get(index))),
        Type::INT2 | Type::INT4 => Ok(Value::Int32(row.get(index))),
        Type::INT8 => Ok(Value::Int64(row.get(index))),
        Type::FLOAT4 => Ok(Value::Float(row.get(index))),
        Type::FLOAT8 => Ok(Value::Double(row.get(index))),
        Type::TEXT | Type::VARCHAR | Type::CHAR => Ok(Value::Text(row.get(index))),
        Type::BYTEA => Ok(Value::Bytes(row.get(index))),
        Type::JSON | Type::JSONB => Ok(Value::Json(row.get(index))),
        Type::TIMESTAMP | Type::TIMESTAMPTZ => Ok(Value::Timestamp(row.get(index))),
        _ => Ok(Value::Text(format!("{:?}", row.get::<_, String>(index)))),
    }
}
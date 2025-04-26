// TODO: Pooled DB connection for all the threads :)
// TODO: Ext Crawler
// TODO: Use redis for dns cache if available


use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use crate::sam::memory::{Config, PostgresQueries};
use tokio_postgres::Row;
use reqwest::Url;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
// use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use trust_dns_resolver::TokioAsyncResolver;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, LevelFilter};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use once_cell::sync::{Lazy, OnceCell};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::path::Path;
use tokio::fs;
use serde_json;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use regex;
use url::ParseError;
use redis::{AsyncCommands, aio::MultiplexedConnection, Client as RedisClient};

static REDIS_URL: &str = "redis://127.0.0.1/";
static REDIS_MANAGER: OnceCell<RedisClient> = OnceCell::new();

async fn redis_client() -> redis::RedisResult<MultiplexedConnection> {
    let client = REDIS_MANAGER.get_or_try_init(|| RedisClient::open(REDIS_URL))
        .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Failed to create Redis client", format!("{:?}", e))))?;
    client.get_multiplexed_async_connection().await
}

pub mod job;
pub mod page;
pub mod runner;

pub use job::CrawlJob;
pub use page::CrawledPage;
pub use runner::{
    crawl_url, start_service_async, stop_service, service_status,
};

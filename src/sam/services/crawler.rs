// TODO: Pooled DB connection for all the threads :)
// TODO: Ext Crawler
// TODO: Use redis for dns cache if available

// use tokio::sync::Mutex;

pub mod circuit_breaker;
pub mod job;
pub mod metrics;
pub mod page;
pub mod robots;
pub mod runner;
pub mod sitemap;

pub use job::CrawlJob;
pub use page::CrawledPage;
pub use runner::{crawl_url, service_status, start_service, start_service_async, stop_service};
pub use robots::{is_url_allowed, DEFAULT_USER_AGENT};
pub use sitemap::{extract_urls_from_sitemaps, fetch_sitemap};
pub use circuit_breaker::{is_domain_allowed, record_domain_failure, record_domain_success};
pub use metrics::{get_crawler_metrics, generate_metrics_report, record_crawl_success, record_crawl_failure};

//! Crawler service module.
//!
//! Re-exports crawler job, page, and runner modules and their main types/functions.

pub mod job;
pub mod page;
pub mod runner;

pub use job::CrawlJob;
pub use page::CrawledPage;
pub use runner::{
    crawl_url, start_service, start_service_async, stop_service, service_status,
};

// TODO: Pooled DB connection for all the threads :)
// TODO: Ext Crawler
// TODO: Use redis for dns cache if available


// use tokio::sync::Mutex;


pub mod job;
pub mod page;
pub mod runner;

pub use job::CrawlJob;
pub use page::CrawledPage;
pub use runner::{
    crawl_url, start_service_async, stop_service, service_status,
};

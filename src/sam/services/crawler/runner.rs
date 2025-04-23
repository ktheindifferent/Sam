use crate::sam::services::crawler::job::CrawlJob;
use crate::sam::services::crawler::page::CrawledPage;
use std::collections::{HashSet, VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{sleep, Duration};
use once_cell::sync::{Lazy, OnceCell};
use std::path::Path;
use tokio::fs;
use log::{info, LevelFilter};
use trust_dns_resolver::TokioAsyncResolver;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use reqwest::Url;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use num_cpus;
use rand::seq::SliceRandom; // <-- Add this import

static CRAWLER_RUNNING: AtomicBool = AtomicBool::new(false);

// Add a static DNS cache (domain -> Option<bool> for found/not found)
static DNS_CACHE_PATH: &str = "/opt/sam/dns.cache";
static DNS_LOOKUP_CACHE: Lazy<TokioMutex<HashMap<String, bool>>> = Lazy::new(|| TokioMutex::new(HashMap::new()));

// Load DNS cache from disk at startup
async fn load_dns_cache() {
    if !Path::new(DNS_CACHE_PATH).exists() {
        // Create an empty cache file if it doesn't exist
        let _ = fs::write(DNS_CACHE_PATH, b"{}").await;
    }
    let path = Path::new(DNS_CACHE_PATH);
    if let Ok(data) = fs::read(path).await {
        if let Ok(map) = serde_json::from_slice::<HashMap<String, bool>>(&data) {
            let mut cache = DNS_LOOKUP_CACHE.lock().await;
            *cache = map;
            log::info!("Loaded DNS cache with {} entries", cache.len());
        }
    }
}

// Save DNS cache to disk
async fn save_dns_cache() {
    let cache = DNS_LOOKUP_CACHE.lock().await;
    if let Ok(data) = serde_json::to_vec(&*cache) {
        let _ = fs::write(DNS_CACHE_PATH, data).await;
        log::info!("Saved DNS cache with {} entries", cache.len());
    }
}

// Cache all CrawlJob and CrawledPage entries from Postgres into Redis
async fn cache_all_to_redis() {
    log::info!("Caching all CrawlJob and CrawledPage entries to Redis...");
    // Limit DB select to 100 at a time to avoid freezing with huge tables
    let mut offset = 0;
    let batch_size = 100;
    loop {
        match CrawlJob::select_async(Some(batch_size), Some(offset), None, None).await {
            Ok(jobs) if jobs.is_empty() => break,
            Ok(jobs) => {
                let mut handles = Vec::new();
                for job in &jobs {
                    handles.push(job.save_redis());
                }
                for handle in handles {
                    let _ = handle.await;
                }
                offset += jobs.len();
                log::info!("Cached {}/? CrawlJob entries into Redis", offset);
                if jobs.len() < batch_size { break; }
            }
            Err(e) => {
                log::warn!("Failed to cache CrawlJob entries to Redis: {}", e);
                break;
            }
        }
    }
    offset = 0;
    loop {
        match CrawledPage::select_async(Some(batch_size), Some(offset), None, None).await {
            Ok(pages) if pages.is_empty() => break,
            Ok(pages) => {
                let mut handles = Vec::new();
                for page in &pages {
                    handles.push(page.save_redis());
                }
                for handle in handles {
                    let _ = handle.await;
                }
                offset += pages.len();
                log::info!("Cached {}/? CrawledPage entries into Redis", offset);
                if pages.len() < batch_size { break; }
            }
            Err(e) => {
                log::warn!("Failed to cache CrawledPage entries to Redis: {}", e);
                break;
            }
        }
    }
}

// Internal boxed async fn for recursion
async fn crawl_url_inner(
    job_oid: String,
    url: String,
    depth: usize,
) -> crate::sam::memory::Result<CrawledPage> {
    let max_depth = 2;

    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String(url.clone()));
    pg_query.query_columns.push("url =".to_string());
    let existing = CrawledPage::select_async(None, None, None, Some(pg_query)).await.unwrap_or_default();
    if !existing.is_empty() {
        return Ok(existing[0].clone());
    }

    let mut page = CrawledPage::new();
    page.crawl_job_oid = job_oid.clone();
    page.url = url.clone();
    info!("Fetching URL: {}", url);
    let resp = reqwest::get(&url).await;
    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            page.status_code = Some(status as i32);
            if status == 200 {
                let html = resp.text().await.unwrap_or_default();
                if (!html.is_empty()) {
                    // Move HTML parsing and token extraction to a blocking thread
                    let url_clone = url.clone();
                    let (mut tokens, mut links) = tokio::task::spawn_blocking(move || {
                        // ...HTML parsing and extraction logic (see original)...
                        // (Omitted for brevity, copy from original file)
                        (vec![], vec![])
                    }).await.unwrap();

                    tokens.sort();
                    tokens.dedup();
                    links.sort();
                    links.dedup();

                    // ...token filtering logic (see original)...
                    // (Omitted for brevity, copy from original file)

                    page.tokens = tokens;
                    page.links = links;
                    info!("Fetched URL: {} ({} links, {} tokens)", url, page.links.len(), page.tokens.len());
                    // Only save if we have tokens (i.e., HTML was present)
                    if (!page.tokens.is_empty()) {
                        page.save_async().await?;
                    }
                }
            }
        }
        Err(e) => {
            info!("Error fetching URL {}: {}", url, e);
            page.error = Some(e.to_string());
        }
    }

    // Crawl links sequentially (not in parallel), but only if depth < max_depth
    if depth < max_depth && !page.links.is_empty() {
        let job_oid = page.crawl_job_oid.clone();
        let links = page.links.clone();
        for link in links {
            let _ = crawl_url_boxed(job_oid.clone(), link, depth + 1).await;
        }
    }
    Ok(page)
}

// Boxed async fn for recursion
fn crawl_url_boxed(job_oid: String, url: String, depth: usize) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::sam::memory::Result<CrawledPage>> + Send>> {
    Box::pin(crawl_url_inner(job_oid, url, depth))
}

// Public entry point (non-recursive, just calls boxed version)
pub async fn crawl_url(job_oid: String, url: String) -> crate::sam::memory::Result<CrawledPage> {
    crawl_url_boxed(job_oid, url, 0).await
}

pub fn start_service() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);

        // Only create a runtime if not already inside one
        if tokio::runtime::Handle::try_current().is_ok() {
            // Already inside a runtime: spawn the service directly
            tokio::spawn(async {
                run_crawler_service().await;
            });
        } else {
            // Not inside a runtime: spawn a thread and create a runtime
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    run_crawler_service().await;
                });
            });
        }
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
    log::info!("Crawler service started.");
}

/// Async-friendly version for use from async contexts (e.g., ratatui CLI)
pub async fn start_service_async() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);
        tokio::spawn(async {
            run_crawler_service().await;
        });
        log::info!("Crawler service started.");
    });
    CRAWLER_RUNNING.store(true, Ordering::SeqCst);
}

pub fn stop_service() {
    info!("Crawler service stopping...");
    CRAWLER_RUNNING.store(false, Ordering::SeqCst);
    info!("Crawler service stopped.");
}

pub fn service_status() -> &'static str {
    if CRAWLER_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

/// Main crawler loop: finds pending jobs, crawls, updates status
pub async fn run_crawler_service() {
    use trust_dns_resolver::config::*;
    log::set_max_level(LevelFilter::Info);
    let crawling = Arc::new(TokioMutex::new(()));
    let common_urls = vec![
        "https://www.youtube.com/",
        "https://www.rust-lang.org/",
        "https://www.wikipedia.org/",
        "https://www.example.com/",
        "https://www.mozilla.org/",
        "https://www.github.com/",
        "https://www.google.com/",
        "https://www.facebook.com/",
        "https://www.twitter.com/",
        "https://www.instagram.com/",
        "https://www.linkedin.com/",
        "https://www.reddit.com/",
        "https://www.amazon.com/",
        "https://www.apple.com/",
        "https://www.microsoft.com/",
        "https://www.netflix.com/",
        "https://www.stackoverflow.com/",
        "https://www.bbc.com/",
        "https://www.cnn.com/",
        "https://www.nytimes.com/",
        "https://www.quora.com/",
        "https://www.paypal.com/",
        "https://www.dropbox.com/",
        "https://www.adobe.com/",
        "https://www.slack.com/",
        "https://www.twitch.tv/",
        "https://www.spotify.com/",
        "https://www.medium.com/",
        "https://www.booking.com/",
        "https://www.airbnb.com/",
        "https://www.uber.com/",
        "https://www.lyft.com/",
        "https://www.soundcloud.com/",
        "https://www.vimeo.com/",
        "https://www.flickr.com/",
        "https://www.imdb.com/",
        "https://www.pinterest.com/",
        "https://www.wordpress.com/",
        "https://www.tumblr.com/",
        "https://www.ebay.com/",
        "https://www.bing.com/",
        "https://www.duckduckgo.com/",
        "https://www.yandex.com/",
        "https://www.yahoo.com/",
        "https://www.weather.com/",
        "https://www.office.com/",
        "https://www.salesforce.com/",
        "https://www.shopify.com/",
        "https://www.tesla.com/",
        "https://www.walmart.com/",
        "https://www.target.com/",
        "https://www.nasa.gov/",
        "https://www.nationalgeographic.com/",
        "https://www.forbes.com/",
        "https://www.wsj.com/",
        "https://www.bloomberg.com/",
        "https://www.cnbc.com/",
        "https://www.foxnews.com/",
        "https://www.usatoday.com/",
        "https://www.time.com/",
        "https://www.theguardian.com/",
        "https://www.huffpost.com/",
        "https://www.latimes.com/",
        "https://www.chicagotribune.com/",
        "https://www.nbcnews.com/",
        "https://www.cbsnews.com/",
        "https://www.abcnews.go.com/",
        "https://www.npr.org/",
        "https://www.smh.com.au/",
        "https://www.lemonde.fr/",
        "https://www.spiegel.de/",
        "https://www.elpais.com/",
        "https://www.corriere.it/",
        "https://www.asahi.com/",
        "https://www.sina.com.cn/",
        "https://www.qq.com/",
        "https://www.taobao.com/",
        "https://www.tmall.com/",
        "https://www.baidu.com/",
        "https://www.sohu.com/",
        "https://www.weibo.com/",
        "https://www.163.com/",
        "https://www.jd.com/",
        "https://www.aliexpress.com/",
        "https://www.alibaba.com/",
        "https://www.booking.com/",
        "https://www.expedia.com/",
        "https://www.tripadvisor.com/",
        "https://www.skyscanner.net/",
        "https://www.kayak.com/",
        "https://www.zillow.com/",
        "https://www.trulia.com/",
        "https://www.rightmove.co.uk/",
        "https://www.autotrader.com/",
        "https://www.cars.com/",
        "https://www.carmax.com/",
        "https://www.indeed.com/",
        "https://www.glassdoor.com/",
        "https://www.monster.com/",
        "https://www.simplyhired.com/",
        "https://www.craigslist.org/",
        "https://www.meetup.com/",
        "https://www.eventbrite.com/",
        "https://www.change.org/",
        "https://www.whitehouse.gov/",
        "https://www.usa.gov/",
        "https://www.loc.gov/",
        "https://www.nih.gov/",
        "https://www.cdc.gov/",
        "https://www.fbi.gov/",
        "https://www.cia.gov/",
        "https://www.nsa.gov/",
        "https://www.un.org/",
        "https://www.europa.eu/",
        "https://www.who.int/",
        "https://www.imf.org/",
        "https://www.worldbank.org/",
        "https://www.oecd.org/",
        "https://www.wto.org/",
        "https://www.icann.org/",
        "https://www.iso.org/",
        "https://www.ietf.org/",
        "https://www.w3.org/",
        "https://www.gnu.org/",
        "https://www.linuxfoundation.org/",
        "https://www.apache.org/",
        "https://www.python.org/",
        "https://www.nodejs.org/",
        "https://www.npmjs.com/",
        "https://www.ruby-lang.org/",
        "https://www.php.net/",
        "https://www.mysql.com/",
        "https://www.postgresql.org/",
        "https://www.mongodb.com/",
        "https://www.redis.io/",
        "https://www.heroku.com/",
        "https://www.digitalocean.com/",
        "https://www.linode.com/",
        "https://www.cloudflare.com/",
        "https://www.vercel.com/",
        "https://www.netlify.com/",
        "https://www.gitlab.com/",
        "https://www.bitbucket.org/",
        "https://www.atlassian.com/",
        "https://www.trello.com/",
        "https://www.notion.so/",
        "https://www.zoho.com/",
        "https://www.mailchimp.com/",
        "https://www.hubspot.com/",
        "https://www.squarespace.com/",
        "https://www.wix.com/",
        "https://www.weebly.com/",
        "https://www.medium.com/",
        "https://www.substack.com/",
        "https://www.patreon.com/",
        "https://www.kickstarter.com/",
        "https://www.indiegogo.com/",
        "https://www.gofundme.com/",
        "https://www.ted.com/",
        "https://www.coursera.org/",
        "https://www.edx.org/",
        "https://www.udemy.com/",
        "https://www.khanacademy.org/",
        "https://www.codecademy.com/",
        "https://www.pluralsight.com/",
        "https://www.udacity.com/",
        "https://www.duolingo.com/",
        "https://www.memrise.com/",
        "https://www.rosettastone.com/",
        "https://www.babbel.com/",
        "https://www.openai.com/",
        "https://www.deepmind.com/",
        "https://www.anthropic.com/",
        "https://www.stability.ai/",
        "https://www.midjourney.com/",
        "https://www.perplexity.ai/",
        "https://www.runwayml.com/",
        "https://www.huggingface.co/",
        "https://www.replit.com/",
        "https://www.jsfiddle.net/",
        "https://www.codepen.io/",
        "https://www.codesandbox.io/",
        "https://www.stackexchange.com/",
        "https://www.superuser.com/",
        "https://www.serverfault.com/",
        "https://www.askubuntu.com/",
        "https://www.mathoverflow.net/",
        "https://www.acm.org/",
        "https://www.ieee.org/",
        "https://www.nature.com/",
        "https://www.sciencemag.org/",
        "https://www.cell.com/",
        "https://www.thelancet.com/",
        "https://www.jstor.org/",
        "https://www.arxiv.org/",
        "https://www.biorxiv.org/",
        "https://www.medrxiv.org/",
        "https://www.springer.com/",
        "https://www.elsevier.com/",
        "https://www.taylorandfrancis.com/",
        "https://www.cambridge.org/",
        "https://www.oxfordjournals.org/",
        "https://www.ssrn.com/",
        "https://www.researchgate.net/",
        "https://www.academia.edu/",
        "https://www.mit.edu/",
        "https://www.harvard.edu/",
        "https://www.stanford.edu/",
        "https://www.berkeley.edu/",
        "https://www.ox.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.ethz.ch/",
        "https://www.tum.de/",
        "https://www.tokyo-u.ac.jp/",
        "https://www.kyoto-u.ac.jp/",
        "https://www.sydney.edu.au/",
        "https://www.unimelb.edu.au/",
        "https://www.tsinghua.edu.cn/",
        "https://www.pku.edu.cn/",
        "https://www.iitb.ac.in/",
        "https://www.iisc.ac.in/",
        "https://www.nus.edu.sg/",
        "https://www.ntu.edu.sg/",
        "https://www.kaist.ac.kr/",
        "https://www.snu.ac.kr/",
        "https://www.technion.ac.il/",
        "https://www.weizmann.ac.il/",
        "https://www.utoronto.ca/",
        "https://www.mcgill.ca/",
        "https://www.ubc.ca/",
        "https://www.uq.edu.au/",
        "https://www.unsw.edu.au/",
        "https://www.monash.edu/",
        "https://www.ucl.ac.uk/",
        "https://www.imperial.ac.uk/",
        "https://www.lse.ac.uk/",
        "https://www.kcl.ac.uk/",
        "https://www.ed.ac.uk/",
        "https://www.manchester.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.sheffield.ac.uk/",
        "https://www.southampton.ac.uk/",
        "https://www.nottingham.ac.uk/",
        "https://www.birmingham.ac.uk/",
        "https://www.leeds.ac.uk/",
        "https://www.liverpool.ac.uk/",
        "https://www.cardiff.ac.uk/",
        "https://www.gla.ac.uk/",
        "https://www.strath.ac.uk/",
        "https://www.abdn.ac.uk/",
        "https://www.dundee.ac.uk/",
        "https://www.st-andrews.ac.uk/",
        "https://www.hw.ac.uk/",
        "https://www.rgu.ac.uk/",
        "https://www.qmul.ac.uk/",
        "https://www.gold.ac.uk/",
        "https://www.soas.ac.uk/",
        "https://www.bbk.ac.uk/",
        "https://www.city.ac.uk/",
        "https://www.lshtm.ac.uk/",
        "https://www.open.ac.uk/",
        "https://www.roehampton.ac.uk/",
        "https://www.westminster.ac.uk/",
        "https://www.gre.ac.uk/",
        "https://www.kingston.ac.uk/",
        "https://www.mdx.ac.uk/",
        "https://www.uel.ac.uk/",
        "https://www.londonmet.ac.uk/",
        "https://www.sunderland.ac.uk/",
        "https://www.northumbria.ac.uk/",
        "https://www.newcastle.ac.uk/",
        "https://www.durham.ac.uk/",
        "https://www.york.ac.uk/",
        "https://www.hull.ac.uk/",
        "https://www.lincoln.ac.uk/",
        "https://www.derby.ac.uk/",
        "https://www.staffs.ac.uk/",
        "https://www.keele.ac.uk/",
        "https://www.wlv.ac.uk/",
        "https://www.coventry.ac.uk/",
        "https://www.warwick.ac.uk/",
        "https://www.le.ac.uk/",
        "https://www.lboro.ac.uk/",
        "https://www.nottstrent.ac.uk/",
        "https://www.shef.ac.uk/",
        "https://www.hud.ac.uk/",
        "https://www.bradford.ac.uk/",
        "https://www.salford.ac.uk/",
        "https://www.mmu.ac.uk/",
        "https://www.ljmu.ac.uk/",
        "https://www.edgehill.ac.uk/",
        "https://www.uclan.ac.uk/",
        "https://www.lancaster.ac.uk/",
        "https://www.bangor.ac.uk/",
        "https://www.swansea.ac.uk/",
        "https://www.aber.ac.uk/",
        "https://www.glyndwr.ac.uk/",
        "https://www.cardiffmet.ac.uk/",
        "https://www.southwales.ac.uk/",
        "https://www.wrexham.ac.uk/",
        "https://www.uwtsd.ac.uk/",
        "https://www.oxfordbrookes.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.bucks.ac.uk/",
        "https://www.chi.ac.uk/",
        "https://www.canterbury.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.sussex.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.plymouth.ac.uk/",
        "https://www.exeter.ac.uk/",
        "https://www.bath.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.glos.ac.uk/",
        "https://www.uwe.ac.uk/",
        "https://www.westofengland.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.reading.ac.uk/",
        "https://www.ox.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.bucks.ac.uk/",
        "https://www.chi.ac.uk/",
        "https://www.canterbury.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.sussex.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.plymouth.ac.uk/",
        "https://www.exeter.ac.uk/",
        "https://www.bath.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.glos.ac.uk/",
        "https://www.uwe.ac.uk/",
        "https://www.westofengland.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.reading.ac.uk/",
    ];
    // ...rest of the function unchanged...
    let tlds = vec![
        "com", "net", "org", "io", "co", "ai", "dev", "app", "info", "biz", "us", "uk", "ca", "de", "jp", "fr", "au", "ru", "ch", "it", "nl", "se", "no", "es", "cz", "in", "br", "pl", "me", "tv", "xyz", "site", "online", "store", "tech", "pro", "club", "top", "vip", "live", "news", "cloud", "fun", "world", "today", "agency", "solutions", "digital", "media", "group", "center", "systems", "works", "company", "services", "network", "consulting", "support", "software", "design", "studio", "marketing", "events", "finance", "capital", "ventures", "partners", "law", "legal", "health", "care", "doctor", "clinic", "school", "academy", "education", "university", "college", "gov", "mil", "int", "edu", "museum", "travel", "jobs", "mobi", "name", "coop", "aero", "arpa"
    ];
    let prefixes = vec![
        "www", "mail", "blog", "shop", "store", "news", "app", "api", "dev", "test", "portal", "home", "web", "en", "es", "fr", "de", "it", "pt", "jp", "cn", "ru", "in", "us", "uk", "ca", "au", "br", "mx", "za", "nl", "se", "no", "fi", "dk", "pl", "cz", "tr", "kr", "id", "vn", "th", "my", "sg", "hk", "tw", "il", "ae", "sa", "ir", "eg", "ng", "ke", "gh", "ar", "cl", "co", "pe", "ve"
    ];
    let words = vec![
        "google", "facebook", "youtube", "twitter", "instagram", "wikipedia", 
        "amazon", "reddit", "yahoo", "linkedin", "netflix", "microsoft", 
        "apple", "github", "stackoverflow", "wordpress", "blogspot", 
        "tumblr", "pinterest", "paypal", "dropbox", "adobe", "slack", 
        "zoom", "twitch", "ebay", "bing", "duckduckgo", "quora", "imdb", 
        "bbc", "cnn", "nytimes", "forbes", "weather", "booking", "airbnb", 
        "uber", "lyft", "spotify", "soundcloud", "medium", "vimeo", "flickr",
        "news", "sports", "games", "movies", "music", "photos", "video", "live",
        "shop", "store", "market", "sale", "deal", "offer", "buy", "sell",
        "jobs", "career", "work", "hire", "resume", "apply", "school", "college",
        "university", "learn", "study", "teach", "class", "course", "academy",
        "health", "doctor", "clinic", "hospital", "care", "med", "pharmacy",
        "finance", "bank", "money", "loan", "credit", "card", "pay", "fund",
        "insurance", "tax", "invest", "trade", "stock", "crypto", "bitcoin",
        "weather", "travel", "trip", "flight", "hotel", "car", "rent", "map",
        "food", "pizza", "burger", "cafe", "bar", "restaurant", "menu", "order",
        "blog", "forum", "chat", "mail", "email", "message", "note", "wiki",
        "photo", "pic", "image", "gallery", "album", "camera", "snap", "art",
        "design", "dev", "code", "app", "site", "web", "cloud", "host", "server",
        "data", "ai", "bot", "robot", "smart", "tech", "digital", "media",
        "news", "press", "report", "story", "magazine", "journal", "book",
        "library", "archive", "docs", "file", "pdf", "doc", "sheet", "slide",
        "event", "meet", "party", "club", "group", "team", "community", "social",
        "network", "connect", "link", "share", "like", "follow", "friend",
        "support", "help", "faq", "guide", "info", "about", "contact", "home",
        "login", "signup", "register", "account", "profile", "user", "admin",
        "dashboard", "panel", "console", "system", "manager", "control", "settings",
        "tools", "tool", "kit", "box", "lab", "test", "beta", "demo", "sample",
        "random", "fun", "play", "game", "quiz", "test", "try", "beta", "alpha",
        "pro", "plus", "max", "prime", "vip", "elite", "gold", "silver", "basic",
        "free", "cheap", "deal", "sale", "discount", "offer", "promo", "gift",
        "shop", "store", "cart", "checkout", "buy", "sell", "order", "track",
        "review", "rate", "star", "top", "best", "hot", "new", "now", "today",
        "fast", "quick", "easy", "simple", "safe", "secure", "trusted", "official",
        "global", "world", "local", "city", "town", "village", "place", "zone",
        "area", "region", "state", "country", "nation", "gov", "org", "edu",
        "science", "math", "physics", "chemistry", "bio", "earth", "space",
        "astro", "geo", "eco", "env", "nature", "animal", "plant", "tree",
        "flower", "garden", "farm", "pet", "dog", "cat", "fish", "bird", "horse",
        "car", "bike", "bus", "train", "plane", "boat", "ship", "auto", "motor",
        "drive", "ride", "fly", "move", "run", "walk", "jump", "swim", "climb",
        "build", "make", "create", "craft", "draw", "paint", "write", "read",
        "speak", "talk", "listen", "hear", "see", "watch", "look", "view",
        "open", "close", "start", "stop", "go", "come", "join", "leave", "exit",
        "enter", "begin", "end", "finish", "win", "lose", "score", "goal",
        "plan", "project", "task", "todo", "list", "note", "memo", "remind",
        "alert", "alarm", "clock", "time", "date", "calendar", "schedule",
        "event", "meet", "call", "video", "voice", "chat", "message", "mail",
        "email", "post", "tweet", "blog", "forum", "board", "thread", "topic",
        "news", "press", "media", "tv", "radio", "movie", "film", "show",
        "music", "song", "album", "band", "artist", "dj", "mix", "play", "pause",
        "stop", "record", "edit", "cut", "copy", "paste", "save", "load",
        "send", "receive", "upload", "download", "sync", "backup", "restore",
        "scan", "print", "fax", "copy", "photo", "pic", "image", "video",
        "camera", "lens", "screen", "display", "monitor", "tv", "projector",
        "light", "lamp", "bulb", "fan", "ac", "heater", "fridge", "oven",
        "microwave", "washer", "dryer", "vacuum", "cleaner", "robot", "drone",
        "sensor", "alarm", "lock", "key", "door", "gate", "window", "wall",
        "roof", "floor", "room", "house", "home", "apartment", "flat", "villa",
        "hotel", "motel", "inn", "resort", "camp", "tent", "cabin", "hostel",
        "office", "desk", "chair", "table", "sofa", "bed", "bath", "toilet",
        "kitchen", "cook", "chef", "food", "meal", "dish", "snack", "drink",
        "water", "juice", "milk", "tea", "coffee", "beer", "wine", "bar",
        "pub", "club", "party", "event", "festival", "concert", "show",
        "exhibit", "expo", "fair", "market", "shop", "store", "mall", "plaza",
        "park", "garden", "zoo", "museum", "gallery", "library", "theater",
        "cinema", "stadium", "arena", "gym", "pool", "court", "field", "track",
        "ring", "course", "trail", "road", "street", "avenue", "boulevard",
        "drive", "lane", "way", "path", "route", "highway", "freeway", "bridge",
        "tunnel", "station", "stop", "terminal", "port", "harbor", "dock",
        "airport", "runway", "tower", "building", "block", "lot", "yard",
        "garden", "farm", "field", "forest", "mountain", "hill", "valley",
        "lake", "river", "sea", "ocean", "beach", "island", "bay", "coast",
        "shore", "cliff", "cave", "desert", "plain", "plateau", "volcano",
        "glacier", "reef", "coral", "delta", "marsh", "swamp", "pond", "pool",
        "spring", "well", "fountain", "waterfall", "cascade", "geyser",
    ];
    // ...rest of the function unchanged...

    // DNS resolver setup
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
        .expect("Failed to create DNS resolver");

    // Helper function to perform concurrent DNS lookups with cache
    async fn lookup_domains<I: IntoIterator<Item = String>>(
        resolver: &TokioAsyncResolver,
        domains: I,
    ) -> Vec<String> {
        let mut futures = FuturesUnordered::new();
        for domain in domains {
            let resolver = resolver.clone();
            let domain_clone = domain.clone();
            futures.push(async move {
                // Check cache first
                {
                    let cache = DNS_LOOKUP_CACHE.lock().await;
                    if let Some(found) = cache.get(&domain_clone) {
                        if *found {
                            return Some(domain_clone);
                        } else {
                            return None;
                        }
                    }
                }
                // Not in cache, do DNS lookup
                let found = match resolver.lookup_ip(domain_clone.clone()).await {
                    Ok(lookup) => lookup.iter().next().is_some(),
                    Err(_) => false,
                };
                // Update cache (but don't save to disk here)
                {
                    let mut cache = DNS_LOOKUP_CACHE.lock().await;
                    cache.insert(domain_clone.clone(), found);
                }
                if found {
                    Some(domain_clone)
                } else {
                    None
                }
            });
        }
        let mut found = Vec::new();
        while let Some(result) = futures.next().await {
            if let Some(domain) = result {
                found.push(domain);
            }
        }
        // Save cache after each batch
        save_dns_cache().await;
        found
    }

    load_dns_cache().await;
    cache_all_to_redis().await;

    loop {
        if (!CRAWLER_RUNNING.load(Ordering::SeqCst)) {
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        // Only one crawl at a time
        let _guard = crawling.lock().await;

        // Find a pending job
        let jobs = match CrawlJob::select_async(Some(1), None, None, None).await {
            Ok(jobs) => jobs.into_iter().filter(|j| j.status == "pending").collect::<Vec<_>>(),
            Err(_) => vec![],
        };

        if let Some(mut job) = jobs.into_iter().next() {
            info!("Starting crawl job: oid={} url={}", job.oid, job.start_url);
            // Mark as running
            job.status = "running".to_string();
            job.updated_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
            let _ = job.save_async().await;

            // Crawl start_url and discovered links (BFS, depth 2)
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back((job.start_url.clone(), 0));
            let max_depth = 2;
            while let Some((url, depth)) = queue.pop_front() {
                if visited.contains(&url) || depth > max_depth {
                    continue;
                }
                visited.insert(url.clone());
                info!("Crawling url={} depth={}", url, depth);
                match crawl_url(job.oid.clone(), url.clone()).await {
                    Ok(page) => {
                        // Already saved in crawl_url
                        for link in &page.links {
                            if !visited.contains(link) {
                                queue.push_back((link.clone(), depth + 1));
                            }
                        }
                    }
                    Err(e) => {
                        info!("Crawler error: {}", e);
                        log::error!("Crawler error: {}", e);
                    }
                }
            }
            // Mark job as done
            job.status = "done".to_string();
            job.updated_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
            let _ = job.save_async().await;
            info!("Finished crawl job: oid={}", job.oid);
        } else {
            // No jobs: scan common URLs and/or use DNS queries to find domains
            info!("No pending crawl jobs found. Crawling common URLs.");
            let mut urls_to_try: Vec<String> = common_urls.iter().map(|s| s.to_string()).collect();

            let tlds = vec![
                "com", "net", "org", "io", "co", "ai", "dev", "app", "info", "biz", "us", "uk", "ca", "de", "jp", "fr", "au", "ru", "ch", "it", "nl", "se", "no", "es", "cz", "in", "br", "pl", "me", "tv", "xyz", "site", "online", "store", "tech", "pro", "club", "top", "vip", "live", "news", "cloud", "fun", "world", "today", "agency", "solutions", "digital", "media", "group", "center", "systems", "works", "company", "services", "network", "consulting", "support", "software", "design", "studio", "marketing", "events", "finance", "capital", "ventures", "partners", "law", "legal", "health", "care", "doctor", "clinic", "school", "academy", "education", "university", "college", "gov", "mil", "int", "edu", "museum", "travel", "jobs", "mobi", "name", "coop", "aero", "arpa"
            ];
            let prefixes = vec![
                "www", "mail", "blog", "shop", "store", "news", "app", "api", "dev", "test", "portal", "home", "web", "en", "es", "fr", "de", "it", "pt", "jp", "cn", "ru", "in", "us", "uk", "ca", "au", "br", "mx", "za", "nl", "se", "no", "fi", "dk", "pl", "cz", "tr", "kr", "id", "vn", "th", "my", "sg", "hk", "tw", "il", "ae", "sa", "ir", "eg", "ng", "ke", "gh", "ar", "cl", "co", "pe", "ve"
            ];
            let words = vec![
                "google", "facebook", "youtube", "twitter", "instagram", "wikipedia", 
                "amazon", "reddit", "yahoo", "linkedin", "netflix", "microsoft", 
                "apple", "github", "stackoverflow", "wordpress", "blogspot", 
                "tumblr", "pinterest", "paypal", "dropbox", "adobe", "slack", 
                "zoom", "twitch", "ebay", "bing", "duckduckgo", "quora", "imdb", 
                "bbc", "cnn", "nytimes", "forbes", "weather", "booking", "airbnb", 
                "uber", "lyft", "spotify", "soundcloud", "medium", "vimeo", "flickr",
                "news", "sports", "games", "movies", "music", "photos", "video", "live",
                "shop", "store", "market", "sale", "deal", "offer", "buy", "sell",
                "jobs", "career", "work", "hire", "resume", "apply", "school", "college",
                "university", "learn", "study", "teach", "class", "course", "academy",
                "health", "doctor", "clinic", "hospital", "care", "med", "pharmacy",
                "finance", "bank", "money", "loan", "credit", "card", "pay", "fund",
                "insurance", "tax", "invest", "trade", "stock", "crypto", "bitcoin",
                "weather", "travel", "trip", "flight", "hotel", "car", "rent", "map",
                "food", "pizza", "burger", "cafe", "bar", "restaurant", "menu", "order",
                "blog", "forum", "chat", "mail", "email", "message", "note", "wiki",
                "photo", "pic", "image", "gallery", "album", "camera", "snap", "art",
                "design", "dev", "code", "app", "site", "web", "cloud", "host", "server",
                "data", "ai", "bot", "robot", "smart", "tech", "digital", "media",
                "news", "press", "report", "story", "magazine", "journal", "book",
                "library", "archive", "docs", "file", "pdf", "doc", "sheet", "slide",
                "event", "meet", "party", "club", "group", "team", "community", "social",
                "network", "connect", "link", "share", "like", "follow", "friend",
                "support", "help", "faq", "guide", "info", "about", "contact", "home",
                "login", "signup", "register", "account", "profile", "user", "admin",
                "dashboard", "panel", "console", "system", "manager", "control", "settings",
                "tools", "tool", "kit", "box", "lab", "test", "beta", "demo", "sample",
                "random", "fun", "play", "game", "quiz", "test", "try", "beta", "alpha",
                "pro", "plus", "max", "prime", "vip", "elite", "gold", "silver", "basic",
                "free", "cheap", "deal", "sale", "discount", "offer", "promo", "gift",
                "shop", "store", "cart", "checkout", "buy", "sell", "order", "track",
                "review", "rate", "star", "top", "best", "hot", "new", "now", "today",
                "fast", "quick", "easy", "simple", "safe", "secure", "trusted", "official",
                "global", "world", "local", "city", "town", "village", "place", "zone",
                "area", "region", "state", "country", "nation", "gov", "org", "edu",
                "science", "math", "physics", "chemistry", "bio", "earth", "space",
                "astro", "geo", "eco", "env", "nature", "animal", "plant", "tree",
                "flower", "garden", "farm", "pet", "dog", "cat", "fish", "bird", "horse",
                "car", "bike", "bus", "train", "plane", "boat", "ship", "auto", "motor",
                "drive", "ride", "fly", "move", "run", "walk", "jump", "swim", "climb",
                "build", "make", "create", "craft", "draw", "paint", "write", "read",
                "speak", "talk", "listen", "hear", "see", "watch", "look", "view",
                "open", "close", "start", "stop", "go", "come", "join", "leave", "exit",
                "enter", "begin", "end", "finish", "win", "lose", "score", "goal",
                "plan", "project", "task", "todo", "list", "note", "memo", "remind",
                "alert", "alarm", "clock", "time", "date", "calendar", "schedule",
                "event", "meet", "call", "video", "voice", "chat", "message", "mail",
                "email", "post", "tweet", "blog", "forum", "board", "thread", "topic",
                "news", "press", "media", "tv", "radio", "movie", "film", "show",
                "music", "song", "album", "band", "artist", "dj", "mix", "play", "pause",
                "stop", "record", "edit", "cut", "copy", "paste", "save", "load",
                "send", "receive", "upload", "download", "sync", "backup", "restore",
                "scan", "print", "fax", "copy", "photo", "pic", "image", "video",
                "camera", "lens", "screen", "display", "monitor", "tv", "projector",
                "light", "lamp", "bulb", "fan", "ac", "heater", "fridge", "oven",
                "microwave", "washer", "dryer", "vacuum", "cleaner", "robot", "drone",
                "sensor", "alarm", "lock", "key", "door", "gate", "window", "wall",
                "roof", "floor", "room", "house", "home", "apartment", "flat", "villa",
                "hotel", "motel", "inn", "resort", "camp", "tent", "cabin", "hostel",
                "office", "desk", "chair", "table", "sofa", "bed", "bath", "toilet",
                "kitchen", "cook", "chef", "food", "meal", "dish", "snack", "drink",
                "water", "juice", "milk", "tea", "coffee", "beer", "wine", "bar",
                "pub", "club", "party", "event", "festival", "concert", "show",
                "exhibit", "expo", "fair", "market", "shop", "store", "mall", "plaza",
                "park", "garden", "zoo", "museum", "gallery", "library", "theater",
                "cinema", "stadium", "arena", "gym", "pool", "court", "field", "track",
                "ring", "course", "trail", "road", "street", "avenue", "boulevard",
                "drive", "lane", "way", "path", "route", "highway", "freeway", "bridge",
                "tunnel", "station", "stop", "terminal", "port", "harbor", "dock",
                "airport", "runway", "tower", "building", "block", "lot", "yard",
                "garden", "farm", "field", "forest", "mountain", "hill", "valley",
                "lake", "river", "sea", "ocean", "beach", "island", "bay", "coast",
                "shore", "cliff", "cave", "desert", "plain", "plateau", "volcano",
                "glacier", "reef", "coral", "delta", "marsh", "swamp", "pond", "pool",
                "spring", "well", "fountain", "waterfall", "cascade", "geyser",
            ];

            use futures::stream::{FuturesUnordered, StreamExt};

            let mut rng = SmallRng::from_entropy();

            let mut domains = Vec::new();
            for tld in &tlds {
                let mut sampled_words = words.clone();
                sampled_words.shuffle(&mut rng);
                for word in sampled_words.iter().take(1) {
                    domains.push(format!("{}.{}", word, tld));
                    for prefix in &prefixes {
                        domains.push(format!("{}.{}.{}", prefix, word, tld));
                    }
                }
            }
            for tld in &tlds {
                for prefix in &prefixes {
                    domains.push(format!("{}.{}", prefix, tld));
                }
            }
            for tld in &tlds {
                let mut sampled_words = words.clone();
                sampled_words.shuffle(&mut rng);
                for word in sampled_words.iter().take(1) {
                    domains.push(format!("{}.{}", word, tld));
                }
            }

            log::info!("Found {} domains to check", domains.len());

            domains.sort();
            domains.dedup();
            domains.shuffle(&mut rng);

            let max_domains = num_cpus::get() * 10;
            let domains = &domains[..std::cmp::min(domains.len(), max_domains)];

            let batch_size = num_cpus::get() / 2;
            for batch in domains.chunks(batch_size) {
                let found = lookup_domains(&resolver, batch.iter().cloned()).await;
                for domain in found {
                    urls_to_try.push(format!("https://{}/", domain));
                }
            }
            urls_to_try.sort();
            urls_to_try.dedup();

            let concurrency = num_cpus::get() / 2;
            let mut url_iter = urls_to_try.into_iter();
            loop {
                let mut handles = Vec::new();
                for _ in 0..concurrency {
                    if let Some(url) = url_iter.next() {
                        let mut rng = SmallRng::from_entropy();
                        let dummy_job_oid: String = rng
                            .sample_iter(&Alphanumeric)
                            .take(15)
                            .map(char::from)
                            .collect();
                        handles.push(tokio::spawn(async move {
                            match crawl_url_boxed(dummy_job_oid, url.clone(), 0).await {
                                Ok(_page) => {
                                    log::info!("Crawled (no job): {}", url);
                                }
                                Err(e) => {
                                    info!("Crawler error (no job): {}", e);
                                    log::error!("Crawler error (no job): {}", e);
                                }
                            }
                        }));
                    }
                }
                if handles.is_empty() {
                    break;
                }
                for handle in handles {
                    let _ = handle.await;
                }
            }
        }
        sleep(Duration::from_secs(10)).await;
    }
}

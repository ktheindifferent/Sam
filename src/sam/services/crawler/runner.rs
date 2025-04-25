use crate::sam::services::crawler::job::CrawlJob;
use crate::sam::services::crawler::page::CrawledPage;
use std::collections::{HashSet, VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU64};
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
use rand::seq::SliceRandom;
use tokio::io::AsyncWriteExt;
use url::ParseError; // <-- Add this import
// use tokio_stream::StreamExt;

// use rand::{seq::SliceRandom, thread_rng};

static CRAWLER_RUNNING: AtomicBool = AtomicBool::new(false);

// Add a static DNS cache (domain -> Option<bool> for found/not found)
static DNS_CACHE_PATH: &str = "/opt/sam/dns.cache";
static DNS_LOOKUP_CACHE: Lazy<TokioMutex<HashMap<String, bool>>> = Lazy::new(|| TokioMutex::new(HashMap::new()));

// Shared sleep-until timestamp (epoch seconds)
static SLEEP_UNTIL: once_cell::sync::Lazy<AtomicU64> = once_cell::sync::Lazy::new(|| AtomicU64::new(0));
static TIMEOUT_COUNT: once_cell::sync::Lazy<std::sync::Mutex<usize>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(0));

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

/// Writes a URL to the retry file (for failed/timed out crawls)
pub async fn write_url_to_retry_file(url: &str) {
    let retry_path = "/opt/sam/tmp/crawl_retry.dmp";
    if let Err(e) = fs::create_dir_all("/opt/sam/tmp").await {
        log::warn!("Failed to create retry dir: {}", e);
        return;
    }
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(retry_path)
        .await
    {
        if let Err(e) = file.write_all(format!("{}\n", url).as_bytes()).await {
            log::warn!("Failed to write timed out URL to retry file: {}", e);
        }
    } else {
        log::warn!("Failed to open retry file for writing");
    }
}

/// Returns true if the input string is a valid URL (absolute, with scheme and host)
pub fn is_valid_url(s: &str) -> bool {
    match Url::parse(s) {
        Ok(url) => url.has_host() && url.scheme() != "",
        Err(_) => false,
    }
}


// Internal boxed async fn for recursion
// Prevent printing crawled webdata to terminal by not using println!, dbg!, eprintln!, or any direct output anywhere in this function or its parsing logic
async fn crawl_url_inner(
    job_oid: String,
    url: String,
    depth: usize,
    established_client: Option<&tokio_postgres::Client>,
) -> crate::sam::memory::Result<CrawledPage> {

    // Shared sleep logic: check if we should sleep before making a request
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let sleep_until = SLEEP_UNTIL.load(Ordering::SeqCst);
    if now < sleep_until {
        let sleep_secs = sleep_until - now;
        log::warn!("Global sleep in effect, sleeping for {} seconds", sleep_secs);
        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }

    // Bugfix: Check if the URL is valid before proceeding
    if !is_valid_url(&url) {
        return Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(format!("Invalid URL"))));
    }

    // Return early if the URL looks like a search endpoint
    let url_lc = url.to_ascii_lowercase();
    if url_lc.contains("/search/")
        || url_lc.contains("search=")
        || url_lc.contains("q=")
        || url_lc.contains("/find/")
        || url_lc.contains("/query/")
        || url_lc.contains("query=")
        || url_lc.contains("/lookup/")
        || url_lc.contains("lookup=")
        || url_lc.contains("/results/")
        || url_lc.contains("results=")
        || url_lc.contains("/explore/")
        || url_lc.contains("explore=")
        || url_lc.contains("/filter/")
        || url_lc.contains("filter=")
        || url_lc.contains("/discover/")
        || url_lc.contains("discover=")
        || url_lc.contains("/browse/")
        || url_lc.contains("browse=")
        || url_lc.contains("u=")
        || url_lc.contains("url=")
        || url_lc.contains("id=")
        || url_lc.contains("backURL=")
        || url_lc.contains("text=")
        || url_lc.contains("searchterm=")
        || url_lc.contains("/list/")

    {
        return Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(
            "URL appears to be a search endpoint, skipping".to_string(),
        )));
    }


    let max_depth = 2;

    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String(url.clone()));
    pg_query.query_columns.push("url =".to_string());
    let existing = match CrawledPage::select_async(None, None, None, Some(pg_query), established_client).await {
        Ok(pages) => pages,
        Err(e) => {
            log::warn!("Failed to query existing CrawledPage: {}", e);
            Vec::new()
        }
    };
    if !existing.is_empty() {
        return Ok(existing[0].clone());
    }

    let mut page = CrawledPage::new();
    page.crawl_job_oid = job_oid.clone();
    page.url = url.clone();
    info!("Fetching URL: {}", url);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(60)) // Increased from 10 to 15
        .build()
        .map_err(|e| crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(format!("Client build error: {}", e))))?;

    let mut resp = None;
    let mut last_err = None;
    for attempt in 0..3 {
        match tokio::time::timeout(Duration::from_secs(60), client.get(&url).send()).await {
            Ok(Ok(r)) => {
                resp = Some(r);
                break;
            }
            Ok(Err(e)) => {
                last_err = Some(e.to_string());
                log::warn!("HTTP request error (attempt {}): {} for {}", attempt + 1, last_err.as_ref().unwrap(), url);
            }
            Err(_) => {
                last_err = Some("Request timed out".to_string());
                log::warn!("HTTP request timed out (attempt {}): {}", attempt + 1, url);
            }
        }
        // Optional: small delay between retries
        sleep(Duration::from_millis(300)).await;
    }
    let resp = match resp {
        Some(r) => Ok(r),
        None => Err(crate::sam::memory::Error::from_kind(crate::sam::memory::ErrorKind::Msg(
            format!("Request failed after retries: {}", last_err.unwrap_or_else(|| "unknown".to_string()))
        )))
    };

    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();



            if status == 200 {
                // Extract headers before consuming resp
                let headers = resp.headers().clone();
            



                let html = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::warn!("Failed to get text for {}: {}", url, e);
                        String::new()
                    }
                };
            
                let url_clone = url.clone();
                let headers_clone = headers.clone();
                // Try to extract the MIME type from the Content-Type header, ignoring parameters like charset
                let mut mime_from_header: Option<String> = None;
                if let Some(mimeh) = headers_clone.get("Content-Type").or_else(|| headers_clone.get("content-type")) {
                    if let Ok(mime_str) = mimeh.to_str() {
                        // Only take the part before ';' (ignore charset, etc.), trim, and lowercase
                        let mime_main = mime_str.split(';').next().unwrap_or(mime_str).trim().to_ascii_lowercase();
                        if !mime_main.is_empty() {
                            mime_from_header = Some(mime_main);
                        }
                    }
                }
                // Pass headers into the closure
                let result = tokio::task::spawn_blocking(move || {
                  
                    let mut tokens = Vec::new();
                    let mut links = Vec::new();
                    let mut mime_tokens = Vec::new();





                    let url_lc = url_clone.to_ascii_lowercase();
                    let mut file_mime: Option<&str> = None;
                    let mime_map = [
                        (".3gp", "video/3gpp"),
                        (".3g2", "video/3gpp2"),
                        (".7z", "application/x-7z-compressed"),
                        (".aac", "audio/aac"),
                        (".ai", "application/postscript"),
                        (".aif", "audio/x-aiff"),
                        (".aiff", "audio/x-aiff"),
                        (".amr", "audio/amr"),
                        (".apk", "application/vnd.android.package-archive"),
                        (".apng", "image/apng"),
                        (".arj", "application/x-arj"),
                        (".asf", "video/x-ms-asf"),
                        (".asp", "application/x-aspx"),
                        (".aspx", "application/x-aspx"),
                        (".avi", "video/x-msvideo"),
                        (".azw3", "application/vnd.amazon.ebook"),
                        (".bat", "application/x-msdownload"),
                        (".bin", "application/octet-stream"),
                        (".bmp", "image/bmp"),
                        (".bz2", "application/x-bzip2"),
                        (".cab", "application/vnd.ms-cab-compressed"),
                        (".c", "text/x-c"),
                        (".cc", "text/x-c++src"),
                        (".chm", "application/vnd.ms-htmlhelp"),
                        (".class", "application/java"),
                        (".clj", "text/x-clojure"),
                        (".cpp", "text/x-c++src"),
                        (".cjs", "application/javascript"),
                        (".conf", "text/plain"),
                        (".cpio", "application/x-cpio"),
                        (".css", "text/css"),
                        (".csv", "text/csv"),
                        (".cue", "application/x-cue"),
                        (".cxx", "text/x-c++src"),
                        (".dart", "application/dart"),
                        (".deb", "application/x-debian-package"),
                        (".dll", "application/x-msdownload"),
                        (".dmg", "application/x-apple-diskimage"),
                        (".doc", "application/msword"),
                        (".docm", "application/vnd.ms-word.document.macroEnabled.12"),
                        (".docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
                        (".dot", "application/msword"),
                        (".dotx", "application/vnd.openxmlformats-officedocument.wordprocessingml.template"),
                        (".dylib", "application/x-dylib"),
                        (".eot", "application/vnd.ms-fontobject"),
                        (".epub", "application/epub+zip"),
                        (".exe", "application/vnd.microsoft.portable-executable"),
                        (".fb2", "application/x-fictionbook+xml"),
                        (".flac", "audio/flac"),
                        (".flv", "video/x-flv"),
                        (".gif", "image/gif"),
                        (".go", "application/x-go"),
                        (".gz", "application/gzip"),
                        (".h", "text/x-c++hdr"),
                        (".h++", "text/x-c++hdr"),
                        (".heic", "image/heic"),
                        (".heif", "image/heif"),
                        (".hh", "text/x-c++hdr"),
                        (".htm", "text/html"),
                        (".html", "text/html"),
                        (".hpp", "text/x-c++hdr"),
                        (".hxx", "text/x-c++hdr"),
                        (".ico", "image/x-icon"),
                        (".ini", "text/plain"),
                        (".iso", "application/x-iso9660-image"),
                        (".jar", "application/java-archive"),
                        (".java", "application/java-vm"),
                        (".jpeg", "image/jpeg"),
                        (".jpg", "image/jpeg"),
                        (".js", "application/javascript"),
                        (".json", "application/json"),
                        (".jsp", "application/x-jsp"),
                        (".jsx", "application/javascript"),
                        (".key", "application/x-iwork-keynote-sffkey"),
                        (".less", "text/x-less"),
                        (".log", "text/plain"),
                        (".lua", "application/lua"),
                        (".m", "text/x-objective-c"),
                        (".m3u", "audio/mpegurl"),
                        (".m3u8", "application/vnd.apple.mpegurl"),
                        (".m4a", "audio/mp4"),
                        (".m4v", "video/x-m4v"),
                        (".md", "text/markdown"),
                        (".midi", "audio/midi"),
                        (".mid", "audio/midi"),
                        (".mjs", "application/javascript"),
                        (".mkv", "video/x-matroska"),
                        (".mm", "text/x-objective-c++"),
                        (".mobi", "application/x-mobipocket-ebook"),
                        (".mov", "video/quicktime"),
                        (".mp3", "audio/mpeg"),
                        (".mp4", "video/mp4"),
                        (".mpg", "video/mpeg"),
                        (".mpeg", "video/mpeg"),
                        (".msi", "application/x-msdownload"),
                        (".odp", "application/vnd.oasis.opendocument.presentation"),
                        (".ods", "application/vnd.oasis.opendocument.spreadsheet"),
                        (".odc", "application/vnd.oasis.opendocument.chart"),
                        (".odf", "application/vnd.oasis.opendocument.formula"),
                        (".odg", "application/vnd.oasis.opendocument.graphics"),
                        (".odm", "application/vnd.oasis.opendocument.text-master"),
                        (".odt", "application/vnd.oasis.opendocument.text"),
                        (".oga", "audio/ogg"),
                        (".ogg", "audio/ogg"),
                        (".ogv", "video/ogg"),
                        (".opus", "audio/opus"),
                        (".otf", "font/otf"),
                        (".pdf", "application/pdf"),
                        (".php", "application/x-httpd-php"),
                        (".pl", "application/x-perl"),
                        (".pls", "audio/x-scpls"),
                        (".png", "image/png"),
                        (".ppt", "application/vnd.ms-powerpoint"),
                        (".pptm", "application/vnd.ms-powerpoint.presentation.macroEnabled.12"),
                        (".pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
                        (".ps", "application/postscript"),
                        (".py", "application/x-python"),
                        (".rar", "application/x-rar-compressed"),
                        (".rb", "application/x-ruby"),
                        (".rst", "text/x-rst"),
                        (".rs", "application/rust"),
                        (".rtf", "application/rtf"),
                        (".sass", "text/x-sass"),
                        (".scss", "text/x-scss"),
                        (".sh", "application/x-sh"),
                        (".so", "application/x-sharedlib"),
                        (".sql", "application/sql"),
                        (".svg", "image/svg+xml"),
                        (".swf", "application/x-shockwave-flash"),
                        (".tar", "application/x-tar"),
                        (".tex", "application/x-tex"),
                        (".tif", "image/tiff"),
                        (".tiff", "image/tiff"),
                        (".toast", "application/x-toast"),
                        (".toml", "application/toml"),
                        (".torrent", "application/x-bittorrent"),
                        (".ts", "application/typescript"),
                        (".tsv", "text/tab-separated-values"),
                        (".ttf", "font/ttf"),
                        (".txt", "text/plain"),
                        (".vcd", "application/x-cd-image"),
                        (".wav", "audio/wav"),
                        (".webm", "video/webm"),
                        (".webp", "image/webp"),
                        (".woff", "font/woff"),
                        (".woff2", "font/woff2"),
                        (".wsdl", "application/wsdl+xml"),
                        (".xhtml", "application/xhtml+xml"),
                        (".xls", "application/vnd.ms-excel"),
                        (".xlsm", "application/vnd.ms-excel.sheet.macroEnabled.12"),
                        (".xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
                        (".xml", "application/xml"),
                        (".xps", "application/vnd.ms-xpsdocument"),
                        (".xz", "application/x-xz"),
                        (".yaml", "application/x-yaml"),
                        (".yml", "application/x-yaml"),
                        (".zip", "application/zip"),
                    ];

                    let file_ext = {
                        let url_no_query = url_lc.split(&['?', '#'][..]).next().unwrap_or("");
                        let path = std::path::Path::new(url_no_query);
                        // Only treat as file if the last segment contains a dot (.) and is not a known TLD
                        if let Some(segment) = path.file_name().and_then(|s| s.to_str()) {
                            if segment.contains('.') {
                                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                                    // List of common TLDs to exclude
                                    // List of all known TLDs (as of 2024-06, from IANA root zone database)
                                    // Source: https://data.iana.org/TLD/tlds-alpha-by-domain.txt
                                    let tlds = [
                                        "aaa","aarp","abarth","abb","abbott","abbvie","abc","able","abogado","abudhabi","ac","academy","accenture","accountant","accountants","aco","actor","ad","adac","ads","adult","ae","aeg","aero","aetna","af","afamilycompany","afl","africa","ag","agakhan","agency","ai","aig","airbus","airforce","airtel","akdn","al","alfaromeo","alibaba","alipay","allfinanz","allstate","ally","alsace","alstom","am","amazon","americanexpress","americanfamily","amex","amfam","amica","amsterdam","analytics","android","anquan","anz","ao","aol","apartments","app","apple","aq","aquarelle","ar","arab","aramco","archi","army","arpa","art","arte","as","asda","asia","associates","at","athleta","attorney","au","auction","audi","audible","audio","auspost","author","auto","autos","avianca","aw","aws","ax","axa","az","azure","ba","baby","baidu","banamex","bananarepublic","band","bank","bar","barcelona","barclaycard","barclays","barefoot","bargains","baseball","basketball","bauhaus","bayern","bb","bbc","bbt","bbva","bcg","bcn","bd","be","beats","beauty","beer","bentley","berlin","best","bestbuy","bet","bf","bg","bh","bharti","bi","bible","bid","bike","bing","bingo","bio","biz","bj","bl","black","blackfriday","blockbuster","blog","bloomberg","blue","bm","bms","bmw","bn","bnl","bnpparibas","bo","boats","boehringer","bofa","bom","bond","boo","book","booking","boots","bosch","bostik","boston","bot","boutique","box","br","bradesco","bridgestone","broadway","broker","brother","brussels","bs","bt","budapest","bugatti","build","builders","business","buy","buzz","bv","bw","by","bz","bzh","ca","cab","cafe","cal","call","calvinklein","cam","camera","camp","cancerresearch","canon","capetown","capital","capitalone","car","caravan","cards","care","career","careers","cars","cartier","casa","case","caseih","cash","casino","cat","catering","catholic","cba","cbn","cbre","cbs","cc","cd","ceb","center","ceo","cern","cf","cfa","cfd","cg","ch","chanel","channel","chase","chat","cheap","chintai","chloe","christmas","chrome","church","ci","cipriani","circle","cisco","citadel","citi","citic","city","cityeats","ck","cl","claims","cleaning","click","clinic","clinique","clothing","cloud","club","clubmed","cm","cn","co","coach","codes","coffee","college","cologne","com","comcast","commbank","community","company","compare","computer","comsec","condos","construction","consulting","contact","contractors","cooking","cookingchannel","cool","coop","corsica","country","coupon","coupons","courses","cpa","cr","credit","creditcard","creditunion","cricket","crown","crs","cruise","cruises","csc","cu","cuisinella","cv","cw","cx","cy","cymru","cyou","cz","dabur","dad","dance","data","date","dating","datsun","day","dclk","dds","de","deal","dealer","deals","degree","delivery","dell","deloitte","delta","democrat","dental","dentist","desi","design","dev","dhl","diamonds","diet","digital","direct","directory","discount","discover","dish","diy","dj","dk","dm","dnp","do","docs","doctor","dodge","dog","doha","domains","dot","download","drive","dtv","dubai","duck","dunlop","duns","dupont","durban","dvag","dvr","dz","earth","eat","ec","eco","edeka","edu","education","ee","eg","email","emerck","energy","engineer","engineering","enterprises","epost","epson","equipment","er","ericsson","erni","es","esq","estate","esurance","et","eu","eurovision","eus","events","everbank","exchange","expert","exposed","express","extraspace","fage","fail","fairwinds","faith","family","fan","fans","farm","farmers","fashion","fast","fedex","feedback","ferrari","ferrero","fi","fiat","fidelity","fido","film","final","finance","financial","fire","firestone","firmdale","fish","fishing","fit","fitness","fj","fk","flickr","flights","flir","florist","flowers","fly","fm","fo","foo","food","foodnetwork","football","ford","forex","forsale","forum","foundation","fox","fr","free","fresenius","frl","frogans","frontdoor","frontier","ftr","fujitsu","fujixerox","fun","fund","furniture","futbol","fyi","ga","gal","gallery","gallo","gallup","game","games","gap","garden","gb","gbiz","gd","gdn","ge","gea","gent","genting","george","gf","gg","ggee","gh","gi","gift","gifts","gives","giving","gl","glade","glass","gle","global","globo","gm","gmail","gmbh","gmo","gmx","gn","godaddy","gold","goldpoint","golf","goo","goodhands","goodyear","goog","google","gop","got","gov","gp","gq","gr","grainger","graphics","gratis","green","gripe","grocery","group","gs","gt","gu","guardian","gucci","guge","guide","guitars","guru","gw","gy","hair","hamburg","hangout","haus","hbo","hdfc","hdfcbank","health","healthcare","help","helsinki","here","hermes","hgtv","hiphop","hisamitsu","hitachi","hiv","hk","hkt","hm","hn","hockey","holdings","holiday","homedepot","homegoods","homes","homesense","honda","honeywell","horse","hospital","host","hosting","hot","hoteles","hotels","hotmail","house","how","hr","hsbc","ht","htc","hu","hughes","hyatt","hyundai","ibm","icbc","ice","icu","id","ie","ieee","ifm","ikano","il","im","imamat","imdb","immo","immobilien","in","inc","industries","infiniti","info","ing","ink","institute","insurance","insure","int","intel","international","intuit","investments","io","ipiranga","iq","ir","irish","is","iselect","ismaili","ist","istanbul","it","itau","itv","iveco","iwc","jaguar","java","jcb","je","jeep","jetzt","jewelry","jio","jll","jm","jmp","jnj","jo","jobs","joburg","jot","joy","jp","jpmorgan","jprs","juegos","juniper","kaufen","kddi","ke","kerryhotels","kerrylogistics","kerryproperties","kfh","kg","kh","ki","kia","kim","kinder","kindle","kitchen","kiwi","km","kn","koeln","komatsu","kosher","kp","kpmg","kpn","kr","krd","kred","kuokgroup","kw","ky","kyoto","kz","la","lacaixa","lamborghini","lamer","lancaster","lancia","lancome","land","landrover","lanxess","lasalle","lat","latino","latrobe","law","lawyer","lb","lc","lds","lease","leclerc","lefrak","legal","lego","lexus","lgbt","li","liaison","lidl","life","lifeinsurance","lifestyle","lighting","like","lilly","limited","limo","lincoln","linde","link","lipsy","live","living","lixil","lk","llc","llp","loan","loans","locker","locus","loft","lol","london","lotte","lotto","love","lpl","lplfinancial","lr","ls","lt","ltd","ltda","lu","lundbeck","lupin","luxe","luxury","lv","ly","ma","macys","madrid","maif","maison","makeup","man","management","mango","map","market","marketing","markets","marriott","marshalls","maserati","mattel","mba","mc","mckinsey","md","me","med","media","meet","melbourne","meme","memorial","men","menu","merckmsd","metlife","mg","mh","miami","microsoft","mil","mini","mint","mit","mitsubishi","mk","ml","mlb","mls","mm","mma","mn","mo","mobi","mobile","mobily","moda","moe","moi","mom","monash","money","monster","mopar","mormon","mortgage","moscow","moto","motorcycles","mov","movie","mp","mq","mr","ms","msd","mt","mtn","mtr","mu","museum","music","mutual","mv","mw","mx","my","mz","na","nab","nadex","nagoya","name","nationwide","natura","navy","nba","nc","ne","nec","net","netbank","netflix","network","neustar","new","newholland","news","next","nextdirect","nexus","nf","nfl","ng","ngo","nhk","ni","nico","nike","nikon","ninja","nissan","nissay","nl","no","nokia","northwesternmutual","norton","now","nowruz","nowtv","np","nr","nra","nrw","ntt","nu","nyc","nz","obi","observer","off","office","okinawa","olayan","olayangroup","oldnavy","ollo","om","omega","one","ong","onl","online","onyourside","ooo","open","oracle","orange","org","organic","origins","osaka","otsuka","ott","ovh","pa","page","pamperedchef","panasonic","panerai","paris","pars","partners","parts","party","passagens","pay","pccw","pe","pet","pf","pfizer","pg","ph","pharmacy","phd","philips","phone","photo","photography","photos","physio","piaget","pics","pictet","pictures","pid","pin","ping","pink","pioneer","pizza","pk","pl","place","play","playstation","plumbing","plus","pm","pn","pnc","pohl","poker","politie","porn","post","pr","pramerica","praxi","press","prime","prod","productions","prof","progressive","promo","properties","property","protection","pru","prudential","ps","pt","pub","pw","pwc","py","qa","qpon","quebec","quest","qvc","racing","radio","raid","read","realestate","realtor","realty","recipes","red","redstone","redumbrella","rehab","reise","reisen","reit","reliance","ren","rent","rentals","repair","report","republican","rest","restaurant","review","reviews","rexroth","rich","richardli","ricoh","rightathome","ril","rio","rip","rmit","ro","rocher","rocks","rodeo","rogers","room","rs","rsvp","ru","rugby","ruhr","run","rw","rwe","ryukyu","sa","saarland","safe","safety","sakura","sale","salon","samsclub","samsung","sandvik","sandvikcoromant","sanofi","sap","sarl","sas","save","saxo","sb","sbi","sbs","sc","sca","scb","schaeffler","schmidt","scholarships","school","schule","schwarz","science","scjohnson","scor","scot","sd","se","search","seat","secure","security","seek","select","sener","services","ses","seven","sew","sex","sexy","sfr","sg","sh","shangrila","sharp","shaw","shell","shia","shiksha","shoes","shop","shopping","shouji","show","showtime","shriram","si","silk","sina","singles","site","sj","sk","ski","skin","sky","skype","sl","sling","sm","smart","smile","sn","sncf","so","soccer","social","softbank","software","sohu","solar","solutions","song","sony","soy","space","spiegel","sport","spot","spreadbetting","sr","srl","srt","st","stada","staples","star","starhub","statebank","statefarm","stc","stcgroup","stockholm","storage","store","stream","studio","study","style","su","sucks","supersport","supplies","supply","support","surf","surgery","suzuki","sv","swatch","swiftcover","swiss","sx","sy","sydney","symantec","systems","sz","tab","taipei","talk","taobao","target","tatamotors","tatar","tattoo","tax","taxi","tc","tci","td","tdk","team","tech","technology","tel","telefonica","temasek","tennis","teva","tf","tg","th","thd","theater","theatre","tiaa","tickets","tienda","tiffany","tips","tires","tirol","tj","tk","tl","tm","tmall","tn","to","today","tokyo","tools","top","toray","toshiba","total","tours","town","toyota","toys","tp","tr","trade","trading","training","travel","travelchannel","travelers","travelersinsurance","trust","trv","tt","tube","tui","tunes","tushu","tv","tvs","tw","tz","ua","ubank","ubs","uconnect","ug","uk","unicom","university","uno","uol","ups","us","uy","uz","va","vacations","vana","vanguard","vc","ve","vegas","ventures","verisign","versicherung","vet","vg","vi","viajes","video","vig","viking","villas","vin","vip","virgin","visa","vision","viva","vivo","vlaanderen","vn","vodka","volkswagen","volvo","vote","voting","voto","voyage","vu","vuelos","wales","walmart","walter","wang","wanggou","warman","watch","watches","weather","weatherchannel","webcam","weber","website","wed","wedding","weibo","weir","wf","whoswho","wien","wiki","williamhill","win","windows","wine","winners","wme","wolterskluwer","woodside","work","works","world","wow","ws","wtc","wtf","xbox","xerox","xfinity","xihuan","xin","xn--11b4c3d","xn--1ck2e1b","xn--1qqw23a","xn--2scrj9c","xn--30rr7y","xn--3bst00m","xn--3ds443g","xn--3oq18vl8pn36a","xn--3pxu8k","xn--42c2d9a","xn--45br5cyl","xn--45brj9c","xn--45q11c","xn--4gbrim","xn--54b7fta0cc","xn--55qw42g","xn--55qx5d","xn--5su34j936bgsg","xn--5tzm5g","xn--6frz82g","xn--6qq986b3xl","xn--80adxhks","xn--80aqecdr1a","xn--80asehdb","xn--80aswg","xn--8y0a063a","xn--9dbq2a","xn--9et52u","xn--9krt00a","xn--b4w605ferd","xn--bck1b9a5dre4c","xn--c1avg","xn--c2br7g","xn--cck2b3b","xn--cg4bki","xn--clchc0ea0b2g2a9gcd","xn--czr694b","xn--czrs0t","xn--czru2d","xn--d1acj3b","xn--e1a4c","xn--eckvdtc9d","xn--efvy88h","xn--estv75g","xn--fct429k","xn--fhbei","xn--fiq228c5hs","xn--fiq64b","xn--fiqs8s","xn--fiqz9s","xn--fjq720a","xn--flw351e","xn--fpcrj9c3d","xn--fzc2c9e2c","xn--g2xx48c","xn--gckr3f0f","xn--gecrj9c","xn--h2breg3eve","xn--h2brj9c","xn--h2brj9c8c","xn--hxt814e","xn--i1b6b1a6a2e","xn--imr513n","xn--io0a7i","xn--j1aef","xn--jlq61u9w7b","xn--jvr189m","xn--kcrx77d1x4a","xn--kprw13d","xn--kpry57d","xn--kpu716f","xn--kput3i","xn--l1acc","xn--lgbbat1ad8j","xn--mgb9awbf","xn--mgba3a3ejt","xn--mgba7c0bbn0a","xn--mgbaakc7dvf","xn--mgbaam7a8h","xn--mgbab2bd","xn--mgbah1a3hjkrd","xn--mgbai9azgqp6j","xn--mgbayh7gpa","xn--mgbb9fbpob","xn--mgbbh1a71e","xn--mgbc0a9azcg","xn--mgbca7dzdo","xn--mgberp4a5d4ar","xn--mgbi4ecexp","xn--mgbpl2fh","xn--mgbt3dhd","xn--mgbtx2b","xn--mgbx4cd0ab","xn--mix891f","xn--mk1bu44c","xn--mxtq1m","xn--ngbc5azd","xn--ngbe9e0a","xn--node","xn--nqv7f","xn--nqv7fs00ema","xn--nyqy26a","xn--o3cw4h","xn--ogbpf8fl","xn--otu796d","xn--p1acf","xn--p1ai","xn--pbt977c","xn--pgbs0dh","xn--pssy2u","xn--q9jyb4c","xn--qcka1pmc","xn--qxam","xn--rhqv96g","xn--rovu88b","xn--rvc1e0am3e","xn--s9brj9c","xn--ses554g","xn--t60b56a","xn--tckwe","xn--tiq49xqyj","xn--unup4y","xn--vermgensberater-ctb","xn--vermgensberatung-pwb","xn--vhquv","xn--vuq861b","xn--w4r85el8fhu5dnra","xn--w4rs40l","xn--xhq521b","xn--xkc2al3hye2a","xn--xkc2dl3a5ee0h","xn--y9a3aq","xn--yfro4i67o","xn--ygbi2ammx","xn--zfr164b","xperia","xxx","xyz","yachts","yahoo","yamaxun","yandex","ye","yodobashi","yoga","yokohama","you","youtube","yt","yun","za","zappos","zara","zero","zip","zippo","zm","zone","zuerich","zw"
                                    ];
                                    if !tlds.contains(&ext) {
                                        Some(format!(".{}", ext))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(ref ext) = file_ext {
                        for (map_ext, mime) in mime_map.iter() {
                            if ext.eq_ignore_ascii_case(map_ext) {
                                file_mime = Some(*mime);
                                break;
                            }
                        }
                    }

                    // Prefer MIME type from header, then file extension, then default
                    if let Some(mimeh) = mime_from_header {
                        mime_tokens.push(mimeh);
                    } else if let Some(mime) = file_mime {
                        mime_tokens.push(mime.to_string());
                    } else {
                        mime_tokens.push("application/octet-stream".to_string());
                    }
                    
                
                    let body_selector = match scraper::Selector::parse("body") {
                        Ok(sel) => sel,
                        Err(e) => {
                            log::warn!("Failed to parse selector 'body': {}", e);
                            return (mime_tokens, tokens, links);
                        }
                    };
                    let skip_tags = ["script", "style", "noscript", "svg", "canvas", "iframe", "template"];
                    let skip_selector = skip_tags
                        .iter()
                        .filter_map(|tag| match scraper::Selector::parse(tag) {
                            Ok(selector) => Some(selector),
                            Err(e) => {
                                log::warn!("Failed to parse selector '{}': {}", tag, e);
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    // Check for special replacement character (�) in the HTML body or any tag text
                    let document = scraper::Html::parse_document(&html);
                    let contains_replacement_char = html.contains('�')
                        || document.root_element().text().any(|t| t.contains('�'));
                    if contains_replacement_char {
                        return (mime_tokens, tokens, links);
                    }

                    fn extract_text(element: &scraper::ElementRef, skip_selector: &[scraper::Selector], tokens: &mut Vec<String>) {
                        for sel in skip_selector {
                            if sel.matches(element) {
                                return;
                            }
                        }
                        for child in element.children() {
                            match child.value() {
                                scraper::node::Node::Text(t) => {
                                    for word in t.text.split_whitespace() {
                                        let w = word.trim_matches(|c: char| !c.is_alphanumeric());
                                        if !w.is_empty() {
                                            tokens.push(w.to_lowercase());
                                        }
                                    }
                                }
                                scraper::node::Node::Element(_) => {
                                    if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                                        extract_text(&child_elem, skip_selector, tokens);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    
                

                    // Treat .php, .asp, .aspx, .jsp, .jspx, .htm, .html, .xhtml, .shtml, .cgi, .pl, .cfm, .rb, .py, .xml, .json, .md, .txt, etc. as "document" types that may contain links
                    let doc_exts = [
                        ".html", ".htm", ".xhtml", ".shtml", ".php", ".asp", ".aspx", ".jsp", ".jspx", ".cgi", ".pl", ".cfm", ".rb", ".py", ".xml", ".json", ".md", ".txt", "/"
                    ];
                    // If no extension, treat as document; otherwise, check if extension is in doc_exts
                    let is_document = match &file_ext {
                        Some(ext) => doc_exts.iter().any(|d| ext.eq_ignore_ascii_case(d)),
                        None => true,
                    };
                    
                    if is_document {

                        for body in document.select(&body_selector) {
                            extract_text(&body, &skip_selector, &mut tokens);
                        }

                        let a_selector = match scraper::Selector::parse("a[href]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'a[href]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&a_selector) {
                            if let Some(link) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(link)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(link)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let img_selector = match scraper::Selector::parse("img[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'img[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&img_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let audio_selector = match scraper::Selector::parse("audio[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'audio[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&audio_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let source_selector = match scraper::Selector::parse("audio source[src], video source[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'audio source[src], video source[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&source_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let video_selector = match scraper::Selector::parse("video[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'video[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&video_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let link_selector = match scraper::Selector::parse("link[rel=\"stylesheet\"]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'link[rel=\"stylesheet\"]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&link_selector) {
                            if let Some(href) = element.value().attr("href") {
                                if let Ok(abs) = Url::parse(href)
                                    .or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(href)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }

                        let script_selector = match scraper::Selector::parse("script[src]") {
                            Ok(sel) => sel,
                            Err(e) => {
                                log::warn!("Failed to parse selector 'script[src]': {}", e);
                                return (mime_tokens, tokens, links);
                            }
                        };
                        for element in document.select(&script_selector) {
                            if let Some(src) = element.value().attr("src") {
                                if let Ok(abs) = Url::parse(src).or_else(|_| Url::parse(&url_clone).and_then(|base| base.join(src)))
                                {
                                    links.push(abs.to_string());
                                }
                            }
                        }
                    } else {
                        log::info!("Skipping non-document file: {}", url_clone.clone());
                        // println!("{:?}: {}", file_ext, is_document);
                
                    }

                    
                    
                    (mime_tokens, tokens, links)
                }).await;

                let (mut mime_tokens, mut tokens, mut links) = match result {
                    Ok((mime_tokens, tokens, links)) => (mime_tokens, tokens, links),
                    Err(e) => {
                        log::warn!("Failed to parse HTML for {}: {}", url, e);
                        (Vec::new(), Vec::new(), Vec::new())
                    }
                };

                tokens.sort();
                tokens.dedup();
                links.sort();
                links.dedup();

        
                let common_tokens = vec![
                    // English
                    "the", "is", "in", "and", "to", "a", "of", "for", "on", "that", "this", "it", "with",
                    "as", "at", "by", "an", "be", "are", "was", "were", "from", "or", "but", "not", "have",
                    "has", "had", "will", "would", "can", "could", "should", "do", "does", "did", "so",
                    "if", "then", "than", "which", "who", "whom", "whose", "what", "when", "where", "why",
                    "how", "about", "all", "any", "each", "few", "more", "most", "other", "some", "such",
                    "no", "nor", "only", "own", "same", "too", "very", "just", "over", "under", "again",
                    "once", "also", "into", "out", "up", "down", "off", "above", "below", "between", "after",
                    "before", "during", "through", "because", "while", "both", "either", "neither", "may",
                    "might", "must", "our", "your", "their", "his", "her", "its", "them", "they", "he", "she",
                    "we", "you", "i", "me", "my", "mine", "yours", "theirs", "ours", "us",
                    "him", "hers", "himself", "herself", "itself", "themselves", "ourselves", "yourself",
                    "yourselves", "am", "shall",
                    // Numbers
                    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
                    "21", "22", "23", "24", "25", "26", "27", "28", "29", "15", "31", "32", "33", "34", "35", "36", "37", "38", "39", "40",
                    "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "100", "1000",

                    // Spanish
                    "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "del", "al", "y", "o", "u", "en", "con", "por", "para",
                    "es", "que", "se", "no", "sí", "su", "sus", "le", "lo", "como", "más", "pero", "ya", "o", "muy", "sin", "sobre",
                    "entre", "también", "hasta", "desde", "todo", "todos", "todas", "toda", "mi", "mis", "tu", "tus", "su", "sus",
                    "este", "esta", "estos", "estas", "ese", "esa", "esos", "esas", "aquel", "aquella", "aquellos", "aquellas",
                    "yo", "tú", "él", "ella", "nosotros", "vosotros", "ellos", "ellas", "me", "te", "se", "nos", "os", "les",

                    // French
                    "le", "la", "les", "un", "une", "des", "du", "de", "en", "et", "à", "au", "aux", "pour", "par", "sur", "dans",
                    "est", "ce", "cette", "ces", "il", "elle", "ils", "elles", "nous", "vous", "tu", "je", "me", "te", "se", "leur",
                    "lui", "son", "sa", "ses", "mon", "ma", "mes", "ton", "ta", "tes", "notre", "nos", "votre", "vos", "leur", "leurs",
                    "qui", "que", "quoi", "dont", "où", "quand", "comment", "pourquoi", "avec", "sans", "sous", "entre", "aussi",
                    "plus", "moins", "très", "bien", "mal", "comme", "mais", "ou", "donc", "or", "ni", "car",

                    // German
                    "der", "die", "das", "ein", "eine", "einer", "eines", "einem", "einen", "und", "oder", "aber", "den", "dem", "des",
                    "zu", "mit", "auf", "für", "von", "an", "im", "in", "am", "aus", "bei", "nach", "über", "unter", "vor", "zwischen",
                    "ist", "war", "sind", "sein", "hat", "haben", "wird", "werden", "nicht", "kein", "keine", "mehr", "weniger", "auch",
                    "nur", "schon", "noch", "immer", "man", "wir", "ihr", "sie", "er", "es", "ich", "du", "mein", "dein", "sein", "ihr",
                    "unser", "euer", "dies", "diese", "dieser", "dieses", "jener", "jene", "jenes",

                    // Italian
                    "il", "lo", "la", "i", "gli", "le", "un", "una", "uno", "dei", "delle", "degli", "del", "della", "dello", "dei",
                    "e", "o", "ma", "per", "con", "su", "tra", "fra", "di", "da", "a", "al", "ai", "agli", "alla", "alle", "allo",
                    "che", "chi", "cui", "come", "quando", "dove", "perché", "quale", "quali", "questo", "questa", "questi", "queste",
                    "quello", "quella", "quelli", "quelle", "io", "tu", "lui", "lei", "noi", "voi", "mi", "ti", "si", "ci", "vi",

                    // Portuguese
                    "o", "a", "os", "as", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
                    "por", "para", "com", "sem", "sobre", "entre", "e", "ou", "mas", "também", "como", "mais", "menos", "muito", "pouco",
                    "já", "ainda", "só", "todo", "toda", "todos", "todas", "meu", "minha", "meus", "minhas", "teu", "tua", "teus", "tuas",
                    "seu", "sua", "seus", "suas", "nosso", "nossa", "nossos", "nossas", "vosso", "vossa", "vossos", "vossas", "ele", "ela",
                    "eles", "elas", "nós", "vós", "eu", "tu", "você", "vocês", "lhe", "lhes", "me", "te", "se", "nos", "vos",

                    // Dutch
                    "de", "het", "een", "en", "of", "maar", "want", "dus", "voor", "na", "met", "zonder", "over", "onder", "tussen",
                    "in", "op", "aan", "bij", "tot", "van", "uit", "door", "om", "tot", "als", "dan", "dat", "die", "dit", "deze",
                    "die", "wie", "wat", "waar", "wanneer", "hoe", "waarom", "welke", "wij", "jij", "hij", "zij", "het", "ik", "je",
                    "mijn", "jouw", "zijn", "haar", "ons", "onze", "hun", "uw", "hun", "ze", "u", "men", "er", "hier", "daar",

                    // Russian (transliterated)
                    "i", "a", "no", "da", "net", "on", "ona", "ono", "oni", "my", "vy", "ty", "ya", "moy", "tvoy", "ego", "ee", "nas",
                    "vas", "ikh", "kto", "chto", "gde", "kogda", "pochemu", "kak", "eto", "v", "na", "s", "k", "o", "po", "za", "ot",
                    "do", "iz", "u", "nad", "pod", "pervyy", "vtoroy", "odin", "dva", "tri", "chetyre", "pyat", "shest", "sem", "vosem",
                    "devyat", "desyat", "bolshe", "menshe", "vse", "vsyo", "vsego", "eto", "tak", "zdes", "tam", "tut", "to", "eto",

                    // Chinese (pinyin, most common stopwords)
                    "de", "shi", "bu", "le", "zai", "ren", "wo", "ni", "ta", "men", "zhe", "na", "yi", "ge", "you", "he", "ye", "ma",
                    "ba", "ne", "li", "dui", "dao", "zai", "shang", "xia",

                    // Japanese (romaji, common particles and pronouns)
                    "no", "ni", "wa", "ga", "wo", "de", "to", "mo", "kara", "made", "yori", "e", "ka", "ne", "yo", "kore", "sore", "are",
                    "dore", "kono", "sono", "ano", "dono", "watashi", "anata", "kare", "kanojo", "watashitachi", "anatatachi", "karera",
                    "kanojotachi", "koko", "soko", "asoko", "doko", "itsu", "dare", "nani", "nan", "ikutsu", "ikura", "doushite", "dou",

                    // Turkish
                    "ve", "bir", "bu", "da", "de", "için", "ile", "ama", "veya", "çok", "az", "daha", "en", "gibi", "mi",
                    "mu", "mü", "ben", "sen", "o", "biz", "siz", "şu", "bu", "şey", "her", "hiç", "bazı", "bazı", "bazı",

                    // Arabic (transliterated)
                    "wa", "fi", "min", "ila", "an", "ala", "ma", "la", "huwa", "hiya", "anta", "anti", "nahnu", "antum", "antunna",
                    "hum", "hunna", "hadha", "hadhi", "dhalika", "tilka", "huna", "hunaka", "ayna", "mata", "kayfa", "limadha",

                    // Hindi (transliterated)
                    "hai", "ka", "ki", "ke", "mein", "par", "aur", "ya", "lekin", "bhi", "ko", "se", "tak", "ko", "mein", "tum", "main",
                    "vah", "yeh", "ham", "aap", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka", "unka",

                    // Polish
                    "i", "w", "na", "z", "do", "o", "za", "po", "przez", "dla", "od", "bez", "pod", "nad", "przy", "między",
                    "jest", "być", "był", "była", "było", "byli", "były", "ten", "ta", "to", "ci", "te", "tam", "tu", "kto",
                    "co", "gdzie", "kiedy", "jak", "dlaczego", "który", "która", "które", "którzy",

                    // Scandinavian (Danish, Norwegian, Swedish)
                    "och", "att", "det", "som", "en", "ett", "den", "de", "på", "av", "med", "till", "för", "från", "är", "var", "har",
                    "hade", "inte", "men", "om", "eller", "så", "vi", "ni", "han", "hon", "de", "vi", "ni", "jag", "du", "mig", "dig",

                    // Greek (transliterated)
                    "kai", "se", "apo", "me", "gia", "os", "stin", "sto", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin",
                    "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin", "stin",

                    // Add more as needed for other languages...

                    // Extended with most common tokens from crawls:
                    "copyright", "privacy", "use", "rights", "contact", "new", "content", "home", "reserved", "search", "skip", "get",
                    "help", "2025", "one", "terms", "these", "work", "list", "travel", "business", "information", "please", "services",
                    "first", "support", "service", "online", "make", "sign", "see", "options", "site", "start", "policy", "english",
                    "create", "find", "version", "way", "close", "inc", "available", "review", "free", "world", "related", "account",
                    "security", "careers", "press", "time", "need", "part", "using", "top", "access", "here", "statement", "data",
                    "conditions", "best", "modern", "there", "software", "events", "check", "know", "features", "view", "company",
                    "news", "partner", "current", "staff", "awards", "open", "property", "language", "like", "human", "discover",
                    "stay", "community", "show", "centre", "without", "end", "become", "login", "continue", "set", "articles", "date",
                    "unique", "people", "safety", "customer", "room", "resource", "interest", "changes", "booking.com", "faqs",
                    "slavery", "foundation", "sustainability", "usd", "yet", "holdings", "1996–2025", "register", "license", "day",
                    "well", "project", "countries", "corporate", "don't", "popular", "within", "real", "calendar", "covid-19",
                    "facebook", "adding", "reviews", "choose", "international", "next", "cookie", "regions", "friendly", "used",
                    "page", "good", "based", "seasonal", "guidelines", "explore", "including", "places", "relations", "great",
                    "monthly", "mobile", "words", "leave", "days", "guest", "million", "provides", "code", "latest", "verify", "tell",
                    "many", "finally", "development", "offers", "read", "deals", "team", "cookies", "hire", "leader", "traveller",
                    "type", "united", "take", "experience", "followed", "learn", "languages", "holiday", "website", "hotels", "that's",
                    "trip", "research", "affiliate", "public", "starts", "back", "coronavirus", "cities", "place", "started",
                    "verified", "now", "email", "location", "extranet", "number", "investor", "stays", "homes", "group", "guests",
                    "try", "flight", "resources", "been", "resorts", "reporting", "accommodation", "area", "houses", "name",
                    "airport", "hostels", "around", "hotel", "different", "villas", "finder", "source", "districts", "dispute",
                    "airports", "less", "center", "dialog", "digital", "quiet", "b&bs", "apartments", "select", "offer", "private",
                    "they're", "download", "restaurant", "full", "local", "agents", "menu", "booked", "provide", "authenticity",
                    "stayed", "naughty", "watch", "written", "key", "check-in", "overview", "include", "personal", "follow", "issues",
                    "every", "taxis", "flights", "technology", "manage", "rentals", "dates", "via", "map", "check-out", "two", "right",
                    "tools", "questions", "thanks", "book", "advanced", "various", "2024", "add", "process", "details", "users",
                    "enter", "management", "getting", "states", "blog", "history", "reservations", "guide", "logo", "easy",
                    "trademarks", "performance", "last", "additional", "large", "general", "following", "accessibility", "across",
                    "located", "special", "facilities", "much", "system", "media", "provided", "long", "views", "attractions", "value",
                    "building", "documentation", "working", "shimbun", "university", "welcome", "price", "web", "case", "found",
                    "another", "small", "click", "report", "children", "log", "april", "central", "health", "permission", "possible",
                    "even", "month", "near", "those", "reservation", "settings", "social", "apply", "projects", "making", "park",
                    "release", "note", "address", "wifi", "high", "links", "short", "share", "design", "prices", "feather", "rooms",
                    "systems", "family", "user", "city", "night", "registered", "food", "change", "adults", "global", "currency",
                    "offering", "specific", "learning", "google", "look", "money", "air", "life", "away", "style", "perfect", "cost",
                    "throughout", "build", "library", "status", "state", "mailing", "applications", "internet", "everything", "areas",
                    "needs", "level", "things", "being", "years", "request", "required", "issue", "visit", "partners", "activities",
                    "parking", "releases", "kingdom", "globe", "house", "lists", "event", "fitness", "score", "south", "policies",
                    "filter", "better", "results", "利用規約", "faq", "space", "availability", "api", "course", "flexible", "各国語サイト",
                    "asia&japan", "quality", "table", "reproduction", "file", "highly", "network", "storage", "recent", "bar",
                    "cnn.co.jp", "products", "always", "u.s", "save", "desk", "year", "cases", "looking", "application", "past",
                    "minutes", "future", "好書好日", "example", "star", "info", "multiple", "bed", "japan", "join", "something",
                    "subject", "dollar", "still", "big", "hours", "shared", "common", "outdoor", "order", "link", "nice", "extra",
                    "walk", "quick", "republication", "impact", "enjoy", "description", "topics", "situated", "costs", "clean",
                    "needed", "2023", "reports", "distance", "requests", "whether", "2.0", "サイトマップ", "control", "shopping", "old",
                    "computer", "sponsorship", "range", "kitchen", "machine", "office", "run", "week", "important", "contribute",
                    "feature", "includes", "board", "update", "outside",



                    "instagram", "students", "science", "education", "newsletter", "youtube", "twitter", "live", "linkedin", "video",
                    "notice", "student", "stories", "study", "store", "professional", "opportunities", "training", "collection",
                    "plan", "school", "daily", "studies", "2021", "academic", "jobs", "career", "feedback", "enterprise", "campus",
                    "legal", "submit", "living", "innovation", "financial", "tech", "brand", "sport", "funding", "environment",
                    "2022", "courses", "national", "español", "care", "meet", "program", "choices", "skills", "connect", "culture",
                    "college", "art", "teaching", "alumni", "test", "browser", "solutions", "sciences", "product", "knowledge",
                    "improve", "times", "navigation", "podcasts", "2020", "institute", "engagement", "action", "government",
                    "groups", "sports", "hub", "podcast", "consumer", "communication", "members", "material", "categories",
                    "leadership", "freedom", "agreement", "industry", "app", "delivery", "shop", "engineering", "schools", "series",
                    "updated", "opportunity", "form", "focus", "benefits", "sell", "strategy", "subscribe", "planning", "edition", 
                    "department", "executive", "degree", "today", "return", "gift", "2018", "recently", "2019", "job", "give", 
                    "guidance", "newsletters", "purchase", "protection", "undergraduate", "premium", "arts", "payments", "together", 
                    "diversity", "developer", "trust", "festival", "partnerships", "graduate", "ways", "videos", "platform", "expand", 
                    "mission", "involved", "women", "north", "types", "works", "develop", "breadcrumb", "usa", "buy", "published", 
                    "black", "updates", "understand", "postgraduate", "ideas", "listed", "further", "loading", "track", "three", 
                    "member", "category", "directory", "keep", "researchers", "law", "ambiente", "searches", "solo", "requirements", 
                    "medical", "summary", "models", "notifications", "model", "governance", "moda", "field", "line", "tips", "selected", 
                    "practice", "insights", "san", "having", "match", "2017", "messages", "medicine", "virtual", "power", "cards", 
                    "argentina", "publications", "futuro", "collections", "role", "finance", "cultura", "early", "green", "post", 
                    "saved", "archive", "apr", "higher", "programs", "inclusion", "chile", "italiano", "white", "organization", 
                    "departments", "experiences", "technical", "america", "guides", "items", "advice", "asked", "call", "teams", 
                    "computing", "2016", "materials", "window", "programme", "text", "standard", "means", "secure", "act", "since", 
                    "sites", "china", "due", "understanding", "pricing", "books", "transfer", "framework", "ensure", "mundo", 
                    "individual", "american", "marketing", "feed", "challenges", "months", "creative", "europa", "size", "ask", 
                    "plans", "africa", "pay", "march", "helps", "fast", "casa", "points", "award", "vision", "featured", "left", 
                    "giving", "rare", "conference", "potential", "chat", "analysis", "gallery", "phone", "complete", "mar", 
                    "technologies", "january", "country", "spaces", "fees", "shipping", "music", "body", "sizes", "single", "designed", 
                    "external", "society", "sales", "able", "websites", "artificial", "film", "maps", "colombia", "fund", "lab", 
                    "records", "connected", "it's", "royal", "send", "era", "images", "values", "developers", "love", "collaboration", 
                    "excellence", "server", "annual", "receive", "you're", "activity", "guarantee", "sellers", "champions", 
                    "article", "credit", "play", "young", "david", "icon", "españa", "pro", "print", "creating", "tiktok", 
                    "architecture", "cart", "sharing", "frequently", "success", "previous", "september", "processes", "selling", 
                    "uses", "section", "strategic", "healthcare", "games", "academy", "base", "directly", "experts", "item", 
                    "lifestyle", "cloud", "core", "visual", "opens", "returns", "otherwise", "studio", "partnership", "james", 
                    "beyond", "director", "apps", "london", "administration", "approach", "toggle", "keyword", "developing", 
                    "accounts", "touch", "wellbeing", "makes", "viewed", "environmental", "desktop", "scholarships", "survey", 
                    "york", "steps", "june", "portal", "communities", "structure", "institutes", "wide", "accept", "image", "parte", 
                    "risk", "john", "league", "politics", "cook", "docs", "organizations", "safe", "term", "1995-2025", "applying", 
                    "adchoice", "beauty", "senior", "east", "running", "physical", "clear", "operations", "supporting", "progress", 
                    "providing", "essential", "responsible", "writing", "summer", "card", "programmes", "profile", "canada", 
                    "currently", "networks", "supported", "non", "it's", "class", "against", "magazine", "positive", "leading", 
                    "official", "exchange", "standards", "condition", "sustainable", "registration", "mental", "archives", "contacts", 
                    "trump", "applied", "collaborate", "managing", "manager", "watchlist", "final", "india", "documents", "growth", 
                    "analytics", "quickly", "méxico", "mini", "british", "alto", "story", "animal", "useful", "radio", "others", 
                    "existing", "initiatives", "water", "february", "journey", "actions", "civil", "active", "original", "italia", 
                    "forms", "method", "committee", "tab", "benefit", "garage", "universities", "august", "outlet", "publication", 
                    "box", "communications", "employment", "connecting", "upcoming", "reference", "tool", "2015", "vault", "energy", 
                    "locations", "sign-in", "auto", "infrastructure", "2012", "futura", "ready", "relevant", "speed", "host", 
                    "intelligence", "audio", "browse", "lead", "function", "president", "grants", "serie",

                ];
                let date_regex = regex::Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}$");
                let date2_regex = regex::Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$");
                let date3_regex = regex::Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$");
                let date4_regex = regex::Regex::new(r"^\d{8}$");
                let date5_regex = regex::Regex::new(r"^\d{4}\.\d{1,2}\.\d{1,2}$");
                let date6_regex = regex::Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{4}$");
                let date7_regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?(Z|([+-]\d{2}:\d{2}))?)?$");

                tokens.retain(|token| {
                    !common_tokens.contains(&token.as_str())
                        || date_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date2_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date3_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date4_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date5_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date6_regex.as_ref().map_or(false, |re| re.is_match(token))
                        || date7_regex.as_ref().map_or(false, |re| re.is_match(token))
                });
                tokens.retain(|token| token.len() > 2 && token.len() < 50);
                let url_tokens: HashSet<_> = url.split('/').map(|s| s.to_lowercase()).collect();
                tokens.retain(|token| !url_tokens.contains(&token.to_lowercase()));
                if let Ok(domain) = Url::parse(&url).and_then(|u| {
                    u.domain()
                        .map(|d| d.to_string())
                        .ok_or_else(|| ParseError::EmptyHost)
                }) {
                    let domain_tokens: HashSet<_> = domain.split('.').map(|s| s.to_lowercase()).collect();
                    tokens.retain(|token| !domain_tokens.contains(&token.to_lowercase()));
                }

                let mut all_tokens = mime_tokens;
                all_tokens.extend(tokens);
                tokens = all_tokens;

                page.tokens = tokens;

                // Filter links: keep only those that start with "http://" or "https://", and do not start with "data:"
                links.retain(|link| {
                    let link_lc = link.to_ascii_lowercase();
                    (link_lc.starts_with("http://") || link_lc.starts_with("https://"))
                        && !link_lc.starts_with("data:")
                });

                page.links = links;
                
                
                if let Err(e) = page.save_async(established_client).await {
                    log::warn!("Failed to save page for {}: {}", url, e);
                    write_url_to_retry_file(&url).await;
                }
                
                
            } else {
                write_url_to_retry_file(&url).await;
            }
        }
        Err(e) => {
            log::warn!("Error fetching URL {}: {}", url, e);

            write_url_to_retry_file(&url).await;

            // If the error is a timeout, increment a static counter and occasionally sleep all threads
            
            let err_str = e.to_string().to_ascii_lowercase();
            if err_str.contains("timed out") || err_str.contains("timeout") {
                let mut count = TIMEOUT_COUNT.lock().unwrap();
                *count += 1;
                if *count % 10 == 0 {
                    // Set global sleep for all threads for a random duration between 10 and 120 seconds
                    let mut rng = rand::thread_rng();
                    let sleep_secs = rng.gen_range(10..=120);
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let until = now + sleep_secs;
                    SLEEP_UNTIL.store(until, Ordering::SeqCst);
                    log::warn!("Timeout detected {} times, sleeping ALL threads for {} seconds to avoid ban", *count, sleep_secs);
                }
            }
        }
    }

    if depth < max_depth && !page.links.is_empty() {
        let job_oid = page.crawl_job_oid.clone();
        let links = page.links.clone();
        // Use buffer_unordered instead of FuturesUnordered for concurrency limiting
        use futures::stream;
        let fut_stream = stream::iter(links.into_iter().map(move |link| {
            let job_oid = job_oid.clone();
            let link = link.clone();
            async move {
                let _ = crawl_url_boxed(job_oid, link, depth + 1, established_client).await;
            }
        }));
        // Limit concurrency to 16
        fut_stream.buffer_unordered(num_cpus::get()).for_each(|_| async {}).await;
    } else {
        if !page.links.is_empty() {
            for link in &page.links {
                write_url_to_retry_file(link).await;
            }
        }
    }
    Ok(page)
}

// Boxed async fn for recursion
fn crawl_url_boxed<'a>(job_oid: String, url: String, depth: usize, established_client: Option<&'a tokio_postgres::Client>) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::sam::memory::Result<CrawledPage>> + Send + 'a>> {
    Box::pin(crawl_url_inner(job_oid, url, depth, established_client))
}

// Public entry point (non-recursive, just calls boxed version)
pub async fn crawl_url(job_oid: String, url: String, established_client: Option<&tokio_postgres::Client>) -> crate::sam::memory::Result<CrawledPage> {
    crawl_url_boxed(job_oid, url, 0, established_client).await
}

pub fn start_service() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        log::info!("Crawler service starting...");
        CRAWLER_RUNNING.store(true, Ordering::SeqCst);

        let cpu_cores = num_cpus::get();

        // Only create a runtime if not already inside one
        if tokio::runtime::Handle::try_current().is_ok() {
            // Already inside a runtime: spawn the service on 8 tasks
           
            // To increase stack size for each task, spawn a thread with a larger stack and run the async task inside a new runtime.
            std::thread::Builder::new()
                .name("crawler-service".to_string())
                .stack_size(4 * 1024 * 1024) // 4 MB stack
                .spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build Tokio runtime");
                    rt.block_on(async {
                        run_crawler_service().await;
                    });
                })
                .expect("Failed to spawn crawler-service thread");
            
        } else {
            // Not inside a runtime: spawn a thread and create a runtime
            // Spawn 8 threads, each with its own Tokio runtime running the crawler service
          
            std::thread::spawn(|| {
                match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .thread_stack_size(4 * 1024 * 1024) // 4 MB stack
                    .build() {
                    Ok(rt) => {
                        rt.block_on(async {
                            run_crawler_service().await;
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to create Tokio runtime: {}", e);
                    }
                }
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
pub async fn run_crawler_service() -> crate::sam::memory::Result<()> {
    use trust_dns_resolver::config::*;
    log::set_max_level(LevelFilter::Info);
    let crawling = Arc::new(TokioMutex::new(()));

    let established_client = Arc::new(crate::sam::memory::Config::client_async().await?);


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
        "https://www.sussex.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.oxfordbrookes.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.lboro.ac.uk/",
        "https://www.le.ac.uk/",
        "https://www.derby.ac.uk/",
        "https://www.lincoln.ac.uk/",
        "https://www.hull.ac.uk/",
        "https://www.york.ac.uk/",
        "https://www.durham.ac.uk/",
        "https://www.northumbria.ac.uk/",
        "https://www.sunderland.ac.uk/",
        "https://www.tees.ac.uk/",
        "https://www.edgehill.ac.uk/",
        "https://www.lancaster.ac.uk/",
        "https://www.uclan.ac.uk/",
        "https://www.ljmu.ac.uk/",
        "https://www.mmu.ac.uk/",
        "https://www.salford.ac.uk/",
        "https://www.bradford.ac.uk/",
        "https://www.hud.ac.uk/",
        "https://www.shef.ac.uk/",
        "https://www.southwales.ac.uk/",
        "https://www.cardiff.ac.uk/",
        "https://www.bangor.ac.uk/",
        "https://www.swansea.ac.uk/",
        "https://www.aber.ac.uk/",
        "https://www.glyndwr.ac.uk/",
        "https://www.cardiffmet.ac.uk/",
        "https://www.wrexham.ac.uk/",
        "https://www.uwtsd.ac.uk/",
        "https://www.st-andrews.ac.uk/",
        "https://www.abdn.ac.uk/",
        "https://www.dundee.ac.uk/",
        "https://www.hw.ac.uk/",
        "https://www.rgu.ac.uk/",
        "https://www.strath.ac.uk/",
        "https://www.gla.ac.uk/",
        "https://www.ed.ac.uk/",
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
        "https://www.chi.ac.uk/",
        "https://www.bucks.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.staffs.ac.uk/",
        "https://www.keele.ac.uk/",
        "https://www.wlv.ac.uk/",
        "https://www.coventry.ac.uk/",
        "https://www.warwick.ac.uk/",
        "https://www.nottstrent.ac.uk/",
        "https://www.derby.ac.uk/",
        "https://www.lincoln.ac.uk/",
        "https://www.hull.ac.uk/",
        "https://www.leeds.ac.uk/",
        "https://www.liverpool.ac.uk/",
        "https://www.manchester.ac.uk/",
        "https://www.bristol.ac.uk/",
        "https://www.bath.ac.uk/",
        "https://www.exeter.ac.uk/",
        "https://www.plymouth.ac.uk/",
        "https://www.southampton.ac.uk/",
        "https://www.sussex.ac.uk/",
        "https://www.surrey.ac.uk/",
        "https://www.kent.ac.uk/",
        "https://www.essex.ac.uk/",
        "https://www.herts.ac.uk/",
        "https://www.beds.ac.uk/",
        "https://www.brookes.ac.uk/",
        "https://www.oxfordbrookes.ac.uk/",
        "https://www.bournemouth.ac.uk/",
        "https://www.solent.ac.uk/",
        "https://www.winchester.ac.uk/",
        "https://www.soton.ac.uk/",
        "https://www.port.ac.uk/",
        "https://www.anglia.ac.uk/",
        "https://www.aru.ac.uk/",
        "https://www.eastanglia.ac.uk/",
        "https://www.cam.ac.uk/",
        "https://www.lboro.ac.uk/",
        "https://www.le.ac.uk/",
        "https://www.derby.ac.uk/",
        "https://www.lincoln.ac.uk/",
        "https://www.hull.ac.uk/",
        "https://www.york.ac.uk/",
        "https://www.durham.ac.uk/",
        "https://www.northumbria.ac.uk/",
        "https://www.sunderland.ac.uk/",
        "https://www.tees.ac.uk/",
        "https://www.edgehill.ac.uk/",
        "https://www.lancaster.ac.uk/",
        "https://www.uclan.ac.uk/",
        "https://www.ljmu.ac.uk/",
        "https://www.mmu.ac.uk/",
        "https://www.salford.ac.uk/",
        "https://www.bradford.ac.uk/",
        "https://www.hud.ac.uk/",
        "https://www.shef.ac.uk/",
        "https://www.southwales.ac.uk/",
        "https://www.cardiff.ac.uk/",
        "https://www.bangor.ac.uk/",
        "https://www.swansea.ac.uk/",
        "https://www.aber.ac.uk/",
        "https://www.glyndwr.ac.uk/",
        "https://www.cardiffmet.ac.uk/",
        "https://www.wrexham.ac.uk/",
        "https://www.uwtsd.ac.uk/",
        "https://www.st-andrews.ac.uk/",
        "https://www.abdn.ac.uk/",
        "https://www.dundee.ac.uk/",
        "https://www.hw.ac.uk/",
        "https://www.rgu.ac.uk/",
        "https://www.strath.ac.uk/",
        "https://www.gla.ac.uk/",
        "https://www.ed.ac.uk/",
        "https://www.tiktok.com/",
        "https://www.snapchat.com/",
        "https://www.whatsapp.com/",
        "https://www.telegram.org/",
        "https://www.signal.org/",
        "https://www.wechat.com/",
        "https://www.line.me/",
        "https://www.vk.com/",
        "https://www.ok.ru/",
        "https://www.baidu.com/",
        "https://www.taobao.com/",
        "https://www.jd.com/",
        "https://www.sohu.com/",
        "https://www.sina.com.cn/",
        "https://www.163.com/",
        "https://www.qq.com/",
        "https://www.aliexpress.com/",
        "https://www.alibaba.com/",
        "https://www.yandex.ru/",
        "https://www.mail.ru/",
        "https://www.rambler.ru/",
        "https://www.naver.com/",
        "https://www.daum.net/",
        "https://www.coupang.com/",
        "https://www.zalando.de/",
        "https://www.otto.de/",
        "https://www.lazada.com/",
        "https://www.shopee.com/",
        "https://www.flipkart.com/",
        "https://www.olx.in/",
        "https://www.mercadolibre.com.ar/",
        "https://www.mercadolivre.com.br/",
        "https://www.uol.com.br/",
        "https://www.globo.com/",
        "https://www.nikkei.com/",
        "https://www.yahoo.co.jp/",
        "https://www.dmm.com/",
        "https://www.rakuten.co.jp/",
        "https://www.auone.jp/",
        "https://www.livedoor.com/",
        "https://www.nicovideo.jp/",
        "https://www.booking.cn/",
        "https://www.trivago.com/",
        "https://www.hotels.com/",
        "https://www.agoda.com/",
        "https://www.expedia.co.jp/",
        "https://www.cdiscount.com/",
        "https://www.leboncoin.fr/",
        "https://www.orange.fr/",
        "https://www.free.fr/",
        "https://www.sfr.fr/",
        "https://www.bouyguestelecom.fr/",
        "https://www.ebay.co.uk/",
        "https://www.rightmove.co.uk/",
        "https://www.autotrader.co.uk/",
        "https://www.gumtree.com/",
        "https://www.zillow.com/",
        "https://www.realtor.com/",
        "https://www.trulia.com/",
        "https://www.redfin.com/",
        "https://www.craigslist.org/",
        "https://www.kijiji.ca/",
        "https://www.canada.ca/",
        "https://www.cbc.ca/",
        "https://www.abc.net.au/",
        "https://www.smh.com.au/",
        "https://www.seek.com.au/",
        "https://www.domain.com.au/",
        "https://www.realestate.com.au/",
        "https://www.gumtree.com.au/",
        "https://www.news.com.au/",
        "https://www.nzherald.co.nz/",
        "https://www.trademe.co.nz/",
        "https://www.stuff.co.nz/",
        "https://www.abc.es/",
        "https://www.elmundo.es/",
        "https://www.marca.com/",
        "https://www.as.com/",
        "https://www.elconfidencial.com/",
        "https://www.corriere.it/",
        "https://www.repubblica.it/",
        "https://www.ilsole24ore.com/",
        "https://www.gazzetta.it/",
        "https://www.lefigaro.fr/",
        "https://www.leparisien.fr/",
        "https://www.liberation.fr/",
        "https://www.zeit.de/",
        "https://www.sueddeutsche.de/",
        "https://www.faz.net/",
        "https://www.handelsblatt.com/",
        "https://www.spiegel.de/",
        "https://www.focus.de/",
        "https://www.nzz.ch/",
        "https://www.blick.ch/",
        "https://www.20min.ch/",
        "https://www.derstandard.at/",
        "https://www.kurier.at/",
        "https://www.krone.at/",
        "https://www.orf.at/",
        "https://www.hvg.hu/",
        "https://www.index.hu/",
        "https://www.origo.hu/",
        "https://www.delfi.ee/",
        "https://www.postimees.ee/",
        "https://www.err.ee/",
        "https://www.iltalehti.fi/",
        "https://www.is.fi/",
        "https://www.hs.fi/",
        "https://www.aftonbladet.se/",
        "https://www.expressen.se/",
        "https://www.dn.se/",
        "https://www.vg.no/",
        "https://www.dagbladet.no/",
        "https://www.aftenposten.no/",
        "https://www.politiken.dk/",
        "https://www.berlingske.dk/",
        "https://www.bt.dk/",
        "https://www.dr.dk/",
        "https://www.tv2.dk/",
        "https://www.rte.ie/",
        "https://www.irishtimes.com/",
        "https://www.independent.ie/",
        "https://www.thetimes.co.uk/",
        "https://www.ft.com/",
        "https://www.economist.com/",
        "https://www.spectator.co.uk/",
        "https://www.newyorker.com/",
        "https://www.theatlantic.com/",
        "https://www.nationalreview.com/",
        "https://www.politico.com/",
        "https://www.vox.com/",
        "https://www.buzzfeed.com/",
        "https://www.huffpost.com/",
        "https://www.slate.com/",
        "https://www.salon.com/",
        "https://www.vice.com/",
        "https://www.mashable.com/",
        "https://www.techcrunch.com/",
        "https://www.engadget.com/",
        "https://www.gizmodo.com/",
        "https://www.theverge.com/",
        "https://www.cnet.com/",
        "https://www.zdnet.com/",
        "https://www.tomshardware.com/",
        "https://www.pcworld.com/",
        "https://www.macrumors.com/",
        "https://www.androidcentral.com/",
        "https://www.xda-developers.com/",
        "https://www.imore.com/",
        "https://www.windowscentral.com/",
        "https://www.linux.com/",
        "https://www.phoronix.com/",
        "https://www.distrowatch.com/",
        "https://www.osnews.com/",
        "https://www.hackernews.com/",
        "https://www.producthunt.com/",
        "https://www.crunchbase.com/",
        "https://www.angel.co/",
        "https://www.cbinsights.com/",
        "https://www.pitchbook.com/",
        "https://www.dealroom.co/",
        "https://www.startupnation.com/",
        "https://www.startupgrind.com/",
        "https://www.seedrs.com/",
        "https://www.crowdcube.com/",
        "https://www.f6s.com/",
        "https://www.betalist.com/",
        "https://www.launchingnext.com/",
        "https://www.sideprojectors.com/",
        "https://www.indiehackers.com/",
        "https://www.remotive.io/",
        "https://www.weworkremotely.com/",
        "https://www.remoteok.com/",
        "https://www.flexjobs.com/",
        "https://www.dice.com/",
        "https://www.angel.co/jobs",
        "https://www.hired.com/",
        "https://www.toptal.com/",
        "https://www.upwork.com/",
        "https://www.freelancer.com/",
        "https://www.peopleperhour.com/",
        "https://www.guru.com/",
        "https://www.fiverr.com/",
        "https://www.99designs.com/",
        "https://www.designcrowd.com/",
        "https://www.crowdspring.com/",
        "https://www.topcoder.com/",
        "https://www.kaggle.com/",
        "https://www.dribbble.com/",
        "https://www.behance.net/",
        "https://www.coroflot.com/",
        "https://www.artstation.com/",
        "https://www.deviantart.com/",
        "https://www.pixiv.net/",
        "https://www.cgtrader.com/",
        "https://www.turbosquid.com/",
        "https://www.shutterstock.com/",
        "https://www.gettyimages.com/",
        "https://www.istockphoto.com/",
        "https://www.pexels.com/",
        "https://www.unsplash.com/",
        "https://www.pixabay.com/",
        "https://www.canva.com/",
        "https://www.figma.com/",
        "https://www.sketch.com/",
        "https://www.adobe.com/products/xd.html",
        "https://www.invisionapp.com/",
        "https://www.marvelapp.com/",
        "https://www.protopie.io/",
        "https://www.framer.com/",
        "https://www.webflow.com/",
        "https://www.wix.com/",
        "https://www.squarespace.com/",
        "https://www.weebly.com/",
        "https://www.site123.com/",
        "https://www.jimdo.com/",
        "https://www.strikingly.com/",
        "https://www.carrd.co/",
        "https://www.unbounce.com/",
        "https://www.instapage.com/",
        "https://www.leadpages.com/",
        "https://www.clickfunnels.com/",
        "https://www.mailchimp.com/",
        "https://www.constantcontact.com/",
        "https://www.sendinblue.com/",
        "https://www.getresponse.com/",
        "https://www.aweber.com/",
        "https://www.campaignmonitor.com/",
        "https://www.activecampaign.com/",
        "https://www.hubspot.com/",
        "https://www.salesforce.com/",
        "https://www.zoho.com/",
        "https://www.pipedrive.com/",
        "https://www.freshworks.com/",
        "https://www.insightly.com/",
        "https://www.copper.com/",
        "https://www.nimble.com/",
        "https://www.keap.com/",
        "https://www.sugarcrm.com/",
        "https://www.vtiger.com/",
        "https://www.bitrix24.com/",
        "https://www.monday.com/",
        "https://www.asana.com/",
        "https://www.trello.com/",
        "https://www.clickup.com/",
        "https://www.wrike.com/",
        "https://www.smartsheet.com/",
        "https://www.airtable.com/",
        "https://www.notion.so/",
        "https://www.coda.io/",
        "https://www.quip.com/",
        "https://www.zoho.com/projects/",
        "https://www.basecamp.com/",
        "https://www.teamwork.com/",
        "https://www.proofhub.com/",
        "https://www.redbooth.com/",
        "https://www.flow.com/",
        "https://www.taskworld.com/",
        "https://www.meistertask.com/",
        "https://www.todoist.com/",
        "https://www.ticktick.com/",
        "https://www.any.do/",
        "https://www.rememberthemilk.com/",
        "https://www.evernote.com/",
        "https://www.onenote.com/",
        "https://www.simplenote.com/",
        "https://www.bear.app/",
        "https://www.notability.com/",
        "https://www.goodnotes.com/",
        "https://www.milanote.com/",
        "https://www.obsidian.md/",
        "https://www.roamresearch.com/",
        "https://www.logseq.com/",
        "https://www.zettlr.com/",
        "https://www.typora.io/",
        "https://www.marktext.app/",
        "https://www.stackedit.io/",
        "https://www.dillinger.io/",
        "https://www.overleaf.com/",
        "https://www.authorea.com/",
        "https://www.sharelatex.com/",
        "https://www.pandoc.org/",
        "https://www.latex-project.org/",
        "https://www.ctan.org/",
        "https://www.arxiv.org/",
        "https://www.biorxiv.org/",
        "https://www.medrxiv.org/",
        "https://www.researchgate.net/",
        "https://www.academia.edu/",
        "https://www.jstor.org/",
        "https://www.springer.com/",
        "https://www.elsevier.com/",
        "https://www.taylorandfrancis.com/",
        "https://www.cambridge.org/",
        "https://www.oxfordjournals.org/",
        "https://www.nature.com/",
        "https://www.sciencemag.org/",
        "https://www.cell.com/",
        "https://www.thelancet.com/",
        "https://www.ssrn.com/",
        "https://www.ieee.org/",
        "https://www.acm.org/",
        "https://www.usenix.org/",
        "https://www.siggraph.org/",
        "https://www.sigmod.org/",
        "https://www.sigplan.org/",
        "https://www.sigcomm.org/",
        "https://www.sigchi.org/",
        "https://www.sigir.org/",
        "https://www.sigkdd.org/",
        "https://www.sigmetrics.org/",
        "https://www.sigops.org/",
        "https://www.sigsoft.org/",
        "https://www.sigmobile.org/",
        "https://www.sigmicro.org/",
        "https://www.sigbed.org/",
        "https://www.sigbioinformatics.org/",
        "https://www.sigcse.org/",
        "https://www.siggraph.org/",
        "https://www.sigmod.org/",
        "https://www.sigplan.org/",
        "https://www.sigcomm.org/",
        "https://www.sigchi.org/",
        "https://www.sigir.org/",
        "https://www.sigkdd.org/",
        "https://www.sigmetrics.org/",
        "https://www.sigops.org/",
        "https://www.sigsoft.org/",
        "https://www.sigmobile.org/",
        "https://www.sigmicro.org/",
        "https://www.sigbed.org/",
        "https://www.sigbioinformatics.org/",
        "https://www.sigcse.org/",
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

    // Helper function to perform a single DNS lookup with cache
    async fn lookup_domain(
        resolver: &TokioAsyncResolver,
        domain: &str,
    ) -> bool {
        // Check cache first
        {
            let cache = DNS_LOOKUP_CACHE.lock().await;
            if let Some(found) = cache.get(domain) {
                return *found;
            }
        }
        // Not in cache, do DNS lookup
        let mut found = false;
        for attempt in 0..3 {
            let result = match tokio::time::timeout(
                Duration::from_secs(15), // Increased from 10 to 15
                resolver.lookup_ip(domain)
            ).await {
                Ok(Ok(lookup)) if lookup.iter().next().is_some() => {
                    // DNS exists, now check HTTP/HTTPS HEAD
                    let http_url = format!("http://{}/", domain);
                    let https_url = format!("https://{}/", domain);

                    // Try both HTTP and HTTPS in parallel, return true if either responds
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(15)) // Increased from 10 to 15
                        .redirect(reqwest::redirect::Policy::limited(5))
                        .build()
                        .ok();

                    if let Some(client) = client {
                        let mut http_ok = false;
                        let mut https_ok = false;
                        for http_attempt in 0..3 {
                            let http_fut = client.head(&http_url).send();
                            let https_fut = client.head(&https_url).send();
                            let result = tokio::time::timeout(
                                Duration::from_secs(15),
                                async {
                                    tokio::select! {
                                        resp = http_fut => resp.ok().map(|r| r.status().is_success() || r.status().is_redirection()),
                                        resp = https_fut => resp.ok().map(|r| r.status().is_success() || r.status().is_redirection()),
                                    }
                                }
                            ).await;
                            match result {
                                Ok(Some(true)) => {
                                    http_ok = true;
                                    break;
                                }
                                Ok(Some(false)) | Ok(None) | Err(_) => {
                                    log::warn!("HEAD request timed out or failed (attempt {}): {}", http_attempt + 1, domain);
                                }
                            }
                            sleep(Duration::from_millis(300)).await;
                        }
                        if http_ok || https_ok {
                            found = true;
                            break;
                        }
                    }
                    false
                }
                Ok(_) | Err(_) => {
                    log::warn!("DNS lookup timed out or failed (attempt {}): {}", attempt + 1, domain);
                    false
                }
            };
            if result {
                found = true;
                break;
            }
            sleep(Duration::from_millis(300)).await;
        }
        // Update cache (but don't save to disk here)
        {
            let mut cache = DNS_LOOKUP_CACHE.lock().await;
            cache.insert(domain.to_string(), found);
        }
        found
    }

    load_dns_cache().await;
    // cache_all_to_redis().await;

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
            job.updated_at = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    log::warn!("SystemTime before UNIX EPOCH: {:?}", e);
                    0
                }
            };
            // let _ = job.save_async().await;

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
                match crawl_url(job.oid.clone(), url.clone(), Some(&*established_client)).await {
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
                        write_url_to_retry_file(&url).await;
                    }
                }
            }
            // Mark job as done
            job.status = "done".to_string();
            job.updated_at = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(e) => {
                    log::warn!("SystemTime before UNIX EPOCH: {:?}", e);
                    0
                }
            };
            // let _ = job.save_async().await;
            info!("Finished crawl job: oid={}", job.oid);
        } else {
            // No jobs: scan common URLs and/or use DNS queries to find domains
            info!("No pending crawl jobs found. Crawling common URLs.");
            let mut urls_to_try: Vec<String> = common_urls.iter().map(|s| s.to_string()).collect();


            // Load retry URLs from the retry file and remove the file after loading
            let retry_path = "/opt/sam/tmp/crawl_retry.dmp";
            if let Ok(data) = fs::read_to_string(retry_path).await {
                let retry_urls: Vec<String> = data
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect();
                if !retry_urls.is_empty() {
                    log::info!("Loaded {} retry URLs from {}", retry_urls.len(), retry_path);
                    urls_to_try.extend(retry_urls);
                }
                // Remove the retry file after loading
                let _ = fs::remove_file(retry_path).await.unwrap_or_else(|_| {
                    log::warn!("Failed to remove retry file: {}", retry_path);
                });
            }



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


            
            // use tokio_stream::StreamExt;

            let mut rng = SmallRng::from_entropy();

            let mut domains = Vec::new();
            for tld in &tlds {
                let mut sampled_words = words.clone();
                sampled_words.shuffle(&mut rng);
                for word in sampled_words.iter() {
                    domains.push(format!("{}.{}", word, tld));
                    for prefix in &prefixes {
                        domains.push(format!("{}.{}.{}", prefix, word, tld));
                        // for word2 in sampled_words.iter() {
                        //     domains.push(format!("{}.{}.{}.{}", prefix, word, word2, tld));
                        // }
                    }
                }
                for prefix in &prefixes {
                    domains.push(format!("{}.{}", prefix, tld));
                }
                for word in sampled_words.iter() {
                    domains.push(format!("{}.{}", word, tld));
                }
            }
      

            // log::info!("Found {} domains to check", domains.len());
            // loop{}

            domains.sort();
            domains.dedup();
            domains.shuffle(&mut rng);

            let max_domains = 1000;
            let domains = &domains[..std::cmp::min(domains.len(), max_domains)];

            let mut urls_found = Vec::new();

            // Use concurrency to speed up DNS lookups
            let concurrency = num_cpus::get() * 8;
            let concurrency = std::cmp::max(concurrency, 50);

            let found_domains = tokio_stream::iter(domains.iter().cloned())
                .map(|domain| {
                    let resolver = resolver.clone();
                    async move {
                        if lookup_domain(&resolver, &domain).await {
                            Some(domain)
                        } else {
                            None
                        }
                    }
                })
                .buffer_unordered(concurrency)
                .filter_map(|opt| async move { opt })
                .collect::<Vec<String>>()
                .await;

            for domain in found_domains {
                urls_found.push(format!("https://{}/", domain));
            }
            urls_to_try.extend(urls_found);
            
            urls_to_try.sort();
            urls_to_try.dedup();

            // Increase concurrency for faster crawling
            let concurrency = num_cpus::get() * 8;
            let concurrency = std::cmp::max(concurrency, 50);
            let mut rng = SmallRng::from_entropy();

            let urls_to_try: Vec<String> = urls_to_try.into_iter().collect();

            let established_client = established_client.clone();
            tokio_stream::iter(urls_to_try)
                .for_each_concurrent(concurrency, move |url| {
                    let established_client = established_client.clone();
                    let dummy_job_oid: String = thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(15)
                        .map(char::from)
                        .collect();
                    async move {
                        match crawl_url_boxed(dummy_job_oid, url.clone(), 0, Some(&*established_client)).await {
                            Ok(_page) => log::info!("Crawled (no job): {}", url),
                            Err(e) => {
                                info!("Crawler error (no job): {}", e);
                                log::error!("Crawler error (no job): {}", e);
                            }
                        }
                    }
                })
                .await;
        }
        sleep(Duration::from_secs(10)).await;
    }
}

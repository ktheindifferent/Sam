//! Data export functionality for crawled content
//! 
//! This module provides export capabilities for crawled data in various formats
//! including JSON, CSV, XML, and custom formats.

use anyhow::Result;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use log::info;

/// Supported export formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    JsonLines,
    Csv,
    Xml,
    Html,
    Markdown,
    Sitemap,
    Custom(String),
}

/// Export options and filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_content: bool,
    pub include_metadata: bool,
    pub include_links: bool,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub domain_filter: Option<Vec<String>>,
    pub status_filter: Option<Vec<u16>>,
    pub content_type_filter: Option<Vec<String>>,
    pub max_records: Option<usize>,
    pub compress: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_content: true,
            include_metadata: true,
            include_links: true,
            date_from: None,
            date_to: None,
            domain_filter: None,
            status_filter: None,
            content_type_filter: None,
            max_records: None,
            compress: false,
        }
    }
}

/// Exported data container
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub export_info: ExportInfo,
    pub pages: Vec<ExportedPage>,
    pub statistics: ExportStatistics,
}

/// Export metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportInfo {
    pub export_date: DateTime<Utc>,
    pub export_format: String,
    pub total_records: usize,
    pub options: ExportOptions,
    pub crawler_version: String,
}

/// Simplified page data for export
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedPage {
    pub url: String,
    pub domain: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub status_code: i16,
    pub content_length: i64,
    pub language: Option<String>,
    pub links: Vec<String>,
    pub crawled_at: i64,
    pub metadata: HashMap<String, String>,
}

/// Export statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportStatistics {
    pub total_pages: usize,
    pub unique_domains: usize,
    pub total_size_bytes: i64,
    pub success_rate: f64,
    pub content_types: HashMap<String, usize>,
    pub status_codes: HashMap<u16, usize>,
    pub languages: HashMap<String, usize>,
}

/// Data exporter
pub struct DataExporter;

impl DataExporter {
    /// Export crawled data to file
    pub async fn export_to_file(
        path: &Path,
        options: ExportOptions,
    ) -> Result<ExportInfo> {
        info!("Starting data export to {:?} with format {:?}", path, options.format);
        
        // Fetch data from database
        let data = Self::fetch_data(&options).await?;
        
        // Export based on format
        let content = match options.format {
            ExportFormat::Json => Self::export_json(&data, &options)?,
            ExportFormat::JsonLines => Self::export_jsonlines(&data, &options)?,
            ExportFormat::Csv => Self::export_csv(&data, &options)?,
            ExportFormat::Xml => Self::export_xml(&data, &options)?,
            ExportFormat::Html => Self::export_html(&data, &options)?,
            ExportFormat::Markdown => Self::export_markdown(&data, &options)?,
            ExportFormat::Sitemap => Self::export_sitemap(&data, &options)?,
            ExportFormat::Custom(ref format) => {
                return Err(anyhow::anyhow!("Custom format '{}' not implemented", format));
            }
        };
        
        // Write to file (with optional compression)
        if options.compress {
            Self::write_compressed(path, content.as_bytes()).await?;
        } else {
            Self::write_file(path, content.as_bytes()).await?;
        }
        
        info!("Export completed: {} records written to {:?}", data.pages.len(), path);
        
        Ok(data.export_info)
    }
    
    /// Export crawled data to bytes
    pub async fn export_to_bytes(
        options: ExportOptions,
    ) -> Result<Vec<u8>> {
        let data = Self::fetch_data(&options).await?;
        
        let content = match options.format {
            ExportFormat::Json => Self::export_json(&data, &options)?,
            ExportFormat::JsonLines => Self::export_jsonlines(&data, &options)?,
            ExportFormat::Csv => Self::export_csv(&data, &options)?,
            ExportFormat::Xml => Self::export_xml(&data, &options)?,
            ExportFormat::Html => Self::export_html(&data, &options)?,
            ExportFormat::Markdown => Self::export_markdown(&data, &options)?,
            ExportFormat::Sitemap => Self::export_sitemap(&data, &options)?,
            ExportFormat::Custom(ref format) => {
                return Err(anyhow::anyhow!("Custom format '{}' not implemented", format));
            }
        };
        
        if options.compress {
            Self::compress_data(content.as_bytes())
        } else {
            Ok(content.into_bytes())
        }
    }
    
    /// Fetch data from database based on options
    async fn fetch_data(options: &ExportOptions) -> Result<ExportData> {
        // Get database connection
        let client = super::get_db_connection().await
            .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;
        
        // Build query based on filters
        let mut query = String::from(
            "SELECT cc.*, cp.links, cp.tokens 
             FROM crawled_content cc
             LEFT JOIN crawled_pages cp ON cc.url = cp.url
             WHERE 1=1"
        );
        
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = Vec::new();
        let mut param_idx = 1;
        
        // Apply filters
        if let Some(from) = &options.date_from {
            query.push_str(&format!(" AND cc.crawled_at >= ${}", param_idx));
            params.push(Box::new(from.timestamp()));
            param_idx += 1;
        }
        
        if let Some(to) = &options.date_to {
            query.push_str(&format!(" AND cc.crawled_at <= ${}", param_idx));
            params.push(Box::new(to.timestamp()));
            param_idx += 1;
        }
        
        if let Some(statuses) = &options.status_filter {
            let status_list = statuses.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            query.push_str(&format!(" AND cc.status_code IN ({})", status_list));
        }
        
        query.push_str(" ORDER BY cc.crawled_at DESC");
        
        if let Some(limit) = options.max_records {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        // Execute query - convert Box<dyn> to references
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
            params.iter().map(|p| p.as_ref()).collect();
        let rows = client.query(&query, &param_refs[..]).await?;
        
        // Convert to export format
        let mut pages = Vec::new();
        let mut statistics = ExportStatistics {
            total_pages: 0,
            unique_domains: 0,
            total_size_bytes: 0,
            success_rate: 0.0,
            content_types: HashMap::new(),
            status_codes: HashMap::new(),
            languages: HashMap::new(),
        };
        
        let mut domains = std::collections::HashSet::new();
        let mut success_count = 0;
        
        for row in rows {
            let url: String = row.get("url");
            let domain = url::Url::parse(&url)
                .ok()
                .and_then(|u| u.host_str().map(|s| s.to_string()))
                .unwrap_or_default();
            
            domains.insert(domain.clone());
            
            let status_code: i16 = row.get("status_code");
            if (200..300).contains(&status_code) {
                success_count += 1;
            }
            
            let content_type: Option<String> = row.get("content_type");
            if let Some(ct) = &content_type {
                *statistics.content_types.entry(ct.clone()).or_insert(0) += 1;
            }
            
            *statistics.status_codes.entry(status_code as u16).or_insert(0) += 1;
            
            let language: Option<String> = row.get("language");
            if let Some(lang) = &language {
                *statistics.languages.entry(lang.clone()).or_insert(0) += 1;
            }
            
            let content_length: i64 = row.get("content_length");
            statistics.total_size_bytes += content_length;
            
            let mut page = ExportedPage {
                url,
                domain,
                title: row.get("title"),
                description: row.get("description"),
                content: if options.include_content {
                    row.get("content_text")
                } else {
                    None
                },
                content_type,
                status_code,
                content_length,
                language,
                links: if options.include_links {
                    row.try_get("links").unwrap_or_default()
                } else {
                    Vec::new()
                },
                crawled_at: row.get("crawled_at"),
                metadata: HashMap::new(),
            };
            
            // Add metadata if requested
            if options.include_metadata {
                if let Ok(headers) = row.try_get::<_, serde_json::Value>("headers") {
                    if let Some(obj) = headers.as_object() {
                        for (k, v) in obj {
                            page.metadata.insert(k.clone(), v.to_string());
                        }
                    }
                }
            }
            
            pages.push(page);
        }
        
        statistics.total_pages = pages.len();
        statistics.unique_domains = domains.len();
        statistics.success_rate = if pages.is_empty() {
            0.0
        } else {
            (success_count as f64) / (pages.len() as f64)
        };
        
        Ok(ExportData {
            export_info: ExportInfo {
                export_date: Utc::now(),
                export_format: format!("{:?}", options.format),
                total_records: pages.len(),
                options: options.clone(),
                crawler_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            pages,
            statistics,
        })
    }
    
    /// Export as JSON
    fn export_json(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        Ok(serde_json::to_string_pretty(data)?)
    }
    
    /// Export as JSON Lines (one JSON object per line)
    fn export_jsonlines(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        let mut lines = Vec::new();
        for page in &data.pages {
            lines.push(serde_json::to_string(page)?);
        }
        Ok(lines.join("\n"))
    }
    
    /// Export as CSV
    fn export_csv(data: &ExportData, options: &ExportOptions) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        
        // Write headers
        wtr.write_record([
            "url", "domain", "title", "description", "status_code",
            "content_type", "content_length", "language", "crawled_at",
        ])?;
        
        // Write data
        for page in &data.pages {
            wtr.write_record([
                &page.url,
                &page.domain,
                page.title.as_deref().unwrap_or(""),
                page.description.as_deref().unwrap_or(""),
                &page.status_code.to_string(),
                page.content_type.as_deref().unwrap_or(""),
                &page.content_length.to_string(),
                page.language.as_deref().unwrap_or(""),
                &page.crawled_at.to_string(),
            ])?;
        }
        
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }
    
    /// Export as XML
    fn export_xml(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<crawl_export>\n");
        xml.push_str("  <export_info>\n");
        xml.push_str(&format!("    <date>{}</date>\n", data.export_info.export_date));
        xml.push_str(&format!("    <total_records>{}</total_records>\n", data.export_info.total_records));
        xml.push_str("  </export_info>\n");
        xml.push_str("  <pages>\n");
        
        for page in &data.pages {
            xml.push_str("    <page>\n");
            xml.push_str(&format!("      <url><![CDATA[{}]]></url>\n", page.url));
            xml.push_str(&format!("      <domain>{}</domain>\n", page.domain));
            if let Some(title) = &page.title {
                xml.push_str(&format!("      <title><![CDATA[{}]]></title>\n", title));
            }
            xml.push_str(&format!("      <status_code>{}</status_code>\n", page.status_code));
            xml.push_str(&format!("      <crawled_at>{}</crawled_at>\n", page.crawled_at));
            xml.push_str("    </page>\n");
        }
        
        xml.push_str("  </pages>\n");
        xml.push_str("</crawl_export>\n");
        
        Ok(xml)
    }
    
    /// Export as HTML report
    fn export_html(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>Crawl Export Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h1 { color: #333; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        tr:nth-child(even) { background-color: #f9f9f9; }
    </style>
</head>
<body>
    <h1>Crawl Export Report</h1>
"#);
        
        html.push_str(&format!("<p>Export Date: {}</p>\n", data.export_info.export_date));
        html.push_str(&format!("<p>Total Records: {}</p>\n", data.export_info.total_records));
        
        html.push_str("<h2>Statistics</h2>\n");
        html.push_str("<ul>\n");
        html.push_str(&format!("  <li>Unique Domains: {}</li>\n", data.statistics.unique_domains));
        html.push_str(&format!("  <li>Success Rate: {:.2}%</li>\n", data.statistics.success_rate * 100.0));
        html.push_str(&format!("  <li>Total Size: {} MB</li>\n", data.statistics.total_size_bytes / 1_000_000));
        html.push_str("</ul>\n");
        
        html.push_str("<h2>Crawled Pages</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>URL</th><th>Title</th><th>Status</th><th>Type</th><th>Size</th></tr>\n");
        
        for page in &data.pages {
            html.push_str("<tr>");
            html.push_str(&format!("<td><a href=\"{}\">{}</a></td>", page.url, page.url));
            html.push_str(&format!("<td>{}</td>", page.title.as_deref().unwrap_or("-")));
            html.push_str(&format!("<td>{}</td>", page.status_code));
            html.push_str(&format!("<td>{}</td>", page.content_type.as_deref().unwrap_or("-")));
            html.push_str(&format!("<td>{} KB</td>", page.content_length / 1000));
            html.push_str("</tr>\n");
        }
        
        html.push_str("</table>\n");
        html.push_str("</body>\n</html>");
        
        Ok(html)
    }
    
    /// Export as Markdown
    fn export_markdown(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        let mut md = String::from("# Crawl Export Report\n\n");
        
        md.push_str(&format!("**Export Date:** {}\n", data.export_info.export_date));
        md.push_str(&format!("**Total Records:** {}\n\n", data.export_info.total_records));
        
        md.push_str("## Statistics\n\n");
        md.push_str(&format!("- Unique Domains: {}\n", data.statistics.unique_domains));
        md.push_str(&format!("- Success Rate: {:.2}%\n", data.statistics.success_rate * 100.0));
        md.push_str(&format!("- Total Size: {} MB\n\n", data.statistics.total_size_bytes / 1_000_000));
        
        md.push_str("## Crawled Pages\n\n");
        md.push_str("| URL | Title | Status | Type | Size |\n");
        md.push_str("|-----|-------|--------|------|------|\n");
        
        for page in &data.pages {
            md.push_str(&format!(
                "| [{}]({}) | {} | {} | {} | {} KB |\n",
                page.url.replace("|", "\\|"),
                page.url,
                page.title.as_deref().unwrap_or("-").replace("|", "\\|"),
                page.status_code,
                page.content_type.as_deref().unwrap_or("-"),
                page.content_length / 1000
            ));
        }
        
        Ok(md)
    }
    
    /// Export as XML Sitemap
    fn export_sitemap(data: &ExportData, _options: &ExportOptions) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
        
        for page in &data.pages {
            if page.status_code >= 200 && page.status_code < 300 {
                xml.push_str("  <url>\n");
                xml.push_str(&format!("    <loc>{}</loc>\n", page.url));
                
                let dt = chrono::NaiveDateTime::from_timestamp_opt(page.crawled_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                
                if !dt.is_empty() {
                    xml.push_str(&format!("    <lastmod>{}</lastmod>\n", dt));
                }
                
                xml.push_str("  </url>\n");
            }
        }
        
        xml.push_str("</urlset>\n");
        
        Ok(xml)
    }
    
    /// Write data to file
    async fn write_file(path: &Path, data: &[u8]) -> Result<()> {
        let mut file = File::create(path).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        Ok(())
    }
    
    /// Write compressed data to file
    async fn write_compressed(path: &Path, data: &[u8]) -> Result<()> {
        let compressed = Self::compress_data(data)?;
        
        // Add .gz extension if not present
        let path = if path.extension().is_some_and(|e| e == "gz") {
            path.to_path_buf()
        } else {
            path.with_extension(format!("{}.gz", 
                path.extension().and_then(|e| e.to_str()).unwrap_or("")))
        };
        
        Self::write_file(&path, &compressed).await
    }
    
    /// Compress data using gzip
    fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
}

/// Export preset configurations
pub struct ExportPresets;

impl ExportPresets {
    /// Full export with all data
    pub fn full() -> ExportOptions {
        ExportOptions {
            format: ExportFormat::Json,
            include_content: true,
            include_metadata: true,
            include_links: true,
            ..Default::default()
        }
    }
    
    /// Summary export without content
    pub fn summary() -> ExportOptions {
        ExportOptions {
            format: ExportFormat::Csv,
            include_content: false,
            include_metadata: false,
            include_links: false,
            ..Default::default()
        }
    }
    
    /// Sitemap export
    pub fn sitemap() -> ExportOptions {
        ExportOptions {
            format: ExportFormat::Sitemap,
            include_content: false,
            include_metadata: false,
            include_links: false,
            status_filter: Some(vec![200, 201, 202, 203, 204, 205, 206]),
            ..Default::default()
        }
    }
    
    /// Failed URLs export
    pub fn failures() -> ExportOptions {
        ExportOptions {
            format: ExportFormat::Csv,
            include_content: false,
            include_metadata: true,
            include_links: false,
            status_filter: Some(vec![400, 401, 403, 404, 500, 502, 503, 504]),
            ..Default::default()
        }
    }
}
//! Content type handling for various file formats
//! 
//! This module provides extraction and processing capabilities for different
//! content types including PDFs, images, documents, and more.

use anyhow::Result;
use std::collections::HashMap;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

/// Supported content types and their metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Html,
    Pdf,
    Image(ImageType),
    Document(DocumentType),
    Archive(ArchiveType),
    Video,
    Audio,
    Json,
    Xml,
    Text,
    JavaScript,
    Css,
    Binary,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageType {
    Jpeg,
    Png,
    Gif,
    WebP,
    Svg,
    Bmp,
    Ico,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Word,
    Excel,
    PowerPoint,
    OpenDocument,
    Rtf,
    Markdown,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveType {
    Zip,
    TarGz,
    Rar,
    SevenZip,
    Other(String),
}

/// Extracted content from various file types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub content_type: ContentType,
    pub text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub links: Vec<String>,
    pub size_bytes: usize,
    pub hash: String,
    pub thumbnail: Option<Vec<u8>>,
}

impl ContentType {
    /// Detect content type from MIME type string
    pub fn from_mime(mime: &str) -> Self {
        let mime_lower = mime.to_lowercase();
        
        match mime_lower.as_str() {
            "text/html" | "application/xhtml+xml" => ContentType::Html,
            "application/pdf" => ContentType::Pdf,
            "application/json" => ContentType::Json,
            "application/xml" | "text/xml" => ContentType::Xml,
            "text/plain" => ContentType::Text,
            "text/javascript" | "application/javascript" | "application/x-javascript" => ContentType::JavaScript,
            "text/css" => ContentType::Css,
            
            // Images
            "image/jpeg" | "image/jpg" => ContentType::Image(ImageType::Jpeg),
            "image/png" => ContentType::Image(ImageType::Png),
            "image/gif" => ContentType::Image(ImageType::Gif),
            "image/webp" => ContentType::Image(ImageType::WebP),
            "image/svg+xml" => ContentType::Image(ImageType::Svg),
            "image/bmp" => ContentType::Image(ImageType::Bmp),
            "image/x-icon" | "image/vnd.microsoft.icon" => ContentType::Image(ImageType::Ico),
            mime if mime.starts_with("image/") => ContentType::Image(ImageType::Other(mime.to_string())),
            
            // Documents
            "application/msword" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => 
                ContentType::Document(DocumentType::Word),
            "application/vnd.ms-excel" | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => 
                ContentType::Document(DocumentType::Excel),
            "application/vnd.ms-powerpoint" | "application/vnd.openxmlformats-officedocument.presentationml.presentation" => 
                ContentType::Document(DocumentType::PowerPoint),
            "application/vnd.oasis.opendocument.text" => ContentType::Document(DocumentType::OpenDocument),
            "application/rtf" => ContentType::Document(DocumentType::Rtf),
            "text/markdown" => ContentType::Document(DocumentType::Markdown),
            
            // Archives
            "application/zip" => ContentType::Archive(ArchiveType::Zip),
            "application/x-tar" | "application/gzip" => ContentType::Archive(ArchiveType::TarGz),
            "application/x-rar-compressed" => ContentType::Archive(ArchiveType::Rar),
            "application/x-7z-compressed" => ContentType::Archive(ArchiveType::SevenZip),
            
            // Media
            mime if mime.starts_with("video/") => ContentType::Video,
            mime if mime.starts_with("audio/") => ContentType::Audio,
            
            // Default
            _ => {
                if mime.starts_with("text/") {
                    ContentType::Text
                } else if mime.contains("application/octet-stream") {
                    ContentType::Binary
                } else {
                    ContentType::Unknown(mime.to_string())
                }
            }
        }
    }
    
    /// Detect content type from file extension
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        
        match ext_lower.as_str() {
            "html" | "htm" | "xhtml" => ContentType::Html,
            "pdf" => ContentType::Pdf,
            "json" => ContentType::Json,
            "xml" => ContentType::Xml,
            "txt" | "text" => ContentType::Text,
            "js" | "mjs" | "cjs" => ContentType::JavaScript,
            "css" => ContentType::Css,
            
            // Images
            "jpg" | "jpeg" => ContentType::Image(ImageType::Jpeg),
            "png" => ContentType::Image(ImageType::Png),
            "gif" => ContentType::Image(ImageType::Gif),
            "webp" => ContentType::Image(ImageType::WebP),
            "svg" => ContentType::Image(ImageType::Svg),
            "bmp" => ContentType::Image(ImageType::Bmp),
            "ico" => ContentType::Image(ImageType::Ico),
            
            // Documents
            "doc" | "docx" => ContentType::Document(DocumentType::Word),
            "xls" | "xlsx" => ContentType::Document(DocumentType::Excel),
            "ppt" | "pptx" => ContentType::Document(DocumentType::PowerPoint),
            "odt" => ContentType::Document(DocumentType::OpenDocument),
            "rtf" => ContentType::Document(DocumentType::Rtf),
            "md" | "markdown" => ContentType::Document(DocumentType::Markdown),
            
            // Archives
            "zip" => ContentType::Archive(ArchiveType::Zip),
            "tar" | "gz" | "tgz" => ContentType::Archive(ArchiveType::TarGz),
            "rar" => ContentType::Archive(ArchiveType::Rar),
            "7z" => ContentType::Archive(ArchiveType::SevenZip),
            
            // Media
            "mp4" | "avi" | "mov" | "wmv" | "flv" | "webm" => ContentType::Video,
            "mp3" | "wav" | "flac" | "aac" | "ogg" => ContentType::Audio,
            
            _ => ContentType::Unknown(ext.to_string()),
        }
    }
    
    /// Check if this content type should be crawled for links
    pub fn should_extract_links(&self) -> bool {
        matches!(self, 
            ContentType::Html | 
            ContentType::Xml | 
            ContentType::Json |
            ContentType::Document(_) |
            ContentType::Pdf
        )
    }
    
    /// Check if this content type should have text extracted
    pub fn should_extract_text(&self) -> bool {
        matches!(self,
            ContentType::Html |
            ContentType::Pdf |
            ContentType::Document(_) |
            ContentType::Json |
            ContentType::Xml |
            ContentType::Text
        )
    }
    
    /// Get the appropriate storage strategy for this content type
    pub fn storage_strategy(&self) -> StorageStrategy {
        match self {
            ContentType::Html | ContentType::Text | ContentType::Json | ContentType::Xml => 
                StorageStrategy::FullText,
            ContentType::Pdf | ContentType::Document(_) => 
                StorageStrategy::ExtractedText,
            ContentType::Image(_) => 
                StorageStrategy::Metadata,
            ContentType::Archive(_) => 
                StorageStrategy::Index,
            ContentType::Video | ContentType::Audio => 
                StorageStrategy::Metadata,
            _ => StorageStrategy::Skip,
        }
    }
}

/// Storage strategy for different content types
#[derive(Debug, Clone)]
pub enum StorageStrategy {
    FullText,      // Store complete content
    ExtractedText, // Store extracted text only
    Metadata,      // Store metadata only
    Index,         // Store file listing/index
    Skip,          // Don't store
}

/// Content processor for different file types
pub struct ContentProcessor;

impl ContentProcessor {
    /// Process content based on its type
    pub async fn process(
        content: &[u8],
        content_type: &ContentType,
        url: &str,
    ) -> Result<ExtractedContent> {
        // Calculate hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = format!("{:x}", hasher.finalize());
        
        let mut extracted = ExtractedContent {
            content_type: content_type.clone(),
            text: None,
            metadata: HashMap::new(),
            links: Vec::new(),
            size_bytes: content.len(),
            hash,
            thumbnail: None,
        };
        
        // Add URL to metadata
        extracted.metadata.insert("url".to_string(), url.to_string());
        
        match content_type {
            ContentType::Html => {
                extracted.text = Some(String::from_utf8_lossy(content).to_string());
                extracted.links = Self::extract_html_links(content);
            }
            ContentType::Pdf => {
                match Self::extract_pdf_text(content).await {
                    Ok((text, metadata)) => {
                        extracted.text = Some(text);
                        extracted.metadata.extend(metadata);
                    }
                    Err(e) => {
                        warn!("Failed to extract PDF text from {}: {}", url, e);
                    }
                }
            }
            ContentType::Image(img_type) => {
                match Self::extract_image_metadata(content, img_type).await {
                    Ok(metadata) => {
                        extracted.metadata.extend(metadata);
                        // TODO: Generate thumbnail
                    }
                    Err(e) => {
                        warn!("Failed to extract image metadata from {}: {}", url, e);
                    }
                }
            }
            ContentType::Json => {
                extracted.text = Some(String::from_utf8_lossy(content).to_string());
                // Extract URLs from JSON if present
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(content) {
                    extracted.links = Self::extract_json_urls(&json);
                }
            }
            ContentType::Xml => {
                extracted.text = Some(String::from_utf8_lossy(content).to_string());
                extracted.links = Self::extract_xml_links(content);
            }
            ContentType::Text => {
                extracted.text = Some(String::from_utf8_lossy(content).to_string());
            }
            ContentType::Document(doc_type) => {
                match Self::extract_document_text(content, doc_type).await {
                    Ok((text, metadata)) => {
                        extracted.text = Some(text);
                        extracted.metadata.extend(metadata);
                    }
                    Err(e) => {
                        warn!("Failed to extract document text from {}: {}", url, e);
                    }
                }
            }
            _ => {
                debug!("Skipping content extraction for type {:?}", content_type);
            }
        }
        
        Ok(extracted)
    }
    
    /// Extract links from HTML content
    fn extract_html_links(content: &[u8]) -> Vec<String> {
        let html = String::from_utf8_lossy(content);
        let mut links = Vec::new();
        
        // Simple regex-based extraction (in production, use proper HTML parser)
        let href_regex = regex::Regex::new(r#"href\s*=\s*["']([^"']+)["']"#).unwrap();
        for cap in href_regex.captures_iter(&html) {
            if let Some(url) = cap.get(1) {
                links.push(url.as_str().to_string());
            }
        }
        
        let src_regex = regex::Regex::new(r#"src\s*=\s*["']([^"']+)["']"#).unwrap();
        for cap in src_regex.captures_iter(&html) {
            if let Some(url) = cap.get(1) {
                links.push(url.as_str().to_string());
            }
        }
        
        links.sort();
        links.dedup();
        links
    }
    
    /// Extract text from PDF content
    async fn extract_pdf_text(content: &[u8]) -> Result<(String, HashMap<String, String>)> {
        // TODO: Implement actual PDF extraction using a library like pdf-extract or lopdf
        // For now, return a placeholder
        let metadata = HashMap::new();
        let text = format!("[PDF content - {} bytes]", content.len());
        Ok((text, metadata))
    }
    
    /// Extract metadata from images
    async fn extract_image_metadata(
        content: &[u8],
        _img_type: &ImageType,
    ) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        
        // TODO: Implement actual image metadata extraction using image crate
        // For now, just add basic info
        metadata.insert("size_bytes".to_string(), content.len().to_string());
        
        Ok(metadata)
    }
    
    /// Extract URLs from JSON content
    fn extract_json_urls(value: &serde_json::Value) -> Vec<String> {
        let mut urls = Vec::new();
        Self::extract_json_urls_recursive(value, &mut urls);
        urls.sort();
        urls.dedup();
        urls
    }
    
    fn extract_json_urls_recursive(value: &serde_json::Value, urls: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    urls.push(s.clone());
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::extract_json_urls_recursive(item, urls);
                }
            }
            serde_json::Value::Object(obj) => {
                for (_key, val) in obj {
                    Self::extract_json_urls_recursive(val, urls);
                }
            }
            _ => {}
        }
    }
    
    /// Extract links from XML content
    fn extract_xml_links(content: &[u8]) -> Vec<String> {
        let xml = String::from_utf8_lossy(content);
        let mut links = Vec::new();
        
        // Simple extraction of common URL patterns in XML
        let url_regex = regex::Regex::new(r"<(?:url|link|href|src)>([^<]+)</").unwrap();
        for cap in url_regex.captures_iter(&xml) {
            if let Some(url) = cap.get(1) {
                links.push(url.as_str().to_string());
            }
        }
        
        // Also check attributes
        let attr_regex = regex::Regex::new(r#"(?:href|src|url)\s*=\s*["']([^"']+)["']"#).unwrap();
        for cap in attr_regex.captures_iter(&xml) {
            if let Some(url) = cap.get(1) {
                links.push(url.as_str().to_string());
            }
        }
        
        links.sort();
        links.dedup();
        links
    }
    
    /// Extract text from document formats
    async fn extract_document_text(
        content: &[u8],
        _doc_type: &DocumentType,
    ) -> Result<(String, HashMap<String, String>)> {
        // TODO: Implement actual document extraction using appropriate libraries
        // For now, return a placeholder
        let metadata = HashMap::new();
        let text = format!("[Document content - {} bytes]", content.len());
        Ok((text, metadata))
    }
}

/// Configuration for content type handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeConfig {
    pub max_pdf_size: usize,
    pub max_image_size: usize,
    pub max_document_size: usize,
    pub extract_pdf_text: bool,
    pub generate_thumbnails: bool,
    pub store_binary_content: bool,
    pub allowed_types: Vec<String>,
    pub blocked_types: Vec<String>,
}

impl Default for ContentTypeConfig {
    fn default() -> Self {
        Self {
            max_pdf_size: 10 * 1024 * 1024,      // 10MB
            max_image_size: 5 * 1024 * 1024,     // 5MB
            max_document_size: 10 * 1024 * 1024, // 10MB
            extract_pdf_text: true,
            generate_thumbnails: false,
            store_binary_content: false,
            allowed_types: vec![],
            blocked_types: vec!["application/x-executable".to_string()],
        }
    }
}

/// Check if a content type is allowed by configuration
pub fn is_content_type_allowed(content_type: &str, config: &ContentTypeConfig) -> bool {
    // Check blocked types first
    if config.blocked_types.iter().any(|t| content_type.contains(t)) {
        return false;
    }
    
    // If allowed_types is empty, allow all non-blocked types
    if config.allowed_types.is_empty() {
        return true;
    }
    
    // Check if explicitly allowed
    config.allowed_types.iter().any(|t| content_type.contains(t))
}

/// Get size limit for a content type
pub fn get_size_limit(content_type: &ContentType, config: &ContentTypeConfig) -> usize {
    match content_type {
        ContentType::Pdf => config.max_pdf_size,
        ContentType::Image(_) => config.max_image_size,
        ContentType::Document(_) => config.max_document_size,
        _ => usize::MAX, // No limit for other types
    }
}
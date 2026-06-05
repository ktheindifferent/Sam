use serde::{Deserialize, Serialize};
// use serde_valid::Validate; // Not using serde_valid due to compatibility issues
// use validator::{Validate as ValidatorValidate, ValidationError};
use ammonia::clean;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// Enhanced validation patterns
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]{3,32}$").unwrap());

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap());

static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$").unwrap()
});

// Validation trait for all input types
pub trait InputValidation {
    fn validate_and_sanitize(&mut self) -> Result<(), ValidationErrors>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrors {
    pub errors: HashMap<String, Vec<String>>,
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationErrors {
    pub fn new() -> Self {
        ValidationErrors {
            errors: HashMap::new(),
        }
    }

    pub fn add_error(&mut self, field: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_default()
            .push(message.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

// User input validation structures
#[derive(Debug, Deserialize, Serialize)]
pub struct UserRegistrationInput {
    // Email validation handled in validate_and_sanitize method
    pub email: String,

    // Username validation handled in validate_and_sanitize method
    pub username: String,

    // Password validation handled in validate_and_sanitize method
    pub password: String,

    // Name validation handled in validate_and_sanitize method
    pub name: Option<String>,
}

impl InputValidation for UserRegistrationInput {
    fn validate_and_sanitize(&mut self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        // Validate email
        self.email = self.email.trim().to_lowercase();
        if !EMAIL_REGEX.is_match(&self.email) {
            errors.add_error("email", "Invalid email format");
        }

        // Validate username
        self.username = self.username.trim().to_string();
        if !USERNAME_REGEX.is_match(&self.username) {
            errors.add_error(
                "username",
                "Username must be alphanumeric with underscores/hyphens only",
            );
        }

        // Validate password strength
        if !validate_password_strength(&self.password) {
            errors.add_error(
                "password",
                "Password must contain uppercase, lowercase, number, and special character",
            );
        }

        // Sanitize name if provided
        if let Some(ref mut name) = self.name {
            *name = sanitize_text(name);
            if name.len() > 100 {
                errors.add_error("name", "Name too long");
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// API request validation
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiQueryParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

impl InputValidation for ApiQueryParams {
    fn validate_and_sanitize(&mut self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        // Validate page
        if let Some(page) = self.page {
            if page == 0 || page > 10000 {
                errors.add_error("page", "Page must be between 1 and 10000");
            }
        }

        // Validate limit
        if let Some(limit) = self.limit {
            if limit == 0 || limit > 100 {
                errors.add_error("limit", "Limit must be between 1 and 100");
            }
        }

        // Validate sort parameter
        if let Some(ref mut sort) = self.sort {
            *sort = sanitize_sort_parameter(sort);
            if !is_valid_sort_field(sort) {
                errors.add_error("sort", "Invalid sort field");
            }
        }

        // Sanitize filter parameter
        if let Some(ref mut filter) = self.filter {
            *filter = sanitize_text(filter);
            if filter.len() > 500 {
                errors.add_error("filter", "Filter string too long");
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// File upload validation
#[derive(Debug, Deserialize, Serialize)]
pub struct FileUploadInput {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub data: Vec<u8>,
}

impl InputValidation for FileUploadInput {
    fn validate_and_sanitize(&mut self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        // Sanitize filename
        self.filename = sanitize_filename(&self.filename);
        if self.filename.is_empty() || self.filename.len() > 255 {
            errors.add_error("filename", "Invalid filename");
        }

        // Validate content type
        if !is_allowed_content_type(&self.content_type) {
            errors.add_error("content_type", "File type not allowed");
        }

        // Validate file size (max 50MB)
        const MAX_SIZE: usize = 50 * 1024 * 1024;
        if self.size > MAX_SIZE {
            errors.add_error("size", "File size exceeds 50MB limit");
        }

        // Check for malicious content in file data
        if contains_malicious_patterns(&self.data) {
            errors.add_error("data", "File contains potentially malicious content");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// Helper functions
fn sanitize_text(input: &str) -> String {
    // Remove HTML tags and dangerous characters
    let cleaned = clean(input);
    // Remove null bytes and control characters
    cleaned
        .chars()
        .filter(|c| !c.is_control() && *c != '\0')
        .collect()
}

fn sanitize_filename(filename: &str) -> String {
    // Remove path traversal attempts and dangerous characters
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>()
        .replace("..", "")
        .trim_start_matches('.')
        .to_string()
}

fn sanitize_sort_parameter(sort: &str) -> String {
    // Allow only alphanumeric, underscore, and dash for sort fields
    sort.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

fn is_valid_sort_field(field: &str) -> bool {
    // Define allowed sort fields
    const ALLOWED_FIELDS: &[&str] = &[
        "id",
        "name",
        "email",
        "created_at",
        "updated_at",
        "title",
        "date",
        "priority",
        "status",
    ];

    // Check if field is in format "field" or "field-desc"/"field-asc"
    let base_field = field.trim_end_matches("-asc").trim_end_matches("-desc");
    ALLOWED_FIELDS.contains(&base_field)
}

fn validate_password_strength(password: &str) -> bool {
    // Password must contain at least one uppercase, lowercase, number, and special character
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_number = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    has_uppercase && has_lowercase && has_number && has_special
}

fn is_allowed_content_type(content_type: &str) -> bool {
    const ALLOWED_TYPES: &[&str] = &[
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "application/pdf",
        "text/plain",
        "text/csv",
        "application/json",
        "application/xml",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ];

    ALLOWED_TYPES.contains(&content_type)
}

fn contains_malicious_patterns(data: &[u8]) -> bool {
    // Check for common malicious patterns in file headers
    const MALICIOUS_PATTERNS: &[&[u8]] = &[
        b"<script",
        b"javascript:",
        b"onerror=",
        b"onclick=",
        b"<iframe",
        b"<?php",
        b"<%",
        b"eval(",
        b"exec(",
        b"system(",
    ];

    for pattern in MALICIOUS_PATTERNS {
        if data.windows(pattern.len()).any(|window| window == *pattern) {
            return true;
        }
    }

    false
}

// JSON input validation
pub fn validate_json_input<T: InputValidation + for<'de> serde::Deserialize<'de>>(
    json_str: &str,
) -> Result<T, ValidationErrors> {
    let mut input: T = serde_json::from_str(json_str).map_err(|_| {
        let mut errors = ValidationErrors::new();
        errors.add_error("json", "Invalid JSON format");
        errors
    })?;

    input.validate_and_sanitize()?;
    Ok(input)
}

// Request body size validation
pub fn validate_body_size(size: usize, max_size: usize) -> Result<(), ValidationErrors> {
    if size > max_size {
        let mut errors = ValidationErrors::new();
        errors.add_error("body", &format!("Request body exceeds {} bytes", max_size));
        return Err(errors);
    }
    Ok(())
}

// SQL injection prevention
pub fn sanitize_sql_parameter(input: &str) -> String {
    // Remove or escape SQL special characters
    input
        .replace('\'', "''")
        .replace('"', "\"\"")
        .replace(';', "")
        .replace("--", "")
        .replace("/*", "")
        .replace("*/", "")
        .replace("xp_", "")
        .replace("sp_", "")
}

// XSS prevention for output encoding
pub fn encode_for_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

pub fn encode_for_javascript(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('/', "\\/")
}

pub fn encode_for_url(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_registration_validation() {
        let mut input = UserRegistrationInput {
            email: "test@example.com".to_string(),
            username: "testuser123".to_string(),
            password: "SecureP@ss123".to_string(),
            name: Some("Test User".to_string()),
        };

        assert!(input.validate_and_sanitize().is_ok());

        // Test invalid email
        input.email = "invalid-email".to_string();
        assert!(input.validate_and_sanitize().is_err());
    }

    #[test]
    fn test_filename_sanitization() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_filename("file.txt"), "file.txt");
        assert_eq!(sanitize_filename("file<script>.txt"), "filescript.txt");
    }

    #[test]
    fn test_sql_sanitization() {
        assert_eq!(
            sanitize_sql_parameter("'; DROP TABLE users;"),
            "''; DROP TABLE users"
        );
        assert_eq!(sanitize_sql_parameter("normal input"), "normal input");
    }

    #[test]
    fn test_xss_encoding() {
        assert_eq!(
            encode_for_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );
    }
}

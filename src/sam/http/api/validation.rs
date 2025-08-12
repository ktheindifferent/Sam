use rouille::{Request, Response};
use crate::sam::security::validation_middleware::{
    InputValidation, ValidationErrors, ApiQueryParams,
    FileUploadInput, encode_for_html, encode_for_javascript,
    validate_json_input, validate_body_size
};
use serde_json::json;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB default

/// Validate and sanitize request body
pub fn validate_request_body<T: InputValidation + serde::de::DeserializeOwned>(
    request: &Request,
) -> Result<T, Response> {
    // Read body data
    let body = match rouille::input::plain_text_body(request) {
        Ok(body) => body,
        Err(_) => {
            return Err(Response::json(&json!({
                "error": "Failed to read request body"
            })).with_status_code(400));
        }
    };
    
    // Validate body size
    if let Err(errors) = validate_body_size(body.len(), MAX_BODY_SIZE) {
        return Err(Response::json(&json!({
            "errors": errors.errors
        })).with_status_code(413));
    }
    
    // Parse and validate JSON
    match validate_json_input::<T>(&body) {
        Ok(validated) => Ok(validated),
        Err(errors) => {
            Err(Response::json(&json!({
                "errors": errors.errors
            })).with_status_code(400))
        }
    }
}

/// Validate query parameters
pub fn validate_query_params(request: &Request) -> Result<ApiQueryParams, Response> {
    let mut params = ApiQueryParams {
        page: None,
        limit: None,
        sort: None,
        filter: None,
    };
    
    // Parse query string
    if let Some(query) = request.raw_query_string() {
        for pair in query.split('&') {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let key = parts[0];
            let value = urlencoding::decode(parts[1]).unwrap_or_default();
            
            match key {
                "page" => params.page = value.parse().ok(),
                "limit" => params.limit = value.parse().ok(),
                "sort" => params.sort = Some(value.to_string()),
                "filter" => params.filter = Some(value.to_string()),
                _ => {}
            }
        }
    }
    
    // Validate parameters
    match params.validate_and_sanitize() {
        Ok(()) => Ok(params),
        Err(errors) => {
            Err(Response::json(&json!({
                "errors": errors.errors
            })).with_status_code(400))
        }
    }
}

/// Validate file upload
pub fn validate_file_upload(request: &Request) -> Result<FileUploadInput, Response> {
    // Parse multipart form data
    let input = match rouille::input::multipart::get_multipart_input(request) {
        Ok(mut multipart) => {
            let mut file_input = None;
            
            while let Some(entry) = multipart.next() {
                if entry.headers.name == "file_data" {
                    let filename = entry.headers.filename.clone().unwrap_or_default();
                    let content_type = entry.headers.content_type.clone()
                        .map(|ct| ct.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    
                    let mut data = Vec::new();
                    if let Err(_) = entry.data.read_to_end(&mut data) {
                        return Err(Response::json(&json!({
                            "error": "Failed to read file data"
                        })).with_status_code(400));
                    }
                    
                    file_input = Some(FileUploadInput {
                        filename,
                        content_type,
                        size: data.len(),
                        data,
                    });
                    break;
                }
            }
            
            file_input.ok_or_else(|| Response::json(&json!({
                "error": "No file data found in request"
            })).with_status_code(400))?
        }
        Err(_) => {
            return Err(Response::json(&json!({
                "error": "Invalid multipart form data"
            })).with_status_code(400));
        }
    };
    
    // Validate file input
    let mut file_input = input;
    match file_input.validate_and_sanitize() {
        Ok(()) => Ok(file_input),
        Err(errors) => {
            Err(Response::json(&json!({
                "errors": errors.errors
            })).with_status_code(400))
        }
    }
}

/// Sanitize output for safe rendering
pub fn sanitize_output_json(data: &serde_json::Value) -> serde_json::Value {
    match data {
        serde_json::Value::String(s) => {
            // HTML encode strings for safe display
            serde_json::Value::String(encode_for_html(s))
        }
        serde_json::Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (key, value) in map {
                sanitized.insert(key.clone(), sanitize_output_json(value));
            }
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(arr) => {
            let sanitized: Vec<_> = arr.iter().map(sanitize_output_json).collect();
            serde_json::Value::Array(sanitized)
        }
        other => other.clone(),
    }
}

/// Create safe error response
pub fn error_response(message: &str, status_code: u16) -> Response {
    let safe_message = encode_for_html(message);
    Response::json(&json!({
        "error": safe_message
    })).with_status_code(status_code)
}

/// Validate path parameter to prevent directory traversal
pub fn validate_path_param(path: &str) -> Result<String, Response> {
    // Check for path traversal attempts
    if path.contains("..") || path.contains("//") || path.starts_with('/') {
        return Err(error_response("Invalid path parameter", 400));
    }
    
    // Allow only alphanumeric, dash, underscore, dot
    let sanitized: String = path.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    
    if sanitized.is_empty() || sanitized.len() > 255 {
        return Err(error_response("Invalid path parameter", 400));
    }
    
    Ok(sanitized)
}

/// Validate ID parameter (UUID or numeric)
pub fn validate_id_param(id: &str) -> Result<String, Response> {
    // Check if it's a valid UUID
    if id.len() == 36 {
        // Basic UUID format check
        let uuid_regex = regex::Regex::new(r"^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$").unwrap();
        if uuid_regex.is_match(id) {
            return Ok(id.to_string());
        }
    }
    
    // Check if it's a numeric ID
    if id.chars().all(|c| c.is_numeric()) && id.len() <= 20 {
        return Ok(id.to_string());
    }
    
    // Check if it's an alphanumeric OID
    let sanitized: String = id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    
    if sanitized.is_empty() || sanitized.len() > 64 {
        return Err(error_response("Invalid ID parameter", 400));
    }
    
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_validation() {
        assert!(validate_path_param("normal-file.txt").is_ok());
        assert!(validate_path_param("../etc/passwd").is_err());
        assert!(validate_path_param("/absolute/path").is_err());
        assert!(validate_path_param("../../traversal").is_err());
    }

    #[test]
    fn test_id_validation() {
        // Valid UUID
        assert!(validate_id_param("550e8400-e29b-41d4-a716-446655440000").is_ok());
        // Valid numeric
        assert!(validate_id_param("12345").is_ok());
        // Valid alphanumeric
        assert!(validate_id_param("user_123").is_ok());
        // Invalid
        assert!(validate_id_param("../../etc/passwd").is_err());
    }

    #[test]
    fn test_output_sanitization() {
        let input = json!({
            "name": "<script>alert('xss')</script>",
            "safe": "normal text"
        });
        
        let sanitized = sanitize_output_json(&input);
        let name = sanitized["name"].as_str().unwrap();
        assert!(!name.contains("<script>"));
        assert!(name.contains("&lt;script&gt;"));
    }
}
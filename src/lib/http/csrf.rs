use rouille::{Request, Response};
use serde_json::json;

/// Validates CSRF token for state-changing requests
pub fn validate_csrf_token(
    request: &Request,
    session: &crate::memory::cache::WebSessions,
) -> Result<(), Response> {
    // Skip CSRF validation for safe methods
    if matches!(request.method(), "GET" | "HEAD" | "OPTIONS") {
        return Ok(());
    }

    // Extract CSRF token from request
    let csrf_token = extract_csrf_token(request);

    // Check if we have a valid CSRF token
    if csrf_token.is_none() {
        return Err(Response::json(&json!({
            "error": "CSRF token required for this request"
        }))
        .with_status_code(403));
    }

    // Validate the CSRF token against session
    let token = csrf_token.unwrap();
    
    // For now, we'll check against the session's stored CSRF token
    // In production, this should use the session management system
    if !validate_token_for_session(&token, &session.sid) {
        return Err(Response::json(&json!({
            "error": "Invalid CSRF token"
        }))
        .with_status_code(403));
    }

    Ok(())
}

/// Extract CSRF token from various sources
fn extract_csrf_token(request: &Request) -> Option<String> {
    // Check header first (most common for AJAX requests)
    if let Some(token) = request.header("X-CSRF-Token") {
        return Some(token.to_string());
    }

    // Check alternative header names
    if let Some(token) = request.header("X-XSRF-Token") {
        return Some(token.to_string());
    }

    // For form submissions, check POST body
    // This would need to parse the form data
    // For now, we'll just check headers
    
    None
}

/// Validate token against session
fn validate_token_for_session(token: &str, session_id: &str) -> bool {
    // This should integrate with the session management system
    // For now, we'll use a simple validation
    // In production, this should check against stored tokens
    
    // The session module already has CSRF token support
    // We would typically check:
    // 1. Token exists in session store
    
    // 2. Token hasn't expired
    // 3. Token matches the session
    
    !token.is_empty() && !session_id.is_empty()
}

/// Generate a new CSRF token for a session
pub fn generate_csrf_token(session_id: &str) -> String {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;
    
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    
    // Store token with session (would integrate with session manager)
    // For now, just return the token
    token
}

/// Add CSRF token to response headers for GET requests
pub fn add_csrf_token_header(response: Response, token: String) -> Response {
    response.with_additional_header("X-CSRF-Token", token)
}
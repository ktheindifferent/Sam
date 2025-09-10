use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
        }
    }
}

pub struct ValidationResult {
    errors: Vec<ValidationError>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }
}

pub struct Validator {
    field_name: String,
    value: Option<String>,
    errors: Vec<ValidationError>,
}

impl Validator {
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            value: None,
            errors: Vec::new(),
        }
    }

    pub fn validate(field_name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            value: Some(value.into()),
            errors: Vec::new(),
        }
    }

    pub fn required(mut self) -> Self {
        if self.value.as_ref().is_none_or(|v| v.trim().is_empty()) {
            self.errors.push(ValidationError::new(
                &self.field_name,
                format!("{} is required", self.field_name),
                "required",
            ));
        }
        self
    }

    pub fn min_length(mut self, min: usize) -> Self {
        if let Some(ref value) = self.value {
            if value.len() < min {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be at least {} characters", self.field_name, min),
                    "min_length",
                ));
            }
        }
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        if let Some(ref value) = self.value {
            if value.len() > max {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be at most {} characters", self.field_name, max),
                    "max_length",
                ));
            }
        }
        self
    }

    pub fn pattern(mut self, pattern: &str, message: &str) -> Self {
        if let Some(ref value) = self.value {
            if let Ok(re) = Regex::new(pattern) {
                if !re.is_match(value) {
                    self.errors.push(ValidationError::new(
                        &self.field_name,
                        message,
                        "pattern",
                    ));
                }
            }
        }
        self
    }

    pub fn email(mut self) -> Self {
        if let Some(ref value) = self.value {
            let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
            if !email_regex.is_match(value) {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be a valid email address", self.field_name),
                    "email",
                ));
            }
        }
        self
    }

    pub fn url(mut self) -> Self {
        if let Some(ref value) = self.value {
            if Url::parse(value).is_err() {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be a valid URL", self.field_name),
                    "url",
                ));
            }
        }
        self
    }

    pub fn ip_address(mut self) -> Self {
        if let Some(ref value) = self.value {
            if value.parse::<IpAddr>().is_err() {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be a valid IP address", self.field_name),
                    "ip_address",
                ));
            }
        }
        self
    }

    pub fn socket_address(mut self) -> Self {
        if let Some(ref value) = self.value {
            if value.parse::<SocketAddr>().is_err() {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be a valid socket address", self.field_name),
                    "socket_address",
                ));
            }
        }
        self
    }

    pub fn port(mut self) -> Self {
        if let Some(ref value) = self.value {
            match value.parse::<u16>() {
                Ok(port) if port > 0 => {}
                _ => {
                    self.errors.push(ValidationError::new(
                        &self.field_name,
                        format!("{} must be a valid port number (1-65535)", self.field_name),
                        "port",
                    ));
                }
            }
        }
        self
    }

    pub fn numeric(mut self) -> Self {
        if let Some(ref value) = self.value {
            if value.parse::<f64>().is_err() {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be a numeric value", self.field_name),
                    "numeric",
                ));
            }
        }
        self
    }

    pub fn integer(mut self) -> Self {
        if let Some(ref value) = self.value {
            if value.parse::<i64>().is_err() {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be an integer", self.field_name),
                    "integer",
                ));
            }
        }
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        if let Some(ref value) = self.value {
            if let Ok(num) = value.parse::<f64>() {
                if num < min || num > max {
                    self.errors.push(ValidationError::new(
                        &self.field_name,
                        format!("{} must be between {} and {}", self.field_name, min, max),
                        "range",
                    ));
                }
            }
        }
        self
    }

    pub fn one_of(mut self, options: &[&str]) -> Self {
        if let Some(ref value) = self.value {
            if !options.contains(&value.as_str()) {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    format!("{} must be one of: {}", self.field_name, options.join(", ")),
                    "one_of",
                ));
            }
        }
        self
    }

    pub fn custom<F>(mut self, validator: F, message: &str) -> Self
    where
        F: Fn(&str) -> bool,
    {
        if let Some(ref value) = self.value {
            if !validator(value) {
                self.errors.push(ValidationError::new(
                    &self.field_name,
                    message,
                    "custom",
                ));
            }
        }
        self
    }

    pub fn build(self) -> ValidationResult {
        let mut result = ValidationResult::new();
        for error in self.errors {
            result.add_error(error);
        }
        result
    }
}

pub struct ConfigValidator {
    errors: Vec<ValidationError>,
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn validate_field(&mut self, field_name: &str, value: Option<&str>) -> Validator {
        Validator {
            field_name: field_name.to_string(),
            value: value.map(|v| v.to_string()),
            errors: Vec::new(),
        }
    }

    pub fn merge(&mut self, result: ValidationResult) {
        self.errors.extend(result.errors);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    pub fn build(self) -> Result<(), Vec<ValidationError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }
}

pub fn validate_config<T, F>(config: &T, validator: F) -> Result<(), Vec<ValidationError>>
where
    F: Fn(&T, &mut ConfigValidator),
{
    let mut config_validator = ConfigValidator::new();
    validator(config, &mut config_validator);
    config_validator.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_validation() {
        let result = Validator::validate("username", "").required().build();
        assert!(!result.is_valid());
        assert_eq!(result.errors().len(), 1);
        assert_eq!(result.errors()[0].code, "required");

        let result = Validator::validate("username", "john").required().build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_email_validation() {
        let result = Validator::validate("email", "invalid").email().build();
        assert!(!result.is_valid());

        let result = Validator::validate("email", "user@example.com").email().build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_url_validation() {
        let result = Validator::validate("website", "not-a-url").url().build();
        assert!(!result.is_valid());

        let result = Validator::validate("website", "https://example.com").url().build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_port_validation() {
        let result = Validator::validate("port", "0").port().build();
        assert!(!result.is_valid());

        let result = Validator::validate("port", "8080").port().build();
        assert!(result.is_valid());

        let result = Validator::validate("port", "70000").port().build();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_range_validation() {
        let result = Validator::validate("age", "150").range(0.0, 120.0).build();
        assert!(!result.is_valid());

        let result = Validator::validate("age", "25").range(0.0, 120.0).build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_one_of_validation() {
        let options = vec!["red", "green", "blue"];
        let result = Validator::validate("color", "yellow").one_of(&options).build();
        assert!(!result.is_valid());

        let result = Validator::validate("color", "green").one_of(&options).build();
        assert!(result.is_valid());
    }

    #[test]
    fn test_chain_validation() {
        let result = Validator::validate("username", "ab")
            .required()
            .min_length(3)
            .max_length(20)
            .pattern("^[a-zA-Z0-9]+$", "Username must be alphanumeric")
            .build();

        assert!(!result.is_valid());
        assert_eq!(result.errors().len(), 1); // min_length error
    }
}
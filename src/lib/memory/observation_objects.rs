// Flexible observation objects structure
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ObservationObjects {
    pub name: String,
    pub confidence: f32,
    pub location: Option<String>, // Bounding box or location info
}

impl ObservationObjects {
    pub fn new(name: String, confidence: f32) -> Self {
        Self {
            name,
            confidence,
            location: None,
        }
    }
    
    pub fn with_location(name: String, confidence: f32, location: String) -> Self {
        Self {
            name,
            confidence,
            location: Some(location),
        }
    }
}

impl FromStr for ObservationObjects {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // For backward compatibility, parse simple object names as basic objects
        Ok(Self::new(s.to_string(), 1.0))
    }
}
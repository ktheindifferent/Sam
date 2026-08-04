//! User agent rotation for improved crawl success
//!
//! This module provides a collection of user agents and rotation strategies
//! to help avoid detection and blocking by websites.

use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Common desktop browser user agents
const DESKTOP_USER_AGENTS: &[&str] = &[
    // Chrome on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",

    // Chrome on Mac
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",

    // Firefox on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:119.0) Gecko/20100101 Firefox/119.0",

    // Firefox on Mac
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15) Gecko/20100101 Firefox/120.0",

    // Safari on Mac
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",

    // Edge on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
];

/// Mobile browser user agents
const MOBILE_USER_AGENTS: &[&str] = &[
    // iPhone Safari
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1",

    // Android Chrome
    "Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",

    // iPad Safari
    "Mozilla/5.0 (iPad; CPU OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
];

/// Bot/crawler user agents (for transparent crawling)
const BOT_USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (compatible; SAMBot/1.0; +https://github.com/sam)",
    "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
    "Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)",
];

/// User agent rotation strategy
#[derive(Debug, Clone, PartialEq)]
pub enum RotationStrategy {
    /// Use a single fixed user agent
    Fixed(String),
    /// Randomly select from a pool on each request
    Random,
    /// Round-robin through the pool
    RoundRobin,
    /// Stick to one user agent per domain
    PerDomain,
    /// Use desktop agents for HTML, bot agents for assets
    ContentAware,
}

/// User agent type for filtering
#[derive(Debug, Clone, PartialEq)]
pub enum UserAgentType {
    Desktop,
    Mobile,
    Bot,
    All,
}

/// User agent rotator with various strategies
pub struct UserAgentRotator {
    strategy: RotationStrategy,
    agent_type: UserAgentType,
    agents: Vec<String>,
    current_index: Arc<RwLock<usize>>,
    domain_agents: Arc<RwLock<HashMap<String, String>>>,
    domain_agent_expiry: Arc<RwLock<HashMap<String, Instant>>>,
    expiry_duration: Duration,
}

impl UserAgentRotator {
    /// Create a new user agent rotator
    pub fn new(strategy: RotationStrategy, agent_type: UserAgentType) -> Self {
        let agents = Self::get_agents_for_type(&agent_type);

        Self {
            strategy,
            agent_type,
            agents,
            current_index: Arc::new(RwLock::new(0)),
            domain_agents: Arc::new(RwLock::new(HashMap::new())),
            domain_agent_expiry: Arc::new(RwLock::new(HashMap::new())),
            expiry_duration: Duration::from_secs(3600), // 1 hour per domain
        }
    }

    /// Create with default settings (random desktop agents)
    pub fn default() -> Self {
        Self::new(RotationStrategy::Random, UserAgentType::Desktop)
    }

    /// Create a fixed user agent rotator
    pub fn fixed(user_agent: String) -> Self {
        let mut rotator = Self::default();
        rotator.strategy = RotationStrategy::Fixed(user_agent);
        rotator
    }

    /// Get agents for a specific type
    fn get_agents_for_type(agent_type: &UserAgentType) -> Vec<String> {
        match agent_type {
            UserAgentType::Desktop => DESKTOP_USER_AGENTS.iter().map(|s| s.to_string()).collect(),
            UserAgentType::Mobile => MOBILE_USER_AGENTS.iter().map(|s| s.to_string()).collect(),
            UserAgentType::Bot => BOT_USER_AGENTS.iter().map(|s| s.to_string()).collect(),
            UserAgentType::All => {
                let mut all = Vec::new();
                all.extend(DESKTOP_USER_AGENTS.iter().map(|s| s.to_string()));
                all.extend(MOBILE_USER_AGENTS.iter().map(|s| s.to_string()));
                all.extend(BOT_USER_AGENTS.iter().map(|s| s.to_string()));
                all
            }
        }
    }

    /// Get the next user agent based on the rotation strategy
    pub async fn get_user_agent(&self, url: &str) -> String {
        match &self.strategy {
            RotationStrategy::Fixed(agent) => agent.clone(),
            RotationStrategy::Random => self.get_random_agent(),
            RotationStrategy::RoundRobin => self.get_round_robin_agent().await,
            RotationStrategy::PerDomain => self.get_per_domain_agent(url).await,
            RotationStrategy::ContentAware => self.get_content_aware_agent(url),
        }
    }

    /// Get a random user agent
    fn get_random_agent(&self) -> String {
        if self.agents.is_empty() {
            return super::robots::DEFAULT_USER_AGENT.to_string();
        }

        let mut rng = rand::thread_rng();
        self.agents
            .choose(&mut rng)
            .cloned()
            .unwrap_or_else(|| super::robots::DEFAULT_USER_AGENT.to_string())
    }

    /// Get the next user agent in round-robin fashion
    async fn get_round_robin_agent(&self) -> String {
        if self.agents.is_empty() {
            return super::robots::DEFAULT_USER_AGENT.to_string();
        }

        let mut index = self.current_index.write().await;
        let agent = self.agents[*index % self.agents.len()].clone();
        *index = (*index + 1) % self.agents.len();
        agent
    }

    /// Get a consistent user agent per domain
    async fn get_per_domain_agent(&self, url: &str) -> String {
        // Extract domain from URL
        let domain = if let Ok(parsed) = url::Url::parse(url) {
            parsed.host_str().unwrap_or("unknown").to_string()
        } else {
            "unknown".to_string()
        };

        // Clean up expired entries
        self.cleanup_expired_domains().await;

        // Check if we already have an agent for this domain
        {
            let agents = self.domain_agents.read().await;
            let expiries = self.domain_agent_expiry.read().await;

            if let Some(agent) = agents.get(&domain) {
                if let Some(expiry) = expiries.get(&domain) {
                    if Instant::now() < *expiry {
                        return agent.clone();
                    }
                }
            }
        }

        // Assign a new agent for this domain
        let agent = self.get_random_agent();
        let expiry = Instant::now() + self.expiry_duration;

        let mut agents = self.domain_agents.write().await;
        let mut expiries = self.domain_agent_expiry.write().await;

        agents.insert(domain.clone(), agent.clone());
        expiries.insert(domain, expiry);

        agent
    }

    /// Get user agent based on content type
    fn get_content_aware_agent(&self, url: &str) -> String {
        let url_lower = url.to_lowercase();

        // Use bot agents for assets and APIs
        if url_lower.ends_with(".css")
            || url_lower.ends_with(".js")
            || url_lower.ends_with(".json")
            || url_lower.contains("/api/")
            || url_lower.contains("/static/")
            || url_lower.contains("/assets/")
        {
            BOT_USER_AGENTS[0].to_string()
        } else {
            // Use desktop agent for HTML content
            self.get_random_agent()
        }
    }

    /// Clean up expired domain-agent mappings
    async fn cleanup_expired_domains(&self) {
        let now = Instant::now();
        let mut agents = self.domain_agents.write().await;
        let mut expiries = self.domain_agent_expiry.write().await;

        let expired_domains: Vec<String> = expiries
            .iter()
            .filter(|(_, expiry)| now > **expiry)
            .map(|(domain, _)| domain.clone())
            .collect();

        for domain in expired_domains {
            agents.remove(&domain);
            expiries.remove(&domain);
        }
    }

    /// Get statistics about user agent usage
    pub async fn get_stats(&self) -> UserAgentStats {
        let domain_count = self.domain_agents.read().await.len();
        let current_index = *self.current_index.read().await;

        UserAgentStats {
            strategy: format!("{:?}", self.strategy),
            agent_type: format!("{:?}", self.agent_type),
            total_agents: self.agents.len(),
            domains_tracked: domain_count,
            current_round_robin_index: current_index,
        }
    }

    /// Add a custom user agent to the pool
    pub fn add_custom_agent(&mut self, agent: String) {
        self.agents.push(agent);
    }

    /// Clear all custom agents and reset to defaults
    pub fn reset_to_defaults(&mut self) {
        self.agents = Self::get_agents_for_type(&self.agent_type);
    }
}

/// Statistics about user agent usage
#[derive(Debug, Clone)]
pub struct UserAgentStats {
    pub strategy: String,
    pub agent_type: String,
    pub total_agents: usize,
    pub domains_tracked: usize,
    pub current_round_robin_index: usize,
}

/// Global user agent rotator instance
static USER_AGENT_ROTATOR: once_cell::sync::Lazy<Arc<UserAgentRotator>> =
    once_cell::sync::Lazy::new(|| {
        Arc::new(UserAgentRotator::new(
            RotationStrategy::PerDomain,
            UserAgentType::Desktop,
        ))
    });

/// Get the global user agent rotator
pub fn get_rotator() -> Arc<UserAgentRotator> {
    USER_AGENT_ROTATOR.clone()
}

/// Get a user agent for a specific URL
pub async fn get_user_agent_for_url(url: &str) -> String {
    get_rotator().get_user_agent(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_rotation() {
        let rotator = UserAgentRotator::new(RotationStrategy::Random, UserAgentType::Desktop);
        let agent = rotator.get_user_agent("https://example.com").await;
        assert!(!agent.is_empty());
    }

    #[tokio::test]
    async fn test_round_robin_rotation() {
        let rotator = UserAgentRotator::new(RotationStrategy::RoundRobin, UserAgentType::Desktop);
        let agent1 = rotator.get_user_agent("https://example.com").await;
        let agent2 = rotator.get_user_agent("https://example.com").await;
        assert!(!agent1.is_empty());
        assert!(!agent2.is_empty());
    }

    #[tokio::test]
    async fn test_per_domain_consistency() {
        let rotator = UserAgentRotator::new(RotationStrategy::PerDomain, UserAgentType::Desktop);
        let agent1 = rotator.get_user_agent("https://example.com/page1").await;
        let agent2 = rotator.get_user_agent("https://example.com/page2").await;
        assert_eq!(agent1, agent2); // Same domain should get same agent

        let agent3 = rotator.get_user_agent("https://other.com/page1").await;
        // Different domain might get different agent (not guaranteed but likely)
        assert!(!agent3.is_empty());
    }
}

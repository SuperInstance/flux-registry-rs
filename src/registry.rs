//! FLUX Registry — remote registry client.
//!
//! Fetches policy JSON from the static GitHub raw URL index.

use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

/// Base URL for the static registry on GitHub raw.
const REGISTRY_BASE: &str =
    "https://raw.githubusercontent.com/SuperInstance/flux-registry/main";
const INDEX_URL: &str =
    "https://raw.githubusercontent.com/SuperInstance/flux-registry/main/registry/index.json";

/// Timeout for HTTP requests.
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Minimum-age before the cache is considered stale (5 min).
const CACHE_TTL: Duration = Duration::from_secs(300);

/// Client for the remote registry.
pub struct RegistryClient {
    agent: ureq::Agent,
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClient {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(HTTP_TIMEOUT)
            .user_agent("flux-registry-rs/0.1.0")
            .build();
        Self { agent }
    }

    /// Fetch the index JSON from the registry.
    pub fn fetch_index(&self) -> Result<RegistryIndex> {
        let resp = self
            .agent
            .get(INDEX_URL)
            .call()
            .map_err(|e| anyhow!("registry index fetch failed: {e}"))?;
        let index: RegistryIndex = resp.into_json()?;
        Ok(index)
    }

    /// Fetch a full policy JSON from the registry by name.
    pub fn fetch_policy(&self, name: &str) -> Result<crate::policy::Policy> {
        let url = format!("{REGISTRY_BASE}/registry/{name}.json");
        let resp = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| anyhow!("policy fetch failed for '{name}': {e}"))?;
        let policy: crate::policy::Policy = resp
            .into_json()
            .with_context(|| format!("parsing policy JSON from {url}"))?;
        Ok(policy)
    }

    /// Check if the on-disk cache is still fresh.
    pub fn cache_is_fresh(cache_path: &std::path::Path) -> bool {
        if !cache_path.exists() {
            return false;
        }
        let metadata = std::fs::metadata(cache_path);
        match metadata {
            Ok(m) => m
                .modified()
                .ok()
                .map(|t| SystemTime::now().duration_since(t).unwrap_or(CACHE_TTL) < CACHE_TTL)
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

// ------------------------------------------------------------------
// Response types
// ------------------------------------------------------------------

/// Top-level index file.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryIndex {
    pub registry: String,
    pub version: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub policies: Vec<IndexEntry>,
    #[serde(default)]
    pub total_policies: u32,
}

/// One entry in the index.
#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub bytecode_size: usize,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

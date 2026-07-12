//! FLUX Registry — local policy store.
//!
//! Manages installed FLUX policies in `~/.flux/policies/`.

use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::policy::Policy;

/// The local filesystem location for installed policies.
pub fn flux_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".flux")
}

pub fn policy_dir() -> PathBuf {
    flux_home().join("policies")
}

/// Local policy store — CRUD on `~/.flux/policies/*.json`.
pub struct PolicyStore {
    policy_dir: PathBuf,
}

impl PolicyStore {
    /// Create a store pointing at the default `~/.flux/policies/`.
    pub fn new() -> Self {
        Self::with_dir(policy_dir())
    }

    /// Create a store with a custom directory (used in tests).
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { policy_dir: dir }
    }

    /// Ensure the directory exists.
    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.policy_dir)
            .with_context(|| format!("creating {}", self.policy_dir.display()))?;
        Ok(())
    }

    /// Return the path for a given policy name.
    fn path(&self, name: &str) -> PathBuf {
        self.policy_dir.join(format!("{name}.json"))
    }

    // ------------------------------------------------------------------
    // Install / remove
    // ------------------------------------------------------------------

    /// Write a policy JSON to the local store.
    pub fn save(&self, policy: &Policy) -> Result<PathBuf> {
        self.ensure_dir()?;
        let dest = self.path(&policy.name);
        let json = serde_json::to_string_pretty(policy)?;
        fs::write(&dest, json)?;
        Ok(dest)
    }

    /// Remove a policy from the local store.
    pub fn remove(&self, name: &str) -> bool {
        let path = self.path(name);
        path.exists() && fs::remove_file(path).is_ok()
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    /// Load an installed policy.
    pub fn get(&self, name: &str) -> Result<Policy> {
        let path = self.path(name);
        if !path.exists() {
            return Err(anyhow!(
                "Policy '{}' is not installed. Run: flux-registry install {name}",
                name
            ));
        }
        let data = fs::read_to_string(&path)?;
        let policy: Policy = serde_json::from_str(&data)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(policy)
    }

    /// Return `true` if the policy is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        self.path(name).exists()
    }

    /// List metadata for every installed policy.
    pub fn list_installed(&self) -> Vec<InstalledPolicy> {
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(&self.policy_dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(p) = serde_json::from_str::<Policy>(&data) {
                    out.push(InstalledPolicy {
                        name: p.name.clone(),
                        version: p.version.clone(),
                        description: p.description.clone(),
                        bytecode_size: p.bytecode_size,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Iterate over all installed policy paths.
    pub fn iter_files(&self) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if !self.policy_dir.exists() {
            return Ok(files);
        }
        for entry in fs::read_dir(&self.policy_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }
}

impl Default for PolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight metadata for `list` output.
#[derive(Debug, Clone, Deserialize)]
pub struct InstalledPolicy {
    pub name: String,
    pub version: String,
    pub description: String,
    pub bytecode_size: usize,
}

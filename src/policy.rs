//! FLUX Registry — policy data model.
//!
//! The JSON manifest installed under `~/.flux/policies/<name>.json`.

use serde::{Deserialize, Serialize};

/// A FLUX policy — the full JSON manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Base64-encoded FLX0 binary.
    pub bytecode: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub bytecode_hash: String,
    #[serde(default)]
    pub bytecode_size: usize,
    #[serde(default)]
    pub inputs: Vec<PolicyField>,
    #[serde(default)]
    pub outputs: Vec<PolicyField>,
    #[serde(default)]
    pub conservation: Conservation,
    #[serde(default)]
    pub conformance: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_license")]
    pub license: String,
}

fn default_license() -> String {
    "MIT".into()
}

/// An input or output field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub register: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub values: std::collections::HashMap<String, String>,
}

/// Conservation limits declared by the policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conservation {
    #[serde(default = "default_max_steps")]
    pub max_steps: u64,
    #[serde(default = "default_memory_budget")]
    pub memory_budget: u64,
}

fn default_max_steps() -> u64 {
    1000
}
fn default_memory_budget() -> u64 {
    1024
}

impl Policy {
    /// Decode the base64-encoded bytecode.
    pub fn decode_bytecode(&self) -> anyhow::Result<Vec<u8>> {
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.decode(&self.bytecode)?)
    }
}

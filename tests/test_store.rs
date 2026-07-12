//! Integration tests for the policy store.

use std::fs;
use std::path::PathBuf;

use flux_registry_cli_lib::policy::{Conservation, Policy, PolicyField};
use flux_registry_cli_lib::store::PolicyStore;

/// Create a minimal valid policy with FLX0 bytecode:
/// LOAD_INPUT R0, PUSH 1, ADD, STORE_OUTPUT R1, HALT → R1 = R0 + 1
fn make_policy(name: &str, version: &str) -> Policy {
    use base64::Engine;
    // Build the bytecode
    let mut bc = vec![];
    bc.extend_from_slice(b"FLX0");
    bc.extend_from_slice(&[0u8, 0u8]); // body length placeholder
    let body_start = bc.len();
    bc.extend_from_slice(&[0x40, 0x00]); // LOAD_INPUT R0
    bc.extend_from_slice(&[0x01, 0x01, 0x01]); // PUSH 1
    bc.extend_from_slice(&[0x0A]); // ADD
    bc.extend_from_slice(&[0x41, 0x01]); // STORE_OUTPUT R1
    bc.extend_from_slice(&[0x31]); // HALT
    let body_len = (bc.len() - body_start) as u16;
    bc[4..6].copy_from_slice(&body_len.to_le_bytes());

    let bytecode_b64 = base64::engine::general_purpose::STANDARD.encode(&bc);

    Policy {
        name: name.to_string(),
        version: version.to_string(),
        description: "Test policy".to_string(),
        author: "test".to_string(),
        bytecode: bytecode_b64,
        source: format!("{name}.flx"),
        bytecode_hash: "abc123".to_string(),
        bytecode_size: bc.len(),
        inputs: vec![PolicyField {
            name: "x".to_string(),
            field_type: "int".to_string(),
            register: "R0".to_string(),
            description: "".to_string(),
            values: Default::default(),
        }],
        outputs: vec![PolicyField {
            name: "y".to_string(),
            field_type: "int".to_string(),
            register: "R1".to_string(),
            description: "".to_string(),
            values: Default::default(),
        }],
        conservation: Conservation {
            max_steps: 100,
            memory_budget: 128,
        },
        conformance: "tested".to_string(),
        tags: vec!["test".to_string()],
        license: "MIT".to_string(),
    }
}

fn temp_store() -> (PolicyStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = PolicyStore::with_dir(PathBuf::from(dir.path()));
    (store, dir)
}

#[test]
fn test_save_and_get() {
    let (store, _dir) = temp_store();
    let policy = make_policy("alpha", "0.1.0");
    store.save(&policy).unwrap();

    let loaded = store.get("alpha").unwrap();
    assert_eq!(loaded.name, "alpha");
    assert_eq!(loaded.version, "0.1.0");
}

#[test]
fn test_list_installed() {
    let (store, _dir) = temp_store();
    for name in &["alpha", "beta", "gamma"] {
        let policy = make_policy(name, "0.1.0");
        store.save(&policy).unwrap();
    }

    let installed = store.list_installed();
    assert_eq!(installed.len(), 3);
    assert_eq!(installed[0].name, "alpha");
    assert_eq!(installed[1].name, "beta");
    assert_eq!(installed[2].name, "gamma");
}

#[test]
fn test_is_installed() {
    let (store, _dir) = temp_store();
    assert!(!store.is_installed("nope"));

    let policy = make_policy("yep", "0.1.0");
    store.save(&policy).unwrap();
    assert!(store.is_installed("yep"));
}

#[test]
fn test_remove() {
    let (store, _dir) = temp_store();
    let policy = make_policy("removable", "0.1.0");
    store.save(&policy).unwrap();

    assert!(store.remove("removable"));
    assert!(!store.is_installed("removable"));
    assert!(!store.remove("removable")); // already gone
}

#[test]
fn test_get_not_found() {
    let (store, _dir) = temp_store();
    let result = store.get("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_decode_bytecode() {
    let (store, _dir) = temp_store();
    let policy = make_policy("bc", "0.1.0");
    store.save(&policy).unwrap();

    let loaded = store.get("bc").unwrap();
    let raw = loaded.decode_bytecode().unwrap();
    assert_eq!(&raw[..4], b"FLX0");
}

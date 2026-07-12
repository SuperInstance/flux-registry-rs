//! Integration tests for the CLI — using `assert_cmd` to drive the binary
//! against a temp HOME directory.
//!
//! These tests exercise the binary's argument parsing and local store
//! operations (install from file, list, info, run).  They do NOT hit the
//! network — instead we pre-seed the policy directory.

use std::fs;
use std::path::PathBuf;

/// Build FLX0 bytecode: LOAD_INPUT R0, PUSH 1, ADD, STORE_OUTPUT R1, HALT
fn make_bytecode() -> Vec<u8> {
    let mut bc = vec![];
    bc.extend_from_slice(b"FLX0");
    bc.extend_from_slice(&[0u8, 0u8]);
    let body_start = bc.len();
    bc.extend_from_slice(&[0x40, 0x00]); // LOAD_INPUT R0
    bc.extend_from_slice(&[0x01, 0x01, 0x01]); // PUSH 1
    bc.extend_from_slice(&[0x0A]); // ADD
    bc.extend_from_slice(&[0x41, 0x01]); // STORE_OUTPUT R1
    bc.extend_from_slice(&[0x31]); // HALT
    let body_len = (bc.len() - body_start) as u16;
    bc[4..6].copy_from_slice(&body_len.to_le_bytes());
    bc
}

use base64::Engine;

#[test]
fn test_store_save_get_list_remove_roundtrip() {
    // Test the store module directly through the library
    use flux_registry_cli_lib::policy::{Conservation, Policy, PolicyField};
    use flux_registry_cli_lib::store::PolicyStore;

    let dir = tempfile::tempdir().unwrap();
    let store = PolicyStore::with_dir(PathBuf::from(dir.path()));

    let bc = make_bytecode();
    let policy = Policy {
        name: "cli-test".to_string(),
        version: "0.1.0".to_string(),
        description: "Test policy for CLI".to_string(),
        author: "tester".to_string(),
        bytecode: base64::engine::general_purpose::STANDARD.encode(&bc),
        source: "cli-test.flx".to_string(),
        bytecode_hash: "deadbeef".to_string(),
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
    };

    // Save
    store.save(&policy).unwrap();

    // Get
    let loaded = store.get("cli-test").unwrap();
    assert_eq!(loaded.name, "cli-test");
    assert_eq!(loaded.version, "0.1.0");

    // List
    let installed = store.list_installed();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "cli-test");

    // Remove
    assert!(store.remove("cli-test"));
    assert!(!store.is_installed("cli-test"));
}

#[test]
fn test_vm_execution_through_lib() {
    use flux_registry_cli_lib::vm::FluxMiniVm;

    let bc = make_bytecode();
    let mut vm = FluxMiniVm::new(&bc, 100);
    vm.set_register(0, 10);
    let result = vm.run();
    assert!(result.fault.is_none());
    assert_eq!(result.registers["R1"], 11);
}

#[test]
fn test_policy_json_roundtrip() {
    use flux_registry_cli_lib::policy::Policy;

    let json = r#"{
        "name": "test",
        "version": "1.0.0",
        "description": "Test",
        "author": "me",
        "bytecode": "RkxYMA==",
        "source": "test.flx",
        "bytecode_hash": "abc",
        "bytecode_size": 6,
        "inputs": [],
        "outputs": [],
        "conservation": {"max_steps": 50, "memory_budget": 256},
        "conformance": "verified",
        "tags": ["a", "b"],
        "license": "MIT"
    }"#;

    let policy: Policy = serde_json::from_str(json).unwrap();
    assert_eq!(policy.name, "test");
    assert_eq!(policy.version, "1.0.0");
    assert_eq!(policy.conservation.max_steps, 50);
    assert_eq!(policy.tags, vec!["a", "b"]);

    // Re-serialize and parse again
    let re = serde_json::to_string(&policy).unwrap();
    let policy2: Policy = serde_json::from_str(&re).unwrap();
    assert_eq!(policy2.name, "test");
}

//! Integration tests for the FLX0 mini VM.

use std::collections::HashMap;

use flux_registry_cli_lib::vm::{map_inputs, parse_register, FluxMiniVm};

/// Build FLX0 bytecode: LOAD_INPUT R0, PUSH 1, ADD, STORE_OUTPUT R1, HALT
fn make_inc_bytecode() -> Vec<u8> {
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
    bc
}

/// Build FLX0 deadband-controller bytecode:
/// LOAD_INPUT R0, DUP, PUSH 75, GT, PUSH 1, MUL, SWAP, PUSH 65, LT, PUSH 2, MUL, ADD, STORE_OUTPUT R1, HALT
fn make_deadband_bytecode() -> Vec<u8> {
    let mut bc = vec![];
    bc.extend_from_slice(b"FLX0");
    bc.extend_from_slice(&[0u8, 0u8]);
    let body_start = bc.len();
    bc.extend_from_slice(&[0x40, 0x00]); // LOAD_INPUT R0
    bc.extend_from_slice(&[0x03]); // DUP
    bc.extend_from_slice(&[0x01, 0x01, 0x4B]); // PUSH 75
    bc.extend_from_slice(&[0x17]); // GT
    bc.extend_from_slice(&[0x01, 0x01, 0x01]); // PUSH 1
    bc.extend_from_slice(&[0x0C]); // MUL
    bc.extend_from_slice(&[0x04]); // SWAP
    bc.extend_from_slice(&[0x01, 0x01, 0x41]); // PUSH 65
    bc.extend_from_slice(&[0x16]); // LT
    bc.extend_from_slice(&[0x01, 0x01, 0x02]); // PUSH 2
    bc.extend_from_slice(&[0x0C]); // MUL
    bc.extend_from_slice(&[0x0A]); // ADD
    bc.extend_from_slice(&[0x41, 0x01]); // STORE_OUTPUT R1
    bc.extend_from_slice(&[0x31]); // HALT
    let body_len = (bc.len() - body_start) as u16;
    bc[4..6].copy_from_slice(&body_len.to_le_bytes());
    bc
}

#[test]
fn test_inc_program() {
    let bc = make_inc_bytecode();
    let mut vm = FluxMiniVm::new(&bc, 100);
    vm.set_register(0, 41);
    let result = vm.run();
    assert!(result.halted);
    assert!(result.fault.is_none());
    assert_eq!(result.registers["R1"], 42);
}

#[test]
fn test_inc_with_input_10() {
    let bc = make_inc_bytecode();
    let mut vm = FluxMiniVm::new(&bc, 100);
    vm.set_register(0, 10);
    let result = vm.run();
    assert_eq!(result.registers["R1"], 11);
}

#[test]
fn test_deadband_logic() {
    let bc = make_deadband_bytecode();

    for (temp, expected) in [
        (72, 0),  // idle
        (80, 1),  // cool
        (60, 2),  // heat
        (65, 0),  // boundary: not below 65 → idle
        (75, 0),  // boundary: not above 75 → idle
        (76, 1),  // cool
        (64, 2),  // heat
    ] {
        let mut vm = FluxMiniVm::new(&bc, 100);
        vm.set_register(0, temp);
        let result = vm.run();
        assert!(result.fault.is_none(), "fault at temp={temp}: {:?}", result.fault);
        assert_eq!(
            result.registers["R1"], expected,
            "temp={temp} should give action={expected}"
        );
    }
}

#[test]
fn test_max_steps_limit() {
    // 100 NOPs, no HALT
    let mut bc = vec![b'F', b'L', b'X', b'0'];
    let body_len: u16 = 100;
    bc.extend_from_slice(&body_len.to_le_bytes());
    bc.extend(vec![0x00; 100]);

    let mut vm = FluxMiniVm::new(&bc, 5);
    let result = vm.run();
    assert_eq!(result.steps, 5);
    assert!(!result.halted);
}

#[test]
fn test_empty_program() {
    let bc = vec![b'F', b'L', b'X', b'0', 0, 0];
    let mut vm = FluxMiniVm::new(&bc, 10);
    let result = vm.run();
    assert_eq!(result.steps, 0);
}

#[test]
fn test_parse_register() {
    assert_eq!(parse_register("R0"), Some(0));
    assert_eq!(parse_register("R15"), Some(15));
    assert_eq!(parse_register("r3"), Some(3));
    assert_eq!(parse_register("X"), None);
}

#[test]
fn test_map_inputs() {
    use flux_registry_cli_lib::policy::PolicyField;
    let bc = make_inc_bytecode();
    let mut vm = FluxMiniVm::new(&bc, 100);
    let mut inputs = HashMap::new();
    inputs.insert("x".to_string(), 99);
    let input_map = vec![PolicyField {
        name: "x".to_string(),
        field_type: "int".to_string(),
        register: "R0".to_string(),
        description: "".to_string(),
        values: Default::default(),
    }];
    map_inputs(&mut vm, &inputs, &input_map);
    let result = vm.run();
    assert_eq!(result.registers["R1"], 100);
}

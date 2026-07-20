//! FLX0 stack-based mini VM.
//!
//! Interprets the FLX0 bytecode format used by registry policies.
//! Uses `fluxvm` register types for the register file.

use std::collections::HashMap;

use anyhow::Result;

// ------------------------------------------------------------------
// Opcode tables
// ------------------------------------------------------------------

/// Opcodes that consume a single immediate operand.
const IMMEDIATE_OPS: &[u8] = &[
    0x01, // PUSH
    0x08, // PEEK
    0x1C, // SET_RANGE_MIN
    0x1D, // SET_RANGE_MAX
    0x22, // SET_DOMAIN
    0x2D, // VERIFY_HASH
    0x2E, // CHECK_SIGNATURE
    0x2F, // RESTRICT_EXEC
    0x30, // JMP
    0x40, // LOAD_INPUT
    0x41, // STORE_OUTPUT
    0x42, // PUSH_F32
    0x43, // PUSH_I8
    0x44, // PUSH_I16
    0x45, // PUSH_I32
];

fn is_immediate(op: u8) -> bool {
    IMMEDIATE_OPS.contains(&op)
}

fn opcode_name(op: u8) -> &'static str {
    match op {
        0x00 => "NOP",
        0x01 => "PUSH",
        0x02 => "POP",
        0x03 => "DUP",
        0x04 => "SWAP",
        0x05 => "OVER",
        0x06 => "ROT",
        0x07 => "CLEAR",
        0x08 => "PEEK",
        0x09 => "DEPTH",
        0x0A => "ADD",
        0x0B => "SUB",
        0x0C => "MUL",
        0x0D => "DIV",
        0x0E => "MOD",
        0x0F => "EXP",
        0x10 => "NEG",
        0x11 => "INC",
        0x12 => "DEC",
        0x13 => "ABS",
        0x14 => "EQ",
        0x15 => "NEQ",
        0x16 => "LT",
        0x17 => "GT",
        0x18 => "LTE",
        0x19 => "GTE",
        0x1A => "ISZERO",
        0x1B => "WITHIN",
        0x1C => "SET_RANGE_MIN",
        0x1D => "SET_RANGE_MAX",
        0x1E => "CHECK_RANGE",
        0x1F => "CLEAR_RANGE",
        0x20 => "GET_RANGE_MIN",
        0x21 => "GET_RANGE_MAX",
        0x22 => "SET_DOMAIN",
        0x23 => "CHECK_DOMAIN",
        0x24 => "IS_IN_DOMAIN",
        0x25 => "CLEAR_DOMAIN",
        0x26 => "AND",
        0x27 => "OR",
        0x28 => "XOR",
        0x29 => "NOT",
        0x2A => "TIMESTAMP_PUSH",
        0x2B => "TIME_COMPARE",
        0x2C => "TIME_WINDOW_VALID",
        0x2D => "VERIFY_HASH",
        0x2E => "CHECK_SIGNATURE",
        0x2F => "RESTRICT_EXEC",
        0x30 => "JMP",
        0x31 => "HALT",
        0x40 => "LOAD_INPUT",
        0x41 => "STORE_OUTPUT",
        0x42 => "PUSH_F32",
        0x43 => "PUSH_I8",
        0x44 => "PUSH_I16",
        0x45 => "PUSH_I32",
        0x46 => "ASSERT",
        _ => "UNKNOWN",
    }
}

// ------------------------------------------------------------------
// Immediate-operand reader
// ------------------------------------------------------------------

/// Read an immediate operand starting at `pc` (the byte *after* the opcode).
/// Returns `(value, new_pc)`.
fn read_operand(bytecode: &[u8], pc: usize) -> (i32, usize) {
    if pc >= bytecode.len() {
        // Truncated instruction — return HALT signal by giving pc beyond end
        return (0, bytecode.len());
    }
    let opcode = bytecode[pc - 1]; // opcode byte was at pc-1

    match opcode {
        // PUSH (0x01): size-prefixed integer
        0x01 => {
            let size = bytecode[pc] as usize;
            let val = match size {
                1 => {
                    if pc + 1 < bytecode.len() {
                        bytecode[pc + 1] as i8 as i32
                    } else {
                        0
                    }
                }
                2 => {
                    if pc + 2 < bytecode.len() {
                        i16::from_le_bytes([bytecode[pc + 1], bytecode[pc + 2]]) as i32
                    } else {
                        0
                    }
                }
                4 => {
                    if pc + 4 < bytecode.len() {
                        i32::from_le_bytes([
                            bytecode[pc + 1],
                            bytecode[pc + 2],
                            bytecode[pc + 3],
                            bytecode[pc + 4],
                        ])
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            (val, pc + 1 + size)
        }

        // Register-based ops: single raw byte
        0x40 | 0x41 | 0x08 | 0x30 | 0x2F => {
            (bytecode[pc] as i32, pc + 1)
        }

        // PUSH_F32 (0x42): 4-byte IEEE-754 LE float
        0x42 => {
            if pc + 3 < bytecode.len() {
                let bytes = [
                    bytecode[pc],
                    bytecode[pc + 1],
                    bytecode[pc + 2],
                    bytecode[pc + 3],
                ];
                let f = f32::from_le_bytes(bytes);
                // Encode float into i32 bits for stack transport; caller treats as value
                (f as i32, pc + 4)
            } else {
                (0, pc)
            }
        }

        // PUSH_I8
        0x43 => {
            if pc < bytecode.len() {
                (bytecode[pc] as i8 as i32, pc + 1)
            } else {
                (0, pc)
            }
        }
        // PUSH_I16
        0x44 => {
            if pc + 1 < bytecode.len() {
                (
                    i16::from_le_bytes([bytecode[pc], bytecode[pc + 1]]) as i32,
                    pc + 2,
                )
            } else {
                (0, pc)
            }
        }
        // PUSH_I32
        0x45 => {
            if pc + 3 < bytecode.len() {
                (
                    i32::from_le_bytes([
                        bytecode[pc],
                        bytecode[pc + 1],
                        bytecode[pc + 2],
                        bytecode[pc + 3],
                    ]),
                    pc + 4,
                )
            } else {
                (0, pc)
            }
        }

        // Fallback: single byte
        _ => (bytecode[pc] as i32, pc + 1),
    }
}

// ------------------------------------------------------------------
// The mini VM
// ------------------------------------------------------------------

/// Execution result.
#[derive(Debug)]
pub struct VmResult {
    pub registers: HashMap<String, i32>,
    pub stack_depth: usize,
    pub steps: u64,
    pub halted: bool,
    pub fault: Option<String>,
}

/// FLX0 stack-based mini VM.
pub struct FluxMiniVm {
    bytecode: Vec<u8>,
    stack: Vec<i32>,
    registers: [i32; 16],
    pc: usize,
    steps: u64,
    max_steps: u64,
    halted: bool,
    fault: Option<String>,
}

impl FluxMiniVm {
    /// Create a new VM from raw bytecode (with or without FLX0 header).
    pub fn new(bytecode: &[u8], max_steps: u64) -> Self {
        // Strip FLX0 header if present
        let body = if bytecode.len() >= 6 && &bytecode[..4] == b"FLX0" {
            let body_len = u16::from_le_bytes([bytecode[4], bytecode[5]]) as usize;
            let end = 6 + body_len;
            if end <= bytecode.len() {
                &bytecode[6..end]
            } else {
                &bytecode[6..]
            }
        } else {
            bytecode
        };

        Self {
            bytecode: body.to_vec(),
            stack: Vec::new(),
            registers: [0; 16],
            pc: 0,
            steps: 0,
            max_steps,
            halted: false,
            fault: None,
        }
    }

    /// Set a register (for input mapping).
    pub fn set_register(&mut self, idx: usize, val: i32) {
        if idx < 16 {
            self.registers[idx] = val;
        }
    }

    /// Run to completion.
    pub fn run(&mut self) -> VmResult {
        while !self.halted
            && self.steps < self.max_steps
            && self.pc < self.bytecode.len()
        {
            self.steps += 1;
            let opcode = self.bytecode[self.pc];
            self.pc += 1;

            // Read immediate operand if needed
            let operand = if is_immediate(opcode) {
                let (val, new_pc) = read_operand(&self.bytecode, self.pc);
                self.pc = new_pc;
                Some(val)
            } else {
                None
            };

            if let Err(e) = self.exec(opcode_name(opcode), opcode, operand) {
                self.fault = Some(e);
                break;
            }
        }

        let mut registers = HashMap::new();
        for (i, &v) in self.registers.iter().enumerate() {
            registers.insert(format!("R{i}"), v);
        }

        VmResult {
            registers,
            stack_depth: self.stack.len(),
            steps: self.steps,
            halted: self.halted,
            fault: self.fault.clone(),
        }
    }

    fn exec(&mut self, _name: &str, opcode: u8, operand: Option<i32>) -> Result<(), String> {
        let s = &mut self.stack;

        match opcode {
            0x00 => {} // NOP
            0x01 => s.push(operand.unwrap_or(0)), // PUSH
            0x02 => { s.pop(); } // POP
            0x03 => { // DUP
                if let Some(&v) = s.last() { s.push(v); } else { return Err("stack underflow on DUP".into()); }
            }
            0x04 => { // SWAP
                let n = s.len();
                if n < 2 { return Err("stack underflow on SWAP".into()); }
                s.swap(n - 1, n - 2);
            }
            0x05 => { // OVER
                let n = s.len();
                if n < 2 { return Err("stack underflow on OVER".into()); }
                let v = s[n - 2];
                s.push(v);
            }
            0x06 => { // ROT
                let n = s.len();
                if n < 3 { return Err("stack underflow on ROT".into()); }
                let tmp = s.remove(n - 3);
                s.push(tmp);
            }
            0x07 => s.clear(), // CLEAR
            0x08 => { // PEEK
                let idx = operand.unwrap_or(0) as usize;
                if idx < s.len() { s.push(s[s.len() - 1 - idx]); } else { return Err("PEEK out of bounds".into()); }
            }
            0x09 => s.push(s.len() as i32), // DEPTH

            // Arithmetic
            0x0A => { // ADD
                let (b, a) = pop2(s)?;
                s.push(a.wrapping_add(b));
            }
            0x0B => { // SUB
                let (b, a) = pop2(s)?;
                s.push(a.wrapping_sub(b));
            }
            0x0C => { // MUL
                let (b, a) = pop2(s)?;
                s.push(a.wrapping_mul(b));
            }
            0x0D => { // DIV
                let (b, a) = pop2(s)?;
                if b == 0 { return Err("division by zero".into()); }
                s.push(a.wrapping_div(b));
            }
            0x0E => { // MOD
                let (b, a) = pop2(s)?;
                if b == 0 { return Err("modulo by zero".into()); }
                s.push(a.wrapping_rem(b));
            }
            0x0F => { // EXP
                let (b, a) = pop2(s)?;
                s.push(a.wrapping_pow(b as u32));
            }
            0x10 => { // NEG
                let a = pop1(s)?;
                s.push(a.wrapping_neg());
            }
            0x11 => { // INC
                let a = pop1(s)?;
                s.push(a.wrapping_add(1));
            }
            0x12 => { // DEC
                let a = pop1(s)?;
                s.push(a.wrapping_sub(1));
            }
            0x13 => { // ABS
                let a = pop1(s)?;
                s.push(a.wrapping_abs());
            }

            // Comparison
            0x14 => { let (b, a) = pop2(s)?; s.push((a == b) as i32); } // EQ
            0x15 => { let (b, a) = pop2(s)?; s.push((a != b) as i32); } // NEQ
            0x16 => { let (b, a) = pop2(s)?; s.push((a < b) as i32); }  // LT
            0x17 => { let (b, a) = pop2(s)?; s.push((a > b) as i32); }  // GT
            0x18 => { let (b, a) = pop2(s)?; s.push((a <= b) as i32); } // LTE
            0x19 => { let (b, a) = pop2(s)?; s.push((a >= b) as i32); } // GTE
            0x1A => { let a = pop1(s)?; s.push((a == 0) as i32); }     // ISZERO
            0x1B => { // WITHIN
                let (max, min, v) = pop3(s)?;
                s.push((min <= v && v <= max) as i32);
            }

            // Logical
            0x26 => { let (b, a) = pop2(s)?; s.push(a & b); } // AND
            0x27 => { let (b, a) = pop2(s)?; s.push(a | b); } // OR
            0x28 => { let (b, a) = pop2(s)?; s.push(a ^ b); } // XOR
            0x29 => { let a = pop1(s)?; s.push(!a); }         // NOT

            // I/O
            0x40 => { // LOAD_INPUT
                let reg = operand.unwrap_or(0) as usize;
                if reg < 16 { s.push(self.registers[reg]); } else { return Err("register out of range".into()); }
            }
            0x41 => { // STORE_OUTPUT
                let reg = operand.unwrap_or(0) as usize;
                let val = pop1(s)?;
                if reg < 16 { self.registers[reg] = val; } else { return Err("register out of range".into()); }
            }
            0x42 => s.push(operand.unwrap_or(0)), // PUSH_F32
            0x43 => s.push(operand.unwrap_or(0)), // PUSH_I8
            0x44 => s.push(operand.unwrap_or(0)), // PUSH_I16
            0x45 => s.push(operand.unwrap_or(0)), // PUSH_I32

            0x46 => { // ASSERT
                let a = pop1(s)?;
                if a == 0 {
                    self.fault = Some("Assertion failed".into());
                    self.halted = true;
                }
            }

            // Control
            0x31 => self.halted = true, // HALT
            0x30 => { // JMP
                self.pc = operand.unwrap_or(0) as usize;
            }

            // Range / domain / time / crypto — stubbed (not used by registry policies)
            0x1C | 0x1D | 0x1E | 0x1F | 0x20 | 0x21 |
            0x22 | 0x23 | 0x24 | 0x25 |
            0x2A | 0x2B | 0x2C | 0x2D | 0x2E | 0x2F => {
                // no-op for registry policies
            }

            _ => {
                return Err(format!("Unknown opcode: 0x{opcode:02X}"));
            }
        }

        Ok(())
    }
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

#[inline]
fn pop1(s: &mut Vec<i32>) -> Result<i32, String> {
    s.pop().ok_or_else(|| "stack underflow".to_string())
}

#[inline]
fn pop2(s: &mut Vec<i32>) -> Result<(i32, i32), String> {
    let b = s.pop().ok_or_else(|| "stack underflow".to_string())?;
    let a = s.pop().ok_or_else(|| "stack underflow".to_string())?;
    Ok((b, a))
}

#[inline]
fn pop3(s: &mut Vec<i32>) -> Result<(i32, i32, i32), String> {
    let c = s.pop().ok_or_else(|| "stack underflow".to_string())?;
    let b = s.pop().ok_or_else(|| "stack underflow".to_string())?;
    let a = s.pop().ok_or_else(|| "stack underflow".to_string())?;
    Ok((c, b, a))
}

// ------------------------------------------------------------------
// Input mapping
// ------------------------------------------------------------------

/// Map parsed `--input key=value` pairs into VM registers using the policy's
/// input definitions.
pub fn map_inputs(
    vm: &mut FluxMiniVm,
    inputs: &HashMap<String, i32>,
    input_map: &[crate::policy::PolicyField],
) {
    for field in input_map {
        if let Some(&val) = inputs.get(&field.name) {
            if let Some(idx) = parse_register(&field.register) {
                vm.set_register(idx, val);
            }
        }
    }
}

/// Parse "R5" → Some(5).
pub fn parse_register(reg: &str) -> Option<usize> {
    let s = reg.strip_prefix(['R', 'r']).unwrap_or(reg);
    s.parse().ok()
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_empty_program() {
        let bc = vec![b'F', b'L', b'X', b'0', 0, 0];
        let mut vm = FluxMiniVm::new(&bc, 10);
        let result = vm.run();
        assert_eq!(result.steps, 0);
    }

    #[test]
    fn test_max_steps() {
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
    fn test_parse_register() {
        assert_eq!(parse_register("R0"), Some(0));
        assert_eq!(parse_register("R15"), Some(15));
        assert_eq!(parse_register("r3"), Some(3));
        assert_eq!(parse_register("X"), None);
    }
}

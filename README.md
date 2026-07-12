# 📦 FLUX Registry (Rust)

![Crates.io](https://img.shields.io/crates/v/flux-registry-cli)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tests](https://img.shields.io/badge/tests-passing-brightgreen)
![License](https://img.shields.io/badge/License-MIT-yellow)

> **npm for agent policies — now in Rust.** 🦀 Install and run pre-compiled FLUX bytecode policies from the static GitHub registry.

A Rust CLI for the FLUX Registry: download, inspect, and execute pre-compiled agent policies. Same policies, same bytecode, different shell.

---

## Philosophy

Part of [Working Animal Architecture](https://github.com/SuperInstance/AI-Writings), where **γ + η = C** (genome + nurture = capability). The FLUX Registry is the **breed catalog** — pre-compiled, verified policies that any working animal can be trained on. Install a policy like choosing a breed for a task: the right dog for the right job, the right bytecode for the right fence.

## Installation

```bash
cargo install flux-registry-cli
```

Or build from source:

```bash
git clone https://github.com/SuperInstance/flux-registry-rs.git
cd flux-registry-rs
cargo build --release
# Binary at target/release/flux-registry
```

## Quick Start

```bash
# Browse available policies in the remote registry
flux-registry list --remote

# Install a policy locally
flux-registry install deadband-controller

# Run it with inputs
flux-registry run deadband-controller --input temperature=72
# → action=idle

flux-registry run deadband-controller --input temperature=80
# → action=cool

flux-registry run deadband-controller --input temperature=60
# → action=heat

# Inspect policy metadata
flux-registry info deadband-controller
```

## Commands

| Command | Description |
|---------|-------------|
| `flux-registry install <policy>` | Install a policy from the remote registry |
| `flux-registry list [--remote]` | List installed (or remote) policies |
| `flux-registry info <policy>` | Show detailed metadata, inputs, and outputs |
| `flux-registry run <policy> --input K=V [--input K2=V2]` | Execute a policy with given inputs |
| `flux-registry remove <policy>` | Remove an installed policy |
| `flux-registry update-index` | Refresh the local registry cache |

## Available Policies

| Policy | Description | Inputs | Outputs |
|--------|-------------|--------|---------|
| `deadband-controller` | Thermostat with hysteresis — AC at 75°F, heat at 65°F | `temperature` (°F) | `action`: idle / cool / heat |
| `budget-tracker` | Conservation budget enforcement — track resource depletion | `cost`, `budget` | `remaining`, `status`: ok / exceeded |
| `rate-limiter` | Token bucket rate limiting — check and consume tokens | `tokens`, `cost` | `remaining`, `allowed`: denied / allowed |
| `security-scanner` | Basic vulnerability detection — threshold-based policy check | `value`, `threshold` | `verdict`: safe / violation, `severity` |

## Policy Format

Each policy is a self-contained JSON file:

```json
{
  "name": "deadband-controller",
  "version": "0.1.0",
  "description": "Thermostat deadband controller",
  "author": "SuperInstance",
  "bytecode": "<base64-encoded FLX0 binary>",
  "source": "deadband-controller.flx",
  "bytecode_hash": "sha256...",
  "bytecode_size": 30,
  "inputs": [
    {"name": "temperature", "type": "float", "register": "R0"}
  ],
  "outputs": [
    {"name": "action", "type": "int", "register": "R1",
     "values": {"0": "idle", "1": "cool", "2": "heat"}}
  ],
  "conservation": {"max_steps": 100, "memory_budget": 256},
  "conformance": "verified on flux-vm 0.1.0, fluxvm 0.1.0, flux-js 0.1.0",
  "tags": ["iot", "thermostat", "hvac"],
  "license": "MIT"
}
```

### Conservation Guarantees

Every policy declares conservation limits:

```json
"conservation": {
  "max_steps": 100,
  "memory_budget": 256
}
```

The VM enforces these at runtime — a policy cannot exceed its step count or memory allocation. This makes FLUX policies safe to run as untrusted code.

## API Reference (Library)

The CLI also exposes a library crate (`flux_registry_cli_lib`) for embedding in other Rust applications:

### `policy` Module

| Type/Function | Description |
|---------------|-------------|
| `Policy` | Deserialized policy struct with metadata, bytecode, I/O specs |
| `Policy::from_json(s)` | Parse a policy from JSON string |
| `Policy::decode_bytecode()` | Decode base64 bytecode to raw bytes |

### `registry` Module

| Function | Description |
|----------|-------------|
| `fetch_index()` | Fetch the remote registry index from GitHub |
| `fetch_policy(name)` | Download a specific policy JSON |
| `list_remote()` | List all available policies in the remote registry |

### `store` Module

| Function | Description |
|----------|-------------|
| `install_policy(policy)` | Save policy to `~/.flux/policies/` |
| `list_installed()` | List all locally installed policies |
| `get_policy(name)` | Load a locally installed policy |
| `remove_policy(name)` | Delete a locally installed policy |

### `vm` Module

Built-in FLX0 stack-based mini VM for executing registry policies, plus `fluxvm` crate integration for register-based FLUX ISA compatibility.

```rust
use flux_registry_cli_lib::vm::run_policy;

let result = run_policy(&bytecode, inputs).unwrap();
println!("Output: {:?}", result.outputs);
```

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                 FLUX Registry (Rust)                  │
│                                                       │
│  GitHub (static JSON)          Local (~/.flux/)       │
│  ┌─────────────────┐          ┌──────────────────┐   │
│  │ registry/       │  install │ policies/        │   │
│  │   index.json    │ ───────▶ │   deadband.json  │   │
│  │   deadband.json │          │   budget.json    │   │
│  │   budget.json   │          └────────┬─────────┘   │
│  │   rate-limiter  │                   │              │
│  │   security.json │          ┌────────▼─────────┐   │
│  └─────────────────┘          │ flux-registry    │   │
│                               │ ┌──────────────┐ │   │
│                               │ │ FLX0 Mini VM │ │   │
│                               │ │ (built-in)   │ │   │
│                               │ └──────────────┘ │   │
│                               │ ┌──────────────┐ │   │
│                               │ │ fluxvm crate │ │   │
│                               │ │ (register    │ │   │
│                               │ │  ISA support)│ │   │
│                               │ └──────────────┘ │   │
│                               └──────────────────┘   │
└───────────────────────────────────────────────────────┘
```

The CLI includes a built-in FLX0 stack-based mini VM for executing registry policies locally. It also depends on the [`fluxvm`](https://crates.io/crates/fluxvm) crate for register-based FLUX ISA compatibility.

## Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Integration tests use tempfile for isolated policy installs
cargo test --test integration
```

## Cross-Implementation

| Runtime | Language | Install |
|---------|----------|---------|
| [flux-registry](https://github.com/SuperInstance/flux-registry) | Python | `pip install flux-registry` |
| **flux-registry-cli** | **Rust** | `cargo install flux-registry-cli` |
| [flux-js](https://github.com/SuperInstance/flux-js) | JavaScript | `npm install flux-js` |

The same policy bytecode runs across all FLUX implementations. Same bytecode — different shells.

## Ecosystem

### FLUX Runtime
- [flux-vm](https://github.com/SuperInstance/flux-vm) — Python VM (`pip install flux-vm`)
- [flux-core](https://github.com/SuperInstance/flux-core) — Rust VM (`cargo add fluxvm`)
- [flux-js](https://github.com/SuperInstance/flux-js) — JavaScript VM (`npm install flux-js`)

### Conservation
- [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer-rs) — Conservation-law enforcement for LLM outputs
- [flux-policy-tester](https://github.com/SuperInstance/flux-policy-tester-rs) — Testing framework for policies

### Tooling
- [flux-compiler](https://github.com/SuperInstance/flux-compiler-rs) — Bytecode assembler, disassembler, validator

### Theory
- [AI-Writings](https://github.com/SuperInstance/AI-Writings) — Paradigm essays

## License

MIT — Same bytecode, different shells. 🦀

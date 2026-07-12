# FLUX Registry (Rust)

> **npm for agent policies — now in Rust.** 🦀 Pre-compiled FLUX bytecode, installable and runnable.

A Rust CLI for the FLUX Registry — download, inspect, and execute pre-compiled
agent policies from the static GitHub registry. Same policies, same bytecode,
different shell.

## Philosophy

Part of [Working Animal Architecture](https://github.com/SuperInstance/AI-Writings), where **γ + η = C** (genome + nurture = capability). The FLUX Registry is the **breed catalog** — pre-compiled, verified policies that any working animal can be trained on. Install a policy like choosing a breed for a task: the right dog for the right job, the right bytecode for the right fence.

## Install

```bash
cargo install flux-registry-cli
```

## Quick Start

```bash
# Browse available policies
flux-registry list --remote

# Install a policy
flux-registry install deadband-controller

# Run it
flux-registry run deadband-controller --input temperature=72
# Output: action=idle

flux-registry run deadband-controller --input temperature=80
# Output: action=cool

flux-registry run deadband-controller --input temperature=60
# Output: action=heat

# Get details
flux-registry info deadband-controller
```

## Commands

| Command | Description |
|---------|-------------|
| `flux-registry install <policy>` | Install a policy from the remote registry |
| `flux-registry list [--remote]` | List installed (or remote) policies |
| `flux-registry info <policy>` | Show detailed metadata, inputs, outputs |
| `flux-registry run <policy> --input K=V` | Execute a policy with given inputs |
| `flux-registry remove <policy>` | Remove an installed policy |
| `flux-registry update-index` | Refresh the local registry cache |

## Available Policies

| Policy | Description | Inputs | Outputs |
|--------|-------------|--------|---------|
| `deadband-controller` | Thermostat with hysteresis — AC at 75°, heat at 65° | temperature (°F) | action: idle/cool/heat |
| `budget-tracker` | Conservation budget enforcement — track resource depletion | cost, budget | remaining, status: ok/exceeded |
| `rate-limiter` | Token bucket rate limiting — check and consume tokens | tokens, cost | remaining, allowed: denied/allowed |
| `security-scanner` | Basic vulnerability detection — threshold-based policy check | value, threshold | verdict: safe/violation, severity |

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

The CLI includes a built-in FLX0 stack-based mini VM for executing registry
policies locally. It also depends on the [`fluxvm`](https://crates.io/crates/fluxvm)
crate for register-based FLUX ISA compatibility.

## Policy Format

Each policy is a JSON file containing:

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
  "inputs": [{"name": "temperature", "type": "float", "register": "R0"}],
  "outputs": [{"name": "action", "type": "int", "register": "R1",
               "values": {"0": "idle", "1": "cool", "2": "heat"}}],
  "conservation": {"max_steps": 100, "memory_budget": 256},
  "conformance": "verified on flux-vm 0.1.0, fluxvm 0.1.0, flux-js 0.1.0",
  "tags": ["iot", "thermostat", "hvac"],
  "license": "MIT"
}
```

## Conservation Guarantees

Every policy declares conservation limits:

```json
"conservation": {
  "max_steps": 100,
  "memory_budget": 256
}
```

The VM enforces these at runtime — a policy cannot exceed its step count or
memory allocation. This makes FLUX policies safe to run as untrusted code.

## Cross-Implementation

This component exists in two languages:
- **Python** (`pip install flux-registry`) — [SuperInstance/flux-registry](https://github.com/SuperInstance/flux-registry)
- **Rust** (`cargo install flux-registry-cli`) — [SuperInstance/flux-registry-rs](https://github.com/SuperInstance/flux-registry-rs)

Both implement the same specification. Choose based on your runtime.

### Cross-Language Compatibility

The same policy bytecode runs across all FLUX implementations:

| Runtime | Language | Install |
|---------|----------|---------|
| [flux-registry](https://pypi.org/project/flux-registry/) | Python | `pip install flux-registry` |
| **flux-registry-cli** | **Rust** | `cargo install flux-registry-cli` |
| [flux-js](https://www.npmjs.com/package/flux-js) | JavaScript | `npm install flux-js` |

Same bytecode — different shells.

## Building from Source

```bash
git clone https://github.com/SuperInstance/flux-registry-rs.git
cd flux-registry-rs
cargo build --release
# Binary at target/release/flux-registry
```

## Testing

```bash
cargo test
```

## License

MIT — Same bytecode, different shells. 🦀

## Ecosystem

### FLUX Runtime
- [flux-vm](https://github.com/SuperInstance/flux-vm) — Python VM (`pip install flux-vm`)
- [flux-core](https://github.com/SuperInstance/flux-core) — Rust VM (`cargo add fluxvm`)
- [flux-js](https://github.com/SuperInstance/flux-js) — JavaScript VM (`npm install flux-js`)
- [flux-compiler](https://github.com/SuperInstance/flux-compiler) — Formal-methods compiler

### Registries
- [flux-registry](https://github.com/SuperInstance/flux-registry) — Python registry CLI
- **flux-registry-rs** — Rust registry CLI (this repo)

### Conservation
- [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer) — Conservation-law enforcement
- [flux-policy-tester](https://github.com/SuperInstance/flux-policy-tester) — Testing framework

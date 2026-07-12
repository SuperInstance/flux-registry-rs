//! FLUX Registry CLI — install and run pre-compiled agent policies.
//!
//! Commands:
//!   flux-registry install <policy>           Install a policy from the remote registry
//!   flux-registry list [--remote]            List installed (or remote) policies
//!   flux-registry info <policy>             Show detailed info about a policy
//!   flux-registry run <policy> --input K=V  Run a policy with given inputs
//!   flux-registry remove <policy>           Remove an installed policy
//!   flux-registry update-index              Refresh the local registry cache

use std::collections::HashMap;
use std::io::{self, Write};

use clap::{Parser, Subcommand};

use flux_registry_cli_lib::policy;
use flux_registry_cli_lib::registry;
use flux_registry_cli_lib::store::PolicyStore;
use flux_registry_cli_lib::vm::FluxMiniVm;

#[derive(Parser)]
#[command(
    name = "flux-registry",
    version = "0.1.0",
    about = "FLUX Registry — install and run pre-compiled agent policies",
    long_about = "Pre-compiled FLUX agent policies, installable and runnable.\n\
                  The registry is a static JSON index on GitHub — no server required."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a policy from the remote registry
    Install {
        /// Policy name (e.g. deadband-controller)
        name: String,
    },
    /// List policies (installed or remote)
    List {
        /// List remote registry instead of local
        #[arg(short, long)]
        remote: bool,
    },
    /// Show detailed info about a policy
    Info {
        /// Policy name
        name: String,
    },
    /// Run a policy with given inputs
    Run {
        /// Policy name
        name: String,
        /// Input as key=value (e.g. temperature=72)
        #[arg(short, long = "input", value_name = "KEY=VALUE")]
        inputs: Vec<String>,
        /// Show execution stats
        #[arg(short, long)]
        verbose: bool,
    },
    /// Remove an installed policy
    Remove {
        /// Policy name
        name: String,
    },
    /// Refresh the local registry index cache
    UpdateIndex,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_command(&cli.command) {
        eprintln!("❌ Error: {e:#}");
        std::process::exit(1);
    }
}

fn run_command(cmd: &Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Install { name } => cmd_install(name),
        Commands::List { remote } => cmd_list(*remote),
        Commands::Info { name } => cmd_info(name),
        Commands::Run {
            name,
            inputs,
            verbose,
        } => cmd_run(name, inputs, *verbose),
        Commands::Remove { name } => cmd_remove(name),
        Commands::UpdateIndex => cmd_update_index(),
    }
}

// ------------------------------------------------------------------
// Commands
// ------------------------------------------------------------------

fn cmd_install(name: &str) -> anyhow::Result<()> {
    let client = registry::RegistryClient::new();
    println!("⬇ Installing '{name}' from FLUX registry...");
    let policy = client.fetch_policy(name)?;
    let store = PolicyStore::new();
    let dest = store.save(&policy)?;
    println!("✅ Installed {} v{}", policy.name, policy.version);
    println!("   {}", policy.description);
    println!("   Bytecode: {} bytes", policy.bytecode_size);
    println!("   Location: {}", dest.display());
    Ok(())
}

fn cmd_list(remote: bool) -> anyhow::Result<()> {
    if remote {
        println!("📡 Remote registry policies:\n");
        let client = registry::RegistryClient::new();
        let index = client.fetch_index()?;
        let rows: Vec<PolicyRow> = index
            .policies
            .iter()
            .map(|e| PolicyRow {
                name: &e.name,
                version: &e.version,
                size: e.bytecode_size,
                description: &e.description,
            })
            .collect();
        print_policy_table(&rows);
    } else {
        let store = PolicyStore::new();
        let installed = store.list_installed();
        if installed.is_empty() {
            println!(
                "No policies installed. Use 'flux-registry install <name>' or 'flux-registry list --remote'."
            );
            return Ok(());
        }
        println!("📦 Installed policies:\n");
        let rows: Vec<PolicyRow> = installed
            .iter()
            .map(|p| PolicyRow {
                name: &p.name,
                version: &p.version,
                size: p.bytecode_size,
                description: &p.description,
            })
            .collect();
        print_policy_table(&rows);
    }
    Ok(())
}

fn cmd_info(name: &str) -> anyhow::Result<()> {
    let store = PolicyStore::new();

    let policy = match store.get(name) {
        Ok(p) => p,
        Err(_) => {
            // Check if it exists remotely
            let client = registry::RegistryClient::new();
            if let Ok(index) = client.fetch_index() {
                if index.policies.iter().any(|p| p.name == name) {
                    println!("Policy '{name}' exists in registry but is not installed.");
                    println!("Run: flux-registry install {name}");
                    return Ok(());
                }
            }
            return Err(anyhow::anyhow!("Policy '{name}' not found."));
        }
    };

    println!("  Name:     {}", policy.name);
    println!("  Version:  {}", policy.version);
    println!("  Author:   {}", policy.author);
    println!("  Source:   {}", policy.source);
    println!("  License:  {}", policy.license);
    println!();
    println!("  {}", policy.description);
    println!();
    println!("  Inputs:");
    for inp in &policy.inputs {
        println!(
            "    {}: {} ({}) — {}",
            inp.register, inp.name, inp.field_type, inp.description
        );
    }
    println!("  Outputs:");
    for out in &policy.outputs {
        let vals = if out.values.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> =
                out.values.iter().map(|(k, v)| format!("{k}={v}")).collect();
            format!(" [{}]", pairs.join(", "))
        };
        println!(
            "    {}: {} ({}){vals} — {}",
            out.register, out.name, out.field_type, out.description
        );
    }
    println!();
    println!(
        "  Conservation: max_steps={}, memory_budget={} bytes",
        policy.conservation.max_steps, policy.conservation.memory_budget
    );
    println!("  Bytecode:     {} bytes", policy.bytecode_size);
    if !policy.bytecode_hash.is_empty() {
        let short = &policy.bytecode_hash[..policy.bytecode_hash.len().min(32)];
        println!("  Hash:         {short}...");
    }
    println!("  Conformance:  {}", policy.conformance);

    Ok(())
}

fn cmd_run(name: &str, raw_inputs: &[String], verbose: bool) -> anyhow::Result<()> {
    let store = PolicyStore::new();

    // Auto-install if not present
    if !store.is_installed(name) {
        println!("⚠ '{name}' not installed. Auto-installing...");
        let client = registry::RegistryClient::new();
        let policy = client.fetch_policy(name)?;
        store.save(&policy)?;
    }

    let policy = store.get(name)?;
    let bytecode = policy.decode_bytecode()?;

    // Parse inputs
    let mut inputs: HashMap<String, i32> = HashMap::new();
    for kv in raw_inputs {
        let (key, val) = kv
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid input '{kv}', expected key=value"))?;
        let parsed: i32 = val
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid integer value for '{key}': {val}"))?;
        inputs.insert(key.to_string(), parsed);
    }

    // Warn about missing inputs
    for inp in &policy.inputs {
        if !inputs.contains_key(&inp.name) {
            eprintln!("⚠ Missing input '{}' ({}), using 0", inp.name, inp.register);
        }
    }

    let max_steps = policy.conservation.max_steps;
    let mut flux_vm = FluxMiniVm::new(&bytecode, max_steps);
    flux_registry_cli_lib::vm::map_inputs(&mut flux_vm, &inputs, &policy.inputs);
    let result = flux_vm.run();

    if let Some(fault) = &result.fault {
        return Err(anyhow::anyhow!("Execution fault: {fault}"));
    }

    // Decode outputs
    println!("Outputs:");
    for out in &policy.outputs {
        let reg_name = &out.register;
        let raw_val = *result.registers.get(reg_name).unwrap_or(&0);
        if out.values.is_empty() {
            println!("  {} = {}", out.name, raw_val);
        } else {
            let label = out
                .values
                .get(&raw_val.to_string())
                .cloned()
                .unwrap_or_else(|| raw_val.to_string());
            println!("  {} = {}", out.name, label);
        }
    }

    if verbose {
        println!(
            "\nSteps: {}, Stack depth: {}",
            result.steps, result.stack_depth
        );
    }

    Ok(())
}

fn cmd_remove(name: &str) -> anyhow::Result<()> {
    let store = PolicyStore::new();
    if store.remove(name) {
        println!("🗑 Removed '{name}'");
    } else {
        eprintln!("⚠ '{name}' was not installed");
    }
    Ok(())
}

fn cmd_update_index() -> anyhow::Result<()> {
    println!("🔄 Updating registry index...");
    let client = registry::RegistryClient::new();
    let index = client.fetch_index()?;
    println!(
        "✅ Registry updated — {} policies available",
        index.total_policies
    );
    Ok(())
}

// ------------------------------------------------------------------
// Table printer
// ------------------------------------------------------------------

struct PolicyRow<'a> {
    name: &'a str,
    version: &'a str,
    size: usize,
    description: &'a str,
}

fn print_policy_table(rows: &[PolicyRow]) {
    let stdout = io::stdout();
    let mut w = stdout.lock();

    let name_w = rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(4)
        .max(4)
        + 2;
    let ver_w = 10usize;

    let _ = writeln!(
        w,
        "  {:<name_w$} {:<ver_w$} {:<8} DESCRIPTION",
        "NAME",
        "VERSION",
        "SIZE"
    );
    let _ = writeln!(
        w,
        "  {:<name_w$} {:<ver_w$} {:<8} ──────────",
        "──────",
        "──────────",
        "─────"
    );
    for r in rows {
        let desc: String = r.description.chars().take(40).collect();
        let _ = writeln!(
            w,
            "  {:<name_w$} {:<ver_w$} {:<8} {}",
            r.name, r.version, r.size, desc
        );
    }
}

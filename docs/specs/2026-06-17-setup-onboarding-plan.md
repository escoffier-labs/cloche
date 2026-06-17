# Cloche Setup Onboarding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `cloche setup`, one guided command that installs the screenshot hotkey, registers the MCP server with installed agents, and verifies all of it works, so a fresh Linux install is usable in one step.

**Architecture:** A new `setup` module group dispatched from `cli.rs`. Pure logic (desktop detection, config-edit construction, idempotency) is split from side effects (running `gsettings`, spawning processes) behind small function seams so it is unit-testable without a desktop session. Existing `cloche mcp`, `capture`, and `doctor` are reused unchanged; setup only wires and verifies them.

**Tech Stack:** Rust 2024, clap (derive), serde_json (already a dep; used for `.claude.json` and `openclaw.json`), `include_str!` for the embedded grab script, std `process::Command` for `gsettings`/`claude`/`cloche mcp`. No new dependencies (Codex TOML is edited by guarded text-append, not a TOML library).

**Spec:** `docs/specs/2026-06-17-setup-onboarding.md`

---

## File Structure

- Create `src/setup.rs` - `SetupArgs`, `SetupCommand`, the guided flow, plan/summary printing, confirmation prompt, the `SetupReport` JSON contract, and dispatch into submodules.
- Create `src/setup/hotkey.rs` - `Desktop` detection, grab-script install, GNOME `gsettings` binding (idempotent), per-desktop print instructions.
- Create `src/setup/agents.rs` - per-client detection + registration (Claude Code, Codex, OpenClaw, generic print), backups, idempotency.
- Create `src/setup/verify.rs` - the three checks (capture pipeline, hotkey, agent/MCP handshake), shared by `setup` and `setup verify`.
- Modify `src/lib.rs` - declare `mod setup;`, add `Command::Setup` dispatch arm.
- Modify `src/cli.rs` - add `Setup(SetupArgs)` to the `Command` enum (the args type lives in `setup.rs`).
- Modify `README.md` - lead the Hotkey and MCP sections with `cloche setup`.
- Modify `CHANGELOG.md` - `Unreleased` entry.

Rust modules: because `setup.rs` declares `mod hotkey;` etc., the submodules live at `src/setup/hotkey.rs` (Rust resolves `src/setup.rs` + `src/setup/` automatically).

---

## Task 1: Desktop detection (pure)

**Files:**
- Create: `src/setup.rs`
- Create: `src/setup/hotkey.rs`
- Modify: `src/lib.rs:1-12` (add `mod setup;`)

- [ ] **Step 1: Add the module and a failing detection test**

In `src/setup/hotkey.rs`:

```rust
//! Desktop detection and screenshot-hotkey installation.

/// Desktop environments we treat distinctly for hotkey binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desktop {
    Gnome,
    Kde,
    Sway,
    I3,
    Other,
}

/// Classify a desktop from `XDG_CURRENT_DESKTOP` / `DESKTOP_SESSION` values.
/// Matching is case-insensitive and substring-based because real values are
/// noisy (e.g. `ubuntu:GNOME`, `plasma`, `sway`).
pub fn classify_desktop(current_desktop: Option<&str>, desktop_session: Option<&str>) -> Desktop {
    let hay = format!(
        "{} {}",
        current_desktop.unwrap_or(""),
        desktop_session.unwrap_or("")
    )
    .to_lowercase();
    if hay.contains("gnome") {
        Desktop::Gnome
    } else if hay.contains("kde") || hay.contains("plasma") {
        Desktop::Kde
    } else if hay.contains("sway") {
        Desktop::Sway
    } else if hay.contains("i3") {
        Desktop::I3
    } else {
        Desktop::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_gnome_variants() {
        assert_eq!(classify_desktop(Some("ubuntu:GNOME"), None), Desktop::Gnome);
        assert_eq!(classify_desktop(None, Some("gnome")), Desktop::Gnome);
    }

    #[test]
    fn classifies_kde_plasma_sway_i3_other() {
        assert_eq!(classify_desktop(Some("KDE"), None), Desktop::Kde);
        assert_eq!(classify_desktop(Some("plasma"), None), Desktop::Kde);
        assert_eq!(classify_desktop(Some("sway"), None), Desktop::Sway);
        assert_eq!(classify_desktop(Some("i3"), None), Desktop::I3);
        assert_eq!(classify_desktop(Some("xfce"), None), Desktop::Other);
        assert_eq!(classify_desktop(None, None), Desktop::Other);
    }
}
```

In `src/setup.rs`:

```rust
//! `cloche setup`: install the hotkey, register the MCP server, and verify.

pub mod agents;
pub mod hotkey;
pub mod verify;
```

In `src/lib.rs`, add `mod setup;` alongside the other `mod` lines (keep alphabetical-ish ordering: after `mod polish;`).

- [ ] **Step 2: Run the test to verify it fails to compile/build then passes once added**

Run: `cargo test -p cloche setup::hotkey::tests -- --nocapture`
Expected: the two tests PASS. (They are self-contained; this step mainly confirms the module wiring compiles. `agents`/`verify` modules are empty files for now - create `src/setup/agents.rs` and `src/setup/verify.rs` as empty files so `pub mod` resolves.)

- [ ] **Step 3: Commit**

```bash
git add src/setup.rs src/setup/ src/lib.rs
git commit -m "feat(setup): add setup module skeleton and desktop detection"
```

---

## Task 2: CLI surface and dispatch (dry-run plan)

**Files:**
- Modify: `src/cli.rs:38-52` (Command enum)
- Modify: `src/setup.rs`
- Modify: `src/lib.rs` (dispatch arm)

- [ ] **Step 1: Add the args types and a plan-printing entry point**

In `src/setup.rs`, add:

```rust
use std::process::ExitCode;

use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[command(subcommand)]
    pub command: Option<SetupCommand>,
    /// Apply changes without the confirmation prompt.
    #[arg(long, global = true)]
    pub yes: bool,
    /// Print every change that would be made and exit without changing anything.
    #[arg(long, global = true)]
    pub print: bool,
    /// Output format. `text` is the human default; `json` emits the SetupReport.
    #[arg(long, global = true, value_enum, default_value = "text")]
    pub format: SetupFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SetupFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    /// Install and bind the screenshot hotkey only.
    Hotkey,
    /// Register the MCP server with agent clients only.
    Agent(AgentArgs),
    /// Re-run the confirmation checks only.
    Verify,
}

#[derive(Debug, Args)]
pub struct AgentArgs {
    /// Configure a specific client; auto-detects all installed clients when omitted.
    #[arg(long, value_enum)]
    pub client: Option<AgentClient>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AgentClient {
    ClaudeCode,
    Openclaw,
    Codex,
    Print,
}

pub fn run(args: SetupArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    // Implemented incrementally; for now route to the guided flow which, until
    // later tasks land, only prints the detected plan.
    crate::setup::flow::run(args)
}
```

Create `src/setup/flow.rs` (add `pub mod flow;` to `src/setup.rs`) with a minimal plan printer:

```rust
//! The guided `cloche setup` flow: detect -> plan -> confirm -> apply -> verify.

use std::process::ExitCode;

use crate::setup::SetupArgs;
use crate::setup::hotkey;

pub fn run(args: SetupArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let desktop = hotkey::detect_desktop();
    println!("Cloche setup");
    println!("  desktop: {desktop:?}");
    let _ = &args; // remaining behavior added in later tasks
    Ok(ExitCode::SUCCESS)
}
```

Add to `src/setup/hotkey.rs`:

```rust
use crate::util;

/// Detect the desktop from live environment variables.
pub fn detect_desktop() -> Desktop {
    classify_desktop(
        util::env_var("XDG_CURRENT_DESKTOP").as_deref(),
        util::env_var("DESKTOP_SESSION").as_deref(),
    )
}
```

In `src/cli.rs`, add to the `Command` enum:

```rust
    Setup(crate::setup::SetupArgs),
```

In `src/lib.rs` dispatch match add:

```rust
        Command::Setup(args) => setup::run(args),
```

- [ ] **Step 2: Verify it builds and the command exists**

Run: `cargo run -q -- setup --help`
Expected: clap shows `Usage: cloche setup [OPTIONS] [COMMAND]` with `hotkey`, `agent`, `verify` subcommands and `--yes`, `--print`, `--format`.

Run: `cargo run -q -- setup --print`
Expected: prints `Cloche setup` and `desktop: <Variant>`.

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs src/lib.rs src/setup.rs src/setup/flow.rs src/setup/hotkey.rs
git commit -m "feat(setup): wire setup CLI surface and dispatch"
```

---

## Task 3: Grab-script install (embedded, idempotent)

**Files:**
- Modify: `src/setup/hotkey.rs`

- [ ] **Step 1: Write the failing test for the install decision**

The pure decision is "where does the script go and do we need to write it". Add to `src/setup/hotkey.rs`:

```rust
use std::path::Path;
use std::path::PathBuf;

/// The grab script shipped in the repo, embedded so an installed binary with no
/// source checkout can still lay it down. `scripts/*.sh` is in the crate include.
const GRAB_SCRIPT: &str = include_str!("../../scripts/cloche-grab.sh");

/// Target install path for the grab script.
pub fn grab_script_path() -> PathBuf {
    let home = util::env_var("HOME").unwrap_or_else(|| "/root".to_string());
    PathBuf::from(home).join(".local/bin/cloche-grab")
}

/// True when the file at `path` already holds the current embedded script.
pub fn grab_script_current(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|existing| existing == GRAB_SCRIPT)
        .unwrap_or(false)
}
```

```rust
#[cfg(test)]
mod install_tests {
    use super::*;

    #[test]
    fn reports_missing_script_as_not_current() {
        let path = std::env::temp_dir().join(format!("cloche-grab-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(!grab_script_current(&path));
    }

    #[test]
    fn reports_matching_script_as_current() {
        let path = std::env::temp_dir().join(format!("cloche-grab-match-{}", std::process::id()));
        std::fs::write(&path, GRAB_SCRIPT).unwrap();
        assert!(grab_script_current(&path));
        let _ = std::fs::remove_file(&path);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p cloche setup::hotkey -- --nocapture`
Expected: PASS. (The `include_str!` path is relative to `src/setup/hotkey.rs`, so `../../scripts/...` reaches the repo root.)

- [ ] **Step 3: Add the install side effect**

```rust
use std::io::Write;

/// Install the grab script to `~/.local/bin/cloche-grab` with mode 0755.
/// Returns Ok(true) when written, Ok(false) when already current (idempotent).
pub fn install_grab_script() -> std::io::Result<bool> {
    let path = grab_script_path();
    if grab_script_current(&path) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&path)?;
    file.write_all(GRAB_SCRIPT.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(true)
}

/// True when `~/.local/bin` is on PATH (so the binding can find cloche-grab).
pub fn local_bin_on_path() -> bool {
    let target = grab_script_path();
    let dir = target.parent().map(Path::to_path_buf);
    match (dir, util::env_var("PATH")) {
        (Some(dir), Some(path)) => std::env::split_paths(&path).any(|p| p == dir),
        _ => false,
    }
}
```

- [ ] **Step 4: Run tests again**

Run: `cargo test -p cloche setup::hotkey`
Expected: PASS (existing tests still pass; new functions compile).

- [ ] **Step 5: Commit**

```bash
git add src/setup/hotkey.rs
git commit -m "feat(setup): embed and install the cloche-grab hotkey script"
```

---

## Task 4: GNOME binding (idempotent) and non-GNOME print

**Files:**
- Modify: `src/setup/hotkey.rs`

- [ ] **Step 1: Write the failing test for binding-slot selection**

GNOME stores custom keybindings as a gsettings string array of dconf paths. We
must pick an unused slot but reuse an existing cloche slot. The pure part is
choosing the slot path given the current list and which existing slots already
point at `cloche-grab`. Add:

```rust
const KEYBIND_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KEYBIND_PREFIX: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/";

/// Decide which dconf slot path the cloche binding should occupy.
/// `existing` is the current custom-keybindings list; `cloche_slot` is the slot
/// (if any) whose command already resolves to cloche-grab. Reuse the cloche slot
/// when present; otherwise allocate the first unused `customN` index.
pub fn choose_binding_slot(existing: &[String], cloche_slot: Option<&str>) -> String {
    if let Some(slot) = cloche_slot {
        return slot.to_string();
    }
    let mut n = 0;
    loop {
        let candidate = format!("{KEYBIND_PREFIX}custom{n}/");
        if !existing.iter().any(|e| e == &candidate) {
            return candidate;
        }
        n += 1;
    }
}
```

```rust
#[cfg(test)]
mod gnome_tests {
    use super::*;

    #[test]
    fn allocates_first_free_slot() {
        let existing = vec![format!("{KEYBIND_PREFIX}custom0/")];
        assert_eq!(
            choose_binding_slot(&existing, None),
            format!("{KEYBIND_PREFIX}custom1/")
        );
    }

    #[test]
    fn reuses_existing_cloche_slot() {
        let slot = format!("{KEYBIND_PREFIX}custom3/");
        let existing = vec![slot.clone()];
        assert_eq!(choose_binding_slot(&existing, Some(&slot)), slot);
    }

    #[test]
    fn allocates_custom0_when_empty() {
        assert_eq!(
            choose_binding_slot(&[], None),
            format!("{KEYBIND_PREFIX}custom0/")
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p cloche setup::hotkey::gnome_tests`
Expected: PASS.

- [ ] **Step 3: Add the gsettings side effects**

These shell out to `gsettings`; they are only invoked on GNOME at runtime, not in tests. Use the existing `util::run_output` / `util::run_status` helpers.

```rust
/// Parse a gsettings array value like `['a', 'b']` or `@as []` into entries.
pub fn parse_gsettings_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed == "@as []" || trimmed == "[]" {
        return Vec::new();
    }
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn read_custom_keybindings() -> Vec<String> {
    util::run_output("gsettings", &["get", KEYBIND_SCHEMA, "custom-keybindings"])
        .map(|raw| parse_gsettings_array(&raw))
        .unwrap_or_default()
}

fn slot_command(slot: &str) -> Option<String> {
    let key = format!("{KEYBIND_SCHEMA}.custom-keybinding:{slot}");
    util::run_output("gsettings", &["get", &key, "command"])
        .ok()
        .map(|raw| raw.trim().trim_matches('\'').to_string())
}

/// Outcome of attempting to bind the hotkey.
#[derive(Debug, PartialEq, Eq)]
pub enum HotkeyOutcome {
    /// Bound automatically (GNOME). True when a change was made.
    Bound { changed: bool },
    /// Printed manual instructions for this desktop.
    Manual,
}
```

```rust
/// Bind `cloche-grab` to Print on GNOME, idempotently. `apply` false = dry-run.
pub fn bind_gnome(apply: bool) -> Result<HotkeyOutcome, crate::util::AppError> {
    let existing = read_custom_keybindings();
    let cloche_slot = existing
        .iter()
        .find(|slot| slot_command(slot).as_deref() == Some("cloche-grab"))
        .cloned();
    let already = cloche_slot.is_some();
    let slot = choose_binding_slot(&existing, cloche_slot.as_deref());

    if !apply {
        return Ok(HotkeyOutcome::Bound { changed: !already });
    }

    if !existing.contains(&slot) {
        let mut updated = existing.clone();
        updated.push(slot.clone());
        let list = format!(
            "[{}]",
            updated
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        util::run_status("gsettings", &["set", KEYBIND_SCHEMA, "custom-keybindings", &list])?;
    }
    let key = format!("{KEYBIND_SCHEMA}.custom-keybinding:{slot}");
    util::run_status("gsettings", &["set", &key, "name", "Cloche Grab"])?;
    util::run_status("gsettings", &["set", &key, "command", "cloche-grab"])?;
    util::run_status("gsettings", &["set", &key, "binding", "Print"])?;
    Ok(HotkeyOutcome::Bound { changed: !already })
}

/// Print copy-pasteable binding steps for non-GNOME desktops.
pub fn print_manual_binding(desktop: Desktop, grab: &Path) {
    let cmd = grab.display();
    match desktop {
        Desktop::Kde => {
            println!("KDE: System Settings -> Shortcuts -> Custom Shortcuts -> Edit -> New ->");
            println!("  Global Shortcut -> Command/URL, command `{cmd}`, then assign Print.");
        }
        Desktop::Sway | Desktop::I3 => {
            println!("Add to your WM config and reload:");
            println!("  bindsym Print exec {cmd}");
        }
        _ => {
            println!("Bind a key to `{cmd}` in your desktop's keyboard settings.");
        }
    }
}
```

Add a parser test:

```rust
#[test]
fn parses_gsettings_arrays() {
    assert_eq!(parse_gsettings_array("@as []"), Vec::<String>::new());
    assert_eq!(
        parse_gsettings_array("['/a/custom0/', '/a/custom1/']"),
        vec!["/a/custom0/".to_string(), "/a/custom1/".to_string()]
    );
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p cloche setup::hotkey`
Expected: PASS.

- [ ] **Step 5: Add the top-level hotkey orchestration**

```rust
/// Install the grab script and bind the hotkey for the detected desktop.
/// Returns the outcome plus any warnings (e.g. PATH not set).
pub fn setup_hotkey(apply: bool) -> (HotkeyOutcome, Vec<String>) {
    let mut warnings = Vec::new();
    if apply {
        match install_grab_script() {
            Ok(_) => {}
            Err(err) => warnings.push(format!("could not install cloche-grab: {err}")),
        }
    }
    if !local_bin_on_path() {
        warnings.push(format!(
            "{} is not on PATH; add it so the hotkey can find cloche-grab",
            grab_script_path().parent().unwrap().display()
        ));
    }
    let desktop = detect_desktop();
    let outcome = if desktop == Desktop::Gnome {
        bind_gnome(apply).unwrap_or_else(|err| {
            warnings.push(format!("gsettings binding failed: {err}"));
            HotkeyOutcome::Manual
        })
    } else {
        if apply {
            print_manual_binding(desktop, &grab_script_path());
        }
        HotkeyOutcome::Manual
    };
    (outcome, warnings)
}
```

- [ ] **Step 6: Run tests and commit**

Run: `cargo test -p cloche setup::hotkey`
Expected: PASS.

```bash
git add src/setup/hotkey.rs
git commit -m "feat(setup): bind Print to cloche-grab on GNOME, print steps elsewhere"
```

---

## Task 5: Agent registration for JSON clients (Claude Code, OpenClaw)

**Files:**
- Modify: `src/setup/agents.rs`

- [ ] **Step 1: Write the failing test for idempotent JSON injection**

The pure operation: given an existing JSON document (or none) and a dotted key
path to the server map, return the updated document with the `cloche` entry, and
report whether a change was made. Add to `src/setup/agents.rs`:

```rust
//! Register the cloche MCP server with agent clients.

use serde_json::Value;
use serde_json::json;

/// The canonical MCP server entry every client gets.
pub fn cloche_server_entry() -> Value {
    json!({ "command": "cloche", "args": ["mcp"] })
}

/// Insert/update `cloche` inside the server map at `doc[map_keys...]`, creating
/// intermediate objects as needed. Returns (updated_doc, changed).
pub fn upsert_server(mut doc: Value, map_keys: &[&str]) -> (Value, bool) {
    if !doc.is_object() {
        doc = json!({});
    }
    let mut cursor = &mut doc;
    for key in map_keys {
        if !cursor.get(*key).map(Value::is_object).unwrap_or(false) {
            cursor[*key] = json!({});
        }
        cursor = cursor.get_mut(*key).unwrap();
    }
    let entry = cloche_server_entry();
    let changed = cursor.get("cloche") != Some(&entry);
    cursor["cloche"] = entry;
    (doc, changed)
}
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_cloche_to_empty_doc_for_claude() {
        let (doc, changed) = upsert_server(json!({}), &["mcpServers"]);
        assert!(changed);
        assert_eq!(doc["mcpServers"]["cloche"]["command"], "cloche");
        assert_eq!(doc["mcpServers"]["cloche"]["args"][0], "mcp");
    }

    #[test]
    fn preserves_existing_servers() {
        let start = json!({ "mcpServers": { "other": { "command": "x" } } });
        let (doc, changed) = upsert_server(start, &["mcpServers"]);
        assert!(changed);
        assert_eq!(doc["mcpServers"]["other"]["command"], "x");
        assert_eq!(doc["mcpServers"]["cloche"]["command"], "cloche");
    }

    #[test]
    fn second_run_is_idempotent() {
        let (doc, _) = upsert_server(json!({}), &["mcp", "servers"]);
        let (_, changed) = upsert_server(doc, &["mcp", "servers"]);
        assert!(!changed);
    }

    #[test]
    fn nested_openclaw_path() {
        let (doc, _) = upsert_server(json!({}), &["mcp", "servers"]);
        assert_eq!(doc["mcp"]["servers"]["cloche"]["command"], "cloche");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p cloche setup::agents::tests`
Expected: PASS.

- [ ] **Step 3: Add file read/edit/backup with a temp-file test**

```rust
use std::path::Path;
use std::path::PathBuf;

/// Read a JSON file into a Value, returning `json!({})` when absent and an error
/// only when the file exists but is unparseable (we never clobber bad config).
pub fn read_json_or_empty(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is not valid JSON: {e}", path.display())),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("could not read {}: {e}", path.display())),
    }
}

/// Back up `path` to `<path>.cloche.bak` when it exists.
pub fn backup(path: &Path) -> std::io::Result<Option<PathBuf>> {
    if path.exists() {
        let bak = path.with_extension(format!(
            "{}cloche.bak",
            path.extension().map(|e| format!("{}.", e.to_string_lossy())).unwrap_or_default()
        ));
        std::fs::copy(path, &bak)?;
        Ok(Some(bak))
    } else {
        Ok(None)
    }
}

/// Register cloche in a JSON-config client at `path`, under `map_keys`.
/// Returns (changed, backup_path). `apply` false = dry-run (no writes).
pub fn register_json_client(
    path: &Path,
    map_keys: &[&str],
    apply: bool,
) -> Result<(bool, Option<PathBuf>), String> {
    let doc = read_json_or_empty(path)?;
    let (updated, changed) = upsert_server(doc, map_keys);
    if !apply || !changed {
        return Ok((changed, None));
    }
    let bak = backup(path).map_err(|e| format!("backup failed: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("write failed: {e}"))?;
    Ok((true, bak))
}
```

```rust
#[test]
fn register_json_client_writes_and_backs_up() {
    let dir = std::env::temp_dir().join(format!("cloche-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("openclaw.json");
    std::fs::write(&path, r#"{"mcp":{"servers":{"x":{"command":"x"}}}}"#).unwrap();

    let (changed, bak) = register_json_client(&path, &["mcp", "servers"], true).unwrap();
    assert!(changed);
    assert!(bak.unwrap().exists());
    let written: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written["mcp"]["servers"]["cloche"]["command"], "cloche");
    assert_eq!(written["mcp"]["servers"]["x"]["command"], "x");

    let (changed2, _) = register_json_client(&path, &["mcp", "servers"], true).unwrap();
    assert!(!changed2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_json_is_reported_not_clobbered() {
    let dir = std::env::temp_dir().join(format!("cloche-agent-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(".claude.json");
    std::fs::write(&path, "{not json").unwrap();
    assert!(register_json_client(&path, &["mcpServers"], true).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 4: Run tests and commit**

Run: `cargo test -p cloche setup::agents`
Expected: PASS.

```bash
git add src/setup/agents.rs
git commit -m "feat(setup): register cloche MCP in JSON agent configs idempotently"
```

---

## Task 6: Codex TOML, Claude CLI path, client detection, generic print

**Files:**
- Modify: `src/setup/agents.rs`

- [ ] **Step 1: Write the failing test for the Codex TOML block decision**

Codex config is TOML. The static block never changes, so "already present" =
"configured"; we only append when absent. The pure decision is "does the text
already contain the cloche block". Add:

```rust
/// The TOML block appended to ~/.codex/config.toml.
pub const CODEX_BLOCK: &str = "\n[mcp_servers.cloche]\ncommand = \"cloche\"\nargs = [\"mcp\"]\n";

/// True when the Codex config text already declares the cloche server.
pub fn codex_block_present(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "[mcp_servers.cloche]")
}
```

```rust
#[test]
fn detects_existing_codex_block() {
    assert!(codex_block_present("[mcp_servers.cloche]\ncommand = \"cloche\""));
    assert!(codex_block_present("[mcp_servers.other]\n\n[mcp_servers.cloche]\n"));
    assert!(!codex_block_present("[mcp_servers.other]\ncommand = \"x\""));
    assert!(!codex_block_present(""));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p cloche setup::agents::tests::detects_existing_codex_block`
Expected: PASS.

- [ ] **Step 3: Add Codex append, Claude CLI, detection, and the client driver**

```rust
use crate::util;

/// Register cloche in Codex's config.toml by appending the block when absent.
pub fn register_codex(path: &Path, apply: bool) -> Result<(bool, Option<PathBuf>), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    if codex_block_present(&text) {
        return Ok((false, None));
    }
    if !apply {
        return Ok((true, None));
    }
    let bak = backup(path).map_err(|e| format!("backup failed: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let updated = format!("{}{}", text.trim_end(), CODEX_BLOCK);
    std::fs::write(path, updated).map_err(|e| format!("write failed: {e}"))?;
    Ok((true, bak))
}

fn home() -> PathBuf {
    PathBuf::from(util::env_var("HOME").unwrap_or_else(|| "/root".to_string()))
}

/// Which clients look installed on this machine.
#[derive(Debug, PartialEq, Eq)]
pub struct DetectedClients {
    pub claude_code: bool,
    pub codex: bool,
    pub openclaw: bool,
}

pub fn detect_clients() -> DetectedClients {
    let h = home();
    DetectedClients {
        claude_code: util::has_command("claude") || h.join(".claude.json").exists(),
        codex: h.join(".codex").exists(),
        openclaw: h.join(".openclaw/openclaw.json").exists(),
    }
}

/// Result of configuring one client.
#[derive(Debug)]
pub struct ClientResult {
    pub client: &'static str,
    pub status: ClientStatus,
    pub backup: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClientStatus {
    Applied,
    AlreadyConfigured,
    Printed,
    Skipped,
    Error,
}

/// Register Claude Code: prefer the official CLI, else edit ~/.claude.json.
pub fn register_claude(apply: bool) -> ClientResult {
    if util::has_command("claude") {
        if !apply {
            return ClientResult {
                client: "claude-code",
                status: ClientStatus::Applied,
                backup: None,
                message: "would run: claude mcp add cloche -s user -- cloche mcp".to_string(),
            };
        }
        match util::run_status(
            "claude",
            &["mcp", "add", "cloche", "-s", "user", "--", "cloche", "mcp"],
        ) {
            Ok(()) => ClientResult { client: "claude-code", status: ClientStatus::Applied, backup: None, message: "registered via claude CLI".to_string() },
            Err(err) => ClientResult { client: "claude-code", status: ClientStatus::Error, backup: None, message: format!("claude CLI failed: {err}") },
        }
    } else {
        let path = home().join(".claude.json");
        match register_json_client(&path, &["mcpServers"], apply) {
            Ok((true, bak)) => ClientResult { client: "claude-code", status: ClientStatus::Applied, backup: bak, message: format!("edited {}", path.display()) },
            Ok((false, _)) => ClientResult { client: "claude-code", status: ClientStatus::AlreadyConfigured, backup: None, message: "already configured".to_string() },
            Err(msg) => ClientResult { client: "claude-code", status: ClientStatus::Error, backup: None, message: msg },
        }
    }
}

/// Print the generic snippet for clients we do not auto-edit.
pub fn print_generic() {
    println!("Add this to your MCP client config:");
    println!("  {{ \"command\": \"cloche\", \"args\": [\"mcp\"] }}");
    println!("(stdio MCP server; the command is `cloche mcp`)");
}
```

Add a small helper that maps a JSON result to a `ClientResult` for Codex/OpenClaw and is reused by the driver:

```rust
fn json_result(client: &'static str, path: &Path, r: Result<(bool, Option<PathBuf>), String>) -> ClientResult {
    match r {
        Ok((true, bak)) => ClientResult { client, status: ClientStatus::Applied, backup: bak, message: format!("edited {}", path.display()) },
        Ok((false, _)) => ClientResult { client, status: ClientStatus::AlreadyConfigured, backup: None, message: "already configured".to_string() },
        Err(msg) => ClientResult { client, status: ClientStatus::Error, backup: None, message: msg },
    }
}

/// Configure all detected clients (or the one requested). `apply` false = dry-run.
pub fn setup_agents(only: Option<crate::setup::AgentClient>, apply: bool) -> Vec<ClientResult> {
    use crate::setup::AgentClient;
    let h = home();
    let detected = detect_clients();
    let mut out = Vec::new();

    let want = |c: AgentClient| only.is_none() || only == Some(c);

    if only == Some(AgentClient::Print) {
        print_generic();
        out.push(ClientResult { client: "print", status: ClientStatus::Printed, backup: None, message: "printed generic snippet".to_string() });
        return out;
    }
    if want(AgentClient::ClaudeCode) && (detected.claude_code || only.is_some()) {
        out.push(register_claude(apply));
    }
    if want(AgentClient::Codex) && (detected.codex || only.is_some()) {
        let path = h.join(".codex/config.toml");
        out.push(json_result("codex", &path, register_codex(&path, apply)));
    }
    if want(AgentClient::Openclaw) && (detected.openclaw || only.is_some()) {
        let path = h.join(".openclaw/openclaw.json");
        out.push(json_result("openclaw", &path, register_json_client(&path, &["mcp", "servers"], apply)));
    }
    if out.is_empty() {
        print_generic();
        out.push(ClientResult { client: "print", status: ClientStatus::Printed, backup: None, message: "no known client detected; printed generic snippet".to_string() });
    }
    out
}
```

Add a Codex round-trip test:

```rust
#[test]
fn register_codex_appends_once() {
    let dir = std::env::temp_dir().join(format!("cloche-codex-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "[mcp_servers.other]\ncommand = \"x\"\n").unwrap();
    let (changed, bak) = register_codex(&path, true).unwrap();
    assert!(changed && bak.unwrap().exists());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(codex_block_present(&text));
    let (again, _) = register_codex(&path, true).unwrap();
    assert!(!again);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 4: Run tests and commit**

Run: `cargo test -p cloche setup::agents`
Expected: PASS.

```bash
git add src/setup/agents.rs
git commit -m "feat(setup): add Codex, Claude CLI, client detection, generic print"
```

---

## Task 7: Verification checks

**Files:**
- Modify: `src/setup/verify.rs`

- [ ] **Step 1: Write the failing test for the MCP handshake parser**

The verify module spawns `cloche mcp` and reads its `tools/list` reply. The pure
part is "given the JSON-RPC reply text, are capture and polish listed". Add to
`src/setup/verify.rs`:

```rust
//! Post-setup verification: prove capture, hotkey, and MCP actually work.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone)]
pub struct Check {
    pub name: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

/// Parse a `tools/list` JSON-RPC reply and confirm capture+polish are exposed.
pub fn tools_list_has_core(reply: &str) -> bool {
    let value: Value = match serde_json::from_str(reply) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let tools = match value["result"]["tools"].as_array() {
        Some(t) => t,
        None => return false,
    };
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.contains(&"capture") && names.contains(&"polish")
}
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_core_tools_in_reply() {
        let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[
            {"name":"capture"},{"name":"polish"},{"name":"doctor"}]}}"#;
        assert!(tools_list_has_core(reply));
    }

    #[test]
    fn rejects_reply_missing_polish() {
        let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"capture"}]}}"#;
        assert!(!tools_list_has_core(reply));
    }

    #[test]
    fn rejects_garbage() {
        assert!(!tools_list_has_core("not json"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p cloche setup::verify::tests`
Expected: PASS.

- [ ] **Step 3: Add the three live checks**

```rust
use std::io::Write;
use std::path::Path;

use crate::setup::hotkey;
use crate::util;

/// Check 1: run a real non-interactive screen capture and confirm artifacts.
pub fn check_capture_pipeline() -> Check {
    let has_gui = util::env_var("DISPLAY").is_some() || util::env_var("WAYLAND_DISPLAY").is_some();
    if !has_gui {
        return Check { name: "capture-pipeline", status: CheckStatus::Skip,
            detail: "no DISPLAY/WAYLAND_DISPLAY; cannot test capture here".to_string() };
    }
    let dir = std::env::temp_dir().join(format!("cloche-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let exe = std::env::current_exe().unwrap_or_else(|_| "cloche".into());
    let status = std::process::Command::new(&exe)
        .args(["capture", "--target", "screen", "--presentation", "both",
               "--out-dir", &dir.to_string_lossy(), "--format", "json"])
        .output();
    let shot = dir.join("shot.png");
    let card = dir.join("shot-card.png");
    let ok = matches!(status, Ok(ref o) if o.status.success())
        && nonempty(&shot) && nonempty(&card);
    let detail = if ok { "captured shot.png and shot-card.png".to_string() }
        else { "capture did not produce both shot.png and shot-card.png".to_string() };
    let _ = std::fs::remove_dir_all(&dir);
    Check { name: "capture-pipeline", status: if ok { CheckStatus::Pass } else { CheckStatus::Fail }, detail }
}

fn nonempty(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

/// Check 2: cloche-grab on PATH, and on GNOME a binding points at it.
pub fn check_hotkey() -> Check {
    let grab = hotkey::grab_script_path();
    if !grab.exists() {
        return Check { name: "hotkey", status: CheckStatus::Fail,
            detail: format!("{} not installed; run `cloche setup hotkey`", grab.display()) };
    }
    if hotkey::detect_desktop() != hotkey::Desktop::Gnome {
        return Check { name: "hotkey", status: CheckStatus::Skip,
            detail: "binding is manual on this desktop; grab script is installed".to_string() };
    }
    match hotkey::bind_gnome(false) {
        Ok(hotkey::HotkeyOutcome::Bound { changed: false }) => Check {
            name: "hotkey", status: CheckStatus::Pass, detail: "Print is bound to cloche-grab".to_string() },
        _ => Check { name: "hotkey", status: CheckStatus::Fail,
            detail: "no Print binding for cloche-grab; run `cloche setup hotkey`".to_string() },
    }
}

/// Check 3: spawn `cloche mcp`, handshake, confirm core tools.
pub fn check_agent_mcp() -> Check {
    let exe = std::env::current_exe().unwrap_or_else(|_| "cloche".into());
    let mut child = match std::process::Command::new(&exe)
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(err) => return Check { name: "agent-mcp", status: CheckStatus::Fail,
            detail: format!("could not start `cloche mcp`: {err}") },
    };
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = writeln!(stdin, "{init}");
        let _ = writeln!(stdin, "{list}");
    }
    let output = child.wait_with_output();
    let ok = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).lines().any(tools_list_has_core),
        Err(_) => false,
    };
    Check {
        name: "agent-mcp",
        status: if ok { CheckStatus::Pass } else { CheckStatus::Fail },
        detail: if ok { "`cloche mcp` lists capture and polish".to_string() }
            else { "`cloche mcp` did not return the core tools".to_string() },
    }
}

/// Run all three checks.
pub fn run_all() -> Vec<Check> {
    vec![check_capture_pipeline(), check_hotkey(), check_agent_mcp()]
}
```

Note: `cloche mcp` reads stdin line-by-line and exits at EOF, so writing the two
requests then dropping stdin (via `wait_with_output`, which closes the pipe)
lets it process both and terminate. This is a real end-to-end handshake.

- [ ] **Step 4: Run tests**

Run: `cargo test -p cloche setup::verify`
Expected: PASS (the live checks are exercised in Task 8's integration run, not unit tests).

- [ ] **Step 5: Commit**

```bash
git add src/setup/verify.rs
git commit -m "feat(setup): add capture, hotkey, and MCP verification checks"
```

---

## Task 8: Guided flow, JSON contract, summary

**Files:**
- Modify: `src/setup/flow.rs`
- Modify: `src/setup.rs` (re-export the report type)

- [ ] **Step 1: Define the report type and the flow**

Replace `src/setup/flow.rs` with the full flow:

```rust
//! The guided `cloche setup` flow: detect -> plan -> confirm -> apply -> verify.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use serde_json::json;

use crate::setup::AgentClient;
use crate::setup::SetupArgs;
use crate::setup::SetupCommand;
use crate::setup::SetupFormat;
use crate::setup::agents;
use crate::setup::agents::ClientStatus;
use crate::setup::hotkey;
use crate::setup::verify;
use crate::setup::verify::CheckStatus;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupReport {
    ok: bool,
    mode: String,
    applied: Vec<String>,
    skipped: Vec<String>,
    printed: Vec<String>,
    backups: Vec<String>,
    checks: Vec<serde_json::Value>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

pub fn run(args: SetupArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let apply = !args.print && confirm_if_needed(&args)?;
    let mut report = SetupReport {
        ok: true, mode: mode_label(&args.command), applied: vec![], skipped: vec![],
        printed: vec![], backups: vec![], checks: vec![], warnings: vec![], errors: vec![],
    };

    let do_hotkey = matches!(args.command, None | Some(SetupCommand::Hotkey));
    let do_agents = matches!(args.command, None | Some(SetupCommand::Agent(_)));
    let do_verify = matches!(args.command, None | Some(SetupCommand::Verify));

    if do_hotkey {
        let (outcome, warns) = hotkey::setup_hotkey(apply);
        report.warnings.extend(warns);
        match outcome {
            hotkey::HotkeyOutcome::Bound { changed: true } => report.applied.push("hotkey:gnome-binding".into()),
            hotkey::HotkeyOutcome::Bound { changed: false } => report.skipped.push("hotkey:already-bound".into()),
            hotkey::HotkeyOutcome::Manual => report.printed.push("hotkey:manual-instructions".into()),
        }
    }

    if do_agents {
        let only = match &args.command { Some(SetupCommand::Agent(a)) => a.client, _ => None };
        for r in agents::setup_agents(only, apply) {
            match r.status {
                ClientStatus::Applied => report.applied.push(format!("agent:{}", r.client)),
                ClientStatus::AlreadyConfigured => report.skipped.push(format!("agent:{}", r.client)),
                ClientStatus::Printed => report.printed.push(format!("agent:{}", r.client)),
                ClientStatus::Skipped => report.skipped.push(format!("agent:{}", r.client)),
                ClientStatus::Error => { report.errors.push(format!("agent:{}: {}", r.client, r.message)); }
            }
            if let Some(b) = r.backup { report.backups.push(b.display().to_string()); }
        }
    }

    if do_verify && !args.print {
        for c in verify::run_all() {
            if c.status == CheckStatus::Fail { report.ok = false; }
            report.checks.push(json!({
                "name": c.name,
                "status": match c.status { CheckStatus::Pass => "pass", CheckStatus::Fail => "fail", CheckStatus::Skip => "skip" },
                "detail": c.detail,
            }));
        }
    }

    if !report.errors.is_empty() { report.ok = false; }
    emit(&report, args.format)?;
    Ok(if report.ok { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn mode_label(cmd: &Option<SetupCommand>) -> String {
    match cmd {
        None => "setup", Some(SetupCommand::Hotkey) => "hotkey",
        Some(SetupCommand::Agent(_)) => "agent", Some(SetupCommand::Verify) => "verify",
    }.to_string()
}

fn confirm_if_needed(args: &SetupArgs) -> Result<bool, std::io::Error> {
    if args.yes { return Ok(true); }
    let desktop = hotkey::detect_desktop();
    println!("Cloche setup will, on this {desktop:?} session:");
    println!("  - install ~/.local/bin/cloche-grab and bind it to Print (GNOME) or print steps");
    println!("  - register the cloche MCP server with detected agents (backs up edited files)");
    println!("  - verify capture, hotkey, and MCP");
    print!("Proceed? [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn emit(report: &SetupReport, format: SetupFormat) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        SetupFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        SetupFormat::Text => {
            for a in &report.applied { println!("applied: {a}"); }
            for s in &report.skipped { println!("ok (already done): {s}"); }
            for p in &report.printed { println!("manual step printed: {p}"); }
            for c in &report.checks {
                println!("check {}: {} - {}", c["name"].as_str().unwrap_or(""),
                    c["status"].as_str().unwrap_or(""), c["detail"].as_str().unwrap_or(""));
            }
            for w in &report.warnings { println!("warning: {w}"); }
            for e in &report.errors { println!("error: {e}"); }
            println!("{}", if report.ok { "Setup OK." } else { "Setup finished with problems (see above)." });
        }
    }
    Ok(())
}
```

Remove the now-unused `AgentClient` import warning by referencing it (it is used
in the `only` match). Ensure `src/setup.rs` keeps `pub mod flow;`.

- [ ] **Step 2: Build and run the real flow in print mode**

Run: `cargo run -q -- setup --print --format json`
Expected: valid JSON with `mode: "setup"`, an `applied`/`printed` plan, empty `backups`, empty `checks` (print skips verify), `ok: true`.

- [ ] **Step 3: Run the real verify against the built binary**

Run: `cargo run -q -- setup verify --format json`
Expected: JSON with three `checks`. On this headless-agent context, `capture-pipeline` may be `skip` (no inherited DISPLAY) and `agent-mcp` should be `pass` (the MCP self-test does not need a display). Confirm `agent-mcp` is `pass`.

- [ ] **Step 4: Full test suite**

Run: `cargo test`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/setup.rs src/setup/flow.rs
git commit -m "feat(setup): guided flow, JSON SetupReport, and text summary"
```

---

## Task 9: Docs and changelog

**Files:**
- Modify: `README.md` (Hotkey Workflow section ~line 180, MCP Server section ~line 240)
- Modify: `CHANGELOG.md:7` (Unreleased)

- [ ] **Step 1: README - lead both sections with `cloche setup`**

At the top of the "Hotkey Workflow" section, before the manual steps, add:

```markdown
The fastest path is one command:

```bash
cloche setup
```

It installs `cloche-grab`, binds it to Print on GNOME (and prints the exact
steps on KDE/sway/i3), registers the MCP server with any agent it detects, then
verifies capture, the hotkey, and the MCP server actually work. Run
`cloche setup --print` to see what it would change first, or
`cloche setup verify` any time to re-check.

The manual steps below are what `cloche setup` automates, and the fallback for
unsupported desktops.
```

At the top of the "MCP Server" section, add:

```markdown
`cloche setup agent` registers this server with Claude Code, OpenClaw, and Codex
CLI automatically (backing up any config it edits). The manual config below is
for other clients or if you prefer to wire it yourself.
```

- [ ] **Step 2: CHANGELOG - Unreleased entry**

Under `## [Unreleased]` add:

```markdown
### Added
- `cloche setup`: one guided command that installs the `cloche-grab` hotkey
  script and binds it to Print (GNOME auto, other desktops print exact steps),
  registers the `cloche mcp` server with detected agents (Claude Code via the
  `claude` CLI or `~/.claude.json`, OpenClaw, Codex CLI; generic snippet
  otherwise; every edited file is backed up and the edit is idempotent), then
  verifies the capture pipeline, the hotkey binding, and a live `cloche mcp`
  handshake. `--print` dry-runs, `--yes` skips the prompt, `setup verify`
  re-checks, and `--format json` emits a stable report.
```

- [ ] **Step 3: Verify docs build / links**

Run: `cargo run -q -- setup --help`
Expected: matches the documented behavior (subcommands and flags present).

- [ ] **Step 4: Commit**

```bash
git add README.md CHANGELOG.md
git commit -m "docs: document cloche setup onboarding and verification"
```

---

## Final verification

- [ ] Run `cargo test` - all PASS.
- [ ] Run `cargo clippy --all-targets -- -D warnings` - clean (the repo CI runs clippy; fix any lint).
- [ ] Run `cargo fmt --check` - clean.
- [ ] Run `cargo run -q -- setup --print` - sane plan for the current desktop.
- [ ] Run `cargo run -q -- setup verify --format json` - `agent-mcp` is `pass`.
- [ ] Confirm `scripts/verify` (the repo's verify wrapper) still passes if it exercises the binary.

---

## Self-Review Notes

- **Spec coverage:** command surface (Task 2), hotkey GNOME-auto/other-print (Task 3-4), agent registration for all four targets + backups + idempotency (Task 5-6), the three verification checks (Task 7), guided flow + `--yes`/`--print`/JSON contract + safety (Task 8), docs/rollout (Task 9). Windows: the hotkey path is GNOME-gated and falls through to manual/print on non-GNOME; Windows builds compile because the only `#[cfg(unix)]` block is the chmod. A follow-up may add a Windows-specific message, but no Windows Print binding ships (matches the spec non-goal).
- **Type consistency:** `HotkeyOutcome`, `ClientResult`/`ClientStatus`, `Check`/`CheckStatus`, `SetupReport`, `AgentClient`, `SetupCommand`, `SetupFormat` are defined once and used consistently across flow/hotkey/agents/verify.
- **No placeholders:** every code step shows the actual code; every run step shows the expected result.

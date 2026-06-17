//! Register the cloche MCP server with agent clients.

use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

use crate::util;

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
        let suffix = path
            .extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default();
        let bak = path.with_extension(format!("{suffix}cloche.bak"));
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

/// The TOML block appended to ~/.codex/config.toml.
pub const CODEX_BLOCK: &str = "\n[mcp_servers.cloche]\ncommand = \"cloche\"\nargs = [\"mcp\"]\n";

/// True when the Codex config text already declares the cloche server.
pub fn codex_block_present(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "[mcp_servers.cloche]")
}

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
            Ok(()) => ClientResult {
                client: "claude-code",
                status: ClientStatus::Applied,
                backup: None,
                message: "registered via claude CLI".to_string(),
            },
            Err(err) => ClientResult {
                client: "claude-code",
                status: ClientStatus::Error,
                backup: None,
                message: format!("claude CLI failed: {err}"),
            },
        }
    } else {
        let path = home().join(".claude.json");
        json_result(
            "claude-code",
            &path,
            register_json_client(&path, &["mcpServers"], apply),
        )
    }
}

/// Print the generic snippet for clients we do not auto-edit.
pub fn print_generic() {
    println!("Add this to your MCP client config:");
    println!("  {{ \"command\": \"cloche\", \"args\": [\"mcp\"] }}");
    println!("(stdio MCP server; the command is `cloche mcp`)");
}

fn json_result(
    client: &'static str,
    path: &Path,
    r: Result<(bool, Option<PathBuf>), String>,
) -> ClientResult {
    match r {
        Ok((true, bak)) => ClientResult {
            client,
            status: ClientStatus::Applied,
            backup: bak,
            message: format!("edited {}", path.display()),
        },
        Ok((false, _)) => ClientResult {
            client,
            status: ClientStatus::AlreadyConfigured,
            backup: None,
            message: "already configured".to_string(),
        },
        Err(msg) => ClientResult {
            client,
            status: ClientStatus::Error,
            backup: None,
            message: msg,
        },
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
        out.push(ClientResult {
            client: "print",
            status: ClientStatus::Printed,
            backup: None,
            message: "printed generic snippet".to_string(),
        });
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
        out.push(json_result(
            "openclaw",
            &path,
            register_json_client(&path, &["mcp", "servers"], apply),
        ));
    }
    if out.is_empty() {
        print_generic();
        out.push(ClientResult {
            client: "print",
            status: ClientStatus::Printed,
            backup: None,
            message: "no known client detected; printed generic snippet".to_string(),
        });
    }
    out
}

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
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
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

    #[test]
    fn detects_existing_codex_block() {
        assert!(codex_block_present(
            "[mcp_servers.cloche]\ncommand = \"cloche\""
        ));
        assert!(codex_block_present(
            "[mcp_servers.other]\n\n[mcp_servers.cloche]\n"
        ));
        assert!(!codex_block_present("[mcp_servers.other]\ncommand = \"x\""));
        assert!(!codex_block_present(""));
    }

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
}

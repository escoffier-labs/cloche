//! Post-setup verification: prove capture, hotkey, and MCP actually work.

use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::setup::hotkey;
use crate::util;

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

fn nonempty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() > 0)
        .unwrap_or(false)
}

/// Check 1: run a real non-interactive screen capture and confirm artifacts.
pub fn check_capture_pipeline() -> Check {
    let has_gui = util::env_var("DISPLAY").is_some() || util::env_var("WAYLAND_DISPLAY").is_some();
    if !has_gui {
        return Check {
            name: "capture-pipeline",
            status: CheckStatus::Skip,
            detail: "no DISPLAY/WAYLAND_DISPLAY; cannot test capture here".to_string(),
        };
    }
    let dir = std::env::temp_dir().join(format!("cloche-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let exe = std::env::current_exe().unwrap_or_else(|_| "cloche".into());
    let status = std::process::Command::new(&exe)
        .args([
            "capture",
            "--target",
            "screen",
            "--presentation",
            "both",
            "--out-dir",
            &dir.to_string_lossy(),
            "--format",
            "json",
        ])
        .output();
    let shot = dir.join("shot.png");
    let card = dir.join("shot-card.png");
    let ok =
        matches!(status, Ok(ref o) if o.status.success()) && nonempty(&shot) && nonempty(&card);
    let detail = if ok {
        "captured shot.png and shot-card.png".to_string()
    } else {
        "capture did not produce both shot.png and shot-card.png".to_string()
    };
    let _ = std::fs::remove_dir_all(&dir);
    Check {
        name: "capture-pipeline",
        status: if ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        detail,
    }
}

/// Check 2: cloche-grab on PATH, and on GNOME a binding points at it.
pub fn check_hotkey() -> Check {
    let grab = hotkey::grab_script_path();
    if !grab.exists() {
        return Check {
            name: "hotkey",
            status: CheckStatus::Fail,
            detail: format!(
                "{} not installed; run `cloche setup hotkey`",
                grab.display()
            ),
        };
    }
    if hotkey::detect_desktop() != hotkey::Desktop::Gnome {
        return Check {
            name: "hotkey",
            status: CheckStatus::Skip,
            detail: "binding is manual on this desktop; grab script is installed".to_string(),
        };
    }
    match hotkey::bind_gnome(false) {
        Ok(hotkey::HotkeyOutcome::Bound { changed: false }) => Check {
            name: "hotkey",
            status: CheckStatus::Pass,
            detail: "Print is bound to cloche-grab".to_string(),
        },
        _ => Check {
            name: "hotkey",
            status: CheckStatus::Fail,
            detail: "no Print binding for cloche-grab; run `cloche setup hotkey`".to_string(),
        },
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
        Err(err) => {
            return Check {
                name: "agent-mcp",
                status: CheckStatus::Fail,
                detail: format!("could not start `cloche mcp`: {err}"),
            };
        }
    };
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = writeln!(stdin, "{init}");
        let _ = writeln!(stdin, "{list}");
    }
    let output = child.wait_with_output();
    let ok = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .any(tools_list_has_core),
        Err(_) => false,
    };
    Check {
        name: "agent-mcp",
        status: if ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        detail: if ok {
            "`cloche mcp` lists capture and polish".to_string()
        } else {
            "`cloche mcp` did not return the core tools".to_string()
        },
    }
}

/// Run all three checks.
pub fn run_all() -> Vec<Check> {
    vec![check_capture_pipeline(), check_hotkey(), check_agent_mcp()]
}

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

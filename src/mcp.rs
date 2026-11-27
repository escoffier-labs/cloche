//! Minimal stdio MCP (Model Context Protocol) server that wraps the CLI contract.
//!
//! Speaks newline-delimited JSON-RPC 2.0 over stdin/stdout. Each tool call shells
//! out to this same binary so the JSON contract stays defined in exactly one place.

use std::io::BufRead;
use std::io::Write;
use std::process::ExitCode;

use clap::Args;
use serde_json::Value;
use serde_json::json;

const PROTOCOL_VERSION: &str = "2025-06-18";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Args)]
pub struct McpArgs {}

/// Largest image file (raw bytes) the server will inline as an MCP `image`
/// block. Base64 inflates by a third, so 2 MiB becomes roughly 2.8 MB of JSON on
/// one stdio line. Cards and screenshots in the usual size range fit under it,
/// while a full-resolution multi-monitor frame would not, and agent clients tend
/// to truncate or reject multi-megabyte tool results. Over the cap the server
/// returns a text note instead; the file path in the JSON stays valid.
const MAX_INLINE_IMAGE_BYTES: u64 = 2 * 1024 * 1024;

/// The result of executing one tool.
pub struct ToolRun {
    pub text: String,
    pub is_error: bool,
    /// Further MCP content blocks (an `image` block or a text note), appended
    /// after the leading `text` block, which always holds the CLI JSON verbatim.
    pub extra_content: Vec<Value>,
}

impl ToolRun {
    fn new(text: String, is_error: bool) -> Self {
        Self {
            text,
            is_error,
            extra_content: Vec::new(),
        }
    }
}

pub fn run(_args: McpArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(trimmed) {
            Ok(request) => dispatch(&request, &mut run_tool_via_subprocess),
            Err(_) => Some(error_response(&Value::Null, -32700, "parse error")),
        };
        if let Some(response) = response {
            writeln!(out, "{}", serde_json::to_string(&response)?)?;
            out.flush()?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Handle a single JSON-RPC request. Returns `None` for notifications (no `id`).
///
/// `run_tool` is injected so the dispatch logic can be tested without spawning
/// subprocesses.
pub fn dispatch(
    request: &Value,
    run_tool: &mut dyn FnMut(&str, &Value) -> ToolRun,
) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    // Notifications carry no id and expect no response.
    let id = request.get("id")?;

    let response = match method {
        "initialize" => result_response(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "cloche", "version": VERSION }
            }),
        ),
        "ping" => result_response(id, json!({})),
        "tools/list" => result_response(id, json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            if !is_known_tool(name) {
                error_response(id, -32602, &format!("unknown tool: {name}"))
            } else {
                let run = run_tool(name, &arguments);
                let mut content = vec![json!({ "type": "text", "text": run.text })];
                content.extend(run.extra_content);
                result_response(id, json!({ "content": content, "isError": run.is_error }))
            }
        }
        _ => error_response(id, -32601, &format!("method not found: {method}")),
    };
    Some(response)
}

fn result_response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "capture" | "polish" | "list_windows" | "doctor" | "latest" | "gallery"
    )
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "capture",
            "description": "Capture an app/window screenshot and return the Cloche JSON result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": { "type": "string", "enum": ["active", "screen", "window", "region"], "default": "active", "description": "region opens an interactive selector and needs a human at the desk to drag." },
                    "title": { "type": "string", "description": "Window title to match when target=window." },
                    "app": { "type": "string", "description": "App name to match when target=window." },
                    "windowId": { "type": "string" },
                    "outDir": { "type": "string", "description": "Directory for flat capture artifacts (defaults to ~/Pictures/Cloche). Not a per-shot folder." },
                    "presentation": { "type": "string", "enum": ["raw", "card", "both"], "default": "both" },
                    "detail": { "type": "string", "enum": ["auto", "low", "high", "original"], "default": "high" },
                    "palette": { "type": "string", "enum": crate::polish::palette_names(), "description": "Backdrop palette for the card; falls back to the user's cloche config, then a random pick." },
                    "scene": { "type": "string", "enum": crate::polish::scene_names(), "description": "Pin the deep-space scene look (jwst, alma, cmb, etc.). Only applies to space palettes." },
                    "motif": { "type": "string", "enum": crate::polish::motif_names(), "description": "Pin the geometric motif (plaid, stripe, grid, etc.). Only applies to pattern palettes." },
                    "sky": { "type": "string", "enum": crate::polish::sky_names(), "description": "Pin the sky condition (bolt, mammatus, aurora, etc.). Only applies to sky palettes." },
                    "terrain": { "type": "string", "enum": crate::polish::terrain_names(), "description": "Pin the terrain kind (dunes, mesa, badlands, glacier). Only applies to terrain palettes." },
                    "styleSeed": { "type": "integer" },
                    "clipboard": { "type": "boolean", "description": "Copy the card to the system clipboard after capture." },
                    "format": { "type": "string", "enum": ["json"], "default": "json", "description": "Output format. The MCP wrapper always passes --format json." },
                    "includeImage": { "type": "boolean", "default": true, "description": "Also return the image as an MCP image content block (base64), so clients that cannot open the file path still see it. Skipped with a text note when the file is over 2 MiB. Not passed to the CLI." },
                    "imageVariant": { "type": "string", "enum": ["card", "raw"], "default": "card", "description": "Which image to attach: the presentation card, or the raw frame. Falls back to the other when the preferred one is not in the result. Not passed to the CLI." }
                },
                "required": []
            }
        },
        {
            "name": "polish",
            "description": "Style an existing image into a Cloche presentation card (rounded window, shadows, gradient backdrop) and return the JSON result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Path to the image to style (PNG, JPEG, or WebP)." },
                    "out": { "type": "string", "description": "Output card path; defaults to <input>-card.png next to the input." },
                    "palette": { "type": "string", "enum": crate::polish::palette_names(), "description": "Backdrop palette (gradient or deep-space scene); random when omitted." },
                    "scene": { "type": "string", "enum": crate::polish::scene_names(), "description": "Pin the deep-space scene look (jwst, alma, cmb, etc.); random when omitted. Only applies to space palettes." },
                    "motif": { "type": "string", "enum": crate::polish::motif_names(), "description": "Pin the geometric motif (plaid, stripe, grid, etc.); random when omitted. Only applies to pattern palettes." },
                    "sky": { "type": "string", "enum": crate::polish::sky_names(), "description": "Pin the sky condition (bolt, mammatus, aurora, etc.); random when omitted. Only applies to sky palettes." },
                    "terrain": { "type": "string", "enum": crate::polish::terrain_names(), "description": "Pin the terrain kind (dunes, mesa, badlands, glacier); random when omitted. Only applies to terrain palettes." },
                    "styleSeed": { "type": "integer", "description": "Seed for deterministic styling." },
                    "includeImage": { "type": "boolean", "default": true, "description": "Also return the image as an MCP image content block (base64), so clients that cannot open the file path still see it. Skipped with a text note when the file is over 2 MiB. Not passed to the CLI." },
                    "imageVariant": { "type": "string", "enum": ["card", "raw"], "default": "card", "description": "Which image to attach: the presentation card, or the raw frame. Falls back to the other when the preferred one is not in the result. Not passed to the CLI." }
                },
                "required": ["input"]
            }
        },
        {
            "name": "list_windows",
            "description": "List capturable windows as JSON.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "doctor",
            "description": "Report capture backend health and capabilities as JSON.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "latest",
            "description": "Return the most recent capture summary as JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "includeImage": { "type": "boolean", "default": true, "description": "Also return the image as an MCP image content block (base64), so clients that cannot open the file path still see it. Skipped with a text note when the file is over 2 MiB. Not passed to the CLI." },
                    "imageVariant": { "type": "string", "enum": ["card", "raw"], "default": "card", "description": "Which image to attach: the presentation card, or the raw frame. Falls back to the other when the preferred one is not in the result. Not passed to the CLI." }
                }
            }
        },
        {
            "name": "gallery",
            "description": "List recent captures as JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 20 }
                }
            }
        }
    ])
}

/// Translate a tool call into Cloche CLI arguments. Pure and testable.
pub fn tool_command_args(name: &str, arguments: &Value) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    match name {
        "capture" => {
            args.push("capture".to_string());
            let format = string_arg(arguments, "format").unwrap_or_else(|| "json".to_string());
            if format != "json" {
                return Err(format!(
                    "capture format must be json (got {format}); MCP always returns the CLI JSON contract"
                ));
            }
            args.push("--format".to_string());
            args.push(format);
            if let Some(value) = string_arg(arguments, "target") {
                args.push("--target".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "title") {
                args.push("--title".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "app") {
                args.push("--app".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "windowId") {
                args.push("--window-id".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "outDir") {
                args.push("--out-dir".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "presentation") {
                args.push("--presentation".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "detail") {
                args.push("--detail".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "palette") {
                args.push("--palette".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "scene") {
                args.push("--scene".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "sky") {
                args.push("--sky".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "terrain") {
                args.push("--terrain".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "motif") {
                args.push("--motif".to_string());
                args.push(value);
            }
            if let Some(value) = arguments.get("styleSeed").and_then(Value::as_u64) {
                args.push("--style-seed".to_string());
                args.push(value.to_string());
            }
            if arguments.get("clipboard").and_then(Value::as_bool) == Some(true) {
                args.push("--clipboard".to_string());
            }
        }
        "polish" => {
            let input =
                string_arg(arguments, "input").ok_or("polish requires an input image path")?;
            args.push("polish".to_string());
            args.push("--format".to_string());
            args.push("json".to_string());
            args.push(input);
            if let Some(value) = string_arg(arguments, "out") {
                args.push("--out".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "palette") {
                args.push("--palette".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "scene") {
                args.push("--scene".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "sky") {
                args.push("--sky".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "terrain") {
                args.push("--terrain".to_string());
                args.push(value);
            }
            if let Some(value) = string_arg(arguments, "motif") {
                args.push("--motif".to_string());
                args.push(value);
            }
            if let Some(value) = arguments.get("styleSeed").and_then(Value::as_u64) {
                args.push("--style-seed".to_string());
                args.push(value.to_string());
            }
        }
        "list_windows" => {
            args.push("list-windows".to_string());
            args.push("--format".to_string());
            args.push("json".to_string());
        }
        "doctor" => {
            args.push("doctor".to_string());
            args.push("--format".to_string());
            args.push("json".to_string());
        }
        "latest" => {
            args.push("latest".to_string());
            args.push("--format".to_string());
            args.push("json".to_string());
        }
        "gallery" => {
            args.push("gallery".to_string());
            args.push("--format".to_string());
            args.push("json".to_string());
            if let Some(value) = arguments.get("limit").and_then(Value::as_u64) {
                args.push("--limit".to_string());
                args.push(value.to_string());
            }
        }
        other => return Err(format!("unknown tool: {other}")),
    }
    Ok(args)
}

fn string_arg(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn run_tool_via_subprocess(name: &str, arguments: &Value) -> ToolRun {
    let args = match tool_command_args(name, arguments) {
        Ok(args) => args,
        Err(message) => {
            return ToolRun::new(message, true);
        }
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return ToolRun::new(format!("failed to locate Cloche binary: {err}"), true);
        }
    };
    match std::process::Command::new(exe).args(&args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let text = if output.status.success() {
                stdout
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if stdout.is_empty() { stderr } else { stdout }
            };
            let is_error = !output.status.success();
            let mut run = ToolRun::new(text, is_error);
            if !is_error {
                run.extra_content =
                    image_content(name, arguments, &run.text, MAX_INLINE_IMAGE_BYTES);
            }
            run
        }
        Err(err) => ToolRun::new(format!("failed to run capture: {err}"), true),
    }
}

/// Which image variant a tool call asked for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageVariant {
    Card,
    Raw,
}

/// Build the extra content blocks for a successful image-producing tool call:
/// one `image` block, or a short text note when the image cannot be attached.
///
/// This runs after the subprocess returns and only reads the file path the CLI
/// JSON named, so the CLI stays the source of truth. Image problems never fail
/// the tool call; they become a note.
fn image_content(name: &str, arguments: &Value, cli_json: &str, max_bytes: u64) -> Vec<Value> {
    if !matches!(name, "capture" | "polish" | "latest") {
        return Vec::new();
    }
    if arguments.get("includeImage").and_then(Value::as_bool) == Some(false) {
        return Vec::new();
    }
    let variant = match arguments.get("imageVariant").and_then(Value::as_str) {
        Some("raw") => ImageVariant::Raw,
        _ => ImageVariant::Card,
    };
    let Ok(result) = serde_json::from_str::<Value>(cli_json) else {
        return Vec::new();
    };
    let Some((path, mime)) = pick_image(name, &result, variant) else {
        return Vec::new();
    };
    let note = |reason: String| {
        vec![json!({ "type": "text", "text": format!("Image not attached: {reason}") })]
    };
    let size = match std::fs::metadata(&path) {
        Ok(metadata) => metadata.len(),
        Err(err) => return note(format!("could not read {path}: {err}")),
    };
    if size > max_bytes {
        return note(format!(
            "{path} is {size} bytes, over the {max_bytes} byte inline limit; open the file path instead"
        ));
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            // The CLI's ImageInfo.mime is always "image/png", but `polish`
            // accepts JPEG and WebP input, so trust the file's magic bytes.
            let mime = sniff_image_mime(&bytes).map_or(mime, str::to_string);
            vec![json!({
                "type": "image",
                "data": crate::util::base64_encode(&bytes),
                "mimeType": mime
            })]
        }
        Err(err) => note(format!("could not read {path}: {err}")),
    }
}

/// MIME type from the file signature for the formats `polish` accepts.
fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Find the path and MIME type of the image to attach in a CLI JSON result.
/// Prefers the requested variant and falls back to the other one.
fn pick_image(name: &str, result: &Value, variant: ImageVariant) -> Option<(String, String)> {
    // `latest` nests the summary under `capture`; `polish` calls its input the
    // raw frame and its output the card.
    let (raw_key, card_key, root) = match name {
        "capture" => ("image", "presentationImage", result),
        "latest" => ("image", "presentationImage", result.get("capture")?),
        "polish" => ("input", "card", result),
        _ => return None,
    };
    let image_at = |key: &str| {
        let info = root.get(key)?;
        let path = info.get("path")?.as_str()?.to_string();
        let mime = info
            .get("mime")
            .and_then(Value::as_str)
            .filter(|mime| mime.starts_with("image/"))
            .unwrap_or("image/png")
            .to_string();
        Some((path, mime))
    };
    match variant {
        ImageVariant::Card => image_at(card_key).or_else(|| image_at(raw_key)),
        ImageVariant::Raw => image_at(raw_key).or_else(|| image_at(card_key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never_called(_name: &str, _arguments: &Value) -> ToolRun {
        panic!("tool runner should not be called");
    }

    #[test]
    fn initialize_reports_protocol_and_server() {
        let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" });
        let response = dispatch(&request, &mut never_called).expect("response");
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "cloche");
        assert_eq!(response["id"], 1);
    }

    #[test]
    fn tools_list_includes_capture() {
        let request = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" });
        let response = dispatch(&request, &mut never_called).expect("response");
        let tools = response["result"]["tools"].as_array().expect("tools array");
        assert!(tools.iter().any(|tool| tool["name"] == "capture"));
    }

    #[test]
    fn notification_yields_no_response() {
        let request = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(dispatch(&request, &mut never_called).is_none());
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let request = json!({ "jsonrpc": "2.0", "id": 3, "method": "frobnicate" });
        let response = dispatch(&request, &mut never_called).expect("response");
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn unknown_tool_returns_invalid_params() {
        let request = json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "bogus", "arguments": {} }
        });
        let response = dispatch(&request, &mut never_called).expect("response");
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn tools_call_wraps_runner_output() {
        let request = json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "doctor", "arguments": {} }
        });
        let mut runner = |name: &str, _args: &Value| {
            assert_eq!(name, "doctor");
            ToolRun::new("{\"ok\":true}".to_string(), false)
        };
        let response = dispatch(&request, &mut runner).expect("response");
        assert_eq!(response["result"]["isError"], false);
        assert_eq!(response["result"]["content"][0]["text"], "{\"ok\":true}");
    }

    #[test]
    fn capture_args_include_target_and_title() {
        let args = tool_command_args(
            "capture",
            &json!({ "target": "window", "title": "Firefox", "styleSeed": 7 }),
        )
        .expect("args");
        assert_eq!(args[0], "capture");
        assert!(args.windows(2).any(|w| w == ["--target", "window"]));
        assert!(args.windows(2).any(|w| w == ["--title", "Firefox"]));
        assert!(args.windows(2).any(|w| w == ["--style-seed", "7"]));
    }

    #[test]
    fn gallery_args_pass_limit() {
        let args = tool_command_args("gallery", &json!({ "limit": 5 })).expect("args");
        assert!(args.windows(2).any(|w| w == ["--limit", "5"]));
    }

    #[test]
    fn tools_list_includes_polish() {
        let request = json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/list" });
        let response = dispatch(&request, &mut never_called).expect("response");
        let tools = response["result"]["tools"].as_array().expect("tools array");
        assert!(tools.iter().any(|tool| tool["name"] == "polish"));
    }

    #[test]
    fn polish_args_map_input_out_palette_and_seed() {
        let args = tool_command_args(
            "polish",
            &json!({
                "input": "/tmp/shot.png",
                "out": "/tmp/card.png",
                "palette": "violet-haze",
                "styleSeed": 12
            }),
        )
        .expect("args");
        assert_eq!(args[0], "polish");
        assert!(args.contains(&"/tmp/shot.png".to_string()));
        assert!(args.windows(2).any(|w| w == ["--out", "/tmp/card.png"]));
        assert!(args.windows(2).any(|w| w == ["--palette", "violet-haze"]));
        assert!(args.windows(2).any(|w| w == ["--style-seed", "12"]));
    }

    #[test]
    fn capture_args_map_region_and_clipboard() {
        let args = tool_command_args("capture", &json!({ "target": "region", "clipboard": true }))
            .expect("args");
        assert!(args.windows(2).any(|w| w == ["--target", "region"]));
        assert!(args.contains(&"--clipboard".to_string()));
    }

    #[test]
    fn capture_args_omit_clipboard_when_false() {
        let args = tool_command_args("capture", &json!({ "clipboard": false })).expect("args");
        assert!(!args.contains(&"--clipboard".to_string()));
    }

    #[test]
    fn capture_args_map_palette_and_scene() {
        let args = tool_command_args(
            "capture",
            &json!({ "palette": "orion-emission", "scene": "jwst" }),
        )
        .expect("args");
        assert!(
            args.windows(2)
                .any(|w| w == ["--palette", "orion-emission"])
        );
        assert!(args.windows(2).any(|w| w == ["--scene", "jwst"]));
    }

    #[test]
    fn capture_tool_advertises_every_palette_and_scene() {
        let tools = tool_definitions();
        let capture = tools
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "capture")
            .expect("capture tool");
        let properties = &capture["inputSchema"]["properties"];
        assert_eq!(
            properties["palette"]["enum"]
                .as_array()
                .expect("palettes")
                .len(),
            crate::polish::palette_names().len()
        );
        assert_eq!(
            properties["scene"]["enum"]
                .as_array()
                .expect("scenes")
                .len(),
            crate::polish::scene_names().len()
        );
    }

    #[test]
    fn polish_args_require_input() {
        let error = tool_command_args("polish", &json!({})).expect_err("missing input");
        assert!(error.contains("input"));
    }

    #[test]
    fn capture_schema_documents_format_and_required() {
        let tools = tool_definitions();
        let capture = tools
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "capture")
            .expect("capture tool");
        let schema = &capture["inputSchema"];
        assert!(schema["required"].is_array());
        assert_eq!(schema["properties"]["format"]["enum"], json!(["json"]));
        assert_eq!(schema["properties"]["format"]["default"], json!("json"));
    }

    #[test]
    fn capture_args_accept_explicit_json_format() {
        let args = tool_command_args("capture", &json!({ "format": "json" })).expect("args");
        assert!(args.windows(2).any(|w| w == ["--format", "json"]));
    }

    #[test]
    fn capture_args_reject_non_json_format() {
        let error =
            tool_command_args("capture", &json!({ "format": "text" })).expect_err("bad format");
        assert!(error.contains("json"));
    }

    #[test]
    fn capture_args_forward_terrain() {
        let args = tool_command_args(
            "capture",
            &json!({ "palette": "dunes", "terrain": "mesa", "styleSeed": 3 }),
        )
        .expect("args");
        assert!(args.windows(2).any(|w| w == ["--palette", "dunes"]));
        assert!(args.windows(2).any(|w| w == ["--terrain", "mesa"]));
    }

    #[test]
    fn polish_args_forward_terrain() {
        let args = tool_command_args(
            "polish",
            &json!({ "input": "/tmp/shot.png", "palette": "dunes", "terrain": "glacier" }),
        )
        .expect("args");
        assert!(args.windows(2).any(|w| w == ["--terrain", "glacier"]));
    }

    #[test]
    fn capture_tool_advertises_the_terrain_enum() {
        let tools = tool_definitions();
        let capture = tools
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "capture")
            .expect("capture tool");
        let properties = &capture["inputSchema"]["properties"];
        let names = properties["terrain"]["enum"]
            .as_array()
            .expect("terrain enum");
        assert_eq!(names.len(), crate::polish::terrain_names().len());
        for kind in ["dunes", "mesa", "badlands", "glacier"] {
            assert!(
                names.contains(&json!(kind)),
                "{kind} missing from MCP terrain enum"
            );
        }
        let capture_desc = properties["terrain"]["description"]
            .as_str()
            .expect("capture terrain description");
        for kind in ["dunes", "mesa", "badlands", "glacier"] {
            assert!(
                capture_desc.contains(kind),
                "capture description missing {kind}"
            );
        }
        assert!(
            !capture_desc.contains("cave-mouth"),
            "capture description still mentions cave-mouth"
        );
        assert!(
            !capture_desc.contains("slot-canyon"),
            "capture description still mentions slot-canyon"
        );

        let polish = tools
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "polish")
            .expect("polish tool");
        let polish_props = &polish["inputSchema"]["properties"];
        let polish_desc = polish_props["terrain"]["description"]
            .as_str()
            .expect("polish terrain description");
        for kind in ["dunes", "mesa", "badlands", "glacier"] {
            assert!(
                polish_desc.contains(kind),
                "polish description missing {kind}"
            );
        }
        assert!(
            !polish_desc.contains("cave-mouth"),
            "polish description still mentions cave-mouth"
        );
        assert!(
            !polish_desc.contains("slot-canyon"),
            "polish description still mentions slot-canyon"
        );
    }

    /// A 1x1 PNG: signature plus IHDR/IDAT/IEND, enough bytes to round-trip.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xa7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn image_temp_dir(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cloche-mcp-test-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn capture_json(raw: &std::path::Path, card: &std::path::Path) -> String {
        json!({
            "ok": true,
            "image": { "path": raw, "mime": "image/png" },
            "presentationImage": { "path": card, "mime": "image/png" }
        })
        .to_string()
    }

    #[test]
    fn dispatch_appends_extra_content_after_text() {
        let request = json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": { "name": "capture", "arguments": {} }
        });
        let mut runner = |_name: &str, _args: &Value| {
            let mut run = ToolRun::new("{\"ok\":true}".to_string(), false);
            run.extra_content =
                vec![json!({ "type": "image", "data": "AAAA", "mimeType": "image/png" })];
            run
        };
        let response = dispatch(&request, &mut runner).expect("response");
        let content = response["result"]["content"].as_array().expect("content");
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["mimeType"], "image/png");
    }

    #[test]
    fn dispatch_without_extra_content_is_text_only() {
        let request = json!({
            "jsonrpc": "2.0", "id": 8, "method": "tools/call",
            "params": { "name": "doctor", "arguments": {} }
        });
        let mut runner = |_name: &str, _args: &Value| ToolRun::new("{}".to_string(), false);
        let response = dispatch(&request, &mut runner).expect("response");
        assert_eq!(
            response["result"]["content"]
                .as_array()
                .expect("content")
                .len(),
            1
        );
    }

    #[test]
    fn capture_attaches_card_image_by_default() {
        let dir = image_temp_dir("card-default");
        let raw = dir.join("raw.png");
        let card = dir.join("card.png");
        std::fs::write(&raw, b"raw bytes").expect("raw");
        std::fs::write(&card, TINY_PNG).expect("card");
        let blocks = image_content("capture", &json!({}), &capture_json(&raw, &card), 1024);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "image");
        assert_eq!(blocks[0]["mimeType"], "image/png");
        assert_eq!(blocks[0]["data"], crate::util::base64_encode(TINY_PNG));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn raw_variant_attaches_raw_image() {
        let dir = image_temp_dir("raw-variant");
        let raw = dir.join("raw.png");
        let card = dir.join("card.png");
        std::fs::write(&raw, b"raw bytes").expect("raw");
        std::fs::write(&card, TINY_PNG).expect("card");
        let blocks = image_content(
            "capture",
            &json!({ "imageVariant": "raw" }),
            &capture_json(&raw, &card),
            1024,
        );
        assert_eq!(blocks[0]["data"], crate::util::base64_encode(b"raw bytes"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn include_image_false_skips_image() {
        let dir = image_temp_dir("opt-out");
        let card = dir.join("card.png");
        std::fs::write(&card, TINY_PNG).expect("card");
        let blocks = image_content(
            "capture",
            &json!({ "includeImage": false }),
            &capture_json(&card, &card),
            1024,
        );
        assert!(blocks.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_attaches_card_and_latest_reads_nested_capture() {
        let dir = image_temp_dir("polish-latest");
        let card = dir.join("card.png");
        std::fs::write(&card, TINY_PNG).expect("card");
        let polish_json = json!({ "ok": true, "card": { "path": &card, "mime": "image/png" } });
        let blocks = image_content("polish", &json!({}), &polish_json.to_string(), 1024);
        assert_eq!(blocks[0]["type"], "image");
        let latest_json = json!({
            "ok": true,
            "capture": { "presentationImage": { "path": &card, "mime": "image/png" } }
        });
        let blocks = image_content("latest", &json!({}), &latest_json.to_string(), 1024);
        assert_eq!(blocks[0]["type"], "image");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_raw_when_no_card_and_keeps_jpeg_mime() {
        let dir = image_temp_dir("fallback");
        let raw = dir.join("raw.jpg");
        std::fs::write(&raw, b"jpeg").expect("raw");
        let json_text = json!({ "ok": true, "image": { "path": &raw, "mime": "image/jpeg" } });
        let blocks = image_content("capture", &json!({}), &json_text.to_string(), 1024);
        assert_eq!(blocks[0]["mimeType"], "image/jpeg");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mime_comes_from_file_signature_not_cli_label() {
        // The CLI labels every image "image/png", including a JPEG or WebP
        // `polish` input; the attached block must match the real bytes.
        let dir = image_temp_dir("sniff");
        let jpeg = dir.join("input.jpg");
        std::fs::write(&jpeg, [0xFF, 0xD8, 0xFF, 0xE0, 0, 0]).expect("jpeg");
        let webp = dir.join("input.webp");
        std::fs::write(&webp, b"RIFF\0\0\0\0WEBPVP8 ").expect("webp");
        for (path, want) in [(&jpeg, "image/jpeg"), (&webp, "image/webp")] {
            let json_text = json!({ "ok": true, "input": { "path": path, "mime": "image/png" } });
            let args = json!({ "imageVariant": "raw" });
            let blocks = image_content("polish", &args, &json_text.to_string(), 1024);
            assert_eq!(blocks[0]["mimeType"], want);
        }
        assert_eq!(
            sniff_image_mime(b"\x89PNG\r\n\x1a\nrest"),
            Some("image/png")
        );
        assert_eq!(sniff_image_mime(b"plain"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oversized_image_becomes_a_text_note() {
        let dir = image_temp_dir("oversize");
        let card = dir.join("card.png");
        std::fs::write(&card, TINY_PNG).expect("card");
        let blocks = image_content(
            "capture",
            &json!({}),
            &capture_json(&card, &card),
            (TINY_PNG.len() as u64) - 1,
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "text");
        let note = blocks[0]["text"].as_str().expect("note");
        assert!(note.contains("Image not attached"));
        assert!(note.contains("inline limit"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn image_at_exactly_the_cap_is_attached() {
        let dir = image_temp_dir("at-cap");
        let card = dir.join("card.png");
        std::fs::write(&card, TINY_PNG).expect("card");
        let blocks = image_content(
            "capture",
            &json!({}),
            &capture_json(&card, &card),
            TINY_PNG.len() as u64,
        );
        assert_eq!(blocks[0]["type"], "image");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_image_file_becomes_a_text_note() {
        let dir = image_temp_dir("missing");
        let gone = dir.join("gone.png");
        let blocks = image_content("capture", &json!({}), &capture_json(&gone, &gone), 1024);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "text");
        assert!(
            blocks[0]["text"]
                .as_str()
                .expect("note")
                .contains("could not read")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_image_tools_and_unparseable_json_add_nothing() {
        assert!(image_content("doctor", &json!({}), "{}", 1024).is_empty());
        assert!(image_content("capture", &json!({}), "not json", 1024).is_empty());
        assert!(image_content("capture", &json!({}), "{\"ok\":true}", 1024).is_empty());
    }

    #[test]
    fn image_options_are_not_forwarded_to_the_cli() {
        let args = tool_command_args(
            "capture",
            &json!({ "includeImage": false, "imageVariant": "raw" }),
        )
        .expect("args");
        assert!(
            !args
                .iter()
                .any(|arg| arg.contains("image") || arg.contains("variant"))
        );
    }

    #[test]
    fn image_tools_document_include_image_and_variant() {
        let tools = tool_definitions();
        for name in ["capture", "polish", "latest"] {
            let tool = tools
                .as_array()
                .expect("tools")
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("tool");
            let properties = &tool["inputSchema"]["properties"];
            assert_eq!(properties["includeImage"]["type"], "boolean", "{name}");
            assert_eq!(properties["includeImage"]["default"], true, "{name}");
            assert_eq!(
                properties["imageVariant"]["enum"],
                json!(["card", "raw"]),
                "{name}"
            );
        }
    }
}

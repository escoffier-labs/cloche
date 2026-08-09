//! `cloche studio`: a local page for choosing backdrops without flags.
//!
//! Phase 2 of the styling-preferences work. The page is a front end over the
//! same config `capture` and `polish` read, and over [`polish::render_backdrop`]
//! for its swatches. It writes `config.json` on the machine it runs on, so it
//! binds to loopback by default and there is no authentication: treat the bind
//! address as the security boundary.
//!
//! The HTTP handling here is deliberately small and hand-rolled rather than a
//! dependency. It serves one page, four JSON/PNG endpoints, and one client at a
//! time; anything more would be the wrong shape for a local picker.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Args;
use image::RgbaImage;
use serde::Serialize;

use crate::captures;
use crate::config;
use crate::polish;

/// Refuse absurd swatch sizes: the dimensions arrive on a query string, and a
/// procedural space scene is painted per pixel.
const MIN_SIZE: u32 = 8;
const MAX_SIZE: u32 = 2048;
/// One request should not be able to hold the accept loop open forever.
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 256 * 1024;
/// Browsers pre-open sockets and then sit on them without sending anything.
/// Without a read deadline one of those idles wedges the whole server.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

const PAGE: &str = include_str!("studio.html");

#[derive(Debug, Args)]
pub struct StudioArgs {
    /// Port to listen on. 0 asks the OS for a free one.
    #[arg(long, default_value_t = 4317)]
    pub port: u16,
    /// Address to bind. Defaults to loopback; the page can write your config,
    /// so only widen this on a network you trust.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Print the URL and exit instead of serving. Useful for scripts.
    #[arg(long)]
    pub print_url: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioState {
    config_path: PathBuf,
    exists: bool,
    config: config::ClocheConfig,
    options: crate::contract::StyleOptions,
    /// Newest capture on disk, used for the full-card preview. `None` means the
    /// page shows backdrops only.
    sample: Option<PathBuf>,
    warnings: Vec<String>,
}

pub fn run(args: StudioArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind((args.host.as_str(), args.port))?;
    let url = format!("http://{}/", listener.local_addr()?);

    if args.print_url {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true, "url": url, "configPath": config::path(),
            }))?
        );
        return Ok(ExitCode::SUCCESS);
    }

    eprintln!("cloche studio: {url}");
    eprintln!("editing {}", config::path().display());
    eprintln!("stop with ctrl-c");

    for stream in listener.incoming() {
        match stream {
            // A thread per connection: the page asks for dozens of swatches at
            // once and browsers open several sockets in parallel, so serving
            // them strictly in turn makes the page load one image at a time.
            // One bad client should not end the session either.
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(err) = serve(stream) {
                        eprintln!("studio: {err}");
                    }
                });
            }
            Err(err) => eprintln!("studio: connection failed: {err}"),
        }
    }
    Ok(ExitCode::SUCCESS)
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, value: &serde_json::Value) -> Self {
        Response {
            status,
            content_type: "application/json; charset=utf-8",
            body: serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec()),
        }
    }
    fn error(status: u16, message: &str) -> Self {
        Response::json(
            status,
            &serde_json::json!({ "ok": false, "error": message }),
        )
    }
}

fn serve(mut stream: TcpStream) -> Result<(), String> {
    // Deadlines on both directions so a silent or stalled peer releases the
    // thread instead of holding it for the life of the process.
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);

    let mut line = String::new();
    if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
        // Peer opened a socket and never spoke. Nothing to answer.
        return Ok(());
    }
    let Some((method, path, query)) = parse_request_line(&line) else {
        return write_response(&mut stream, Response::error(400, "malformed request line"));
    };

    let mut content_length = 0usize;
    let mut header_bytes = 0usize;
    loop {
        let mut header = String::new();
        let read = reader
            .read_line(&mut header)
            .map_err(|e| format!("read header: {e}"))?;
        header_bytes += read;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if header_bytes > MAX_HEADER_BYTES {
            return write_response(&mut stream, Response::error(431, "headers too large"));
        }
        if let Some(value) = header_value(&header, "content-length") {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }

    if content_length > MAX_BODY_BYTES {
        return write_response(&mut stream, Response::error(413, "body too large"));
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|e| format!("read body: {e}"))?;
    }

    let response = route(&method, &path, &query, &body);
    write_response(&mut stream, response)
}

/// `GET /api/backdrop?w=10 HTTP/1.1` -> ("GET", "/api/backdrop", "w=10")
fn parse_request_line(line: &str) -> Option<(String, String, String)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    Some((method, path.to_string(), query.to_string()))
}

fn header_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    let (key, value) = header.split_once(':')?;
    // Trim both ends: `Content-Length: 42\r\n` carries a leading space as well
    // as the line ending, and callers should get the bare value.
    key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
}

/// Percent-decoded value for `key`, or `None` when absent or empty.
fn query_get(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| percent_decode(value))
        .filter(|value| !value.is_empty())
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn size_param(query: &str, key: &str, fallback: u32) -> u32 {
    query_get(query, key)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(fallback)
        .clamp(MIN_SIZE, MAX_SIZE)
}

fn route(method: &str, path: &str, query: &str, body: &[u8]) -> Response {
    match (method, path) {
        ("GET", "/") => Response {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: PAGE.as_bytes().to_vec(),
        },
        ("GET", "/api/state") => Response::json(200, &state_value()),
        ("POST", "/api/config") => match save_body(body) {
            Ok(()) => Response::json(200, &state_value()),
            Err(err) => Response::error(400, &err),
        },
        ("GET", "/api/backdrop") => match backdrop_png(query) {
            Ok(bytes) => Response {
                status: 200,
                content_type: "image/png",
                body: bytes,
            },
            Err(err) => Response::error(400, &err),
        },
        ("GET", "/api/card") => match card_png(query) {
            Ok(bytes) => Response {
                status: 200,
                content_type: "image/png",
                body: bytes,
            },
            Err(err) => Response::error(404, &err),
        },
        ("GET", _) => Response::error(404, "not found"),
        _ => Response::error(405, "method not allowed"),
    }
}

fn state_value() -> serde_json::Value {
    let path = config::path();
    let (config, warnings) = config::load_from(&path);
    let state = StudioState {
        exists: path.exists(),
        config_path: path,
        config,
        options: crate::contract::StyleOptions {
            palettes: polish::palette_catalog()
                .into_iter()
                .map(|(name, kind)| crate::contract::PaletteOption {
                    name: name.to_string(),
                    kind: kind.to_string(),
                })
                .collect(),
            scenes: polish::scene_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            motifs: polish::motif_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            skies: polish::sky_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            terrains: polish::terrain_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        sample: newest_capture(),
        warnings,
    };
    serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({}))
}

/// The full-card preview needs a real screenshot. The newest capture on disk is
/// the honest choice: it is what the user last shot.
fn newest_capture() -> Option<PathBuf> {
    captures::find_captures(Vec::new(), 1)
        .into_iter()
        .next()
        .and_then(|capture| capture.image.map(|info| info.path))
}

fn save_body(body: &[u8]) -> Result<(), String> {
    let prefs: config::PolishPrefs =
        serde_json::from_slice(body).map_err(|err| format!("invalid preferences: {err}"))?;
    let known_palette: Vec<&str> = polish::palette_names();
    let known_scene: Vec<&str> = polish::scene_names();
    // The page should never send an unknown name, but this endpoint is reachable
    // by anything that can hit the port, and the file it writes is read on every
    // capture.
    for name in prefs.palettes.iter().chain(prefs.palette.iter()) {
        if !known_palette.contains(&name.as_str()) {
            return Err(format!("unknown palette: {name}"));
        }
    }
    for name in prefs.scenes.iter().chain(prefs.scene.iter()) {
        if !known_scene.contains(&name.as_str()) {
            return Err(format!("unknown scene: {name}"));
        }
    }
    let known_motif: Vec<&str> = polish::motif_names();
    for name in prefs.motifs.iter().chain(prefs.motif.iter()) {
        if !known_motif.contains(&name.as_str()) {
            return Err(format!("unknown motif: {name}"));
        }
    }
    let known_sky: Vec<&str> = polish::sky_names();
    for name in prefs.skies.iter().chain(prefs.sky.iter()) {
        if !known_sky.contains(&name.as_str()) {
            return Err(format!("unknown sky: {name}"));
        }
    }
    let known_terrain: Vec<&str> = polish::terrain_names();
    for name in prefs.terrains.iter().chain(prefs.terrain.iter()) {
        if !known_terrain.contains(&name.as_str()) {
            return Err(format!("unknown terrain: {name}"));
        }
    }
    let path = config::path();
    let (mut stored, _) = config::load_from(&path);
    stored.polish = prefs;
    config::save_to(&path, &stored).map_err(|err| err.to_string())
}

fn style_from_query(query: &str) -> polish::PresentationStyle {
    let seed = query_get(query, "seed")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mut style = query_get(query, "palette")
        .and_then(|name| polish::style_with_palette(seed, &name))
        .unwrap_or_else(|| polish::style_from_seed(seed));
    if let Some(scene) = query_get(query, "scene").and_then(|n| polish::scene_from_name(&n))
        && style.is_space()
    {
        style.scene = Some(scene);
    }
    if let Some(motif) = query_get(query, "motif").and_then(|n| polish::motif_from_name(&n))
        && style.is_pattern()
    {
        style.motif = Some(motif);
    }
    if let Some(sky) = query_get(query, "sky").and_then(|n| polish::sky_from_name(&n))
        && style.is_sky()
    {
        style.sky = Some(sky);
    }
    if let Some(terrain) = query_get(query, "terrain").and_then(|n| polish::terrain_from_name(&n))
        && style.is_terrain()
    {
        style.terrain = Some(terrain);
    }
    style
}

fn backdrop_png(query: &str) -> Result<Vec<u8>, String> {
    let width = size_param(query, "w", 320);
    let height = size_param(query, "h", 240);
    png_bytes(&polish::render_backdrop(
        width,
        height,
        &style_from_query(query),
    ))
}

fn card_png(query: &str) -> Result<Vec<u8>, String> {
    let sample = newest_capture().ok_or("no capture on disk to preview")?;
    let input = image::open(&sample).map_err(|err| format!("{}: {err}", sample.display()))?;
    let card = polish::compose_card(&input, &style_from_query(query));
    png_bytes(&card)
}

fn png_bytes(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok(out.into_inner())
}

fn write_response(stream: &mut TcpStream, response: Response) -> Result<(), String> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len(),
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(&response.body))
        .and_then(|()| stream.flush())
        .map_err(|err| format!("write response: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_requests_all_get_answered() {
        // Regression: the accept loop used to be strictly serial, so a browser
        // pre-opening a socket without sending a request wedged the server and
        // the page loaded no swatches at all.
        use std::io::Read as _;
        use std::io::Write as _;
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let port = listener.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(5).flatten() {
                std::thread::spawn(move || {
                    let _ = serve(stream);
                });
            }
        });

        // An idle socket, opened first and never written to, exactly like a
        // browser pre-connect.
        let _idle = TcpStream::connect(("127.0.0.1", port)).expect("idle connect");

        for _ in 0..3 {
            let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect");
            client
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("timeout");
            client
                .write_all(b"GET /api/state HTTP/1.1\r\nHost: x\r\n\r\n")
                .expect("write");
            let mut response = Vec::new();
            client.read_to_end(&mut response).expect("read");
            let head = String::from_utf8_lossy(&response);
            assert!(
                head.starts_with("HTTP/1.1 200"),
                "got: {}",
                &head[..40.min(head.len())]
            );
        }
    }

    #[test]
    fn an_empty_connection_is_not_an_error() {
        // read_line returning 0 means the peer closed without speaking.
        assert!(parse_request_line("").is_none());
    }

    #[test]
    fn request_line_splits_path_from_query() {
        let (method, path, query) =
            parse_request_line("GET /api/backdrop?palette=jwst&w=10 HTTP/1.1\r\n").expect("parsed");
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/backdrop");
        assert_eq!(query, "palette=jwst&w=10");
    }

    #[test]
    fn request_line_without_a_query_is_still_valid() {
        let (_, path, query) = parse_request_line("GET / HTTP/1.1\r\n").expect("parsed");
        assert_eq!(path, "/");
        assert_eq!(query, "");
    }

    #[test]
    fn garbage_request_line_is_rejected() {
        assert!(parse_request_line("").is_none());
        assert!(parse_request_line("GET\r\n").is_none());
    }

    #[test]
    fn header_lookup_ignores_case_and_whitespace() {
        assert_eq!(
            header_value("Content-Length: 42\r\n", "content-length"),
            Some("42")
        );
        assert_eq!(
            header_value("content-length:7\r\n", "content-length"),
            Some("7")
        );
        assert_eq!(header_value("Host: x\r\n", "content-length"), None);
    }

    #[test]
    fn query_values_are_percent_decoded() {
        assert_eq!(
            query_get("scene=deep%2Dfield", "scene").as_deref(),
            Some("deep-field")
        );
        assert_eq!(query_get("a=1&b=2", "b").as_deref(), Some("2"));
        assert_eq!(
            query_get("a=&b=2", "a"),
            None,
            "empty value reads as absent"
        );
        assert_eq!(query_get("a=1", "missing"), None);
    }

    #[test]
    fn sizes_are_clamped_so_a_query_string_cannot_ask_for_a_huge_render() {
        assert_eq!(size_param("w=999999", "w", 320), MAX_SIZE);
        assert_eq!(size_param("w=1", "w", 320), MIN_SIZE);
        assert_eq!(size_param("w=not-a-number", "w", 320), 320);
        assert_eq!(size_param("", "w", 320), 320);
        assert_eq!(size_param("w=640", "w", 320), 640);
    }

    #[test]
    fn unknown_paths_and_methods_are_refused() {
        assert_eq!(route("GET", "/nope", "", b"").status, 404);
        assert_eq!(route("DELETE", "/api/config", "", b"").status, 405);
    }

    #[test]
    fn the_page_is_served_at_the_root() {
        let response = route("GET", "/", "", b"");
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(!response.body.is_empty());
    }

    #[test]
    fn backdrop_endpoint_returns_a_png() {
        let bytes = backdrop_png("palette=orion-emission&seed=3&w=32&h=24").expect("png");
        assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&bytes).expect("decodes");
        assert_eq!(decoded.width(), 32);
        assert_eq!(decoded.height(), 24);
    }

    #[test]
    fn backdrop_endpoint_honors_the_palette() {
        let a = backdrop_png("palette=ember-glow&seed=3&w=24&h=18").expect("png");
        let b = backdrop_png("palette=aurora-teal&seed=3&w=24&h=18").expect("png");
        assert_ne!(a, b);
    }

    #[test]
    fn a_motif_is_ignored_on_a_non_pattern_palette() {
        let style = style_from_query("palette=orion-emission&motif=plaid&seed=1");
        assert_eq!(style.motif, None);
        let woven = style_from_query("palette=tartan-moss&motif=plaid&seed=1");
        assert_eq!(woven.motif, polish::motif_from_name("plaid"));
    }

    #[test]
    fn posting_an_unknown_motif_is_refused() {
        let err = save_body(br#"{"motifs":["paisley"]}"#).expect_err("rejected");
        assert!(err.contains("paisley"), "{err}");
    }

    #[test]
    fn a_scene_is_ignored_on_a_gradient_palette() {
        // Matches the CLI and config behavior: scenes are space-only.
        let style = style_from_query("palette=ember-glow&scene=jwst&seed=1");
        assert_eq!(style.scene, None);
        let space = style_from_query("palette=orion-emission&scene=jwst&seed=1");
        assert_eq!(space.scene, polish::scene_from_name("jwst"));
    }

    #[test]
    fn posting_an_unknown_name_is_refused_before_it_reaches_the_config() {
        let err = save_body(br#"{"palettes":["hotdog-stand"]}"#).expect_err("rejected");
        assert!(err.contains("hotdog-stand"), "{err}");
        let err = save_body(br#"{"scene":"not-a-scene"}"#).expect_err("rejected");
        assert!(err.contains("not-a-scene"), "{err}");
        let err = save_body(b"not json").expect_err("rejected");
        assert!(err.contains("invalid preferences"), "{err}");
    }

    #[test]
    fn state_reports_the_menu_the_page_needs() {
        let value = state_value();
        assert_eq!(
            value["options"]["palettes"]
                .as_array()
                .expect("palettes")
                .len(),
            polish::palette_names().len()
        );
        assert_eq!(
            value["options"]["scenes"].as_array().expect("scenes").len(),
            polish::scene_names().len()
        );
        assert!(value["configPath"].is_string());
    }

    #[test]
    fn a_terrain_is_ignored_on_a_non_terrain_palette() {
        let style = style_from_query("palette=ember-glow&terrain=dunes&seed=1");
        assert_eq!(style.terrain, None);
        let terrain = style_from_query("palette=dunes&terrain=mesa&seed=1");
        assert_eq!(terrain.terrain, polish::terrain_from_name("mesa"));
    }

    #[test]
    fn posting_an_unknown_terrain_is_refused() {
        let err = save_body(br#"{"terrains":["butte"]}"#).expect_err("rejected");
        assert!(err.contains("butte"), "{err}");
    }

    #[test]
    fn state_reports_the_terrain_menu() {
        let value = state_value();
        assert_eq!(
            value["options"]["terrains"]
                .as_array()
                .expect("terrains")
                .len(),
            polish::terrain_names().len()
        );
    }

    #[test]
    fn idle_rotation_cannot_invoke_hero_fetch_or_render() {
        // Regression for #39: the idle timer used to call paintHero() every
        // 3.2s (`setInterval(() => { ... paintHero(); rollCaption(); }, 3200)`),
        // which reassigned img.src to /api/backdrop or /api/card and forced a
        // full server-side re-render forever while a tab stayed open.
        let idle = js_function_body(PAGE, "idleTick")
            .expect("studio page must define idleTick for the rotation timer");
        assert!(
            !idle.contains("paintHero"),
            "idleTick must not call paintHero (would re-fetch hero PNG):\n{idle}"
        );
        assert!(
            !idle.contains("heroSrc") && !idle.contains("/api/"),
            "idleTick must not build or assign hero URLs:\n{idle}"
        );
        assert!(
            !idle.contains(".src"),
            "idleTick must not change an image src:\n{idle}"
        );
        assert!(
            idle.contains("rollCaption"),
            "idleTick should keep caption motion via rollCaption:\n{idle}"
        );
        assert!(
            idle.contains("document.hidden"),
            "idleTick must pause while the tab is hidden:\n{idle}"
        );
        assert!(
            PAGE.contains("prefers-reduced-motion"),
            "page must honor prefers-reduced-motion for idle motion"
        );
        assert!(
            PAGE.contains("setInterval(idleTick"),
            "idle timer must call idleTick, not an inline paintHero body"
        );
        assert!(
            !PAGE.contains("paintHero(); rollCaption()"),
            "old setInterval body must not call paintHero then rollCaption"
        );
    }

    /// Brace-matched body of `function name(...) { ... }` in embedded JS.
    fn js_function_body<'a>(source: &'a str, name: &str) -> Option<&'a str> {
        let marker = format!("function {name}");
        let start = source.find(&marker)?;
        let after = &source[start..];
        let open = after.find('{')?;
        let bytes = after.as_bytes();
        let mut depth = 0usize;
        for (i, &b) in bytes.iter().enumerate().skip(open) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&after[open..=i]);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

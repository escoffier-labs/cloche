use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use schemars::schema_for;
use serde::Serialize;

use crate::backends;
use crate::captures::CaptureSummary;
use crate::captures::find_captures;
use crate::captures::read_metadata;
use crate::captures::read_metadata_file;
use crate::contract::AppshotResult;
use crate::contract::CaptureTarget;
use crate::contract::ImageDetail;
use crate::contract::ImageInfo;
use crate::contract::ReelRenderResult;
use crate::contract::VideoInfo;
use crate::polish;
use crate::reel_hyperframes;
use crate::text;
use crate::util;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(name = "cloche")]
#[command(version)]
#[command(about = "Open-source desktop capture CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Capture(CaptureArgs),
    Polish(PolishArgs),
    Reels(ReelsArgs),
    Doctor(DoctorArgs),
    ListWindows(ListWindowsArgs),
    Gallery(GalleryArgs),
    Latest(LatestArgs),
    #[command(alias = "open")]
    Preview(PreviewArgs),
    Schema(SchemaArgs),
    CodexPayload(crate::codex::CodexPayloadArgs),
    Mcp(crate::mcp::McpArgs),
    Setup(crate::setup::SetupArgs),
}

/// Style an existing image into a Cloche presentation card: rounded window,
/// layered shadows, and a vibrant gradient backdrop.
#[derive(Debug, Args)]
pub struct PolishArgs {
    /// Image to style (PNG, JPEG, or WebP).
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,
    /// Output card path; defaults to `<input>-card.png` next to the input.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Gradient palette; random when omitted.
    #[arg(long, value_parser = palette_name_parser())]
    pub palette: Option<String>,
    /// Seed for deterministic styling.
    #[arg(long)]
    pub style_seed: Option<u64>,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct ReelsArgs {
    #[command(subcommand)]
    pub command: ReelsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReelsCommand {
    Render(ReelRenderArgs),
}

#[derive(Debug, Args)]
pub struct ReelRenderArgs {
    /// Raw source video to place inside the Cloche reel template.
    #[arg(long)]
    pub input: PathBuf,
    /// Rendered MP4 output path.
    #[arg(long)]
    pub out: PathBuf,
    /// Optional timeline/cue JSON, compatible with the AppReels cue shape.
    #[arg(long)]
    pub cues: Option<PathBuf>,
    /// Title shown on the opening card and in the template metadata.
    #[arg(long)]
    pub title: Option<String>,
    /// Total output duration in milliseconds. Defaults to the longest cue or 6s.
    #[arg(long)]
    pub duration_ms: Option<u64>,
    /// Frames per second.
    #[arg(long, default_value_t = 30)]
    pub fps: u32,
    /// Output width.
    #[arg(long, default_value_t = 1080)]
    pub width: u32,
    /// Output height.
    #[arg(long, default_value_t = 1920)]
    pub height: u32,
    /// Render engine: `remotion` (vendored node project) or `hyperframes`
    /// (HTML composition rendered via `npx hyperframes`).
    #[arg(long, value_enum, default_value = "remotion")]
    pub engine: ReelRenderEngine,
    /// Cloche palette for the reel brand (hyperframes engine). Defaults to the
    /// seed's random pick. Use the same name as `cloche capture --style-seed`
    /// to match a still card.
    #[arg(long, value_parser = palette_name_parser())]
    pub palette: Option<String>,
    /// Style seed for the reel brand (hyperframes engine). Same seed + palette
    /// reproduces the same card identity.
    #[arg(long)]
    pub style_seed: Option<u64>,
    /// Parallel render workers (hyperframes engine). Defaults to 1: some
    /// environments corrupt frames during parallel capture, which fails the
    /// ffmpeg encode. Raise it for faster renders if your setup is stable.
    #[arg(long, default_value_t = 1)]
    pub workers: u32,
    /// Keep the generated props/composition project next to the output for
    /// debugging (Remotion: props JSON; HyperFrames: the staged project dir).
    #[arg(long)]
    pub keep_props: bool,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ReelRenderEngine {
    Remotion,
    Hyperframes,
}

impl ReelRenderEngine {
    fn name(self) -> &'static str {
        match self {
            ReelRenderEngine::Remotion => "remotion",
            ReelRenderEngine::Hyperframes => "hyperframes",
        }
    }
}

fn palette_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::palette_names())
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long, value_enum, default_value = "active")]
    pub target: CaptureTarget,
    /// Directory to write the capture's flat files into (`<stem>.png` card,
    /// `<stem>.raw.png`, `<stem>.json`, `<stem>.txt`). Defaults to the central
    /// gallery dir (~/Pictures/Cloche).
    #[arg(long)]
    pub out_dir: Option<PathBuf>,
    #[arg(long)]
    pub window_id: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub app: Option<String>,
    #[arg(long, value_enum, default_value = "high")]
    pub detail: ImageDetail,
    #[arg(long, value_enum, default_value = "both")]
    pub presentation: PresentationMode,
    #[arg(long)]
    pub style_seed: Option<u64>,
    /// Copy the card (or the raw shot with --presentation raw) to the
    /// clipboard after capture.
    #[arg(long)]
    pub clipboard: bool,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct ListWindowsArgs {
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct GalleryArgs {
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(long)]
    pub root: Vec<PathBuf>,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
    /// Write a self-contained HTML gallery to this path.
    #[arg(long)]
    pub html: Option<PathBuf>,
    /// HTML page title (used with --html).
    #[arg(long, default_value = "Cloche Shots")]
    pub title: String,
    /// Open the exported HTML gallery after writing it (requires --html).
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct LatestArgs {
    #[arg(long)]
    pub root: Vec<PathBuf>,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct PreviewArgs {
    #[arg(value_name = "CAPTURE_DIR")]
    pub capture_dir: Option<PathBuf>,
    #[arg(long)]
    pub raw: bool,
    #[arg(long)]
    pub root: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SchemaArgs {
    /// Which JSON contract to print.
    #[arg(long = "for", value_enum, default_value = "capture")]
    pub contract: SchemaTarget,
    #[arg(long)]
    pub compact: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SchemaTarget {
    Capture,
    Polish,
    ReelRender,
}

/// Accepted for forward compatibility; every command prints JSON today, so
/// the parsed value is not consulted anywhere yet.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum PresentationMode {
    Raw,
    Card,
    Both,
}

fn postprocess_capture<CopyPng, ExtractText>(
    clipboard: bool,
    image_path: Option<&Path>,
    text_path: &Path,
    warnings: &mut Vec<String>,
    mut copy_png: CopyPng,
    mut extract_text: ExtractText,
) -> crate::contract::TextInfo
where
    CopyPng: FnMut(&Path) -> Result<(), String>,
    ExtractText: FnMut(&Path, &mut Vec<String>) -> crate::contract::TextInfo,
{
    if clipboard {
        match image_path {
            Some(path) => {
                if let Err(err) = copy_png(path) {
                    warnings.push(format!("Clipboard copy failed: {err}"));
                }
            }
            None => warnings.push("Clipboard copy skipped: no image was captured.".to_string()),
        }
    }

    match image_path {
        Some(_) => extract_text(text_path, warnings),
        None => Default::default(),
    }
}

pub fn capture(args: CaptureArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    // Flat layout: all artifacts for one shot share a timestamp stem and sit
    // directly in the gallery dir, instead of a folder-per-shot with fixed
    // filenames. `<stem>.png` is the shareable card (or the raw shot when no
    // card is made), `<stem>.raw.png` the raw, `<stem>.json`/`.txt` the sidecars.
    let output_dir = args.out_dir.unwrap_or_else(backends::default_gallery_dir);
    let stem = backends::shot_stem();
    let will_make_card = matches!(
        args.presentation,
        PresentationMode::Card | PresentationMode::Both
    );
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut backend = None;
    let mut window = None;
    let mut image = None;
    let mut presentation_image = None;
    let mut presentation_style = None;

    if let Err(err) = util::create_dir_all(&output_dir) {
        errors.push(err.to_string());
    } else {
        // Raw goes to `<stem>.raw.png` when a card will own `<stem>.png`;
        // otherwise the raw itself is the primary `<stem>.png`.
        let raw_name = if will_make_card {
            format!("{stem}.raw.png")
        } else {
            format!("{stem}.png")
        };
        let image_path = output_dir.join(raw_name);
        match backends::capture(backends::CaptureRequest {
            target: args.target,
            output_path: &image_path,
            window_id: args.window_id.as_deref(),
            title: args.title.as_deref(),
            app: args.app.as_deref(),
        }) {
            Ok(success) => {
                backend = Some(success.backend);
                let frame_extents = success
                    .window
                    .as_ref()
                    .and_then(backends::frame_extents_for_window);
                window = success.window;
                match image_info(&image_path, args.detail) {
                    Ok(info) => {
                        if will_make_card {
                            let card_path = output_dir.join(format!("{stem}.png"));
                            let style = args
                                .style_seed
                                .map(polish::style_from_seed)
                                .unwrap_or_else(polish::random_style);
                            match polish::render_codex_card(
                                &image_path,
                                &card_path,
                                frame_extents,
                                &style,
                            )
                            .and_then(|()| image_info(&card_path, args.detail))
                            {
                                Ok(card_info) => {
                                    presentation_style = Some(style.info());
                                    presentation_image = Some(card_info);
                                }
                                Err(err) => warnings.push(format!(
                                    "Codex-style presentation image could not be created: {err}"
                                )),
                            }
                        }
                        image = Some(info);
                    }
                    Err(err) => errors.push(err.to_string()),
                }
            }
            Err(err) => errors.push(err.to_string()),
        }
    }

    let clipboard_image = presentation_image
        .as_ref()
        .or(image.as_ref())
        .map(|info| info.path.as_path());
    let text = postprocess_capture(
        args.clipboard,
        clipboard_image,
        &output_dir.join(format!("{stem}.txt")),
        &mut warnings,
        crate::clipboard::copy_png,
        text::extract,
    );

    let result = AppshotResult {
        ok: image.is_some() && errors.is_empty(),
        version: VERSION.to_string(),
        created_at: Utc::now(),
        target: args.target,
        backend,
        output_dir: util::canonical_or_original(&output_dir),
        image,
        presentation_image,
        presentation_style,
        window,
        text,
        warnings,
        errors,
    };

    if let Ok(metadata) = serde_json::to_vec_pretty(&result) {
        let _ = util::write(&output_dir.join(format!("{stem}.json")), metadata);
    }
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub fn polish(args: PolishArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let result = run_polish(args);
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub fn reels(args: ReelsArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match args.command {
        ReelsCommand::Render(args) => reels_render(args),
    }
}

pub fn reels_render(args: ReelRenderArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let result = run_reels_render(args);
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn run_reels_render(args: ReelRenderArgs) -> ReelRenderResult {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut props_path = None;
    let mut output = None;

    if !args.input.exists() {
        errors.push(format!(
            "input video does not exist: {}",
            args.input.display()
        ));
    }
    if let Some(parent) = args
        .out
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        && let Err(err) = util::create_dir_all(parent)
    {
        errors.push(err.to_string());
    }

    let cues = match args.cues.as_ref() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(value) => Some(value),
                Err(err) => {
                    errors.push(format!("cue file is not valid JSON: {err}"));
                    None
                }
            },
            Err(err) => {
                errors.push(format!("cue file could not be read: {err}"));
                None
            }
        },
        None => None,
    };
    let duration_ms = args
        .duration_ms
        .unwrap_or_else(|| inferred_reel_duration_ms(cues.as_ref()));

    if errors.is_empty() {
        let cues = cues.unwrap_or_else(|| serde_json::json!({}));
        match args.engine {
            ReelRenderEngine::Remotion => render_reel_with_remotion(
                &args,
                cues,
                duration_ms,
                &mut errors,
                &mut props_path,
                &mut output,
            ),
            ReelRenderEngine::Hyperframes => {
                let style = resolve_reel_style(args.style_seed, args.palette.as_deref());
                let title = args
                    .title
                    .clone()
                    .unwrap_or_else(|| "Cloche Reel".to_string());
                reel_hyperframes::render(
                    &args.input,
                    &args.out,
                    &cues,
                    duration_ms,
                    args.fps,
                    args.width,
                    args.height,
                    args.workers,
                    &style,
                    &title,
                    args.keep_props,
                    &mut errors,
                    &mut props_path,
                    &mut output,
                );
            }
        }
    }

    if output.is_none() && props_path.is_some() {
        warnings.push("render props were written for debugging".to_string());
    }

    ReelRenderResult {
        ok: output.is_some() && errors.is_empty(),
        version: VERSION.to_string(),
        created_at: Utc::now(),
        engine: args.engine.name().to_string(),
        input: util::canonical_or_original(&args.input),
        output,
        props: props_path,
        duration_ms,
        warnings,
        errors,
    }
}

/// Resolve the reel brand the same way `cloche capture` / `polish` pick a card
/// style, so a reel can be pinned to the exact palette of a still shot-card.
fn resolve_reel_style(seed: Option<u64>, palette: Option<&str>) -> polish::PresentationStyle {
    match (seed, palette) {
        (Some(seed), Some(name)) => {
            polish::style_with_palette(seed, name).unwrap_or_else(|| polish::style_from_seed(seed))
        }
        (Some(seed), None) => polish::style_from_seed(seed),
        (None, Some(name)) => polish::style_with_palette(polish::random_seed(), name)
            .unwrap_or_else(polish::random_style),
        (None, None) => polish::random_style(),
    }
}

fn render_reel_with_remotion(
    args: &ReelRenderArgs,
    cues: serde_json::Value,
    duration_ms: u64,
    errors: &mut Vec<String>,
    props_path: &mut Option<PathBuf>,
    output: &mut Option<VideoInfo>,
) {
    let remotion_dir = remotion_template_dir();
    if !remotion_dir.join("package.json").exists() {
        errors.push(format!(
            "Remotion template package is missing: {}",
            remotion_dir.display()
        ));
        return;
    }

    let staged_input = match stage_remotion_video_asset(&remotion_dir, &args.input, errors) {
        Some(path) => path,
        None => return,
    };
    let props = serde_json::json!({
        "inputVideo": staged_input.clone(),
        "title": args.title.clone().unwrap_or_else(|| "Cloche Reel".to_string()),
        "durationMs": duration_ms,
        "fps": args.fps,
        "width": args.width,
        "height": args.height,
        "cues": cues,
    });
    let path = default_reel_props_path(&args.out);
    match serde_json::to_vec_pretty(&props)
        .map_err(|err| err.to_string())
        .and_then(|bytes| util::write(&path, bytes).map_err(|err| err.to_string()))
    {
        Ok(()) => {
            *props_path = Some(util::canonical_or_original(&path));
            let remotion_dir_arg = remotion_dir.display().to_string();
            // Remotion runs with current_dir set to the template dir, so it
            // resolves relative paths against that dir. Pass absolute paths so a
            // relative --out (and its props sidecar) land where the user expects.
            let output_arg = std::path::absolute(&args.out)
                .unwrap_or_else(|_| args.out.clone())
                .display()
                .to_string();
            let props_arg = std::path::absolute(&path)
                .unwrap_or_else(|_| path.clone())
                .display()
                .to_string();
            let render = std::process::Command::new("npm")
                .current_dir(&remotion_dir_arg)
                .args([
                    "exec",
                    "--",
                    "remotion",
                    "render",
                    "src/index.ts",
                    "ClocheReel",
                    &output_arg,
                    "--props",
                    &props_arg,
                    "--log",
                    "error",
                    "--overwrite",
                ])
                .output();
            match render {
                Ok(result) if result.status.success() => {
                    if !args.keep_props {
                        let _ = std::fs::remove_file(&path);
                        *props_path = None;
                    }
                    match util::file_size(&args.out) {
                        Ok(bytes) => {
                            *output = Some(VideoInfo {
                                path: util::canonical_or_original(&args.out),
                                bytes,
                                mime: "video/mp4".to_string(),
                            });
                        }
                        Err(err) => errors.push(err.to_string()),
                    }
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    errors.push(format!(
                        "Remotion render failed with status {}: {}{}",
                        result.status,
                        stderr.trim(),
                        stdout.trim()
                    ));
                }
                Err(err) => errors.push(format!("failed to launch npm/remotion: {err}")),
            }
        }
        Err(err) => errors.push(err),
    }

    // The staged input video is a large intermediate copy under remotion/public,
    // useful only as Remotion's render input. Always remove it after the render
    // attempt (success or failure) so renders never leak a full copy of the
    // source MP4. (keep_props only governs the small props JSON.)
    let _ = std::fs::remove_file(remotion_dir.join("public").join(&staged_input));
}

fn remotion_template_dir() -> PathBuf {
    // 1. Explicit override for packagers / unusual layouts.
    match std::env::var("CLOCHE_REMOTION_DIR") {
        Ok(dir) if !dir.is_empty() => return PathBuf::from(dir),
        _ => {}
    }
    // 2. Alongside an installed binary: the template ships in the crate, so for
    //    `cargo install` / release archives it sits relative to the executable,
    //    not the build machine's source tree.
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        for candidate in [dir.join("remotion"), dir.join("../share/cloche/remotion")] {
            if candidate.join("package.json").exists() {
                return candidate;
            }
        }
    }
    // 3. Dev fallback: the source tree this binary was built from.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("remotion")
}

fn default_reel_props_path(out: &Path) -> PathBuf {
    out.with_extension("remotion-props.json")
}

fn stage_remotion_video_asset(
    remotion_dir: &Path,
    input: &Path,
    errors: &mut Vec<String>,
) -> Option<String> {
    let asset_dir = remotion_dir.join("public").join("cloche-inputs");
    if let Err(err) = util::create_dir_all(&asset_dir) {
        errors.push(err.to_string());
        return None;
    }
    let file_name = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("input.mp4");
    let staged_name = format!("{}-{file_name}", std::process::id());
    let staged_path = asset_dir.join(&staged_name);
    match std::fs::copy(input, &staged_path) {
        Ok(_) => Some(format!("cloche-inputs/{staged_name}")),
        Err(err) => {
            errors.push(format!(
                "input video could not be staged for Remotion: {err}"
            ));
            None
        }
    }
}

fn inferred_reel_duration_ms(cues: Option<&serde_json::Value>) -> u64 {
    let mut max_end = 0;
    if let Some(cues) = cues {
        for key in ["captions", "zooms"] {
            if let Some(items) = cues.get(key).and_then(|value| value.as_array()) {
                for item in items {
                    let end = item
                        .get("endMs")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0);
                    max_end = max_end.max(end);
                }
            }
        }
        for key in ["titleCard", "outroCard"] {
            if let Some(ms) = cues
                .get(key)
                .and_then(|value| value.get("ms"))
                .and_then(|value| value.as_u64())
            {
                max_end = max_end.max(ms);
            }
        }
    }
    max_end.max(6_000)
}

fn run_polish(args: PolishArgs) -> crate::contract::PolishResult {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut input_info = None;
    let mut card_info = None;
    let mut style_info = None;

    let card_path = args
        .out
        .clone()
        .unwrap_or_else(|| default_card_path(&args.input));
    if card_path.extension().and_then(|ext| ext.to_str()) != Some("png") {
        errors.push(format!(
            "output path {} must end in .png; cards are always PNG",
            card_path.display()
        ));
    } else {
        let seed = args.style_seed.unwrap_or_else(polish::random_seed);
        // The palette value is pre-validated by clap, so a miss here is a bug.
        let style = match args.palette.as_deref() {
            Some(name) => polish::style_with_palette(seed, name)
                .ok_or_else(|| format!("unknown palette: {name}")),
            None => Ok(polish::style_from_seed(seed)),
        };
        match style {
            Ok(style) => {
                let parent_ready = card_path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .map_or(Ok(()), util::create_dir_all);
                match parent_ready.map_err(|err| err.to_string()).and_then(|()| {
                    polish::render_codex_card(&args.input, &card_path, None, &style)
                        .map_err(|err| err.to_string())
                }) {
                    Ok(()) => {
                        style_info = Some(style.info());
                        match image_info(&args.input, ImageDetail::Original) {
                            Ok(info) => input_info = Some(info),
                            Err(err) => warnings.push(err.to_string()),
                        }
                        match image_info(&card_path, ImageDetail::Original) {
                            Ok(info) => card_info = Some(info),
                            Err(err) => errors.push(err.to_string()),
                        }
                    }
                    Err(err) => errors.push(err),
                }
            }
            Err(err) => errors.push(err),
        }
    }

    crate::contract::PolishResult {
        ok: card_info.is_some() && errors.is_empty(),
        version: VERSION.to_string(),
        created_at: Utc::now(),
        input: input_info,
        card: card_info,
        presentation_style: style_info,
        warnings,
        errors,
    }
}

/// Sibling path with a `-card.png` suffix: `shot.png` -> `shot-card.png`.
fn default_card_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("shot");
    input.with_file_name(format!("{stem}-card.png"))
}

pub fn doctor(_args: DoctorArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let report = backends::doctor_report();
    print_json(&report)?;
    Ok(if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub fn list_windows(_args: ListWindowsArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let result = backends::list_windows();
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub fn gallery(args: GalleryArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let captures = find_captures(args.root, args.limit);
    let mut html_path = None;
    if let Some(path) = args.html.as_ref() {
        let html = render_gallery_html(&args.title, &captures);
        util::write(path, html)?;
        let written = util::canonical_or_original(path);
        if args.open {
            open_path(&written)?;
        }
        html_path = Some(written);
    }
    print_json(&GalleryOutput {
        ok: true,
        html_path,
        captures,
    })?;
    Ok(ExitCode::SUCCESS)
}

fn render_gallery_html(title: &str, captures: &[CaptureSummary]) -> String {
    let items: Vec<crate::html::GalleryItem> = captures
        .iter()
        .map(|capture| {
            let image = capture
                .presentation_image
                .as_ref()
                .or(capture.image.as_ref());
            let window_title = capture
                .window
                .as_ref()
                .and_then(|window| window.title.clone().or_else(|| window.app_name.clone()))
                .unwrap_or_else(|| "Untitled capture".to_string());
            crate::html::GalleryItem {
                title: window_title,
                app: capture
                    .window
                    .as_ref()
                    .and_then(|window| window.app_name.clone()),
                target: format!("{:?}", capture.target).to_lowercase(),
                width: image.and_then(|info| info.width),
                height: image.and_then(|info| info.height),
                created_at: capture.created_at.to_rfc3339(),
                output_dir: capture.output_dir.display().to_string(),
                image_path: image.map(|info| info.path.as_path()),
            }
        })
        .collect();
    crate::html::render(title, &items)
}

pub fn latest(args: LatestArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let capture = find_captures(args.root, 1).into_iter().next();
    let ok = capture.is_some();
    print_json(&LatestOutput { ok, capture })?;
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub fn preview(args: PreviewArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let path = match args.capture_dir {
        Some(target) => resolve_preview_path(&target, args.raw)?,
        None => {
            let capture = find_captures(args.root, 1)
                .into_iter()
                .next()
                .ok_or("no Cloche captures found")?;
            pick_preview_image(
                args.raw,
                capture.image.as_ref(),
                capture.presentation_image.as_ref(),
            )?
        }
    };
    open_path(&path)?;
    Ok(ExitCode::SUCCESS)
}

/// Resolve an explicit preview target to an image path. Accepts a flat
/// `<stem>.json` sidecar, a legacy capture directory, or a direct image file.
fn resolve_preview_path(target: &Path, raw: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let metadata = if target.is_dir() {
        read_metadata(target)?
    } else if target.extension().and_then(|e| e.to_str()) == Some("json") {
        read_metadata_file(target)?
    } else {
        // A direct image path: open it as given.
        return Ok(target.to_path_buf());
    };
    pick_preview_image(
        raw,
        metadata.image.as_ref(),
        metadata.presentation_image.as_ref(),
    )
}

fn pick_preview_image(
    raw: bool,
    image: Option<&ImageInfo>,
    presentation: Option<&ImageInfo>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if raw {
        image
            .map(|info| info.path.clone())
            .ok_or_else(|| "capture does not include a raw image".into())
    } else {
        presentation
            .or(image)
            .map(|info| info.path.clone())
            .ok_or_else(|| "capture does not include an image".into())
    }
}

pub fn schema(args: SchemaArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let schema = schema_value(args.contract);
    if args.compact {
        println!("{}", serde_json::to_string(&schema)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }
    Ok(ExitCode::SUCCESS)
}

fn schema_value(target: SchemaTarget) -> serde_json::Value {
    let schema = match target {
        SchemaTarget::Capture => schema_for!(AppshotResult),
        SchemaTarget::Polish => schema_for!(crate::contract::PolishResult),
        SchemaTarget::ReelRender => schema_for!(crate::contract::ReelRenderResult),
    };
    serde_json::to_value(schema).expect("schema serializes")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GalleryOutput {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_path: Option<PathBuf>,
    captures: Vec<CaptureSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LatestOutput {
    ok: bool,
    capture: Option<CaptureSummary>,
}

fn open_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.display().to_string();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        if util::has_command("xdg-open") {
            util::desktop_command("xdg-open").arg(&path).spawn()?;
        } else if util::has_command("gio") {
            util::desktop_command("gio").args(["open", &path]).spawn()?;
        } else {
            return Err("no opener found: install xdg-open or gio".into());
        }
        Ok(())
    }
}

fn image_info(path: &std::path::Path, detail: ImageDetail) -> Result<ImageInfo, util::AppError> {
    let bytes = util::file_size(path)?;
    let (width, height) = util::png_dimensions(path)
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or((None, None));
    Ok(ImageInfo {
        path: util::canonical_or_original(path),
        width,
        height,
        bytes,
        mime: "image/png".to_string(),
        detail,
    })
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cloche-polish-test-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write_test_image(path: &Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([180, 180, 180, 255]));
        image.save(path).expect("write test image");
    }

    #[test]
    fn capture_publishes_clipboard_before_best_effort_text_extraction() {
        use std::cell::RefCell;

        let steps = RefCell::new(Vec::new());
        let mut warnings = Vec::new();
        let text = postprocess_capture(
            true,
            Some(Path::new("/tmp/shot.png")),
            Path::new("/tmp/shot.txt"),
            &mut warnings,
            |_| {
                steps.borrow_mut().push("clipboard");
                Ok(())
            },
            |_, _| {
                steps.borrow_mut().push("text");
                Default::default()
            },
        );

        assert_eq!(steps.into_inner(), vec!["clipboard", "text"]);
        assert!(!text.available);
        assert!(warnings.is_empty());
    }

    #[test]
    fn default_card_path_appends_card_suffix() {
        assert_eq!(
            default_card_path(Path::new("/tmp/example/shot.png")),
            PathBuf::from("/tmp/example/shot-card.png")
        );
        assert_eq!(
            default_card_path(Path::new("diff.jpg")),
            PathBuf::from("diff-card.png")
        );
        assert_eq!(
            default_card_path(Path::new("/tmp/noext")),
            PathBuf::from("/tmp/noext-card.png")
        );
    }

    #[test]
    fn polish_writes_card_next_to_input() {
        let dir = temp_dir("default-out");
        let input = dir.join("shot.png");
        write_test_image(&input, 320, 240);
        let result = run_polish(PolishArgs {
            input: input.clone(),
            out: None,
            palette: None,
            style_seed: Some(7),
            format: OutputFormat::Json,
        });
        assert!(result.ok, "errors: {:?}", result.errors);
        let card = result.card.expect("card info");
        assert_eq!(
            card.path,
            util::canonical_or_original(&dir.join("shot-card.png"))
        );
        assert!(card.path.exists());
        // The card adds padding around the input pixels.
        assert!(card.width.expect("width") > 320);
        assert!(card.height.expect("height") > 240);
        let style = result.presentation_style.expect("style info");
        assert_eq!(style.seed, 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_honors_palette_and_out_path() {
        let dir = temp_dir("palette-out");
        let input = dir.join("input.png");
        write_test_image(&input, 200, 160);
        let out = dir.join("nested").join("styled.png");
        let result = run_polish(PolishArgs {
            input,
            out: Some(out.clone()),
            palette: Some("aurora-teal".to_string()),
            style_seed: Some(11),
            format: OutputFormat::Json,
        });
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(
            result.presentation_style.expect("style").palette,
            "aurora-teal"
        );
        assert!(out.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_rejects_non_png_output() {
        let dir = temp_dir("bad-out");
        let input = dir.join("input.png");
        write_test_image(&input, 64, 64);
        let result = run_polish(PolishArgs {
            input,
            out: Some(dir.join("card.jpg")),
            palette: None,
            style_seed: Some(3),
            format: OutputFormat::Json,
        });
        assert!(!result.ok);
        assert!(result.errors.iter().any(|err| err.contains(".png")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_for_capture_describes_the_capture_contract() {
        let value = schema_value(SchemaTarget::Capture);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("outputDir"));
        assert!(properties.contains_key("backend"));
        assert!(!properties.contains_key("card"));
    }

    #[test]
    fn schema_for_polish_describes_the_polish_contract() {
        let value = schema_value(SchemaTarget::Polish);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("card"));
        assert!(properties.contains_key("presentationStyle"));
        assert!(!properties.contains_key("backend"));
    }

    #[test]
    fn schema_for_reel_render_describes_the_reel_contract() {
        let value = schema_value(SchemaTarget::ReelRender);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("engine"));
        assert!(properties.contains_key("durationMs"));
        assert!(properties.contains_key("output"));
    }

    #[test]
    fn reel_duration_uses_longest_cue_with_default_floor() {
        let cues = serde_json::json!({
            "captions": [{ "startMs": 100, "endMs": 2_400, "text": "Caption" }],
            "zooms": [{ "startMs": 1_000, "endMs": 7_200, "scale": 1.1 }]
        });

        assert_eq!(inferred_reel_duration_ms(Some(&cues)), 7_200);
        assert_eq!(inferred_reel_duration_ms(None), 6_000);
    }

    #[test]
    fn polish_accepts_jpeg_input() {
        let dir = temp_dir("jpeg-input");
        let input = dir.join("photo.jpg");
        let image = image::RgbImage::from_pixel(96, 64, image::Rgb([120, 60, 30]));
        image.save(&input).expect("write jpeg");
        let result = run_polish(PolishArgs {
            input,
            out: None,
            palette: None,
            style_seed: Some(13),
            format: OutputFormat::Json,
        });
        assert!(result.ok, "errors: {:?}", result.errors);
        assert!(dir.join("photo-card.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_reports_missing_input() {
        let dir = temp_dir("missing-input");
        let result = run_polish(PolishArgs {
            input: dir.join("does-not-exist.png"),
            out: None,
            palette: None,
            style_seed: Some(5),
            format: OutputFormat::Json,
        });
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
        assert!(result.card.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

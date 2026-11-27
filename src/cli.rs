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
use crate::config;
use crate::contract::AppshotResult;
use crate::contract::CaptureTarget;
use crate::contract::ImageDetail;
use crate::contract::ImageInfo;
use crate::polish;
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
    Doctor(DoctorArgs),
    ListWindows(ListWindowsArgs),
    Gallery(GalleryArgs),
    Latest(LatestArgs),
    #[command(alias = "open")]
    Preview(PreviewArgs),
    Schema(SchemaArgs),
    Config(ConfigArgs),
    CodexPayload(crate::codex::CodexPayloadArgs),
    Mcp(crate::mcp::McpArgs),
    Setup(crate::setup::SetupArgs),
    Studio(crate::studio::StudioArgs),
    #[cfg(target_os = "linux")]
    #[command(hide = true)]
    ClipboardServe {
        #[arg(value_name = "PNG_PATH")]
        png_path: PathBuf,
    },
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
    /// Backdrop palette (gradient or deep-space scene); random when omitted.
    #[arg(long, value_parser = palette_name_parser())]
    pub palette: Option<String>,
    /// Pin the deep-space scene look (e.g. `jwst`, `alma`, `cmb`); random when
    /// omitted. Only applies to space palettes.
    #[arg(long, value_parser = scene_name_parser())]
    pub scene: Option<String>,
    /// Pin the geometric motif (e.g. `plaid`, `stripe`, `grid`); random when
    /// omitted. Only applies to pattern palettes.
    #[arg(long, value_parser = motif_name_parser())]
    pub motif: Option<String>,
    /// Pin the sky condition (e.g. `bolt`, `mammatus`, `aurora`); random when
    /// omitted. Only applies to sky palettes.
    #[arg(long, value_parser = sky_name_parser())]
    pub sky: Option<String>,
    /// Pin the terrain kind (e.g. `dunes`, `mesa`, `badlands`, `glacier`); random when
    /// omitted. Only applies to terrain palettes.
    #[arg(long, value_parser = terrain_name_parser())]
    pub terrain: Option<String>,
    /// Seed for deterministic styling.
    #[arg(long)]
    pub style_seed: Option<u64>,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

fn palette_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::palette_names())
}

fn scene_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::scene_names())
}

fn motif_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::motif_names())
}

fn sky_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::sky_names())
}

fn terrain_name_parser() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(polish::terrain_names())
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
    /// Backdrop palette for the card; falls back to the persisted config, then
    /// to a random pick. See `cloche config options` for the full list.
    #[arg(long, value_parser = palette_name_parser())]
    pub palette: Option<String>,
    /// Pin the deep-space scene look (e.g. `jwst`, `alma`, `cmb`). Only applies
    /// to space palettes.
    #[arg(long, value_parser = scene_name_parser())]
    pub scene: Option<String>,
    /// Pin the geometric motif (e.g. `plaid`, `stripe`, `grid`). Only applies
    /// to pattern palettes.
    #[arg(long, value_parser = motif_name_parser())]
    pub motif: Option<String>,
    /// Pin the sky condition (e.g. `bolt`, `mammatus`, `aurora`). Only applies
    /// to sky palettes.
    #[arg(long, value_parser = sky_name_parser())]
    pub sky: Option<String>,
    /// Pin the terrain kind (e.g. `dunes`, `mesa`, `badlands`, `glacier`). Only applies to
    /// terrain palettes.
    #[arg(long, value_parser = terrain_name_parser())]
    pub terrain: Option<String>,
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
    Config,
}

/// Read and edit the persisted styling preferences that `capture` and `polish`
/// honor without flags.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the effective preferences and the file they came from.
    Show,
    /// Print every palette and scene the pickers accept.
    Options,
    /// Update the persisted preferences.
    ///
    /// Boxed because the args struct carries one field per styling axis and
    /// dwarfs the unit variants, which trips `clippy::large_enum_variant`.
    Set(Box<ConfigSetArgs>),
}

#[derive(Debug, Args)]
pub struct ConfigSetArgs {
    /// `random` draws from the pools below; `pinned` applies `--palette` and
    /// `--scene` to every capture.
    #[arg(long, value_enum)]
    pub mode: Option<ConfigPolishMode>,
    /// Palette to pin. Stored either way, applied only in `pinned` mode.
    #[arg(long, value_parser = palette_name_parser())]
    pub palette: Option<String>,
    /// Scene to pin. Space palettes only.
    #[arg(long, value_parser = scene_name_parser())]
    pub scene: Option<String>,
    /// Motif to pin. Pattern palettes only.
    #[arg(long, value_parser = motif_name_parser())]
    pub motif: Option<String>,
    /// Sky to pin. Sky palettes only.
    #[arg(long, value_parser = sky_name_parser())]
    pub sky: Option<String>,
    /// Terrain to pin. Terrain palettes only.
    #[arg(long, value_parser = terrain_name_parser())]
    pub terrain: Option<String>,
    /// Comma-separated palettes the random picker may choose from. Replaces the
    /// stored pool.
    #[arg(long, value_delimiter = ',', value_parser = palette_name_parser())]
    pub palettes: Option<Vec<String>>,
    /// Comma-separated scenes the random picker may choose from. Replaces the
    /// stored pool.
    #[arg(long, value_delimiter = ',', value_parser = scene_name_parser())]
    pub scenes: Option<Vec<String>>,
    /// Comma-separated motifs the random picker may choose from. Replaces the
    /// stored pool.
    #[arg(long, value_delimiter = ',', value_parser = motif_name_parser())]
    pub motifs: Option<Vec<String>>,
    /// Comma-separated skies the random picker may choose from. Replaces the
    /// stored pool.
    #[arg(long, value_delimiter = ',', value_parser = sky_name_parser())]
    pub skies: Option<Vec<String>>,
    /// Comma-separated terrains the random picker may choose from. Replaces
    /// the stored pool.
    #[arg(long, value_delimiter = ',', value_parser = terrain_name_parser())]
    pub terrains: Option<Vec<String>>,
    /// Reset a preference to its default. Repeatable.
    #[arg(long = "clear", value_enum)]
    pub clear: Vec<ClearTarget>,
    /// Output format; only `json` exists today.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ConfigPolishMode {
    Random,
    Pinned,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ClearTarget {
    /// Drop the pinned palette.
    Palette,
    /// Drop the pinned scene.
    Scene,
    /// Drop the pinned motif.
    Motif,
    /// Drop the pinned sky.
    Sky,
    /// Drop the pinned terrain.
    Terrain,
    /// Empty the random palette pool, restoring every space palette.
    Palettes,
    /// Empty the random scene pool.
    Scenes,
    /// Empty the random motif pool.
    Motifs,
    /// Empty the random sky pool.
    Skies,
    /// Empty the random terrain pool.
    Terrains,
    /// Reset every polish preference.
    All,
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
    // Persisted styling preferences, loaded before the shot so a bad config
    // surfaces as a warning on the capture that ignored it.
    let (stored, mut warnings) = config::load();
    let prefs = stored.polish;
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
                            let style = config::resolve_style(
                                &prefs,
                                args.style_seed,
                                &config::StylePins {
                                    palette: args.palette.as_deref(),
                                    scene: args.scene.as_deref(),
                                    motif: args.motif.as_deref(),
                                    sky: args.sky.as_deref(),
                                    terrain: args.terrain.as_deref(),
                                },
                                &mut warnings,
                            );
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
        out_dir: util::canonical_or_original(&output_dir),
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
    let result = run_polish(args, &config::path());
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// `config_path` is threaded in rather than resolved here so tests never depend
/// on the preferences file of whoever runs them.
fn run_polish(args: PolishArgs, config_path: &Path) -> crate::contract::PolishResult {
    let (stored, mut warnings) = config::load_from(config_path);
    let prefs = stored.polish;
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
        // Flag values are pre-validated by clap; the config's are not, so
        // `resolve_style` reports anything it had to ignore.
        let style = config::resolve_style(
            &prefs,
            args.style_seed,
            &config::StylePins {
                palette: args.palette.as_deref(),
                scene: args.scene.as_deref(),
                motif: args.motif.as_deref(),
                sky: args.sky.as_deref(),
                terrain: args.terrain.as_deref(),
            },
            &mut warnings,
        );
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
                output_dir: capture.out_dir.display().to_string(),
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
        SchemaTarget::Config => schema_for!(crate::contract::ConfigResult),
    };
    serde_json::to_value(schema).expect("schema serializes")
}

pub fn config(args: ConfigArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let result = run_config(args.command, &config::path());
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn run_config(command: ConfigCommand, path: &Path) -> crate::contract::ConfigResult {
    let (mut stored, mut warnings) = config::load_from(path);
    let mut errors = Vec::new();
    let mut options = None;

    match command {
        ConfigCommand::Show => {}
        ConfigCommand::Options => options = Some(style_options()),
        ConfigCommand::Set(args) => {
            apply_config_set(&mut stored.polish, &args);
            // A pinned scene on a gradient palette never renders, so say so at
            // the moment it is stored rather than on every later capture.
            if let Some(warning) = pinned_scene_conflict(&stored.polish) {
                warnings.push(warning);
            }
            if let Err(err) = config::save_to(path, &stored) {
                errors.push(err.to_string());
            }
        }
    }

    crate::contract::ConfigResult {
        ok: errors.is_empty(),
        version: VERSION.to_string(),
        created_at: Utc::now(),
        path: path.to_path_buf(),
        exists: path.exists(),
        config: stored,
        options,
        warnings,
        errors,
    }
}

fn apply_config_set(prefs: &mut config::PolishPrefs, args: &ConfigSetArgs) {
    // Clears run first so `--clear all --palette x` reads left to right: reset,
    // then apply what was named.
    for target in &args.clear {
        match target {
            ClearTarget::Palette => prefs.palette = None,
            ClearTarget::Scene => prefs.scene = None,
            ClearTarget::Motif => prefs.motif = None,
            ClearTarget::Sky => prefs.sky = None,
            ClearTarget::Terrain => prefs.terrain = None,
            ClearTarget::Palettes => prefs.palettes.clear(),
            ClearTarget::Scenes => prefs.scenes.clear(),
            ClearTarget::Motifs => prefs.motifs.clear(),
            ClearTarget::Skies => prefs.skies.clear(),
            ClearTarget::Terrains => prefs.terrains.clear(),
            ClearTarget::All => *prefs = config::PolishPrefs::default(),
        }
    }
    if let Some(mode) = args.mode {
        prefs.mode = match mode {
            ConfigPolishMode::Random => config::PolishMode::Random,
            ConfigPolishMode::Pinned => config::PolishMode::Pinned,
        };
    }
    if let Some(palette) = args.palette.clone() {
        prefs.palette = Some(palette);
    }
    if let Some(scene) = args.scene.clone() {
        prefs.scene = Some(scene);
    }
    if let Some(motif) = args.motif.clone() {
        prefs.motif = Some(motif);
    }
    if let Some(sky) = args.sky.clone() {
        prefs.sky = Some(sky);
    }
    if let Some(terrain) = args.terrain.clone() {
        prefs.terrain = Some(terrain);
    }
    if let Some(palettes) = args.palettes.clone() {
        prefs.palettes = palettes;
    }
    if let Some(scenes) = args.scenes.clone() {
        prefs.scenes = scenes;
    }
    if let Some(motifs) = args.motifs.clone() {
        prefs.motifs = motifs;
    }
    if let Some(skies) = args.skies.clone() {
        prefs.skies = skies;
    }
    if let Some(terrains) = args.terrains.clone() {
        prefs.terrains = terrains;
    }
}

/// A pinned scene only renders behind a space palette; pairing it with a
/// gradient silently drops the scene.
fn pinned_scene_conflict(prefs: &config::PolishPrefs) -> Option<String> {
    let scene = prefs.scene.as_deref()?;
    let palette = prefs.palette.as_deref()?;
    let is_space = polish::style_with_palette(0, palette)
        .map(|style| style.is_space())
        .unwrap_or(true);
    if is_space {
        return None;
    }
    Some(format!(
        "scene {scene} will be ignored while palette {palette} is pinned: it is not a space scene"
    ))
}

fn style_options() -> crate::contract::StyleOptions {
    crate::contract::StyleOptions {
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
    }
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
    use crate::contract::TextInfo;

    #[cfg(target_os = "linux")]
    #[test]
    fn clipboard_serve_subcommand_parses() {
        let cli = Cli::try_parse_from(["cloche", "clipboard-serve", "/tmp/card with spaces.png"])
            .expect("internal clipboard server command must parse");
        assert!(matches!(cli.command, Command::ClipboardServe { png_path }
            if png_path == Path::new("/tmp/card with spaces.png")));
        assert!(Cli::try_parse_from(["cloche", "clipboard-serve"]).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn clipboard_serve_subcommand_is_hidden_from_help() {
        use clap::CommandFactory;

        let help = Cli::command().render_long_help().to_string();
        assert!(!help.contains("clipboard-serve"));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cloche-polish-test-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// Polish against a config path that does not exist, so the run sees the
    /// built-in defaults regardless of what the developer has pinned.
    fn run_polish_defaults(args: PolishArgs) -> crate::contract::PolishResult {
        run_polish(args, Path::new("/nonexistent/cloche/config.json"))
    }

    fn config_set(args: ConfigSetArgs, path: &Path) -> crate::contract::ConfigResult {
        run_config(ConfigCommand::Set(Box::new(args)), path)
    }

    fn empty_set_args() -> ConfigSetArgs {
        ConfigSetArgs {
            mode: None,
            palette: None,
            scene: None,
            palettes: None,
            scenes: None,
            motif: None,
            motifs: None,
            sky: None,
            skies: None,
            terrain: None,
            terrains: None,
            clear: Vec::new(),
            format: OutputFormat::Json,
        }
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
        let result = run_polish_defaults(PolishArgs {
            input: input.clone(),
            out: None,
            palette: None,
            scene: None,
            motif: None,
            sky: None,
            terrain: None,
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
        let result = run_polish_defaults(PolishArgs {
            input,
            out: Some(out.clone()),
            palette: Some("aurora-teal".to_string()),
            scene: None,
            motif: None,
            sky: None,
            terrain: None,
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
    fn polish_uses_the_pinned_palette_from_the_config() {
        let dir = temp_dir("config-pin");
        let config_path = dir.join("config.json");
        config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("aurora-teal".to_string()),
                ..empty_set_args()
            },
            &config_path,
        );
        let input = dir.join("input.png");
        write_test_image(&input, 120, 90);
        let result = run_polish(
            PolishArgs {
                input,
                out: None,
                palette: None,
                scene: None,
                motif: None,
                sky: None,
                terrain: None,
                style_seed: Some(4),
                format: OutputFormat::Json,
            },
            &config_path,
        );
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(
            result.presentation_style.expect("style").palette,
            "aurora-teal"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_flag_overrides_the_pinned_config_palette() {
        let dir = temp_dir("config-override");
        let config_path = dir.join("config.json");
        config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("aurora-teal".to_string()),
                ..empty_set_args()
            },
            &config_path,
        );
        let input = dir.join("input.png");
        write_test_image(&input, 120, 90);
        let result = run_polish(
            PolishArgs {
                input,
                out: None,
                palette: Some("ember-glow".to_string()),
                scene: None,
                motif: None,
                sky: None,
                terrain: None,
                style_seed: Some(4),
                format: OutputFormat::Json,
            },
            &config_path,
        );
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(
            result.presentation_style.expect("style").palette,
            "ember-glow"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_show_reports_defaults_before_anything_is_written() {
        let dir = temp_dir("config-show");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Show, &path);
        assert!(result.ok, "errors: {:?}", result.errors);
        assert!(!result.exists);
        assert_eq!(result.config, config::ClocheConfig::default());
        assert!(result.options.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_persists_and_reloads() {
        let dir = temp_dir("config-set");
        let path = dir.join("config.json");
        let written = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palettes: Some(vec![
                    "orion-emission".to_string(),
                    "milkyway-core".to_string(),
                ]),
                scenes: Some(vec!["alma".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        assert!(written.ok, "errors: {:?}", written.errors);
        assert!(written.exists);

        let reloaded = run_config(ConfigCommand::Show, &path);
        assert_eq!(reloaded.config.polish.mode, config::PolishMode::Pinned);
        assert_eq!(
            reloaded.config.polish.palettes,
            vec!["orion-emission", "milkyway-core"]
        );
        assert_eq!(reloaded.config.polish.scenes, vec!["alma"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_clear_runs_before_the_values_on_the_same_line() {
        let dir = temp_dir("config-clear");
        let path = dir.join("config.json");
        config_set(
            ConfigSetArgs {
                palette: Some("aurora-teal".to_string()),
                scenes: Some(vec!["alma".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        let result = config_set(
            ConfigSetArgs {
                clear: vec![ClearTarget::All],
                palette: Some("ember-glow".to_string()),
                ..empty_set_args()
            },
            &path,
        );
        assert_eq!(result.config.polish.palette.as_deref(), Some("ember-glow"));
        assert!(result.config.polish.scenes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_warns_when_a_pinned_scene_cannot_render() {
        let dir = temp_dir("config-conflict");
        let path = dir.join("config.json");
        let result = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("aurora-teal".to_string()),
                scene: Some("jwst".to_string()),
                ..empty_set_args()
            },
            &path,
        );
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(result.warnings.len(), 1, "warnings: {:?}", result.warnings);
        assert!(result.warnings[0].contains("not a space scene"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_options_lists_every_palette_and_scene() {
        let dir = temp_dir("config-options");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Options, &path);
        let options = result.options.expect("options");
        assert_eq!(options.palettes.len(), polish::palette_names().len());
        assert_eq!(options.scenes, polish::scene_names());
        assert!(
            options
                .palettes
                .iter()
                .any(|option| option.name == "aurora-teal" && option.kind == "gradient")
        );
        assert!(
            options
                .palettes
                .iter()
                .any(|option| option.name == "orion-emission" && option.kind == "space")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_rejects_non_png_output() {
        let dir = temp_dir("bad-out");
        let input = dir.join("input.png");
        write_test_image(&input, 64, 64);
        let result = run_polish_defaults(PolishArgs {
            input,
            out: Some(dir.join("card.jpg")),
            palette: None,
            scene: None,
            motif: None,
            sky: None,
            terrain: None,
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
        assert!(properties.contains_key("outDir"));
        assert!(
            !properties.contains_key("outputDir"),
            "legacy outputDir must not appear in the published schema"
        );
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
    fn schema_for_config_describes_the_config_contract() {
        let value = schema_value(SchemaTarget::Config);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("config"));
        assert!(properties.contains_key("options"));
        assert!(properties.contains_key("path"));
    }

    #[test]
    fn polish_accepts_jpeg_input() {
        let dir = temp_dir("jpeg-input");
        let input = dir.join("photo.jpg");
        let image = image::RgbImage::from_pixel(96, 64, image::Rgb([120, 60, 30]));
        image.save(&input).expect("write jpeg");
        let result = run_polish_defaults(PolishArgs {
            input,
            out: None,
            palette: None,
            scene: None,
            motif: None,
            sky: None,
            terrain: None,
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
        let result = run_polish_defaults(PolishArgs {
            input: dir.join("does-not-exist.png"),
            out: None,
            palette: None,
            scene: None,
            motif: None,
            sky: None,
            terrain: None,
            style_seed: Some(5),
            format: OutputFormat::Json,
        });
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
        assert!(result.card.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn image_info_at(path: &Path) -> ImageInfo {
        ImageInfo {
            path: path.to_path_buf(),
            width: Some(4),
            height: Some(4),
            bytes: 1,
            mime: "image/png".to_string(),
            detail: ImageDetail::High,
        }
    }

    fn write_preview_metadata(
        metadata_path: &Path,
        output_dir: &Path,
        raw: &Path,
        card: Option<&Path>,
    ) {
        let metadata = AppshotResult {
            ok: true,
            version: "0.0.0".to_string(),
            created_at: chrono::Utc::now(),
            target: CaptureTarget::Active,
            backend: None,
            out_dir: output_dir.to_path_buf(),
            image: Some(image_info_at(raw)),
            presentation_image: card.map(image_info_at),
            presentation_style: None,
            window: None,
            text: TextInfo::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let bytes = serde_json::to_vec_pretty(&metadata).expect("serialize metadata");
        std::fs::write(metadata_path, bytes).expect("write metadata");
    }

    #[test]
    fn pick_preview_image_prefers_card_then_falls_back_to_raw() {
        let raw = image_info_at(Path::new("/tmp/shot.raw.png"));
        let card = image_info_at(Path::new("/tmp/shot.png"));
        assert_eq!(
            pick_preview_image(false, Some(&raw), Some(&card)).expect("card"),
            PathBuf::from("/tmp/shot.png")
        );
        assert_eq!(
            pick_preview_image(false, Some(&raw), None).expect("raw fallback"),
            PathBuf::from("/tmp/shot.raw.png")
        );
        assert_eq!(
            pick_preview_image(true, Some(&raw), Some(&card)).expect("raw flag"),
            PathBuf::from("/tmp/shot.raw.png")
        );
        let err = pick_preview_image(true, None, Some(&card)).expect_err("raw missing");
        assert!(err.to_string().contains("raw image"));
    }

    #[test]
    fn resolve_preview_path_accepts_legacy_dir_flat_sidecar_and_direct_image() {
        let dir = temp_dir("preview-resolve");
        let legacy = dir.join("legacy-shot");
        std::fs::create_dir_all(&legacy).expect("legacy dir");
        let legacy_raw = legacy.join("shot.png");
        let legacy_card = legacy.join("shot-card.png");
        std::fs::write(&legacy_raw, b"raw").expect("legacy raw");
        std::fs::write(&legacy_card, b"card").expect("legacy card");
        write_preview_metadata(
            &legacy.join("metadata.json"),
            &legacy,
            &legacy_raw,
            Some(&legacy_card),
        );

        let flat_dir = dir.join("flat-out");
        std::fs::create_dir_all(&flat_dir).expect("flat dir");
        let stem = "cloche-shot-20260725T120000Z-1-0";
        let flat_raw = flat_dir.join(format!("{stem}.raw.png"));
        let flat_card = flat_dir.join(format!("{stem}.png"));
        std::fs::write(&flat_raw, b"raw").expect("flat raw");
        std::fs::write(&flat_card, b"card").expect("flat card");
        write_preview_metadata(
            &flat_dir.join(format!("{stem}.json")),
            &flat_dir,
            &flat_raw,
            Some(&flat_card),
        );

        let direct = dir.join("loose.png");
        std::fs::write(&direct, b"png").expect("direct image");

        assert_eq!(
            resolve_preview_path(&legacy, false).expect("legacy card"),
            legacy_card
        );
        assert_eq!(
            resolve_preview_path(&legacy, true).expect("legacy raw"),
            legacy_raw
        );
        assert_eq!(
            resolve_preview_path(&flat_dir.join(format!("{stem}.json")), false).expect("flat card"),
            flat_card
        );
        assert_eq!(
            resolve_preview_path(&direct, false).expect("direct"),
            direct
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn polish_accepts_the_terrain_flag() {
        let dir = temp_dir("polish-terrain");
        let input = dir.join("shot.png");
        write_test_image(&input, 160, 120);
        let result = run_polish_defaults(PolishArgs {
            input,
            out: None,
            palette: Some("dunes".to_string()),
            scene: None,
            motif: None,
            sky: None,
            terrain: Some("dunes".to_string()),
            style_seed: Some(7),
            format: OutputFormat::Json,
        });
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(result.presentation_style.expect("style").palette, "dunes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_options_expose_the_terrain_names() {
        let dir = temp_dir("config-options-terrain");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Options, &path);
        let options = result.options.expect("options");
        assert_eq!(options.terrains, polish::terrain_names());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_persists_the_terrain_pin() {
        let dir = temp_dir("config-set-terrain");
        let path = dir.join("config.json");
        let written = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("dunes".to_string()),
                terrain: Some("dunes".to_string()),
                terrains: Some(vec!["dunes".to_string(), "mesa".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        assert!(written.ok, "errors: {:?}", written.errors);
        let reloaded = run_config(ConfigCommand::Show, &path);
        assert_eq!(reloaded.config.polish.terrain.as_deref(), Some("dunes"));
        assert_eq!(reloaded.config.polish.terrains, vec!["dunes", "mesa"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

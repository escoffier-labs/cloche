use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

use crate::commands;
use crate::contract::CaptureTarget;
use crate::contract::ImageDetail;
use crate::polish;

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

/// Route a parsed command line to its handler.
pub fn dispatch(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Capture(args) => commands::capture(args),
        Command::Polish(args) => commands::polish(args),
        Command::Doctor(args) => commands::doctor(args),
        Command::ListWindows(args) => commands::list_windows(args),
        Command::Gallery(args) => commands::gallery(args),
        Command::Latest(args) => commands::latest(args),
        Command::Preview(args) => commands::preview(args),
        Command::Schema(args) => commands::schema(args),
        Command::Config(args) => commands::config(args),
        Command::CodexPayload(args) => crate::codex::payload(args),
        Command::Mcp(args) => crate::mcp::run(args),
        Command::Setup(args) => crate::setup::run(args),
        Command::Studio(args) => crate::studio::run(args),
        #[cfg(target_os = "linux")]
        Command::ClipboardServe { png_path } => {
            crate::clipboard::serve_png(&png_path)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

// Every test here exercises the Linux-only `clipboard-serve` command.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn clipboard_serve_subcommand_parses() {
        let cli = Cli::try_parse_from(["cloche", "clipboard-serve", "/tmp/card with spaces.png"])
            .expect("internal clipboard server command must parse");
        assert!(matches!(cli.command, Command::ClipboardServe { png_path }
            if png_path == Path::new("/tmp/card with spaces.png")));
        assert!(Cli::try_parse_from(["cloche", "clipboard-serve"]).is_err());
    }

    #[test]
    fn clipboard_serve_subcommand_is_hidden_from_help() {
        use clap::CommandFactory;

        let help = Cli::command().render_long_help().to_string();
        assert!(!help.contains("clipboard-serve"));
    }
}

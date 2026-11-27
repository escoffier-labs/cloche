use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;

use crate::cli::PolishArgs;
use crate::config;
use crate::contract::ImageDetail;
use crate::polish;
use crate::util;

use super::VERSION;
use super::image_info;
use super::print_json;

pub(crate) fn polish(args: PolishArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
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

#[cfg(test)]
mod tests {
    use super::super::config::config_set;
    use super::super::config::empty_set_args;
    use super::super::temp_dir;
    use super::super::write_test_image;
    use super::*;
    use crate::cli::ConfigPolishMode;
    use crate::cli::ConfigSetArgs;
    use crate::cli::OutputFormat;

    /// Polish against a config path that does not exist, so the run sees the
    /// built-in defaults regardless of what the developer has pinned.
    fn run_polish_defaults(args: PolishArgs) -> crate::contract::PolishResult {
        run_polish(args, Path::new("/nonexistent/cloche/config.json"))
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
}

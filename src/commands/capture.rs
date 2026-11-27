use std::path::Path;
use std::process::ExitCode;

use chrono::Utc;

use crate::backends;
use crate::cli::CaptureArgs;
use crate::cli::PresentationMode;
use crate::config;
use crate::contract::AppshotResult;
use crate::polish;
use crate::text;
use crate::util;

use super::VERSION;
use super::image_info;
use super::print_json;

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

pub(crate) fn capture(args: CaptureArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::captures::find_captures;
use crate::captures::read_metadata;
use crate::captures::read_metadata_file;
use crate::cli::PreviewArgs;
use crate::contract::ImageInfo;

use super::open_path;

pub(crate) fn preview(args: PreviewArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
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

#[cfg(test)]
mod tests {
    use super::super::temp_dir;
    use super::*;
    use crate::contract::AppshotResult;
    use crate::contract::CaptureTarget;
    use crate::contract::ImageDetail;
    use crate::contract::TextInfo;

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
}

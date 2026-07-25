//! Discovery of past captures on disk: scan roots for flat `cloche-shot*.json`
//! sidecars (the current layout), legacy `cloche-shot*` / `appshot*` directories
//! with `metadata.json`, and one level of nested out-dirs that hold flat
//! sidecars (per-shot or custom `--out-dir`). Deserialize each and summarize
//! them for the `gallery`, `latest`, and `preview` commands.

use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;

use crate::contract::AppshotResult;
use crate::contract::CaptureTarget;
use crate::contract::ImageInfo;
use crate::util;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSummary {
    pub output_dir: PathBuf,
    pub created_at: chrono::DateTime<Utc>,
    pub target: CaptureTarget,
    pub image: Option<ImageInfo>,
    pub presentation_image: Option<ImageInfo>,
    pub presentation_style: Option<crate::contract::PresentationStyleInfo>,
    pub window: Option<crate::contract::WindowInfo>,
}

pub fn find_captures(roots: Vec<PathBuf>, limit: usize) -> Vec<CaptureSummary> {
    let roots = if roots.is_empty() {
        vec![
            crate::backends::default_gallery_dir(),
            PathBuf::from("."),
            PathBuf::from("/tmp"),
        ]
    } else {
        roots
    };
    let mut captures = Vec::new();
    let mut seen_roots = std::collections::HashSet::new();
    let mut seen_sidecars = std::collections::HashSet::new();
    for root in roots {
        // Canonicalize so the same dir reached via different paths (e.g. "." and
        // the gallery dir) is scanned once.
        let key = std::fs::canonicalize(&root).unwrap_or(root.clone());
        if seen_roots.insert(key) {
            collect_captures(&root, &mut captures, &mut seen_sidecars);
        }
    }
    captures.sort_by_key(|capture| std::cmp::Reverse(capture.created_at));
    captures.truncate(limit);
    captures
}

fn collect_captures(
    root: &Path,
    captures: &mut Vec<CaptureSummary>,
    seen_sidecars: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_file() {
            // Flat layout at the scan root: `<stem>.json`.
            if name.ends_with(".json") && is_capture_name(name) {
                push_sidecar(captures, seen_sidecars, &path);
            }
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        if is_capture_name(name) {
            // Per-shot out-dir or legacy folder named cloche-shot* / appshot*:
            // collect legacy metadata.json and any flat sidecars inside.
            collect_from_capture_dir(&path, captures, seen_sidecars);
        } else {
            // Custom out-dir one level down (e.g. /tmp/cloche-demo): only look
            // for flat sidecars so unrelated metadata.json files are ignored.
            collect_flat_sidecars(&path, captures, seen_sidecars);
        }
    }
}

fn is_capture_name(name: &str) -> bool {
    name.starts_with("cloche-shot") || name.starts_with("appshot")
}

fn collect_from_capture_dir(
    dir: &Path,
    captures: &mut Vec<CaptureSummary>,
    seen_sidecars: &mut std::collections::HashSet<PathBuf>,
) {
    push_sidecar(captures, seen_sidecars, &dir.join("metadata.json"));
    collect_flat_sidecars(dir, captures, seen_sidecars);
}

fn collect_flat_sidecars(
    dir: &Path,
    captures: &mut Vec<CaptureSummary>,
    seen_sidecars: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_file() && name.ends_with(".json") && is_capture_name(name) {
            push_sidecar(captures, seen_sidecars, &path);
        }
    }
}

fn push_sidecar(
    captures: &mut Vec<CaptureSummary>,
    seen_sidecars: &mut std::collections::HashSet<PathBuf>,
    sidecar: &Path,
) {
    let key = std::fs::canonicalize(sidecar).unwrap_or_else(|_| sidecar.to_path_buf());
    if !seen_sidecars.insert(key) {
        return;
    }
    if let Ok(metadata) = read_metadata_file(sidecar) {
        captures.push(CaptureSummary {
            output_dir: metadata.output_dir,
            created_at: metadata.created_at,
            target: metadata.target,
            image: metadata.image,
            presentation_image: metadata.presentation_image,
            presentation_style: metadata.presentation_style,
            window: metadata.window,
        });
    }
}

/// Read a flat `<stem>.json` metadata sidecar.
pub fn read_metadata_file(path: &Path) -> Result<AppshotResult, Box<dyn std::error::Error>> {
    let bytes = util::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Read a legacy folder-style capture's `metadata.json`.
pub fn read_metadata(capture_dir: &Path) -> Result<AppshotResult, Box<dyn std::error::Error>> {
    read_metadata_file(&capture_dir.join("metadata.json"))
}

/// Resolve a capture path to its metadata sidecar.
///
/// Accepts:
/// - a flat `<stem>.json` file
/// - a directory containing legacy `metadata.json` and/or flat
///   `cloche-shot*.json` / `appshot*.json` sidecars (newest `created_at` wins)
pub fn resolve_metadata_path(capture: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if capture.is_file() {
        return Ok(capture.to_path_buf());
    }
    if !capture.is_dir() {
        return Err(format!("capture path does not exist: {}", capture.display()).into());
    }

    let mut candidates = Vec::new();
    let legacy = capture.join("metadata.json");
    if legacy.is_file()
        && let Ok(metadata) = read_metadata_file(&legacy)
    {
        // Include legacy in the same newest-wins ranking so a reused out-dir
        // with an old metadata.json does not shadow newer flat sidecars.
        candidates.push((metadata.created_at, legacy));
    }
    let entries = std::fs::read_dir(capture)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        if !name.starts_with("cloche-shot") && !name.starts_with("appshot") {
            continue;
        }
        if let Ok(metadata) = read_metadata_file(&path) {
            candidates.push((metadata.created_at, path));
        }
    }
    candidates.sort_by_key(|(created_at, _)| std::cmp::Reverse(*created_at));
    candidates
        .into_iter()
        .next()
        .map(|(_, path)| path)
        .ok_or_else(|| {
            format!(
                "no capture metadata in {}: expected metadata.json or a cloche-shot*.json sidecar",
                capture.display()
            )
            .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::CaptureTarget;
    use crate::contract::ImageDetail;
    use crate::contract::ImageInfo;
    use crate::contract::TextInfo;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cloche-captures-test-{}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    fn write_flat_capture(dir: &Path, stem: &str, created_at: chrono::DateTime<Utc>) {
        std::fs::create_dir_all(dir).expect("create capture dir");
        let image_path = dir.join(format!("{stem}.png"));
        std::fs::write(&image_path, b"png").expect("write image");
        let metadata = AppshotResult {
            ok: true,
            version: "0.0.0".to_string(),
            created_at,
            target: CaptureTarget::Active,
            backend: None,
            output_dir: dir.to_path_buf(),
            image: Some(ImageInfo {
                path: image_path,
                width: Some(4),
                height: Some(4),
                bytes: 1,
                mime: "image/png".to_string(),
                detail: ImageDetail::High,
            }),
            presentation_image: None,
            presentation_style: None,
            window: None,
            text: TextInfo::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let bytes = serde_json::to_vec_pretty(&metadata).expect("serialize");
        std::fs::write(dir.join(format!("{stem}.json")), bytes).expect("write sidecar");
    }

    fn write_legacy_capture(dir: &Path, created_at: chrono::DateTime<Utc>) {
        std::fs::create_dir_all(dir).expect("create capture dir");
        let image_path = dir.join("shot.png");
        std::fs::write(&image_path, b"png").expect("write image");
        let metadata = AppshotResult {
            ok: true,
            version: "0.0.0".to_string(),
            created_at,
            target: CaptureTarget::Active,
            backend: None,
            output_dir: dir.to_path_buf(),
            image: Some(ImageInfo {
                path: image_path,
                width: Some(4),
                height: Some(4),
                bytes: 1,
                mime: "image/png".to_string(),
                detail: ImageDetail::High,
            }),
            presentation_image: None,
            presentation_style: None,
            window: None,
            text: TextInfo::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let bytes = serde_json::to_vec_pretty(&metadata).expect("serialize");
        std::fs::write(dir.join("metadata.json"), bytes).expect("write metadata");
    }

    #[test]
    fn finds_flat_sidecar_nested_in_matching_outdir() {
        // README-style per-shot out-dir: /tmp/cloche-shot-<ts>/<stem>.json
        let root = temp_root("nested-match");
        let out = root.join("cloche-shot-1710000000");
        write_flat_capture(&out, "cloche-shot-20260725T120000Z-1-0", Utc::now());

        let found = find_captures(vec![root.clone()], 10);
        assert_eq!(
            found.len(),
            1,
            "nested flat sidecar in cloche-shot* out-dir should be discovered"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_flat_sidecar_nested_in_custom_outdir() {
        // Stable demo out-dir that does not itself start with cloche-shot.
        let root = temp_root("nested-custom");
        let out = root.join("cloche-demo");
        write_flat_capture(&out, "cloche-shot-20260725T120000Z-1-0", Utc::now());

        let found = find_captures(vec![root.clone()], 10);
        assert_eq!(
            found.len(),
            1,
            "flat sidecar one level under a custom out-dir should be discovered"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn still_finds_legacy_capture_directory() {
        let root = temp_root("legacy");
        let out = root.join("cloche-shot-legacy");
        write_legacy_capture(&out, Utc::now());

        let found = find_captures(vec![root.clone()], 10);
        assert_eq!(found.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn still_finds_flat_sidecar_at_scan_root() {
        let root = temp_root("flat-root");
        write_flat_capture(&root, "cloche-shot-top", Utc::now());

        let found = find_captures(vec![root.clone()], 10);
        assert_eq!(found.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn overlapping_roots_do_not_duplicate_captures() {
        let root = temp_root("overlap");
        let nested = root.join("cloche-demo");
        write_flat_capture(&nested, "cloche-shot-one", Utc::now());

        let found = find_captures(vec![root.clone(), nested.clone()], 10);
        assert_eq!(
            found.len(),
            1,
            "parent + child roots should yield one capture, not two"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

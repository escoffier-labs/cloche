//! Discovery of past captures on disk: scan roots for flat `cloche-shot*.json`
//! sidecars (the current layout) and, for back-compat, legacy `cloche-shot*` /
//! `appshot*` directories with a `metadata.json` inside. Deserialize each and
//! summarize them for the `gallery`, `latest`, and `preview` commands.

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
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        // Canonicalize so the same dir reached via different paths (e.g. "." and
        // the gallery dir) is scanned once.
        let key = std::fs::canonicalize(&root).unwrap_or(root.clone());
        if seen.insert(key) {
            collect_captures(&root, &mut captures);
        }
    }
    captures.sort_by_key(|capture| std::cmp::Reverse(capture.created_at));
    captures.truncate(limit);
    captures
}

fn collect_captures(root: &Path, captures: &mut Vec<CaptureSummary>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("cloche-shot") && !name.starts_with("appshot") {
            continue;
        }
        // Flat layout: `<stem>.json` sidecar. Legacy layout: a directory with a
        // `metadata.json` inside.
        let metadata = if path.is_dir() {
            read_metadata(&path)
        } else if name.ends_with(".json") {
            read_metadata_file(&path)
        } else {
            continue;
        };
        if let Ok(metadata) = metadata {
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

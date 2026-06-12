//! Discovery of past captures on disk: scan roots for `cloche-shot*` /
//! `appshot*` directories, deserialize their `metadata.json`, and summarize
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
        vec![PathBuf::from("."), PathBuf::from("/tmp")]
    } else {
        roots
    };
    let mut captures = Vec::new();
    for root in roots {
        collect_captures(&root, &mut captures);
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
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("cloche-shot") && !name.starts_with("appshot") {
            continue;
        }
        if let Ok(metadata) = read_metadata(&path) {
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

pub fn read_metadata(capture_dir: &Path) -> Result<AppshotResult, Box<dyn std::error::Error>> {
    let bytes = util::read(&capture_dir.join("metadata.json"))?;
    Ok(serde_json::from_slice(&bytes)?)
}

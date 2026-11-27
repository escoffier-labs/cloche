use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;

use crate::captures::CaptureSummary;
use crate::captures::find_captures;
use crate::cli::GalleryArgs;
use crate::cli::LatestArgs;
use crate::util;

use super::open_path;
use super::print_json;

pub(crate) fn gallery(args: GalleryArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
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

pub(crate) fn latest(args: LatestArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let capture = find_captures(args.root, 1).into_iter().next();
    let ok = capture.is_some();
    print_json(&LatestOutput { ok, capture })?;
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
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

//! Handlers for each `cloche` command family. `cli.rs` owns the clap types and
//! dispatch; one module per family lives here, with its unit tests.

use std::path::Path;

use crate::contract::ImageDetail;
use crate::contract::ImageInfo;
use crate::util;

mod capture;
mod config;
mod doctor;
mod gallery;
mod polish;
mod preview;
mod schema;

pub(crate) use capture::capture;
pub(crate) use config::config;
pub(crate) use doctor::doctor;
pub(crate) use doctor::list_windows;
pub(crate) use gallery::gallery;
pub(crate) use gallery::latest;
pub(crate) use polish::polish;
pub(crate) use preview::preview;
pub(crate) use schema::schema;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Fixtures shared by the per-command test modules.
#[cfg(test)]
fn temp_dir(label: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("cloche-polish-test-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[cfg(test)]
fn write_test_image(path: &Path, width: u32, height: u32) {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([180, 180, 180, 255]));
    image.save(path).expect("write test image");
}

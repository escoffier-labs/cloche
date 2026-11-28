//! Copy capture images using wl-copy on Wayland, a detached arboard server on
//! X11, or an in-process clipboard-win write on Windows. Clipboard owners never
//! inherit the caller's output pipes.

use std::path::Path;

#[cfg(target_os = "linux")]
use crate::util;

/// Copy a PNG file to the clipboard. Failures are returned as strings so the
/// caller can surface them as capture warnings, never errors.
pub fn copy_png(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let bytes =
            std::fs::read(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        let (width, height, rgba) = png_to_rgba(&bytes)?;
        // Callers pass PNG files, so reuse the bytes as-is for the PNG format
        // and only re-encode if something else was handed in.
        let png = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            bytes
        } else {
            encode_png(width, height, &rgba)?
        };
        let dibv5 = rgba_to_dibv5(width, height, &rgba)?;
        // Windows keeps clipboard data after the owning process exits, so no
        // detached server is needed (unlike X11).
        write_image_private(&png, &dibv5)
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;

        let wayland = util::env_var("WAYLAND_DISPLAY").is_some();
        match selected_tool(
            wayland,
            util::has_command("wl-copy"),
            util::env_var("DISPLAY").is_some(),
        ) {
            Some(ClipboardTool::WlCopy) => {
                let file = std::fs::File::open(path).map_err(|err| err.to_string())?;
                let status = util::desktop_command("wl-copy")
                    .args(["--type", "image/png"])
                    .stdin(file)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|err| format!("wl-copy failed to start: {err}"))?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("wl-copy exited with {status}"))
                }
            }
            Some(ClipboardTool::X11Serve) => {
                let executable = std::env::current_exe().map_err(|err| err.to_string())?;
                let executable = executable
                    .to_str()
                    .ok_or("clipboard server executable path is not valid UTF-8")?;
                let (reader, writer) = std::io::pipe()
                    .map_err(|err| format!("clipboard readiness pipe failed: {err}"))?;
                let mut command = util::desktop_command(executable);
                command
                    .arg("clipboard-serve")
                    .arg(path)
                    .stdin(Stdio::null())
                    .stdout(writer)
                    .stderr(Stdio::null())
                    .process_group(0);
                let mut child = command
                    .spawn()
                    .map_err(|err| format!("clipboard server failed to start: {err}"))?;
                // Command retains its configured stdout after spawn. Close that
                // copy so an early child exit produces EOF on the private pipe.
                drop(command);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(read_readiness(reader));
                });
                let result = rx
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .map_err(|err| format!("clipboard server readiness failed: {err}"))
                    .and_then(|result| result);
                if result.is_err() {
                    // A timed-out child must not claim the clipboard later.
                    let _ = child.kill();
                    let _ = child.wait();
                }
                result
            }
            None => Err(
                "no clipboard helper found; install wl-clipboard (Wayland) or set DISPLAY (X11)"
                    .to_string(),
            ),
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        Err("clipboard copy is not supported on this platform".to_string())
    }
}

/// Decode image bytes to the (width, height, RGBA8) layout arboard expects.
/// Pure so it can be unit tested without touching a clipboard.
#[cfg(any(target_os = "windows", test))]
fn png_to_rgba(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("cannot decode image for clipboard: {err}"))?
        .into_rgba8();
    let (width, height) = (image.width() as usize, image.height() as usize);
    Ok((width, height, image.into_raw()))
}

/// Clipboard formats Windows reads to decide whether an item may enter
/// Clipboard History and cloud clipboard sync. Registered by name, each holds a
/// DWORD of 0 to opt the item out.
/// <https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats>
#[cfg(any(target_os = "windows", test))]
const EXCLUSION_FORMATS: [&str; 2] = ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];

/// The DWORD 0 payload that each exclusion format carries.
#[cfg(any(target_os = "windows", test))]
const EXCLUSION_PAYLOAD: [u8; 4] = 0u32.to_le_bytes();

#[cfg(any(target_os = "windows", test))]
const BITMAPV5HEADER_SIZE: usize = 124;

/// Re-encode RGBA8 pixels as PNG for the registered "PNG" clipboard format.
#[cfg(any(target_os = "windows", test))]
fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let too_large = |_| "image dimensions too large for the clipboard".to_string();
    let image = image::RgbaImage::from_raw(
        u32::try_from(width).map_err(too_large)?,
        u32::try_from(height).map_err(too_large)?,
        rgba.to_vec(),
    )
    .ok_or("pixel buffer does not match the image dimensions")?;
    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|err| format!("cannot encode image for clipboard: {err}"))?;
    Ok(out.into_inner())
}

/// Build a CF_DIBV5 payload (BITMAPV5HEADER plus 32-bit BGRA pixels, bottom-up)
/// from RGBA8 pixels, matching what arboard wrote. Rows are stored bottom-up
/// because MS Word and WordPad cannot paste a top-down DIB (negative height).
/// Pure so it can be unit tested without touching a clipboard.
#[cfg(any(target_os = "windows", test))]
fn rgba_to_dibv5(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    const LCS_SRGB: u32 = 0x7352_4742; // 'sRGB'
    const LCS_GM_IMAGES: u32 = 4;
    const BI_BITFIELDS: u32 = 3;

    let size_error = || "image dimensions too large for the clipboard".to_string();
    let pixel_bytes = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(size_error)?;
    if width == 0 || height == 0 || rgba.len() != pixel_bytes {
        return Err("pixel buffer does not match the image dimensions".to_string());
    }
    let size_image = u32::try_from(pixel_bytes).map_err(|_| size_error())?;
    let width_field = i32::try_from(width).map_err(|_| size_error())?;
    let height_field = i32::try_from(height).map_err(|_| size_error())?;

    let mut out = Vec::with_capacity(BITMAPV5HEADER_SIZE + pixel_bytes);
    let put_u32 = |out: &mut Vec<u8>, value: u32| out.extend_from_slice(&value.to_le_bytes());
    put_u32(&mut out, BITMAPV5HEADER_SIZE as u32); // bV5Size
    put_u32(&mut out, width_field as u32); // bV5Width
    put_u32(&mut out, height_field as u32); // bV5Height, positive = bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // bV5Planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bV5BitCount
    put_u32(&mut out, BI_BITFIELDS); // bV5Compression
    put_u32(&mut out, size_image); // bV5SizeImage
    put_u32(&mut out, 0); // bV5XPelsPerMeter
    put_u32(&mut out, 0); // bV5YPelsPerMeter
    put_u32(&mut out, 0); // bV5ClrUsed
    put_u32(&mut out, 0); // bV5ClrImportant
    put_u32(&mut out, 0x00ff_0000); // bV5RedMask
    put_u32(&mut out, 0x0000_ff00); // bV5GreenMask
    put_u32(&mut out, 0x0000_00ff); // bV5BlueMask
    put_u32(&mut out, 0xff00_0000); // bV5AlphaMask
    put_u32(&mut out, LCS_SRGB); // bV5CSType
    out.extend_from_slice(&[0; 36]); // bV5Endpoints, ignored for LCS_sRGB
    put_u32(&mut out, 0); // bV5GammaRed
    put_u32(&mut out, 0); // bV5GammaGreen
    put_u32(&mut out, 0); // bV5GammaBlue
    put_u32(&mut out, LCS_GM_IMAGES); // bV5Intent
    put_u32(&mut out, 0); // bV5ProfileData
    put_u32(&mut out, 0); // bV5ProfileSize
    put_u32(&mut out, 0); // bV5Reserved
    debug_assert_eq!(out.len(), BITMAPV5HEADER_SIZE);

    for row in rgba.chunks_exact(width * 4).rev() {
        for &[r, g, b, a] in row.as_chunks::<4>().0 {
            out.extend_from_slice(&[b, g, r, a]);
        }
    }
    Ok(out)
}

/// Write the image and the history/cloud exclusion markers in one clipboard
/// session. Microsoft documents the exclusion formats as data placed on the
/// clipboard alongside the content they describe, so they must be set while the
/// clipboard is still open from the same open/empty that wrote the image.
/// arboard cannot do this: its image path closes the clipboard on return and
/// never adds the markers. If a marker cannot be set, the clipboard is emptied
/// again so a screenshot never lands in History or the cloud by accident.
#[cfg(target_os = "windows")]
fn write_image_private(png: &[u8], dibv5: &[u8]) -> Result<(), String> {
    use clipboard_win::{formats, raw, register_format};

    // Another process (often a clipboard manager) may hold the clipboard for a
    // few milliseconds, so retry with a short sleep between attempts.
    let mut attempts = 0;
    let _session = loop {
        match clipboard_win::Clipboard::new() {
            Ok(session) => break session,
            Err(err) if attempts >= 10 => return Err(format!("clipboard unavailable: {err}")),
            Err(_) => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    };
    let write_error =
        |err: clipboard_win::ErrorCode| format!("clipboard image write failed: {err}");

    raw::empty().map_err(write_error)?;
    // Order matters: some programs take the first format available, and PNG has
    // better compatibility than a DIB, so it goes first.
    let png_format = register_format("PNG").ok_or("cannot register the PNG clipboard format")?;
    raw::set_without_clear(png_format.get(), png).map_err(write_error)?;
    raw::set_without_clear(formats::CF_DIBV5, dibv5).map_err(write_error)?;

    for name in EXCLUSION_FORMATS {
        let marked = register_format(name)
            .ok_or_else(|| format!("cannot register the {name} clipboard format"))
            .and_then(|format| {
                raw::set_without_clear(format.get(), &EXCLUSION_PAYLOAD)
                    .map_err(|err| format!("cannot set {name}: {err}"))
            });
        if let Err(err) = marked {
            let _ = raw::empty();
            return Err(format!("clipboard history exclusion failed: {err}"));
        }
    }
    Ok(())
}

/// Own the X11 clipboard until another application replaces its contents.
#[cfg(target_os = "linux")]
pub fn serve_png(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use arboard::SetExtLinux;
    use std::io::Write;
    use std::os::fd::{FromRawFd, OwnedFd};

    // Do this before opening the image, X connection, or starting any threads.
    let cleanup = close_inherited_fds();
    // SAFETY: this child owns fd 1, supplied by the parent as its private pipe.
    // Unlike std::io::stdout(), dropping this File actually closes fd 1.
    let mut readiness = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(1) });
    let prepared = (|| -> Result<_, Box<dyn std::error::Error>> {
        cleanup?;
        let image = image::open(path)?.into_rgba8();
        check_png_size(std::fs::metadata(path)?.len())?;
        let data = arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: std::borrow::Cow::Owned(image.into_raw()),
        };
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set().image(data.clone())?;
        Ok((clipboard, data))
    })();
    let (mut clipboard, data) = match prepared {
        Ok(prepared) => prepared,
        Err(err) => {
            let _ = writeln!(readiness, "error: {err}");
            let _ = readiness.flush();
            return Err(err);
        }
    };
    readiness.write_all(b"1")?;
    readiness.flush()?;
    drop(readiness);
    // arboard's blocking wait can survive a lost X connection indefinitely.
    // Text is unavailable for our PNG, but querying it still checks X liveness.
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => match clipboard.get().text() {
                    Ok(_) | Err(arboard::Error::ContentNotAvailable) => {}
                    Err(_) => std::process::exit(0),
                },
                Err(_) => std::process::exit(0),
            }
        }
    });
    clipboard.set().wait().image(data)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn close_inherited_fds() -> std::io::Result<()> {
    use std::os::fd::{FromRawFd, OwnedFd};

    let mut descriptors = Vec::new();
    for entry in std::fs::read_dir("/proc/self/fd")? {
        let entry = entry?;
        if let Some(fd) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
            .filter(|fd| *fd > 2)
        {
            descriptors.push(fd);
        }
    }
    // The iterator has closed its own fd. read_link opens no descriptors, so
    // checking which entries remain excludes that fd before taking ownership.
    for fd in descriptors {
        match std::fs::read_link(format!("/proc/self/fd/{fd}")) {
            Ok(_) => {
                // SAFETY: startup is still single-threaded. This is an open,
                // inherited fd with no Rust owner, and no fd has been reused.
                drop(unsafe { OwnedFd::from_raw_fd(fd) });
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn read_readiness(reader: impl std::io::Read) -> Result<(), String> {
    use std::io::{BufRead, Read};

    let mut reader = std::io::BufReader::new(reader);
    let mut first = [0];
    let count = reader
        .read(&mut first)
        .map_err(|err| format!("clipboard readiness read failed: {err}"))?;
    if count == 0 || first == *b"1" {
        return parse_readiness_message(&first[..count]);
    }
    let mut message = vec![first[0]];
    reader
        .take(4096)
        .read_until(b'\n', &mut message)
        .map_err(|err| format!("clipboard readiness read failed: {err}"))?;
    parse_readiness_message(&message)
}

#[cfg(any(target_os = "linux", test))]
fn parse_readiness_message(message: &[u8]) -> Result<(), String> {
    match message {
        b"1" => Ok(()),
        b"" => Err("clipboard server exited before ready".to_string()),
        _ => {
            let text = String::from_utf8_lossy(message);
            match text.trim().strip_prefix("error: ") {
                Some(error) => Err(format!("clipboard server: {error}")),
                None => Err(format!(
                    "unexpected clipboard readiness message: {}",
                    text.trim()
                )),
            }
        }
    }
}

/// Approximate X11's single-property limit using the input PNG's disk size.
/// arboard re-encodes RGBA, so its transmitted PNG can differ in size.
#[cfg(any(target_os = "linux", test))]
fn check_png_size(bytes: u64) -> Result<(), String> {
    if bytes > 15 * 1024 * 1024 {
        return Err(format!(
            "image too large for X11 clipboard ({:.2} MiB > 15 MiB)",
            bytes as f64 / (1024.0 * 1024.0)
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, PartialEq, Eq)]
enum ClipboardTool {
    WlCopy,
    X11Serve,
}

/// Prefer wl-copy on Wayland, otherwise serve X11 when DISPLAY is available
/// (including XWayland). X11 does not require an external clipboard helper.
#[cfg(any(target_os = "linux", test))]
fn selected_tool(wayland: bool, has_wl_copy: bool, has_display: bool) -> Option<ClipboardTool> {
    if wayland && has_wl_copy {
        return Some(ClipboardTool::WlCopy);
    }
    if has_display {
        return Some(ClipboardTool::X11Serve);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_byte_confirms_ownership() {
        assert_eq!(parse_readiness_message(b"1"), Ok(()));
    }

    #[test]
    fn readiness_error_preserves_the_child_failure() {
        assert_eq!(
            parse_readiness_message(b"error: cannot open display\n"),
            Err("clipboard server: cannot open display".to_string())
        );
    }

    #[test]
    fn readiness_eof_and_unexpected_messages_are_errors() {
        assert!(
            parse_readiness_message(b"")
                .unwrap_err()
                .contains("before ready")
        );
        assert!(parse_readiness_message(b"unexpected\n").is_err());
        assert!(parse_readiness_message(b"1unexpected").is_err());
    }

    #[test]
    fn png_size_guard_uses_encoded_bytes_and_allows_the_limit() {
        assert_eq!(check_png_size(0), Ok(()));
        assert_eq!(check_png_size(15 * 1024 * 1024), Ok(()));
        assert!(check_png_size(15 * 1024 * 1024 + 1).is_err());
        assert_eq!(
            check_png_size(16 * 1024 * 1024),
            Err("image too large for X11 clipboard (16.00 MiB > 15 MiB)".to_string())
        );
    }

    fn png_bytes(image: &image::RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn png_to_rgba_returns_dimensions_and_pixels() {
        let mut image = image::RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
        image.put_pixel(1, 0, image::Rgba([40, 50, 60, 128]));
        let (width, height, rgba) = png_to_rgba(&png_bytes(&image)).unwrap();
        assert_eq!((width, height), (2, 1));
        assert_eq!(rgba, vec![10, 20, 30, 255, 40, 50, 60, 128]);
    }

    #[test]
    fn png_to_rgba_expands_opaque_rgb_to_rgba() {
        let image = image::RgbImage::from_pixel(1, 2, image::Rgb([1, 2, 3]));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        let (width, height, rgba) = png_to_rgba(&out.into_inner()).unwrap();
        assert_eq!((width, height), (1, 2));
        assert_eq!(rgba, vec![1, 2, 3, 255, 1, 2, 3, 255]);
    }

    #[test]
    fn png_to_rgba_rejects_non_image_bytes() {
        let err = png_to_rgba(b"not a png").unwrap_err();
        assert!(err.contains("cannot decode image for clipboard"));
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn exclusion_markers_use_the_documented_names_and_a_zero_dword() {
        assert_eq!(
            EXCLUSION_FORMATS,
            ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"]
        );
        assert_eq!(EXCLUSION_PAYLOAD, [0, 0, 0, 0]);
    }

    #[test]
    fn dibv5_header_matches_the_bitmapv5header_layout() {
        let dib = rgba_to_dibv5(3, 2, &[0; 24]).unwrap();
        assert_eq!(dib.len(), BITMAPV5HEADER_SIZE + 24);
        assert_eq!(u32_at(&dib, 0), 124); // bV5Size
        assert_eq!(u32_at(&dib, 4), 3); // bV5Width
        assert_eq!(u32_at(&dib, 8), 2); // bV5Height, positive = bottom-up
        assert_eq!(&dib[12..14], &1u16.to_le_bytes()); // bV5Planes
        assert_eq!(&dib[14..16], &32u16.to_le_bytes()); // bV5BitCount
        assert_eq!(u32_at(&dib, 16), 3); // BI_BITFIELDS
        assert_eq!(u32_at(&dib, 20), 24); // bV5SizeImage
        assert_eq!(u32_at(&dib, 40), 0x00ff_0000); // red mask
        assert_eq!(u32_at(&dib, 44), 0x0000_ff00); // green mask
        assert_eq!(u32_at(&dib, 48), 0x0000_00ff); // blue mask
        assert_eq!(u32_at(&dib, 52), 0xff00_0000); // alpha mask
        assert_eq!(u32_at(&dib, 56), 0x7352_4742); // LCS_sRGB
        assert_eq!(u32_at(&dib, 108), 4); // LCS_GM_IMAGES
        for offset in [24, 28, 32, 36, 100, 104, 112, 116, 120] {
            assert_eq!(u32_at(&dib, offset), 0, "offset {offset}");
        }
    }

    #[test]
    fn dibv5_pixels_are_bgra_and_rows_are_bottom_up() {
        // 1x2: top row is (1,2,3,4), bottom row is (5,6,7,8) in RGBA.
        let dib = rgba_to_dibv5(1, 2, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(&dib[BITMAPV5HEADER_SIZE..], &[7, 6, 5, 8, 3, 2, 1, 4]);
    }

    #[test]
    fn dibv5_rejects_bad_dimensions() {
        assert!(rgba_to_dibv5(0, 1, &[]).is_err());
        assert!(rgba_to_dibv5(1, 0, &[]).is_err());
        assert!(rgba_to_dibv5(2, 2, &[0; 15]).is_err());
        assert!(rgba_to_dibv5(usize::MAX, 2, &[]).is_err());
    }

    #[test]
    fn encode_png_round_trips_pixels() {
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
        let png = encode_png(2, 1, &rgba).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(png_to_rgba(&png).unwrap(), (2, 1, rgba));
    }

    #[test]
    fn encode_png_rejects_a_mismatched_buffer() {
        assert!(encode_png(2, 2, &[0; 4]).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn readiness_reader_does_not_wait_for_eof_after_ready() {
        use std::io::Write;

        let (reader, mut writer) = std::io::pipe().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            tx.send(read_readiness(reader)).unwrap();
        });
        writer.write_all(b"1").unwrap();
        let result = rx.recv_timeout(std::time::Duration::from_secs(1));
        drop(writer);
        thread.join().unwrap();
        assert_eq!(result.unwrap(), Ok(()));
    }

    #[test]
    fn wayland_prefers_wl_copy() {
        assert_eq!(selected_tool(true, true, true), Some(ClipboardTool::WlCopy));
        assert_eq!(
            selected_tool(true, true, false),
            Some(ClipboardTool::WlCopy)
        );
    }

    #[test]
    fn x11_uses_server_without_an_external_helper() {
        assert_eq!(
            selected_tool(false, true, true),
            Some(ClipboardTool::X11Serve)
        );
        assert_eq!(
            selected_tool(false, false, true),
            Some(ClipboardTool::X11Serve)
        );
    }

    #[test]
    fn wayland_without_wl_copy_falls_back_to_x11_server() {
        assert_eq!(
            selected_tool(true, false, true),
            Some(ClipboardTool::X11Serve)
        );
    }

    #[test]
    fn no_helpers_means_none() {
        assert!(selected_tool(false, false, false).is_none());
        assert!(selected_tool(false, true, false).is_none());
        assert!(selected_tool(true, false, false).is_none());
    }
}

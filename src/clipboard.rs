//! Copy capture images using wl-copy on Wayland or a detached arboard server
//! on X11. Clipboard owners never inherit the caller's output pipes.

use std::path::Path;

#[cfg(target_os = "linux")]
use crate::util;

/// Copy a PNG file to the clipboard. Failures are returned as strings so the
/// caller can surface them as capture warnings, never errors.
pub fn copy_png(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = path;
        Err("clipboard copy is not supported on Windows yet".to_string())
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

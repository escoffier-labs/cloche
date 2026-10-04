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
                util::desktop_command(executable)
                    .arg("clipboard-serve")
                    .arg(path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0)
                    .spawn()
                    .map(|_| ())
                    .map_err(|err| format!("clipboard server failed to start: {err}"))
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

    let image = image::open(path)?.into_rgba8();
    let data = arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Owned(image.into_raw()),
    };
    arboard::Clipboard::new()?.set().wait().image(data)?;
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

//! Copy a capture image to the system clipboard by shelling out to the
//! session's clipboard helper, mirroring the capture backends' discovery
//! pattern. No clipboard crate: one process spawn per copy is fine.

use std::path::Path;

#[cfg(not(target_os = "windows"))]
use crate::util;

/// Copy a PNG file to the clipboard. Failures are returned as strings so the
/// caller can surface them as capture warnings, never errors.
pub fn copy_png(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = path;
        Err("clipboard copy is not supported on Windows yet".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let wayland = util::env_var("WAYLAND_DISPLAY").is_some();
        let (command, args) = match selected_tool(
            wayland,
            util::has_command("wl-copy"),
            util::has_command("xclip"),
        ) {
            Some(tool) => tool,
            None => {
                return Err(
                    "no clipboard helper found; install wl-clipboard (Wayland) or xclip (X11)"
                        .to_string(),
                );
            }
        };
        let file = std::fs::File::open(path).map_err(|err| err.to_string())?;
        let status = util::desktop_command(command)
            .args(
                args.iter()
                    .map(|arg| arg.replace("{path}", &path.display().to_string())),
            )
            .stdin(file)
            .status()
            .map_err(|err| format!("{command} failed to start: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{command} exited with {status}"))
        }
    }
}

/// Pure helper-selection logic: Wayland sessions prefer wl-copy, X11 uses
/// xclip, and a Wayland session without wl-copy can still fall back to xclip
/// (XWayland).
#[cfg_attr(target_os = "windows", allow(dead_code))]
fn selected_tool(
    wayland: bool,
    has_wl_copy: bool,
    has_xclip: bool,
) -> Option<(&'static str, Vec<&'static str>)> {
    if wayland && has_wl_copy {
        return Some(("wl-copy", vec!["--type", "image/png"]));
    }
    if has_xclip {
        return Some((
            "xclip",
            vec!["-selection", "clipboard", "-t", "image/png", "-i", "{path}"],
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_prefers_wl_copy() {
        let (command, _) = selected_tool(true, true, true).expect("tool");
        assert_eq!(command, "wl-copy");
    }

    #[test]
    fn x11_uses_xclip() {
        let (command, _) = selected_tool(false, true, true).expect("tool");
        assert_eq!(command, "xclip");
    }

    #[test]
    fn wayland_without_wl_copy_falls_back_to_xclip() {
        let (command, _) = selected_tool(true, false, true).expect("tool");
        assert_eq!(command, "xclip");
    }

    #[test]
    fn no_helpers_means_none() {
        assert!(selected_tool(false, false, false).is_none());
    }
}

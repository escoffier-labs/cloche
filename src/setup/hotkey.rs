//! Desktop detection and screenshot-hotkey installation.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::util;

/// Desktop environments we treat distinctly for hotkey binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desktop {
    Gnome,
    Kde,
    Sway,
    I3,
    Other,
}

/// Classify a desktop from `XDG_CURRENT_DESKTOP` / `DESKTOP_SESSION` values.
/// Matching is case-insensitive and substring-based because real values are
/// noisy (e.g. `ubuntu:GNOME`, `plasma`, `sway`).
pub fn classify_desktop(current_desktop: Option<&str>, desktop_session: Option<&str>) -> Desktop {
    let hay = format!(
        "{} {}",
        current_desktop.unwrap_or(""),
        desktop_session.unwrap_or("")
    )
    .to_lowercase();
    if hay.contains("gnome") {
        Desktop::Gnome
    } else if hay.contains("kde") || hay.contains("plasma") {
        Desktop::Kde
    } else if hay.contains("sway") {
        Desktop::Sway
    } else if hay.contains("i3") {
        Desktop::I3
    } else {
        Desktop::Other
    }
}

/// Detect the desktop from live environment variables.
pub fn detect_desktop() -> Desktop {
    classify_desktop(
        util::env_var("XDG_CURRENT_DESKTOP").as_deref(),
        util::env_var("DESKTOP_SESSION").as_deref(),
    )
}

/// The grab script shipped in the repo, embedded so an installed binary with no
/// source checkout can still lay it down. `scripts/*.sh` is in the crate include.
const GRAB_SCRIPT: &str = include_str!("../../scripts/cloche-grab.sh");

/// Target install path for the grab script.
pub fn grab_script_path() -> PathBuf {
    let home = util::env_var("HOME").unwrap_or_else(|| "/root".to_string());
    PathBuf::from(home).join(".local/bin/cloche-grab")
}

/// True when the file at `path` already holds the current embedded script.
pub fn grab_script_current(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|existing| existing == GRAB_SCRIPT)
        .unwrap_or(false)
}

/// Install the grab script to `~/.local/bin/cloche-grab` with mode 0755.
/// Returns Ok(true) when written, Ok(false) when already current (idempotent).
pub fn install_grab_script() -> std::io::Result<bool> {
    let path = grab_script_path();
    if grab_script_current(&path) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&path)?;
    file.write_all(GRAB_SCRIPT.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(true)
}

/// True when `~/.local/bin` is on PATH (so the binding can find cloche-grab).
pub fn local_bin_on_path() -> bool {
    let target = grab_script_path();
    let dir = target.parent().map(Path::to_path_buf);
    match (dir, util::env_var("PATH")) {
        (Some(dir), Some(path)) => std::env::split_paths(&path).any(|p| p == dir),
        _ => false,
    }
}

const KEYBIND_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KEYBIND_PREFIX: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/";

/// Decide which dconf slot path the cloche binding should occupy.
/// `existing` is the current custom-keybindings list; `cloche_slot` is the slot
/// (if any) whose command already resolves to cloche-grab. Reuse the cloche slot
/// when present; otherwise allocate the first unused `customN` index.
pub fn choose_binding_slot(existing: &[String], cloche_slot: Option<&str>) -> String {
    if let Some(slot) = cloche_slot {
        return slot.to_string();
    }
    let mut n = 0;
    loop {
        let candidate = format!("{KEYBIND_PREFIX}custom{n}/");
        if !existing.iter().any(|e| e == &candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Parse a gsettings array value like `['a', 'b']` or `@as []` into entries.
pub fn parse_gsettings_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed == "@as []" || trimmed == "[]" {
        return Vec::new();
    }
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn read_custom_keybindings() -> Vec<String> {
    util::run_output("gsettings", &["get", KEYBIND_SCHEMA, "custom-keybindings"])
        .map(|raw| parse_gsettings_array(&raw))
        .unwrap_or_default()
}

fn slot_command(slot: &str) -> Option<String> {
    let key = format!("{KEYBIND_SCHEMA}.custom-keybinding:{slot}");
    util::run_output("gsettings", &["get", &key, "command"])
        .ok()
        .map(|raw| raw.trim().trim_matches('\'').to_string())
}

/// True when a binding command points at our grab script, whether bound by the
/// bare name (`cloche-grab`) or an absolute path (`/home/u/.local/bin/cloche-grab`).
/// This keeps re-runs idempotent against hand-rolled bindings from the README.
pub fn is_cloche_command(cmd: &str) -> bool {
    let c = cmd.trim();
    c == "cloche-grab" || c.ends_with("/cloche-grab")
}

/// Outcome of attempting to bind the hotkey.
#[derive(Debug, PartialEq, Eq)]
pub enum HotkeyOutcome {
    /// Bound automatically (GNOME). `changed` is true when a change was made.
    Bound { changed: bool },
    /// Printed manual instructions for this desktop.
    Manual,
}

/// Bind `cloche-grab` to Print on GNOME, idempotently. `apply` false = dry-run.
pub fn bind_gnome(apply: bool) -> Result<HotkeyOutcome, crate::util::AppError> {
    let existing = read_custom_keybindings();
    let cloche_slot = existing
        .iter()
        .find(|slot| slot_command(slot).as_deref().is_some_and(is_cloche_command))
        .cloned();
    let already = cloche_slot.is_some();
    let slot = choose_binding_slot(&existing, cloche_slot.as_deref());

    if !apply {
        return Ok(HotkeyOutcome::Bound { changed: !already });
    }

    if !existing.contains(&slot) {
        let mut updated = existing.clone();
        updated.push(slot.clone());
        let list = format!(
            "[{}]",
            updated
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        util::run_status(
            "gsettings",
            &["set", KEYBIND_SCHEMA, "custom-keybindings", &list],
        )?;
    }
    let key = format!("{KEYBIND_SCHEMA}.custom-keybinding:{slot}");
    util::run_status("gsettings", &["set", &key, "name", "Cloche Grab"])?;
    util::run_status("gsettings", &["set", &key, "command", "cloche-grab"])?;
    util::run_status("gsettings", &["set", &key, "binding", "Print"])?;
    Ok(HotkeyOutcome::Bound { changed: !already })
}

/// Print copy-pasteable binding steps for non-GNOME desktops. Goes to stderr so
/// that `--format json` stdout stays pure JSON for machine consumers.
pub fn print_manual_binding(desktop: Desktop, grab: &Path) {
    let cmd = grab.display();
    match desktop {
        Desktop::Kde => {
            eprintln!("KDE: System Settings -> Shortcuts -> Custom Shortcuts -> Edit -> New ->");
            eprintln!("  Global Shortcut -> Command/URL, command `{cmd}`, then assign Print.");
        }
        Desktop::Sway | Desktop::I3 => {
            eprintln!("Add to your WM config and reload:");
            eprintln!("  bindsym Print exec {cmd}");
        }
        _ => {
            eprintln!("Bind a key to `{cmd}` in your desktop's keyboard settings.");
        }
    }
}

/// Install the grab script and bind the hotkey for the detected desktop.
/// Returns the outcome plus any warnings (e.g. PATH not set).
pub fn setup_hotkey(apply: bool) -> (HotkeyOutcome, Vec<String>) {
    let mut warnings = Vec::new();
    if apply {
        match install_grab_script() {
            Ok(_) => {}
            Err(err) => warnings.push(format!("could not install cloche-grab: {err}")),
        }
    }
    if !local_bin_on_path() {
        warnings.push(format!(
            "{} is not on PATH; add it so the hotkey can find cloche-grab",
            grab_script_path().parent().unwrap().display()
        ));
    }
    let desktop = detect_desktop();
    let outcome = if desktop == Desktop::Gnome {
        bind_gnome(apply).unwrap_or_else(|err| {
            warnings.push(format!("gsettings binding failed: {err}"));
            HotkeyOutcome::Manual
        })
    } else {
        if apply {
            print_manual_binding(desktop, &grab_script_path());
        }
        HotkeyOutcome::Manual
    };
    (outcome, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_gnome_variants() {
        assert_eq!(classify_desktop(Some("ubuntu:GNOME"), None), Desktop::Gnome);
        assert_eq!(classify_desktop(None, Some("gnome")), Desktop::Gnome);
    }

    #[test]
    fn classifies_kde_plasma_sway_i3_other() {
        assert_eq!(classify_desktop(Some("KDE"), None), Desktop::Kde);
        assert_eq!(classify_desktop(Some("plasma"), None), Desktop::Kde);
        assert_eq!(classify_desktop(Some("sway"), None), Desktop::Sway);
        assert_eq!(classify_desktop(Some("i3"), None), Desktop::I3);
        assert_eq!(classify_desktop(Some("xfce"), None), Desktop::Other);
        assert_eq!(classify_desktop(None, None), Desktop::Other);
    }

    #[test]
    fn reports_missing_script_as_not_current() {
        let path = std::env::temp_dir().join(format!("cloche-grab-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(!grab_script_current(&path));
    }

    #[test]
    fn reports_matching_script_as_current() {
        let path = std::env::temp_dir().join(format!("cloche-grab-match-{}", std::process::id()));
        std::fs::write(&path, GRAB_SCRIPT).unwrap();
        assert!(grab_script_current(&path));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn allocates_first_free_slot() {
        let existing = vec![format!("{KEYBIND_PREFIX}custom0/")];
        assert_eq!(
            choose_binding_slot(&existing, None),
            format!("{KEYBIND_PREFIX}custom1/")
        );
    }

    #[test]
    fn reuses_existing_cloche_slot() {
        let slot = format!("{KEYBIND_PREFIX}custom3/");
        let existing = vec![slot.clone()];
        assert_eq!(choose_binding_slot(&existing, Some(&slot)), slot);
    }

    #[test]
    fn allocates_custom0_when_empty() {
        assert_eq!(
            choose_binding_slot(&[], None),
            format!("{KEYBIND_PREFIX}custom0/")
        );
    }

    #[test]
    fn recognizes_cloche_command_by_name_or_path() {
        assert!(is_cloche_command("cloche-grab"));
        assert!(is_cloche_command("  cloche-grab  "));
        assert!(is_cloche_command("/home/u/.local/bin/cloche-grab"));
        assert!(is_cloche_command("/home/u/bin/cloche-grab"));
        assert!(!is_cloche_command("flameshot"));
        assert!(!is_cloche_command("cloche-grab-helper"));
    }

    #[test]
    fn parses_gsettings_arrays() {
        assert_eq!(parse_gsettings_array("@as []"), Vec::<String>::new());
        assert_eq!(
            parse_gsettings_array("['/a/custom0/', '/a/custom1/']"),
            vec!["/a/custom0/".to_string(), "/a/custom1/".to_string()]
        );
    }
}

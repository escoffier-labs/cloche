//! Persisted styling preferences.
//!
//! Every backdrop decision used to require a flag on the command line, so the
//! most taste-driven knob in the tool was the least reachable one. This module
//! holds a small JSON file (`~/.config/cloche/config.json` by default) that
//! `capture` and `polish` read on every run: pin a palette or scene once, or
//! narrow the pool the random picker draws from, and both paths honor it with
//! no flags at all.
//!
//! A missing file means "use the built-in defaults" and is not an error. A
//! malformed one degrades to defaults with a warning rather than failing the
//! capture, because losing a screenshot to a typo in a preferences file is a
//! worse outcome than an unexpected backdrop.

use std::path::Path;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::polish;
use crate::util;
use crate::util::AppError;

/// Absolute path override, used by tests and by anyone keeping preferences
/// somewhere other than the platform config dir.
const CONFIG_ENV: &str = "CLOCHE_CONFIG";

/// The whole preferences file. Only `polish` exists today; new sections should
/// stay `#[serde(default)]` so old files keep loading.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ClocheConfig {
    pub polish: PolishPrefs,
}

/// Whether the persisted pins apply, or the picker stays random.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PolishMode {
    /// Draw from `palettes` / `scenes` (or the built-in pool when both are
    /// empty). `palette` and `scene` stay on file but are not applied.
    #[default]
    Random,
    /// Use `palette` / `scene` when they are set.
    Pinned,
}

/// Backdrop preferences honored by `cloche capture` and `cloche polish`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct PolishPrefs {
    pub mode: PolishMode,
    /// Palette applied when `mode` is `pinned`.
    pub palette: Option<String>,
    /// Scene applied when `mode` is `pinned`. Space palettes only.
    pub scene: Option<String>,
    /// Palettes the random picker may choose from. Empty means the built-in
    /// pool (every space palette).
    pub palettes: Vec<String>,
    /// Scenes the random picker may choose from. Empty leaves the scene to the
    /// seed's own pick inside the space renderer.
    pub scenes: Vec<String>,
}

/// Where the preferences file lives on this machine.
pub fn path() -> PathBuf {
    resolve_path(
        util::env_var(CONFIG_ENV),
        config_home_env(),
        util::home_dir(),
    )
}

/// `XDG_CONFIG_HOME` everywhere, falling back to `APPDATA` on Windows where
/// that is the platform's config root.
fn config_home_env() -> Option<String> {
    let xdg = util::env_var("XDG_CONFIG_HOME");
    if cfg!(windows) {
        xdg.or_else(|| util::env_var("APPDATA"))
    } else {
        xdg
    }
}

fn resolve_path(explicit: Option<String>, config_home: Option<String>, home: PathBuf) -> PathBuf {
    if let Some(explicit) = explicit {
        return PathBuf::from(explicit);
    }
    config_home
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("cloche")
        .join("config.json")
}

/// Read the preferences file, with any problems returned as warnings instead of
/// errors so a bad config never costs a capture.
pub fn load() -> (ClocheConfig, Vec<String>) {
    load_from(&path())
}

pub fn load_from(path: &Path) -> (ClocheConfig, Vec<String>) {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return (ClocheConfig::default(), Vec::new());
        }
        Err(err) => {
            return (
                ClocheConfig::default(),
                vec![format!(
                    "config {} could not be read: {err}",
                    path.display()
                )],
            );
        }
    };
    match serde_json::from_slice::<ClocheConfig>(&bytes) {
        Ok(config) => (config, Vec::new()),
        Err(err) => (
            ClocheConfig::default(),
            vec![format!(
                "config {} ignored, using defaults: {err}",
                path.display()
            )],
        ),
    }
}

/// Write the preferences file, creating the config dir when it is missing.
pub fn save_to(path: &Path, config: &ClocheConfig) -> Result<(), AppError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        util::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(config)
        .map_err(|err| AppError::Message(format!("config could not be serialized: {err}")))?;
    json.push('\n');
    util::write(path, json)
}

/// Resolve the style for one card from flags plus persisted preferences.
///
/// Precedence is explicit flag, then config, then the built-in random pick, so
/// a one-off `--palette` always beats a pinned preference.
pub fn resolve_style(
    prefs: &PolishPrefs,
    seed: Option<u64>,
    palette: Option<&str>,
    scene: Option<&str>,
    warnings: &mut Vec<String>,
) -> polish::PresentationStyle {
    let seed = seed.unwrap_or_else(polish::random_seed);
    let pinned = prefs.mode == PolishMode::Pinned;
    let palette = palette.or_else(|| pinned.then_some(prefs.palette.as_deref()).flatten());
    let scene = scene.or_else(|| pinned.then_some(prefs.scene.as_deref()).flatten());

    let palettes = keep_known(
        &prefs.palettes,
        &polish::palette_names(),
        "palette",
        warnings,
    );
    let scenes = keep_known(&prefs.scenes, &polish::scene_names(), "scene", warnings);

    let mut style = match palette {
        Some(name) => polish::style_with_palette(seed, name).unwrap_or_else(|| {
            warnings.push(format!("unknown palette: {name}"));
            polish::style_from_seed_in_pool(seed, &palettes, &scenes)
        }),
        None => polish::style_from_seed_in_pool(seed, &palettes, &scenes),
    };

    if let Some(name) = scene {
        match polish::scene_from_name(name) {
            Some(kind) if style.is_space() => style.scene = Some(kind),
            Some(_) => warnings.push(format!(
                "scene {name} ignored: palette {} is not a space scene",
                style.palette_name
            )),
            None => warnings.push(format!("unknown scene: {name}")),
        }
    }
    style
}

/// Keep the names the built-in tables recognize, reporting the rest. A name
/// that no longer exists (a renamed palette, say) should be visible rather than
/// silently narrowing the pool.
fn keep_known(
    values: &[String],
    known: &[&str],
    label: &str,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for value in values {
        if !known.iter().any(|name| name == value) {
            warnings.push(format!("config {label} pool: unknown {label} {value}"));
        } else if !kept.iter().any(|name| name == value) {
            kept.push(value.clone());
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cloche-config-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join("config.json")
    }

    fn pinned(palette: Option<&str>, scene: Option<&str>) -> PolishPrefs {
        PolishPrefs {
            mode: PolishMode::Pinned,
            palette: palette.map(str::to_string),
            scene: scene.map(str::to_string),
            ..PolishPrefs::default()
        }
    }

    #[test]
    fn missing_file_loads_defaults_without_warning() {
        let (config, warnings) = load_from(Path::new("/nonexistent/cloche/config.json"));
        assert_eq!(config, ClocheConfig::default());
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }

    #[test]
    fn malformed_file_warns_and_falls_back_to_defaults() {
        let path = temp_config("malformed");
        std::fs::write(&path, b"{ not json").expect("write");
        let (config, warnings) = load_from(&path);
        assert_eq!(config, ClocheConfig::default());
        assert_eq!(warnings.len(), 1, "warnings: {warnings:?}");
        assert!(warnings[0].contains("using defaults"));
    }

    #[test]
    fn partial_file_keeps_defaults_for_absent_fields() {
        let path = temp_config("partial");
        std::fs::write(&path, br#"{"polish":{"palette":"aurora-teal"}}"#).expect("write");
        let (config, warnings) = load_from(&path);
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
        assert_eq!(config.polish.mode, PolishMode::Random);
        assert_eq!(config.polish.palette.as_deref(), Some("aurora-teal"));
        assert!(config.polish.scenes.is_empty());
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_config("round-trip");
        let config = ClocheConfig {
            polish: PolishPrefs {
                mode: PolishMode::Pinned,
                palette: Some("violet-haze".to_string()),
                scene: Some("jwst".to_string()),
                palettes: vec!["orion-emission".to_string()],
                scenes: vec!["alma".to_string()],
            },
        };
        save_to(&path, &config).expect("save");
        let (loaded, warnings) = load_from(&path);
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
        assert_eq!(loaded, config);
    }

    #[test]
    fn saved_file_uses_lowercase_mode_names() {
        let path = temp_config("mode-name");
        save_to(
            &path,
            &ClocheConfig {
                polish: pinned(None, None),
            },
        )
        .expect("save");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("\"mode\": \"pinned\""), "{text}");
    }

    #[test]
    fn explicit_env_path_wins_over_config_home() {
        let path = resolve_path(
            Some("/tmp/pinned.json".to_string()),
            Some("/tmp/xdg".to_string()),
            PathBuf::from("/home/someone"),
        );
        assert_eq!(path, PathBuf::from("/tmp/pinned.json"));
    }

    #[test]
    fn config_home_is_used_before_the_home_fallback() {
        let with_home = resolve_path(None, Some("/tmp/xdg".to_string()), PathBuf::from("/home/x"));
        assert_eq!(with_home, PathBuf::from("/tmp/xdg/cloche/config.json"));
        let without = resolve_path(None, None, PathBuf::from("/home/x"));
        assert_eq!(without, PathBuf::from("/home/x/.config/cloche/config.json"));
    }

    #[test]
    fn flag_palette_beats_a_pinned_config_palette() {
        let mut warnings = Vec::new();
        let style = resolve_style(
            &pinned(Some("violet-haze"), None),
            Some(7),
            Some("aurora-teal"),
            None,
            &mut warnings,
        );
        assert_eq!(style.palette_name, "aurora-teal");
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }

    #[test]
    fn pinned_config_palette_applies_without_any_flag() {
        let mut warnings = Vec::new();
        let style = resolve_style(
            &pinned(Some("violet-haze"), None),
            Some(7),
            None,
            None,
            &mut warnings,
        );
        assert_eq!(style.palette_name, "violet-haze");
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }

    #[test]
    fn random_mode_ignores_the_pinned_palette() {
        let prefs = PolishPrefs {
            palette: Some("violet-haze".to_string()),
            ..PolishPrefs::default()
        };
        let mut warnings = Vec::new();
        let style = resolve_style(&prefs, Some(7), None, None, &mut warnings);
        assert_eq!(style.palette_name, polish::style_from_seed(7).palette_name);
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }

    #[test]
    fn random_pool_narrows_the_picker_across_seeds() {
        let prefs = PolishPrefs {
            palettes: vec!["aurora-teal".to_string(), "orion-emission".to_string()],
            ..PolishPrefs::default()
        };
        for seed in 0..40u64 {
            let mut warnings = Vec::new();
            let style = resolve_style(&prefs, Some(seed), None, None, &mut warnings);
            assert!(
                prefs.palettes.contains(&style.palette_name),
                "seed {seed} escaped the pool with {}",
                style.palette_name
            );
        }
    }

    #[test]
    fn pinned_config_scene_applies_and_is_kept_off_gradients() {
        let mut warnings = Vec::new();
        let space = resolve_style(
            &pinned(Some("orion-emission"), Some("jwst")),
            Some(7),
            None,
            None,
            &mut warnings,
        );
        assert_eq!(space.scene, polish::scene_from_name("jwst"));
        assert!(warnings.is_empty(), "warnings: {warnings:?}");

        let mut warnings = Vec::new();
        let gradient = resolve_style(
            &pinned(Some("aurora-teal"), Some("jwst")),
            Some(7),
            None,
            None,
            &mut warnings,
        );
        assert_eq!(gradient.scene, None);
        assert_eq!(warnings.len(), 1, "warnings: {warnings:?}");
        assert!(warnings[0].contains("not a space scene"));
    }

    #[test]
    fn unknown_pool_names_are_reported_and_dropped() {
        let prefs = PolishPrefs {
            palettes: vec!["orion-emission".to_string(), "not-a-palette".to_string()],
            scenes: vec!["not-a-scene".to_string()],
            ..PolishPrefs::default()
        };
        let mut warnings = Vec::new();
        let style = resolve_style(&prefs, Some(7), None, None, &mut warnings);
        assert_eq!(style.palette_name, "orion-emission");
        assert_eq!(warnings.len(), 2, "warnings: {warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("not-a-palette")));
        assert!(warnings.iter().any(|w| w.contains("not-a-scene")));
    }

    #[test]
    fn duplicate_pool_entries_do_not_skew_the_draw() {
        let known = ["aurora-teal", "orion-emission"];
        let values = vec![
            "aurora-teal".to_string(),
            "aurora-teal".to_string(),
            "orion-emission".to_string(),
        ];
        let mut warnings = Vec::new();
        let kept = keep_known(&values, &known, "palette", &mut warnings);
        assert_eq!(kept, vec!["aurora-teal", "orion-emission"]);
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }
}

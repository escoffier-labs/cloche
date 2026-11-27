use std::path::Path;
use std::process::ExitCode;

use chrono::Utc;

use crate::cli::ClearTarget;
use crate::cli::ConfigArgs;
use crate::cli::ConfigCommand;
use crate::cli::ConfigPolishMode;
use crate::cli::ConfigSetArgs;
use crate::config;
use crate::polish;

use super::VERSION;
use super::print_json;

pub(crate) fn config(args: ConfigArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let result = run_config(args.command, &config::path());
    print_json(&result)?;
    Ok(if result.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

pub(super) fn run_config(command: ConfigCommand, path: &Path) -> crate::contract::ConfigResult {
    let (mut stored, mut warnings) = config::load_from(path);
    let mut errors = Vec::new();
    let mut options = None;

    match command {
        ConfigCommand::Show => {}
        ConfigCommand::Options => options = Some(style_options()),
        ConfigCommand::Set(args) => {
            apply_config_set(&mut stored.polish, &args);
            // A pinned scene on a gradient palette never renders, so say so at
            // the moment it is stored rather than on every later capture.
            if let Some(warning) = pinned_scene_conflict(&stored.polish) {
                warnings.push(warning);
            }
            if let Err(err) = config::save_to(path, &stored) {
                errors.push(err.to_string());
            }
        }
    }

    crate::contract::ConfigResult {
        ok: errors.is_empty(),
        version: VERSION.to_string(),
        created_at: Utc::now(),
        path: path.to_path_buf(),
        exists: path.exists(),
        config: stored,
        options,
        warnings,
        errors,
    }
}

fn apply_config_set(prefs: &mut config::PolishPrefs, args: &ConfigSetArgs) {
    // Clears run first so `--clear all --palette x` reads left to right: reset,
    // then apply what was named.
    for target in &args.clear {
        match target {
            ClearTarget::Palette => prefs.palette = None,
            ClearTarget::Scene => prefs.scene = None,
            ClearTarget::Motif => prefs.motif = None,
            ClearTarget::Sky => prefs.sky = None,
            ClearTarget::Terrain => prefs.terrain = None,
            ClearTarget::Palettes => prefs.palettes.clear(),
            ClearTarget::Scenes => prefs.scenes.clear(),
            ClearTarget::Motifs => prefs.motifs.clear(),
            ClearTarget::Skies => prefs.skies.clear(),
            ClearTarget::Terrains => prefs.terrains.clear(),
            ClearTarget::All => *prefs = config::PolishPrefs::default(),
        }
    }
    if let Some(mode) = args.mode {
        prefs.mode = match mode {
            ConfigPolishMode::Random => config::PolishMode::Random,
            ConfigPolishMode::Pinned => config::PolishMode::Pinned,
        };
    }
    if let Some(palette) = args.palette.clone() {
        prefs.palette = Some(palette);
    }
    if let Some(scene) = args.scene.clone() {
        prefs.scene = Some(scene);
    }
    if let Some(motif) = args.motif.clone() {
        prefs.motif = Some(motif);
    }
    if let Some(sky) = args.sky.clone() {
        prefs.sky = Some(sky);
    }
    if let Some(terrain) = args.terrain.clone() {
        prefs.terrain = Some(terrain);
    }
    if let Some(palettes) = args.palettes.clone() {
        prefs.palettes = palettes;
    }
    if let Some(scenes) = args.scenes.clone() {
        prefs.scenes = scenes;
    }
    if let Some(motifs) = args.motifs.clone() {
        prefs.motifs = motifs;
    }
    if let Some(skies) = args.skies.clone() {
        prefs.skies = skies;
    }
    if let Some(terrains) = args.terrains.clone() {
        prefs.terrains = terrains;
    }
}

/// A pinned scene only renders behind a space palette; pairing it with a
/// gradient silently drops the scene.
fn pinned_scene_conflict(prefs: &config::PolishPrefs) -> Option<String> {
    let scene = prefs.scene.as_deref()?;
    let palette = prefs.palette.as_deref()?;
    let is_space = polish::style_with_palette(0, palette)
        .map(|style| style.is_space())
        .unwrap_or(true);
    if is_space {
        return None;
    }
    Some(format!(
        "scene {scene} will be ignored while palette {palette} is pinned: it is not a space scene"
    ))
}

fn style_options() -> crate::contract::StyleOptions {
    crate::contract::StyleOptions {
        palettes: polish::palette_catalog()
            .into_iter()
            .map(|(name, kind)| crate::contract::PaletteOption {
                name: name.to_string(),
                kind: kind.to_string(),
            })
            .collect(),
        scenes: polish::scene_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
        motifs: polish::motif_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
        skies: polish::sky_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
        terrains: polish::terrain_names()
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

#[cfg(test)]
pub(super) fn config_set(args: ConfigSetArgs, path: &Path) -> crate::contract::ConfigResult {
    run_config(ConfigCommand::Set(Box::new(args)), path)
}

#[cfg(test)]
pub(super) fn empty_set_args() -> ConfigSetArgs {
    ConfigSetArgs {
        mode: None,
        palette: None,
        scene: None,
        palettes: None,
        scenes: None,
        motif: None,
        motifs: None,
        sky: None,
        skies: None,
        terrain: None,
        terrains: None,
        clear: Vec::new(),
        format: crate::cli::OutputFormat::Json,
    }
}

#[cfg(test)]
mod tests {
    use super::super::temp_dir;
    use super::*;

    #[test]
    fn config_show_reports_defaults_before_anything_is_written() {
        let dir = temp_dir("config-show");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Show, &path);
        assert!(result.ok, "errors: {:?}", result.errors);
        assert!(!result.exists);
        assert_eq!(result.config, config::ClocheConfig::default());
        assert!(result.options.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_persists_and_reloads() {
        let dir = temp_dir("config-set");
        let path = dir.join("config.json");
        let written = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palettes: Some(vec![
                    "orion-emission".to_string(),
                    "milkyway-core".to_string(),
                ]),
                scenes: Some(vec!["alma".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        assert!(written.ok, "errors: {:?}", written.errors);
        assert!(written.exists);

        let reloaded = run_config(ConfigCommand::Show, &path);
        assert_eq!(reloaded.config.polish.mode, config::PolishMode::Pinned);
        assert_eq!(
            reloaded.config.polish.palettes,
            vec!["orion-emission", "milkyway-core"]
        );
        assert_eq!(reloaded.config.polish.scenes, vec!["alma"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_clear_runs_before_the_values_on_the_same_line() {
        let dir = temp_dir("config-clear");
        let path = dir.join("config.json");
        config_set(
            ConfigSetArgs {
                palette: Some("aurora-teal".to_string()),
                scenes: Some(vec!["alma".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        let result = config_set(
            ConfigSetArgs {
                clear: vec![ClearTarget::All],
                palette: Some("ember-glow".to_string()),
                ..empty_set_args()
            },
            &path,
        );
        assert_eq!(result.config.polish.palette.as_deref(), Some("ember-glow"));
        assert!(result.config.polish.scenes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_warns_when_a_pinned_scene_cannot_render() {
        let dir = temp_dir("config-conflict");
        let path = dir.join("config.json");
        let result = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("aurora-teal".to_string()),
                scene: Some("jwst".to_string()),
                ..empty_set_args()
            },
            &path,
        );
        assert!(result.ok, "errors: {:?}", result.errors);
        assert_eq!(result.warnings.len(), 1, "warnings: {:?}", result.warnings);
        assert!(result.warnings[0].contains("not a space scene"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_options_lists_every_palette_and_scene() {
        let dir = temp_dir("config-options");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Options, &path);
        let options = result.options.expect("options");
        assert_eq!(options.palettes.len(), polish::palette_names().len());
        assert_eq!(options.scenes, polish::scene_names());
        assert!(
            options
                .palettes
                .iter()
                .any(|option| option.name == "aurora-teal" && option.kind == "gradient")
        );
        assert!(
            options
                .palettes
                .iter()
                .any(|option| option.name == "orion-emission" && option.kind == "space")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_options_expose_the_terrain_names() {
        let dir = temp_dir("config-options-terrain");
        let path = dir.join("config.json");
        let result = run_config(ConfigCommand::Options, &path);
        let options = result.options.expect("options");
        assert_eq!(options.terrains, polish::terrain_names());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_set_persists_the_terrain_pin() {
        let dir = temp_dir("config-set-terrain");
        let path = dir.join("config.json");
        let written = config_set(
            ConfigSetArgs {
                mode: Some(ConfigPolishMode::Pinned),
                palette: Some("dunes".to_string()),
                terrain: Some("dunes".to_string()),
                terrains: Some(vec!["dunes".to_string(), "mesa".to_string()]),
                ..empty_set_args()
            },
            &path,
        );
        assert!(written.ok, "errors: {:?}", written.errors);
        let reloaded = run_config(ConfigCommand::Show, &path);
        assert_eq!(reloaded.config.polish.terrain.as_deref(), Some("dunes"));
        assert_eq!(reloaded.config.polish.terrains, vec!["dunes", "mesa"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

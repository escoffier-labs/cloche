use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;
use clap::ValueEnum;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureTarget {
    Active,
    Screen,
    Window,
    Region,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

impl std::fmt::Display for ImageDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ImageDetail::Auto => "auto",
            ImageDetail::Low => "low",
            ImageDetail::High => "high",
            ImageDetail::Original => "original",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppshotResult {
    pub ok: bool,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub target: CaptureTarget,
    pub backend: Option<BackendInfo>,
    /// Directory where flat capture artifacts were written (`--out-dir` /
    /// default `~/Pictures/Cloche`). Not a per-shot folder.
    #[serde(alias = "outputDir")]
    pub out_dir: PathBuf,
    pub image: Option<ImageInfo>,
    pub presentation_image: Option<ImageInfo>,
    pub presentation_style: Option<PresentationStyleInfo>,
    pub window: Option<WindowInfo>,
    pub text: TextInfo,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Result of styling an existing image into a presentation card.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PolishResult {
    pub ok: bool,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub input: Option<ImageInfo>,
    pub card: Option<ImageInfo>,
    pub presentation_style: Option<PresentationStyleInfo>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Result of a `cloche config` command: the effective preferences, where they
/// live, and the backdrop menu a picker can render.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResult {
    pub ok: bool,
    pub version: String,
    pub created_at: DateTime<Utc>,
    /// The preferences file this run read or wrote.
    pub path: PathBuf,
    /// False when no file exists yet and the defaults are in play.
    pub exists: bool,
    pub config: crate::config::ClocheConfig,
    /// Present for `cloche config options`: every backdrop the pickers accept.
    pub options: Option<StyleOptions>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Every backdrop choice, for menus and for anything building a picker UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StyleOptions {
    pub palettes: Vec<PaletteOption>,
    pub scenes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaletteOption {
    pub name: String,
    /// `gradient` or `space`. Scenes only apply to space palettes.
    pub kind: String,
}

/// Result of rendering a short Cloche reel from an existing recording.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReelRenderResult {
    pub ok: bool,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub engine: String,
    pub input: PathBuf,
    pub output: Option<VideoInfo>,
    pub props: Option<PathBuf>,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub path: PathBuf,
    pub bytes: u64,
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub name: String,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub path: PathBuf,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bytes: u64,
    pub mime: String,
    pub detail: ImageDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresentationStyleInfo {
    pub seed: u64,
    pub palette: String,
    pub padding: u32,
    pub corner_radius: u32,
    pub shadow_blur: f32,
    pub shadow_offset_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: Option<String>,
    pub title: Option<String>,
    pub app_name: Option<String>,
    pub pid: Option<u32>,
    pub geometry: Option<Geometry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub screen: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextInfo {
    pub available: bool,
    pub path: Option<PathBuf>,
    pub bytes: u64,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WindowList {
    pub ok: bool,
    pub backend: Option<BackendInfo>,
    pub windows: Vec<WindowInfo>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub ok: bool,
    pub version: String,
    pub session: SessionInfo,
    pub helpers: Vec<HelperStatus>,
    pub capabilities: Vec<CapabilityStatus>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub xdg_session_type: Option<String>,
    pub wayland_display: Option<String>,
    pub display: Option<String>,
    pub current_desktop: Option<String>,
    pub desktop_session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelperStatus {
    pub name: String,
    pub available: bool,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStatus {
    pub name: String,
    pub available: bool,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use serde_json::json;

    use super::AppshotResult;
    use super::CaptureTarget;
    use super::TextInfo;

    #[test]
    fn region_target_uses_camel_case_wire_value() {
        let value = serde_json::to_value(CaptureTarget::Region).expect("serialize");
        assert_eq!(value, json!("region"));
    }

    #[test]
    fn appshot_result_uses_camel_case_wire_keys() {
        let result = AppshotResult {
            ok: false,
            version: "0.1.0".to_string(),
            created_at: Utc::now(),
            target: CaptureTarget::Active,
            backend: None,
            out_dir: "/tmp/appshot".into(),
            image: None,
            presentation_image: None,
            presentation_style: None,
            window: None,
            text: TextInfo::default(),
            warnings: Vec::new(),
            errors: vec!["no display".to_string()],
        };

        let value = serde_json::to_value(result).expect("serialize appshot result");

        assert!(value["createdAt"].is_string());
        assert_eq!(value["outDir"], json!("/tmp/appshot"));
        assert!(
            value.get("outputDir").is_none(),
            "legacy outputDir must not be emitted on the wire"
        );
        assert_eq!(value["target"], json!("active"));
    }

    #[test]
    fn appshot_result_accepts_legacy_output_dir_alias() {
        let value = json!({
            "ok": true,
            "version": "0.0.0",
            "createdAt": "2026-07-25T00:00:00Z",
            "target": "active",
            "outputDir": "/tmp/legacy-shot",
            "text": { "available": false, "bytes": 0 },
            "warnings": [],
            "errors": []
        });
        let parsed: AppshotResult =
            serde_json::from_value(value).expect("deserialize legacy outputDir");
        assert_eq!(parsed.out_dir, PathBuf::from("/tmp/legacy-shot"));
    }
}

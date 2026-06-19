//! HyperFrames reel engine.
//!
//! A second `cloche reels render` engine alongside Remotion. It takes the same
//! input video + AppReels-shaped cue JSON, generates a self-contained
//! HyperFrames composition (HTML is the source of truth for video), drops a
//! Cloche-branded `DESIGN.md` next to it so the reel shares the still
//! `shot-card` palette, and renders it with `npx hyperframes render`.
//!
//! HyperFrames is invoked through `npx` (no vendored node project, unlike the
//! Remotion engine). Override the launcher with `CLOCHE_HYPERFRAMES_CMD`.

use std::path::Path;
use std::path::PathBuf;

use crate::contract::VideoInfo;
use crate::design;
use crate::polish::PresentationStyle;
use crate::util;

const COMPOSITION_ID: &str = "cloche-reel";
const GSAP_CDN: &str = "https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js";

/// A single on-screen caption, derived from the cue JSON.
#[derive(Debug, Clone)]
pub struct Caption {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub top: bool,
}

/// Everything the HTML generator needs to lay out one reel.
#[derive(Debug, Clone)]
pub struct ReelComposition<'a> {
    /// Video filename relative to the project dir (sits next to index.html).
    pub video_file: &'a str,
    pub title: &'a str,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub style: &'a PresentationStyle,
    pub captions: Vec<Caption>,
    pub title_card_ms: u64,
    pub outro_card_ms: u64,
    pub outro_text: Option<String>,
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn secs(ms: u64) -> f64 {
    ms as f64 / 1000.0
}

/// Parse AppReels-shaped cues into the typed pieces the composition needs.
/// Shape: `{ captions: [{startMs,endMs,text,position}], titleCard: {text,ms},
/// outroCard: {text,ms} }`. Unknown keys are ignored.
pub fn captions_from_cues(cues: &serde_json::Value) -> Vec<Caption> {
    let mut out = Vec::new();
    if let Some(items) = cues.get("captions").and_then(|v| v.as_array()) {
        for item in items {
            let start_ms = item.get("startMs").and_then(|v| v.as_u64()).unwrap_or(0);
            let end_ms = item
                .get("endMs")
                .and_then(|v| v.as_u64())
                .unwrap_or(start_ms);
            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() || end_ms <= start_ms {
                continue;
            }
            let top = item.get("position").and_then(|v| v.as_str()) == Some("top");
            out.push(Caption {
                start_ms,
                end_ms,
                text,
                top,
            });
        }
    }
    out
}

fn card_text(cues: &serde_json::Value, key: &str) -> Option<String> {
    cues.get(key)
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn card_ms(cues: &serde_json::Value, key: &str, default: u64) -> u64 {
    cues.get(key)
        .and_then(|v| v.get("ms"))
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
}

/// Render the standalone HyperFrames `index.html` for a reel. Pure: no I/O, so
/// it is fully unit-testable. Deterministic, no `Math.random`/`Date.now`, no
/// `repeat: -1`, and registers the timeline on `window.__timelines`.
pub fn composition_html(comp: &ReelComposition) -> String {
    let style = comp.style;
    let bg_deep = design::hex(style.stops[0]);
    let bg_mid = design::hex(style.stops[1]);
    let glow_a = design::hex(style.glow_a);
    let glow_b = design::hex(style.glow_b);
    let total = secs(comp.duration_ms);
    let title_secs = secs(comp.title_card_ms.min(comp.duration_ms));
    let outro_secs = secs(comp.outro_card_ms.min(comp.duration_ms));
    let outro_start = (total - outro_secs).max(0.0);
    let title = esc(comp.title);
    let outro = esc(comp.outro_text.as_deref().unwrap_or(comp.title));

    // Caption clips: one distinct track each so overlapping cues never trip the
    // "same-track clips cannot overlap" lint. Visual layering is via z-index.
    let mut caption_clips = String::new();
    let mut caption_tweens = String::new();
    for (i, cap) in comp.captions.iter().enumerate() {
        let start = secs(cap.start_ms);
        let dur = secs(cap.end_ms.saturating_sub(cap.start_ms));
        let pos = if cap.top {
            "caption-top"
        } else {
            "caption-bottom"
        };
        caption_clips.push_str(&format!(
            "    <div id=\"cap-{i}\" class=\"clip caption {pos}\" data-start=\"{start:.3}\" \
data-duration=\"{dur:.3}\" data-track-index=\"{track}\">{text}</div>\n",
            i = i,
            pos = pos,
            start = start,
            dur = dur,
            track = 10 + i,
            text = esc(&cap.text),
        ));
        // Entrance only; the framework hides the clip at the end of its window.
        caption_tweens.push_str(&format!(
            "    tl.from(\"#cap-{i}\", {{ opacity: 0, y: 28, duration: 0.32, ease: \"power2.out\" }}, {start:.3});\n",
            i = i,
            start = start,
        ));
    }

    format!(
        "<!doctype html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\" />\n\
  <title>{title} - Cloche Reel</title>\n\
</head>\n\
<body>\n\
  <div id=\"{comp_id}\" data-composition-id=\"{comp_id}\" data-start=\"0\" data-duration=\"{total:.3}\" \
data-width=\"{w}\" data-height=\"{h}\">\n\
    <div class=\"bg\" data-layout-ignore></div>\n\
    <div class=\"glow glow-a\" data-layout-ignore></div>\n\
    <div class=\"glow glow-b\" data-layout-ignore></div>\n\
    <div class=\"stage\">\n\
      <div class=\"chrome\">\n\
        <div class=\"toolbar\">\n\
          <span class=\"dot dot-a\"></span><span class=\"dot dot-b\"></span><span class=\"dot dot-c\"></span>\n\
          <span class=\"toolbar-title\">{title}</span>\n\
        </div>\n\
        <div class=\"video-shell\">\n\
          <video id=\"clip-video\" data-start=\"0\" data-duration=\"{total:.3}\" \
data-track-index=\"0\" src=\"{video}\" muted playsinline crossorigin=\"anonymous\"></video>\n\
        </div>\n\
      </div>\n\
    </div>\n\
{caption_clips}\
    <div id=\"title-card\" class=\"clip card\" data-start=\"0\" data-duration=\"{title_secs:.3}\" \
data-track-index=\"100\"><div class=\"card-text\">{title}</div></div>\n\
    <div id=\"outro-card\" class=\"clip card\" data-start=\"{outro_start:.3}\" \
data-duration=\"{outro_secs:.3}\" data-track-index=\"101\"><div class=\"card-text\">{outro}</div></div>\n\
    <style>\n\
      #{comp_id} {{\n\
        font-family: {font};\n\
        color: {text};\n\
        overflow: hidden;\n\
      }}\n\
      #{comp_id} .bg {{\n\
        position: absolute; inset: 0; z-index: 0;\n\
        background: radial-gradient(120% 90% at 70% 8%, {bg_mid} 0%, {bg_deep} 60%, {bg_deep} 100%);\n\
      }}\n\
      #{comp_id} .glow {{ position: absolute; filter: blur(64px); z-index: 1; opacity: 0.55; }}\n\
      #{comp_id} .glow-a {{ inset: 4% 14% 56% 8%; background: {glow_a}; transform: rotate(-12deg); }}\n\
      #{comp_id} .glow-b {{ inset: 48% 8% 7% 18%; background: {glow_b}; transform: rotate(9deg); }}\n\
      #{comp_id} .stage {{ position: absolute; inset: 88px; display: flex; align-items: center; justify-content: center; z-index: 10; }}\n\
      #{comp_id} .chrome {{ width: 100%; max-height: 100%; aspect-ratio: 16 / 10; border-radius: 24px; overflow: hidden; background: #101820; box-shadow: 0 44px 110px rgba(4,12,18,0.46), 0 18px 38px rgba(4,12,18,0.34); }}\n\
      #{comp_id} .toolbar {{ height: 54px; display: flex; align-items: center; gap: 12px; padding: 0 20px; background: rgba(11,18,24,0.94); }}\n\
      #{comp_id} .dot {{ width: 14px; height: 14px; border-radius: 99px; flex: 0 0 auto; }}\n\
      #{comp_id} .dot-a {{ background: #ff6b6b; }} [data-composition-id=\"{comp_id}\"] .dot-b {{ background: #feca57; }} [data-composition-id=\"{comp_id}\"] .dot-c {{ background: #1dd1a1; }}\n\
      #{comp_id} .toolbar-title {{ margin-left: 12px; color: rgba(248,251,255,0.82); font-size: 20px; font-weight: 700; line-height: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}\n\
      #{comp_id} .video-shell {{ position: relative; height: calc(100% - 54px); background: #070b10; overflow: hidden; }}\n\
      #{comp_id} #clip-video {{ width: 100%; height: 100%; object-fit: contain; }}\n\
      #{comp_id} .caption {{ position: absolute; left: 96px; right: 96px; z-index: 30; padding: 24px 30px; border-radius: 20px; background: rgba(7,13,19,0.80); border: 1px solid rgba(255,255,255,0.18); box-shadow: 0 20px 42px rgba(0,0,0,0.30); font-size: 42px; font-weight: 850; line-height: 1.14; text-align: center; }}\n\
      #{comp_id} .caption-top {{ top: 116px; }} [data-composition-id=\"{comp_id}\"] .caption-bottom {{ bottom: 116px; }}\n\
      #{comp_id} .card {{ position: absolute; inset: 0; z-index: 50; display: flex; align-items: center; justify-content: center; padding: 96px; background: rgba(6,11,16,0.86); }}\n\
      #{comp_id} .card-text {{ max-width: 820px; color: #ffffff; font-size: 74px; font-weight: 900; line-height: 1.04; text-align: center; }}\n\
    </style>\n\
    <script src=\"{gsap}\"></script>\n\
    <script>\n\
      window.__timelines = window.__timelines || {{}};\n\
      const tl = gsap.timeline({{ paused: true }});\n\
      // Title card: fade in, hold, fade out within its own clip window.\n\
      tl.from(\"#title-card\", {{ opacity: 0, duration: 0.4, ease: \"power2.out\" }}, 0);\n\
      tl.from(\"#title-card .card-text\", {{ y: 36, opacity: 0, duration: 0.5, ease: \"power3.out\" }}, 0.1);\n\
      tl.to(\"#title-card\", {{ opacity: 0, duration: 0.4, ease: \"power2.in\" }}, {title_fade:.3});\n\
{caption_tweens}\
      // Outro card: fade in near the end.\n\
      tl.from(\"#outro-card\", {{ opacity: 0, duration: 0.4, ease: \"power2.out\" }}, {outro_start:.3});\n\
      tl.from(\"#outro-card .card-text\", {{ scale: 0.92, opacity: 0, duration: 0.5, ease: \"power2.out\" }}, {outro_text_in:.3});\n\
      window.__timelines[\"{comp_id}\"] = tl;\n\
    </script>\n\
  </div>\n\
</body>\n\
</html>\n",
        comp_id = COMPOSITION_ID,
        title = title,
        outro = outro,
        video = esc(comp.video_file),
        font = design::FONT_STACK,
        text = design::CANVAS_TEXT,
        gsap = GSAP_CDN,
        total = total,
        w = comp.width,
        h = comp.height,
        title_secs = title_secs,
        title_fade = (title_secs - 0.4).max(0.0),
        outro_start = outro_start,
        outro_secs = outro_secs,
        outro_text_in = (outro_start + 0.12).min(total),
        bg_deep = bg_deep,
        bg_mid = bg_mid,
        glow_a = glow_a,
        glow_b = glow_b,
        caption_clips = caption_clips,
        caption_tweens = caption_tweens,
    )
}

/// The `npx hyperframes`-style launcher, split into program + leading args.
/// Honors `CLOCHE_HYPERFRAMES_CMD` (whitespace-split) for non-standard setups.
fn launcher() -> Vec<String> {
    match std::env::var("CLOCHE_HYPERFRAMES_CMD") {
        Ok(cmd) if !cmd.trim().is_empty() => cmd.split_whitespace().map(str::to_string).collect(),
        _ => vec!["npx".to_string(), "hyperframes".to_string()],
    }
}

/// Render a reel with the HyperFrames engine. Mirrors the Remotion path's
/// out-params so the caller stays engine-agnostic.
#[allow(clippy::too_many_arguments)]
pub fn render(
    input: &Path,
    out: &Path,
    cues: &serde_json::Value,
    duration_ms: u64,
    fps: u32,
    width: u32,
    height: u32,
    workers: u32,
    style: &PresentationStyle,
    title: &str,
    keep_project: bool,
    errors: &mut Vec<String>,
    props_path: &mut Option<PathBuf>,
    output: &mut Option<VideoInfo>,
) {
    let out_parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let project_dir = out_parent.join(format!(".cloche-reel-hf-{}", std::process::id()));
    if let Err(err) = util::create_dir_all(&project_dir) {
        errors.push(err.to_string());
        return;
    }

    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let video_file = format!("input.{ext}");
    if let Err(err) = std::fs::copy(input, project_dir.join(&video_file)) {
        errors.push(format!("input video could not be staged: {err}"));
        let _ = std::fs::remove_dir_all(&project_dir);
        return;
    }

    let comp = ReelComposition {
        video_file: &video_file,
        title,
        duration_ms,
        width,
        height,
        style,
        captions: captions_from_cues(cues),
        title_card_ms: card_ms(cues, "titleCard", 900),
        outro_card_ms: card_ms(cues, "outroCard", 0),
        outro_text: card_text(cues, "outroCard"),
    };
    let index = project_dir.join("index.html");
    if let Err(err) = util::write(&index, composition_html(&comp).into_bytes()) {
        errors.push(err.to_string());
        let _ = std::fs::remove_dir_all(&project_dir);
        return;
    }
    // Share the still card's palette with the reel: drop a brand sheet next to
    // the composition so a human re-opening the project sees the same identity.
    let _ = util::write(
        &project_dir.join("DESIGN.md"),
        design::design_md(style, title).into_bytes(),
    );

    let out_abs = std::path::absolute(out).unwrap_or_else(|_| out.to_path_buf());
    let fps_str = fps.to_string();
    let workers_str = workers.max(1).to_string();
    let launcher = launcher();
    let (program, lead) = launcher.split_first().expect("launcher is non-empty");
    let render = std::process::Command::new(program)
        .current_dir(&project_dir)
        .args(lead)
        .args([
            "render",
            "--output",
            &out_abs.display().to_string(),
            "--fps",
            &fps_str,
            "--workers",
            &workers_str,
        ])
        .output();

    match render {
        Ok(result) if result.status.success() => match util::file_size(out) {
            Ok(bytes) => {
                *output = Some(VideoInfo {
                    path: util::canonical_or_original(out),
                    bytes,
                    mime: "video/mp4".to_string(),
                });
            }
            Err(err) => errors.push(err.to_string()),
        },
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            errors.push(format!(
                "HyperFrames render failed with status {}: {}{}",
                result.status,
                stderr.trim(),
                stdout.trim()
            ));
        }
        Err(err) => errors.push(format!(
            "failed to launch HyperFrames ({program}): {err}. Is Node.js + the \
hyperframes CLI installed? Override with CLOCHE_HYPERFRAMES_CMD."
        )),
    }

    if keep_project {
        *props_path = Some(util::canonical_or_original(&index));
    } else {
        let _ = std::fs::remove_dir_all(&project_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polish;

    fn sample_cues() -> serde_json::Value {
        serde_json::json!({
            "captions": [
                {"startMs": 1000, "endMs": 3000, "text": "First caption"},
                {"startMs": 3200, "endMs": 5000, "text": "Second & <bold>", "position": "top"}
            ],
            "titleCard": {"text": "Hello", "ms": 1200},
            "outroCard": {"text": "Subscribe", "ms": 1500}
        })
    }

    fn comp_html() -> String {
        let style = polish::style_with_palette(3, "midnight-sky").expect("known palette");
        let cues = sample_cues();
        let comp = ReelComposition {
            video_file: "input.mp4",
            title: "Cloche Demo",
            duration_ms: 8000,
            width: 1080,
            height: 1920,
            style: &style,
            captions: captions_from_cues(&cues),
            title_card_ms: 1200,
            outro_card_ms: 1500,
            outro_text: Some("Subscribe".to_string()),
        };
        composition_html(&comp)
    }

    #[test]
    fn captions_parsed_and_filtered() {
        let caps = captions_from_cues(&sample_cues());
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0].text, "First caption");
        assert!(caps[1].top);
    }

    #[test]
    fn captions_drop_empty_or_zero_length() {
        let cues = serde_json::json!({
            "captions": [
                {"startMs": 0, "endMs": 0, "text": "zero"},
                {"startMs": 100, "endMs": 200, "text": ""}
            ]
        });
        assert!(captions_from_cues(&cues).is_empty());
    }

    #[test]
    fn html_is_a_valid_standalone_composition() {
        let html = comp_html();
        assert!(html.contains("data-composition-id=\"cloche-reel\""));
        assert!(html.contains("data-width=\"1080\""));
        assert!(html.contains("data-height=\"1920\""));
        assert!(html.contains("src=\"input.mp4\""));
        assert!(html.contains("muted playsinline"));
        // Standalone root must not be wrapped in a <template>.
        assert!(!html.contains("<template"));
    }

    #[test]
    fn timed_elements_carry_the_clip_class() {
        // Without class="clip" the HyperFrames runtime shows timed elements for
        // the whole composition instead of only their scheduled window.
        let html = comp_html();
        assert!(html.contains("id=\"cap-0\" class=\"clip caption"));
        assert!(html.contains("id=\"title-card\" class=\"clip card\""));
        assert!(html.contains("id=\"outro-card\" class=\"clip card\""));
    }

    #[test]
    fn html_registers_the_timeline() {
        assert!(comp_html().contains("window.__timelines[\"cloche-reel\"] = tl;"));
    }

    #[test]
    fn html_carries_titles_and_captions_escaped() {
        let html = comp_html();
        assert!(html.contains("Cloche Demo"));
        assert!(html.contains("First caption"));
        // Caption text is HTML-escaped.
        assert!(html.contains("Second &amp; &lt;bold&gt;"));
    }

    #[test]
    fn html_obeys_determinism_rules() {
        let html = comp_html();
        assert!(!html.contains("Math.random"));
        assert!(!html.contains("Date.now"));
        assert!(!html.contains("repeat: -1"));
    }

    #[test]
    fn html_uses_the_palette_colors() {
        // midnight-sky stops[0] = [14, 24, 58] -> #0e183a
        assert!(comp_html().contains("#0e183a"));
    }
}

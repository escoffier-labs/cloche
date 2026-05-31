use crate::contract::TextInfo;
use crate::util;
use std::path::Path;

const ATSPI_SCRIPT: &str = r#"
import warnings
warnings.filterwarnings("ignore", category=DeprecationWarning)
import gi
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi

seen = set()
results = []

def add_text(text):
    text = " ".join(text.split())
    if text and text not in seen:
        seen.add(text)
        results.append(text)

def text_for(acc):
    try:
        text = acc.get_text(0, -1)
        if text and text.strip():
            add_text(text)
    except Exception:
        pass

def collect(acc, depth=0, active_only=False):
    if depth > 10:
        return
    try:
        state = acc.get_state_set()
        is_relevant = (
            state.contains(Atspi.StateType.FOCUSED)
            or state.contains(Atspi.StateType.ACTIVE)
            or state.contains(Atspi.StateType.SELECTED)
        )
    except Exception:
        is_relevant = False
    if not active_only or is_relevant:
        text_for(acc)
    try:
        count = acc.get_child_count()
    except Exception:
        return
    for idx in range(count):
        try:
            child = acc.get_child_at_index(idx)
        except Exception:
            continue
        collect(child, depth + 1, active_only)

desktop_count = Atspi.get_desktop_count()
for desktop_idx in range(desktop_count):
    collect(Atspi.get_desktop(desktop_idx), active_only=True)
if results:
    print("\n".join(results[:200]))
    raise SystemExit(0)
raise SystemExit(3)
"#;

pub fn extract(output_dir: &Path, warnings: &mut Vec<String>) -> TextInfo {
    if !util::has_command("python3") {
        warnings.push("Text extraction skipped because python3 is not on PATH.".to_string());
        return TextInfo::default();
    }

    let mut command = if util::has_command("timeout") {
        let mut command = util::desktop_command("timeout");
        command.args(["3", "python3", "-c", ATSPI_SCRIPT]);
        command
    } else {
        let mut command = util::desktop_command("python3");
        command.args(["-c", ATSPI_SCRIPT]);
        command
    };

    let output = match command.output() {
        Ok(output) => output,
        Err(err) => {
            warnings.push(format!("Text extraction failed to start: {err}"));
            return TextInfo::default();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            warnings.push("No accessible focused text was exposed by AT-SPI.".to_string());
        } else {
            warnings.push(format!("AT-SPI text extraction failed: {stderr}"));
        }
        return TextInfo::default();
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        warnings.push("AT-SPI returned no focused text.".to_string());
        return TextInfo::default();
    }

    let path = output_dir.join("text.txt");
    if let Err(err) = util::write(&path, text.as_bytes()) {
        warnings.push(format!("Captured text could not be written: {err}"));
        return TextInfo::default();
    }

    TextInfo {
        available: true,
        path: Some(util::canonical_or_original(&path)),
        bytes: text.len() as u64,
        source: Some("at-spi".to_string()),
    }
}

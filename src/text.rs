use crate::contract::TextInfo;
use crate::util;
use std::path::Path;

#[cfg(not(target_os = "windows"))]
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

def collect(acc, depth=0, active_only=False, inside_active=False):
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
    include_text = inside_active or not active_only or is_relevant
    if include_text:
        text_for(acc)
    try:
        count = acc.get_child_count()
    except Exception:
        return
    child_inside_active = inside_active or is_relevant
    for idx in range(count):
        try:
            child = acc.get_child_at_index(idx)
        except Exception:
            continue
        collect(child, depth + 1, active_only, child_inside_active)

desktop_count = Atspi.get_desktop_count()
for desktop_idx in range(desktop_count):
    collect(Atspi.get_desktop(desktop_idx), active_only=True)
if results:
    print("\n".join(results[:200]))
    raise SystemExit(0)
raise SystemExit(3)
"#;

#[cfg(target_os = "windows")]
const UI_AUTOMATION_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class TextNativeMethods {
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
}
'@

$handle = [TextNativeMethods]::GetForegroundWindow()
if ($handle -eq [IntPtr]::Zero) { exit 3 }

$root = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
if (-not $root) { exit 3 }

$seen = New-Object 'System.Collections.Generic.HashSet[string]'
$items = New-Object 'System.Collections.Generic.List[string]'

function Add-Text([string]$Text) {
    if ([string]::IsNullOrWhiteSpace($Text)) { return }
    $clean = (($Text -split '\s+') -join ' ').Trim()
    if ($clean.Length -eq 0) { return }
    if ($seen.Add($clean)) { [void]$items.Add($clean) }
}

function Collect-Element($Element) {
    try { Add-Text $Element.Current.Name } catch {}

    try {
        $pattern = $null
        if ($Element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
            Add-Text $pattern.Current.Value
        }
    } catch {}

    try {
        $pattern = $null
        if ($Element.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$pattern)) {
            Add-Text $pattern.DocumentRange.GetText(4000)
        }
    } catch {}
}

Collect-Element $root
$elements = $root.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition
)
foreach ($element in $elements) {
    Collect-Element $element
    if ($items.Count -ge 200) { break }
}

if ($items.Count -eq 0) { exit 3 }
$items -join "`n"
"#;

/// Shared tail of both extractors: trim, treat whitespace-only as empty,
/// persist `text.txt`, and build the contract info. Pure apart from the file
/// write, so it carries the unit tests for this module.
fn store_text(
    raw: &str,
    output_dir: &Path,
    source: &'static str,
    empty_warning: &str,
    warnings: &mut Vec<String>,
) -> TextInfo {
    let text = raw.trim();
    if text.is_empty() {
        warnings.push(empty_warning.to_string());
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
        source: Some(source.to_string()),
    }
}

#[cfg(not(target_os = "windows"))]
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        match summarize_atspi_error(&stderr) {
            Some(msg) => warnings.push(msg),
            None => warnings.push("No accessible focused text was exposed by AT-SPI.".to_string()),
        }
        return TextInfo::default();
    }

    store_text(
        &String::from_utf8_lossy(&output.stdout),
        output_dir,
        "at-spi",
        "AT-SPI returned no focused text.",
        warnings,
    )
}

/// Turn an AT-SPI helper's stderr into one concise warning line. Returns None
/// when stderr is empty (treated as "no text exposed"). Avoids dumping a raw
/// multi-line Python traceback; the common "GI bindings missing" case gets a
/// plain message, everything else collapses to the last meaningful line.
#[cfg(not(target_os = "windows"))]
fn summarize_atspi_error(stderr: &str) -> Option<String> {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("Namespace Atspi not available")
        || trimmed.contains("No module named 'gi'")
        || trimmed.contains("Namespace Gdk")
    {
        return Some(
            "Text extraction skipped: AT-SPI / Python GI bindings are not installed.".to_string(),
        );
    }
    let last = trimmed
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or(trimmed);
    Some(format!("AT-SPI text extraction failed: {last}"))
}

#[cfg(target_os = "windows")]
pub fn extract(output_dir: &Path, warnings: &mut Vec<String>) -> TextInfo {
    if !util::has_command("powershell") {
        warnings.push("Text extraction skipped because PowerShell is not on PATH.".to_string());
        return TextInfo::default();
    }

    let text = match crate::capture::windows::run_powershell(UI_AUTOMATION_SCRIPT, &[]) {
        Ok(text) => text,
        Err(err) => {
            warnings.push(format!("UI Automation text extraction failed: {err}"));
            return TextInfo::default();
        }
    };

    store_text(
        &text,
        output_dir,
        "ui-automation",
        "UI Automation returned no focused text.",
        warnings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cloche-text-test-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn summarizes_atspi_errors_without_dumping_tracebacks() {
        assert_eq!(super::summarize_atspi_error("   "), None);
        let traceback = "Traceback (most recent call last):\n  File \"<string>\", line 5\n    raise ValueError('Namespace Atspi not available')\nValueError: Namespace Atspi not available";
        let msg = super::summarize_atspi_error(traceback).unwrap();
        assert!(msg.contains("GI bindings are not installed"));
        assert!(!msg.contains("Traceback"));
        let other = "boom: something broke\nDetail: nope";
        assert_eq!(
            super::summarize_atspi_error(other).unwrap(),
            "AT-SPI text extraction failed: Detail: nope"
        );
    }

    #[test]
    fn store_text_writes_file_and_reports_info() {
        let dir = temp_dir("store");
        let mut warnings = Vec::new();
        let info = store_text("Visible label", &dir, "at-spi", "empty", &mut warnings);
        assert!(info.available);
        assert_eq!(info.bytes, "Visible label".len() as u64);
        assert_eq!(info.source.as_deref(), Some("at-spi"));
        let path = info.path.expect("path");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read back"),
            "Visible label"
        );
        assert!(warnings.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_text_trims_and_treats_whitespace_as_empty() {
        let dir = temp_dir("empty");
        let mut warnings = Vec::new();
        let info = store_text(
            "  \n\t ",
            &dir,
            "at-spi",
            "nothing extracted",
            &mut warnings,
        );
        assert!(!info.available);
        assert!(info.path.is_none());
        assert_eq!(warnings, vec!["nothing extracted".to_string()]);
        assert!(!dir.join("text.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_text_reports_unwritable_output_dir() {
        let dir = temp_dir("unwritable").join("missing-subdir");
        let mut warnings = Vec::new();
        let info = store_text("text", &dir, "at-spi", "empty", &mut warnings);
        assert!(!info.available);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("could not be written"));
    }
}

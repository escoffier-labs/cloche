# Roadmap

## Product Direction

App Shots is a cross-platform, agent-neutral CLI with Linux as the first backend. The binary stays `appshots`; platform support is added behind backend modules while preserving the JSON contract.

## Current: Linux MVP

- Active-window capture on GNOME/X11 with automatic desktop environment discovery for TTY/SSH/agent processes.
- Raw `shot.png`, polished randomized `shot-card.png`, `metadata.json`, and optional `text.txt`.
- Stable JSON output for agent subprocess use.
- Codex app-server payload generation through existing `localImage` input.
- Capture history helpers: `gallery`, `latest`, and `preview`/`open`.

## Next: Windows Support

- Fill in `capture/windows.rs` behind `#[cfg(target_os = "windows")]`.
- Use Win32 APIs for active-window discovery, screenshot capture, title, PID, and process name.
- Use UI Automation for best-effort text extraction.
- Reuse the same output contract: `shot.png`, `shot-card.png`, `metadata.json`, `text.txt`.
- Add a PowerShell install script and GitHub release binary path.

## Release Packaging

- Linux release archives are packaged by `scripts/package-release.sh`.
- Tagged GitHub releases build and upload Linux artifacts through `.github/workflows/release.yml`.
- Add Windows release artifacts once the Windows backend compiles and captures successfully.

## Later

- Wayland compositor-specific active-window support where safe and possible.
- Optional MCP wrapper around the CLI contract.
- Gallery HTML export for sharing batches of appshots.
- Additional presentation styles and user-configurable style presets.

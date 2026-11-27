# Roadmap

## Product Direction

Cloche is an open-source, OS-agnostic desktop capture tool for agents, scripts, and human workflows. The primary command is `cloche`; the existing `appshots` command remains as a compatibility alias while the project transitions from App Shots to Cloche.

Cloche is a screenshot tool. A Shot is a still capture with raw and presentation images, metadata, and optional extracted text. Video recording and GIF export are not planned.

## Current: Shots

- Linux active-window capture on GNOME/X11 with automatic desktop environment discovery for TTY/SSH/agent processes.
- Windows active-window and selected-window capture through Win32 metadata plus `PrintWindow`, with .NET screen capture for virtual-screen captures and fallback cases.
- Windows best-effort text extraction through UI Automation.
- Flat capture artifacts in `~/Pictures/Cloche` by default: `<stem>.png` (card), `<stem>.raw.png`, `<stem>.json`, optional `<stem>.txt` (legacy folder-per-shot layouts still readable).
- `polish` command and MCP tool that style any existing image into the same presentation card, so agents and scripts can reframe screenshots they did not capture with Cloche.
- Stable JSON output for agent subprocess use.
- Codex app-server payload generation through existing `localImage` input.
- Capture history helpers: `gallery`, `latest`, and `preview`/`open`.
- Self-contained HTML gallery export through `gallery --html` for sharing batches.
- Optional stdio MCP server (`cloche mcp`) wrapping the CLI contract.
- Compatibility binary and MCP path through `appshots`.

## Rename And Repository Transition (done)

The rebrand shipped: `cloche` is the primary binary/package name, the repository lives at `escoffier-labs/cloche`, and badges, install scripts, package archives, and smoke scripts all prefer Cloche. Two compatibility items remain live:

- Keep `appshots` as a compatibility binary until existing automation, docs, release assets, and downstream MCP configs have moved.
- Keep the old Appshots context in docs only where it explains compatibility or Codex's documented Appshots feature.

## Windows Hardening

- Improve active-window capture when a window is partially covered or minimized.
- Add Windows integration tests for interactive-session capture.
- Add signed release binaries once the publishing path is stable.

## Distro Test Matrix

- Add small container-based package smoke tests for major Linux distro families:
  Debian/Ubuntu, Fedora/RHEL, Arch, openSUSE, and Alpine where practical.
- Keep container tests focused on build, packaging, CLI contract, `schema`, `doctor`, and helper-detection behavior.
- Keep real screenshot capture in VM or desktop-session tests because it needs a graphical desktop session.
- Add optional VM/desktop smoke targets for GNOME X11, GNOME Wayland, KDE, and wlroots compositors.

## Release Packaging

- Linux release archives are packaged by `scripts/package-release.sh`.
- Windows release archives are packaged by `scripts/package-release.ps1`.
- Tagged GitHub releases build and upload Linux and Windows artifacts through `.github/workflows/release.yml`.

## Later

- Wayland compositor-specific active-window support where safe and possible.
- Additional presentation styles and user-configurable style presets.

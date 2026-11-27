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
- Region capture (`--target region`) through Flameshot or ImageMagick `import` on X11. It needs a human at the desk, so it is not for headless agents.
- Clipboard copy for finished cards and raw shots (`--clipboard`): `wl-copy` on Wayland, a detached arboard server on X11, and an in-process arboard write on Windows.
- Five procedural backdrop families (gradient, space, pattern, sky, terrain), chosen with `--palette` plus a per-family flag. `cloche config options` lists the live catalog.
- Persisted backdrop preferences (`cloche config`) and `cloche studio`, a local page for picking backdrops by eye. Sky and terrain are not on the studio page yet.
- `cloche setup` onboarding: hotkey install, MCP registration for supported agent clients, and a verify step.
- Optional stdio MCP server (`cloche mcp`) wrapping the CLI contract, with the card or raw frame returned as an MCP image content block.
- Compatibility binary and MCP path through `appshots`.

## Rename And Repository Transition (done)

The rebrand shipped: `cloche` is the primary binary/package name, the repository lives at `escoffier-labs/cloche`, and badges, install scripts, package archives, and smoke scripts all prefer Cloche. Two compatibility items remain live:

- Keep `appshots` as a compatibility binary until existing automation, docs, release assets, and downstream MCP configs have moved.
- Keep the old Appshots context in docs only where it explains compatibility or Codex's documented Appshots feature.

## Next: Wayland Capture

The next major item. Wayland has no generic way to read another app's window, so the plan is portal-based capture through `xdg-desktop-portal`:

- Request screenshots and window capture through the portal instead of compositor-specific tools. The portal shows a consent prompt, so a person has to approve each capture. It will not work unattended.
- Keep `grim` for wlroots screen capture and keep the existing X11 paths unchanged.
- Keep real Wayland capture in desktop-session tests, since it cannot run headless.

## Windows Hardening

- Improve active-window capture when a window is partially covered or minimized.
- Add Windows integration tests for interactive-session capture.
- Add signed release binaries once the publishing path is stable.

## Distro Test Matrix

- Add small container-based package smoke tests for major Linux distro families:
  Debian/Ubuntu, Fedora/RHEL, Arch, openSUSE, and Alpine where practical.
- Keep container tests focused on build, packaging, CLI contract, `schema`, `doctor`, and helper-detection behavior.
- Keep real screenshot capture in VM or desktop-session tests because it needs a graphical desktop session.
- Add optional VM/desktop smoke targets for GNOME X11, GNOME Wayland, KDE, and wlroots compositors. The Wayland targets gain value once portal capture lands.

## Release Packaging

- Linux release archives are packaged by `scripts/package-release.sh`.
- Windows release archives are packaged by `scripts/package-release.ps1`.
- Tagged GitHub releases build and upload Linux and Windows artifacts through `.github/workflows/release.yml`.

## Later

- Sky and terrain palettes on the `cloche studio` page.
- Wayland compositor-specific active-window support where safe and possible, for compositors the portal path does not cover.
- Named style presets beyond the persisted preferences in `cloche config`.

# Roadmap

## Product Direction

Cloche is an open-source, OS-agnostic desktop capture tool for agents, scripts, and human workflows. The primary command is `cloche`; the existing `appshots` command remains as a compatibility alias while the project transitions from App Shots to Cloche.

Cloche has two first-class modes:

- **Shots**: still screenshots, available now.
- **Reels**: experimental render is available now (`cloche reels render`);
  desktop `record` and the rest of the capture workflow are still planned from
  the Appreels prototype.

GIF support is planned as an export target after Reels capture lands, not as the
primary recording backend.

## Current: Shots + experimental Reels render

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
- Experimental `cloche reels render` with `--engine remotion` or `--engine hyperframes` (existing MP4 + cue JSON in, framed vertical MP4 out).

## Rename And Repository Transition (done)

The rebrand shipped: `cloche` is the primary binary/package name, the repository lives at `escoffier-labs/cloche`, and badges, install scripts, package archives, and smoke scripts all prefer Cloche. Two compatibility items remain live:

- Keep `appshots` as a compatibility binary until existing automation, docs, release assets, and downstream MCP configs have moved.
- Keep the old Appshots context in docs only where it explains compatibility or Codex's documented Appshots feature.

## Next: Reels capture and polish

`cloche reels render` already ships (Remotion and HyperFrames engines). Remaining
work merges the rest of Appreels into Cloche without making video feel bolted on.

- Ship `cloche reels record` for raw short desktop captures (X11/Linux first).
- Keep deepening `render`: cursor emphasis, richer cue authoring, and tighter
  shared presentation with still cards (HyperFrames already shares
  `--palette`/`--style-seed`; Remotion should stay visually aligned).
- Add `perform-terminal` and `perform-browser` once the scripting path is stable enough.
- Preserve stable JSON contracts with `ok`, `warnings`, `errors`, paths, durations, and generated artifact metadata.
- Treat Windows Reels as later backend work unless a user need forces it earlier. macOS is not a target platform.

## Reels Integration Sequence

1. ~~Command shape for render~~: `cloche reels render` is live; keep top-level
   `cloche capture` as the Shots shortcut. Optional later nesting:
   `cloche shots capture` / `cloche shots gallery` / `cloche reels record`.
2. Extract or vendor shared presentation code so both Remotion and HyperFrames
   stay aligned with still `shot-card` palette, padding, corner radius, and shadow.
3. Port the Appreels script schema under Cloche naming and keep the JSON schema command.
4. Add Reels output metadata:
   - `rawVideo`
   - `reel`
   - `cursorTrack`
   - `cueFile`
   - `durationMs`
   - `presentationStyle`
5. Add MCP tools for Reels only after the CLI contract is stable.
6. Add release packaging for any required video assets, helper docs, and platform dependency checks.

## GIF Export

GIF export is intentionally later.

- Add `cloche reels export-gif --input demo.mp4 --out demo.gif`.
- Prefer generating GIFs from finished Reels so captions, cursor emphasis, zooms, and framing stay consistent.
- Add size controls before shipping:
  - width/height limits
  - fps
  - palette generation
  - max duration
  - optional loop count
- Keep MP4/WebM as the quality defaults and GIF as a sharing fallback.

## Windows Hardening

- Improve active-window capture when a window is partially covered or minimized.
- Add Windows integration tests for interactive-session capture.
- Add signed release binaries once the publishing path is stable.

## Distro And Media Test Matrix

- Add small container-based package smoke tests for major Linux distro families:
  Debian/Ubuntu, Fedora/RHEL, Arch, openSUSE, and Alpine where practical.
- Keep container tests focused on build, packaging, CLI contract, `schema`, `doctor`, and helper-detection behavior.
- Keep real screenshot capture in VM or desktop-session tests because it needs a graphical desktop session.
- Add optional VM/desktop smoke targets for GNOME X11, GNOME Wayland, KDE, and wlroots compositors.
- Add Reels media smoke tests for `cloche reels render` (and `record` once it lands).

## Release Packaging

- Linux release archives are packaged by `scripts/package-release.sh`.
- Windows release archives are packaged by `scripts/package-release.ps1`.
- Tagged GitHub releases build and upload Linux and Windows artifacts through `.github/workflows/release.yml`.

## Later

- Wayland compositor-specific active-window support where safe and possible.
- Additional presentation styles and user-configurable style presets.

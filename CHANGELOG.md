# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.5.0] - 2026-06-17

### Added
- `cloche setup`: one guided command that installs the `cloche-grab` hotkey
  script and binds it to Print (GNOME auto via gsettings, other desktops print
  exact steps), registers the `cloche mcp` server with detected agents (Claude
  Code via the `claude` CLI or `~/.claude.json`, OpenClaw, Codex CLI; a generic
  snippet otherwise; every edited file is backed up and the edit is idempotent),
  then verifies the capture pipeline, the hotkey binding, and a live `cloche
  mcp` handshake. `--print` dry-runs, `--yes` skips the prompt, `setup verify`
  re-checks, and `--format json` emits a stable report. Subcommands `setup
  hotkey`, `setup agent`, and `setup verify` run each piece on its own.

### Fixed
- `cloche setup --format json` now keeps stdout pure JSON: human guidance, the
  confirmation prompt, and the decline notice go to stderr, and declining still
  emits a valid report. Config edits no longer overwrite a valid-JSON but
  non-object `mcpServers`/`mcp.servers`/`cloche` value, and the Codex TOML check
  tolerates whitespace and quoted-key header forms so a duplicate
  `[mcp_servers.cloche]` table is never appended.
- AT-SPI text-extraction failures collapse to one concise warning instead of
  dumping a multi-line Python traceback.

### Docs
- README notes the Rust 1.88 MSRV and that distro `cargo` packages can be too
  old (use rustup).

## [0.4.0] - 2026-06-16

### Added
- `cloche reels render`: render a vertical video reel from a source clip through
  a bundled Remotion template (opening title card, configurable fps/size/duration,
  optional AppReels-compatible cue timeline). The template resolves via
  `CLOCHE_REMOTION_DIR`, then next to the installed binary, then the dev tree.

## [0.3.0] - 2026-06-13

### Added
- `scripts/cloche-grab.sh`: a portable hotkey wrapper (region capture ->
  polish -> clipboard -> notification) with no machine-specific paths, plus
  README binding instructions for GNOME, KDE, and tiling WMs, so any user can
  set up a one-press screenshot-to-card key.
- `capture --target region`: interactive region selection via Flameshot
  (accept-on-select) or ImageMagick `import` drag-select on X11. Aborted
  selections report a clean error. Windows returns a clear unsupported
  message for now.
- `capture --clipboard`: copy the presentation card (or raw shot) to the
  system clipboard after capture, via wl-copy or xclip. Copy failures are
  warnings, never capture errors.

### Fixed
- Presentation cards are now fully opaque to the edges (square canvas, like a
  Codex appshot) instead of having transparent rounded outer corners. The
  transparent corners rendered as white when the PNG was flattened to JPEG or
  pasted into apps that composite alpha on white. The screenshot inside keeps
  its rounded corners and shadow.

## [0.2.0] - 2026-06-12

### Added
- `polish` command and matching MCP tool: style any existing image (PNG, JPEG,
  or WebP) into the presentation card without a live capture, with `--palette`,
  `--style-seed`, and `--out` controls.
- `schema --for polish` exposes the polish JSON contract alongside the capture
  contract.
- MSRV check job in CI; the supported minimum Rust is documented as 1.88
  (required by the image crate), correcting the previously advertised 1.85.
- Unit coverage for the Codex `turn/start` payload contract and the text
  persistence path.

### Changed
- Rebranded from App Shots to Cloche: `cloche` is the primary binary and crate;
  `appshots` remains as a compatibility alias.
- Presentation cards redesigned with vibrant 3-stop gradients, glow spots,
  light streaks, grain, and rounded canvas corners.
- All dependencies now build with `default-features = false` and explicit
  feature lists; clap's color and suggestion machinery dropped from the tree.

### Fixed
- `polish` decodes JPEG and WebP inputs as documented; previously only PNG
  decoding was compiled in.

## [0.1.0] - 2026-06-02

### Added
- Initial release as App Shots: active/window/screen capture on Linux (X11)
  and Windows, raw `shot.png` plus presentation `shot-card.png`, stable JSON
  output with `metadata.json`, best-effort text extraction, `gallery`/`latest`/
  `preview` helpers, HTML gallery export, Codex `turn/start` payload
  generation, and a stdio MCP server.

[Unreleased]: https://github.com/escoffier-labs/cloche/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/escoffier-labs/cloche/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/escoffier-labs/cloche/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/escoffier-labs/cloche/releases/tag/v0.1.0

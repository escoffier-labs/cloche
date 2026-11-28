# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **Five backdrop families.** Cards used to choose between gradients and
  deep-space scenes. They now come from gradient, space, pattern, sky, or
  terrain, each drawn from the style seed at render time with no stored images.
  `cloche config options` prints the live catalog: every palette with its kind,
  plus every scene, motif, sky, and terrain.
- **Pattern backdrops**: woven and ruled geometry. Six palettes (`tartan-moss`,
  `oxford-navy`, `blueprint`, `ledger-cream`, `picnic-red`, `workshop-ochre`)
  and twelve motifs (`plaid`, `gingham`, `stripe`, `rule`, `grid`, `diagonal`,
  `chevron`, `dot`, `crosshatch`, `weave`, `herringbone`, `houndstooth`) via a
  new `--motif`. Nothing is a tiled bitmap: plaid builds a mirrored tartan
  sett, houndstooth comes off a 2/2 twill.
- **Sky backdrops**: cloud decks, storms, lightning, twilight, and aurora. Seven
  palettes (`blue-hour`, `golden-hour`, `supercell`, `mammatus`,
  `aurora-curtain`, `cirrus-blue`, `alpenglow`) and ten conditions (`blue-hour`,
  `golden-hour`, `overcast`, `cirrus`, `mackerel`, `mammatus`, `storm`, `bolt`,
  `crepuscular`, `aurora`) via a new `--sky`.
- **Terrain backdrops**: dunes, mesa, badlands, and glacier, each a sky above a
  horizon with ground below. Four palettes and four kinds share those names, and
  `--terrain` pins the kind.
- **Eight more space and gradient palettes.** Four deep-space (`eagle-pillars`,
  `crab-remnant`, `tarantula-web`, `sombrero-dust`), bringing the default random
  rotation to twelve, and four gradients (`sea-glass`, `peach-dusk`, `ink-wash`,
  `citrus-noon`).
- **`--palette`, `--scene`, `--motif`, `--sky`, and `--terrain` on both
  `cloche capture` and `cloche polish`** (0.7.0 `polish` had only `--palette`
  and `--scene`), plus the same arguments on both MCP tools. Explicit flags override the persisted config.
- **Persisted backdrop preferences.** `cloche config` stores styling choices in
  `~/.config/cloche/config.json` (`$XDG_CONFIG_HOME` / `%APPDATA%` honored,
  `CLOCHE_CONFIG` overrides), and both `cloche capture` and `cloche polish` read
  them with no flags. Pin a look with
  `cloche config set --mode pinned --palette orion-emission --scene jwst`, or
  keep the picker random over a chosen subset with
  `cloche config set --palettes carina-hubble,pleiades-reflection --scenes alma,veil`.
  The config holds a pinned value and a random pool for each of palette, scene,
  motif, sky, and terrain. `cloche config show` prints the effective
  preferences, and `cloche schema --for config` emits the JSON contract.
- **`cloche studio`**, a local page for picking backdrops by eye instead of by
  name. Space, gradient, and pattern palettes and every scene and motif are
  shown as real renders. Clicking toggles a backdrop in or out of the random pool
  (or pins it), and hovering one redraws the scene or motif sheet on it. Every
  change is written to the same `config.json` the CLI reads. Binds to
  `127.0.0.1` with no auth by design. `--host` widens it, `--port 0` takes any
  free port, and `--print-url` prints the URL as JSON without serving. Sky and
  terrain are not on the page yet. No new dependencies: the HTTP handling is
  hand-rolled on `std::net`.
- **`capture --clipboard` on Windows.** The PNG is decoded to RGBA and written
  with arboard in-process. Failures remain warnings. `arboard` is now a Linux
  and Windows dependency (same `image-data` feature only).
- **MCP image content.** The `capture`, `polish`, and `latest` MCP tools now
  append an `image` content block (base64, MIME type from the file's signature)
  with the presentation card after the JSON `text` block, so agents in a sandbox
  or on another machine can see the result without access to the file path. New
  optional arguments: `includeImage` (default `true`) and `imageVariant` (`card`
  or `raw`). Files over 2 MiB, missing files, and unreadable files produce a
  text note instead of an image and never fail the call. The base64 encoder
  moved from `html.rs` to `util.rs`, with no new dependencies.
- `cloche codex-payload --card` attaches the polished presentation image instead
  of the default raw capture frame.
- **`polish::render_backdrop`** paints a backdrop at any size with no card on
  top, so the studio can show the sky on its own. A finished card is only about
  4% backdrop by width, so whole cards are indistinguishable as swatches.

### Changed
- `capture --target region` no longer attempts accessibility text extraction.
  A region selection has no window of its own, so any text the AT-SPI attempt
  found would have come from whichever window had focus, not the selected area.
  In practice it found nothing and only added the "No accessible focused text
  was exposed by AT-SPI." warning and up to 3 seconds. The `text` object keeps its shape
  (`available: false`, `path: null`, `bytes: 0`, `source: null`) and no warning
  is added. `active`, `window`, and `screen` are unchanged.
- **Backdrop grain is half as strong (2.4 to 1.2), so cards are about 11%
  smaller.** The per-pixel grain on every backdrop family is what PNG
  compresses worst, and it only has to be strong enough to hide gradient
  banding. A given `--style-seed` now renders a subtly smoother backdrop than
  before (every channel moves by at most 2 levels; layout and colors are
  unchanged), and the card file is about 11% smaller on a sample of real
  captures. All five families now read one shared `polish::GRAIN_STRENGTH`.
- **BREAKING: capture JSON field `outputDir` is now `outDir`**, matching the
  flat `--out-dir` and gallery semantics (it is not a per-shot folder). Scripts
  that read `outputDir` from `capture` output must switch to `outDir`. Reading
  old on-disk `*.json` sidecars still accepts the legacy `outputDir` key.
- **Backdrop selection scores palette names against the seed instead of
  indexing by position.** Indexing made the pool length the modulus, so adding
  one palette reshuffled what every existing seed rendered. Scoring by name
  means a new palette only takes the seeds it actually wins, so the table is
  safe to grow. This is a one-time break: a given `--style-seed`
  renders a different backdrop than it did in 0.7.0. Reproducibility holds from
  here on, and the same seed with the same pool is still exact.
- Random styling still defaults to the space palettes only. A palette listed in
  the config pool is drawn from regardless of its kind, which is how the
  gradient, pattern, sky, and terrain families join the rotation.
- **Polished cards are about 27% smaller on disk, with identical pixels.** Card
  PNGs were written with `image`'s default `Fast` compression and an alpha
  channel the opaque card never uses. They now use zlib level 6 with adaptive
  filtering and are stored as RGB. Over a 99-card sample the total went from
  47.2 MB to 34.2 MB (median 261 KB to 189 KB, p90 1.20 MB to 0.95 MB). Polish
  takes about 30 ms longer at the median (about 200 ms longer on the largest
  cards). Decoded cards match the previous output exactly.
- Card padding is tighter (58 to 78 px at reference size, down from 100 to 132),
  so the screenshot fills more of the card and the backdrop reads as a frame.
- An unreadable or malformed config is reported in the result's `warnings` and
  falls back to defaults instead of failing the capture.
- MCP `capture` input schema lists `format` (always `json`), an empty `required`
  array, and describes `outDir` as the flat gallery root.
- The CLI's help text and MCP descriptions name all five backdrop families, and
  the README points at `cloche config options` instead of a hand-counted total.
- Internal: `cli.rs` is split into one handler module per command family under
  `src/commands/`. No behavior change.

### Removed
- **BREAKING: Reels.** `cloche reels render` (with `--engine remotion` and
  `--engine hyperframes`), the vendored `remotion/` project, and the
  `cloche schema --for reel-render` contract (`ReelRenderResult`) are gone, along
  with the `CLOCHE_REMOTION_DIR` override.
  Cloche is a screenshot tool again: Shots, polish, gallery, studio, MCP, config,
  setup, and the `appshots` alias are unchanged. Reels was experimental and
  narrowed the project's scope; its Node/Remotion toolchain also kept turning CI
  red on npm audit advisories with no change to the code, and neither engine
  handled desktop recording, only re-rendering an existing MP4. The `Remotion`
  CI job and the Remotion bundle in release archives are removed too.

### Fixed
- X11 clipboard images use a detached arboard server instead of xclip, avoiding
  clipboard stalls after an abandoned large-image transfer. A private readiness
  pipe confirms ownership within 3 seconds or reports a capture warning. The
  child rejects input PNG files above 15 MiB, exits on X connection loss via a
  30-second watchdog, and closes inherited output pipes and wrapper lock fds.
- `capture --target active` on X11 no longer requires xdotool. The
  gnome-screenshot and scrot fallbacks are reachable without it, and
  `cloche doctor` reports only helpers that capture can actually invoke.
- Two captures in the same second no longer overwrite each other. The capture
  stem now includes the process id and a per-process counter.
- `gallery` and `latest` find flat `<stem>.json` sidecars one directory deep, and
  overlapping `--root` values no longer list the same capture twice.
- `codex-payload` accepts a flat-layout out-dir, so the README quickstart works
  with the default output. It picks the newest sidecar, legacy `metadata.json`
  included.
- `cloche studio` served one connection at a time, so a browser pre-opening a
  socket without sending a request wedged the accept loop and the page loaded no
  swatches. Connections are handled per thread with read and write deadlines.
  Idle tabs no longer re-render the hero image every few seconds, and a client
  disconnecting mid-response no longer logs an error.
- Relicensed from Apache-2.0 to MIT. `LICENSE`, `Cargo.toml`, `CONTRIBUTING.md`,
  and the README badge now agree on MIT.

## [0.7.0] - 2026-07-18

### Added
- **Procedural deep-space backdrops for shot-cards.** Space palettes render a
  seeded scene behind the card: domain-warped nebulae with bright ionization
  fronts and dark dust lanes, layered starfields, star-forming knots, galaxy
  smudges, occasional suns, and rare ultra-deep-field seeds. Eight palettes are
  color-sampled from astrophotography of real objects (`orion-emission`,
  `carina-hubble`, `pleiades-reflection`, `rho-ophiuchi`, `milkyway-core`,
  `andromeda-haze`, `horsehead-flame`, `lagoon-trifid`). Random styling now
  picks a space palette; the five original gradient palettes remain available
  by name.
- **Telescope scene variants.** Around 45% of scenes take a JWST look (6-point
  diffraction spikes, clumpy globular dust, an inverted red-arm/blue-core
  galaxy). Other looks in the pool: ALMA protoplanetary discs, SDO extreme-UV
  suns with coronal loops, Chandra fragmented remnant shells, ring and
  Twin-Jet butterfly planetary nebulae, veil-remnant ribbons, edge-on
  dust-lane galaxies, gravitational-lensing arcs, and a rare Planck CMB frame.
- **`cloche polish --scene <name>`** (and the MCP `polish` tool's `scene`
  argument) pins a specific deep-space look instead of the seed's random pick:
  `nebula`, `jwst`, `hubble`, `galaxy`, `alma`, `ring`, `butterfly`, `edge-on`,
  `sun`, `sdo`, `cluster`, `deep-field`, `lensing`, `veil`, `remnant`, `cmb`.
  The same `--style-seed` reproduces a pinned scene exactly. `--scene` only
  applies to space palettes.
- Targeted reel zooms: a zoom cue takes optional `x`/`y` (0 to 1 across the
  footage) to pick its focus point, defaulting to center.

### Changed
- Reel overlay motion (zooms, cards, captions) now animates at the output frame
  rate, and the default `--fps` is 60, so overlays stay smooth over 30fps source
  footage. Zoom cues ease in and out with a continuous envelope instead of a
  linear ramp that snapped scale back to 1.0 at the cue's end. The Remotion
  background moved to a vendored grain-gradient shader driven from the frame.

### Fixed
- Clipboard publication now runs before best-effort text extraction, so an
  immediate paste no longer retrieves the preceding capture on X11.

### Docs
- README documents the space palettes and the `--scene` picker. The workflow
  diagram was standardized, and CI now skips docs-only changes.

## [0.6.0] - 2026-06-27

### Added
- `cloche reels render --engine hyperframes`: a second Reels engine alongside
  Remotion. It generates a self-contained HyperFrames composition (HTML is the
  source of truth for video) from the input video plus AppReels-shaped cue JSON,
  renders it through `npx hyperframes`, and emits the same `ReelRenderResult`
  contract (`engine` now reflects the chosen engine instead of always
  `remotion`). The composition is lint-clean (0 errors) and mirrors the Remotion
  reel look: framed browser chrome, timed caption clips, title and outro cards.
  Override the launcher with `CLOCHE_HYPERFRAMES_CMD`. `--keep-props` keeps the
  staged project (with its `index.html`) for debugging.
- Shared brand between still and motion: `--palette` and `--style-seed` pin the
  reel to the same Cloche presentation palette as a still `shot-card`. The
  hyperframes engine writes a `DESIGN.md` (HyperFrames' Visual Identity Gate
  format) from that palette and uses the exact hex values in the composition, so
  the reel and the card trace back to one identity.
- `--workers` for the hyperframes engine (default `1`). Some environments
  corrupt frames during parallel capture, which fails the ffmpeg encode with
  `image2 ... unspecified size`; single-worker is the safe default, raise it for
  faster renders on stable setups.

### Changed
- **Flat capture layout.** A capture no longer creates a folder-per-shot with
  fixed filenames. Instead it writes flat files that share one timestamp stem:
  `<stem>.png` (the shareable card, or the raw shot when no card is made),
  `<stem>.raw.png` (raw), `<stem>.json` (metadata), and `<stem>.txt` (text).
  Captures also default to a central gallery dir (`~/Pictures/Cloche`) instead
  of the current working directory, so shots collect in one place. Override with
  `--out-dir`. `gallery`, `latest`, and `preview` read the flat layout and still
  read legacy folder-style captures; `codex-payload` and `preview` accept a flat
  `<stem>.json` sidecar (or a legacy directory).
- HyperFrames reel browser frame now sizes to the source footage and fills most
  of the canvas, instead of a fixed small 16:10 box. The engine probes the input
  video aspect with ffprobe and contains it within the canvas, so a taller
  recording yields a taller on-screen frame.

### Fixed
- HyperFrames reel title/outro cards now wrap long unbreakable tokens
  (`overflow-wrap: anywhere`) instead of bleeding off the canvas. A URL outro
  like `escoffierlabs.dev/academy` previously clipped at the frame edge. Also
  finished the CSS switch to `#id` selectors (a few descendant rules still used
  the attribute selector).

### Docs
- README reworked for adoption: a what/why/how-it-differs opening with the
  real shot cards as the lead proof, a crates.io version badge, a prominent
  website link, and new "What it does", "Why not other screenshot tools?", and
  "What Cloche is not" sections.
- Added maintainer-health files: `SECURITY.md`, `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, GitHub issue templates (bug + feature, blank issues
  disabled with contact links), and a pull request template.

## [0.5.1] - 2026-06-17

### Fixed
- Windows build: cloche 0.5.0 failed to compile on Windows because the new
  `setup` modules called Linux-only `util` helpers. Added Windows variants of
  `env_var`/`run_status`/`run_output`, made `setup hotkey` point at Win+Shift+S
  instead of attempting a Linux bind, skipped the hotkey verification check on
  Windows, and resolved the home directory via `USERPROFILE`. Verified on
  Windows 11 (clippy `-D warnings` clean, tests pass, `setup` behaves).

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

[Unreleased]: https://github.com/escoffier-labs/cloche/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/escoffier-labs/cloche/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/escoffier-labs/cloche/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/escoffier-labs/cloche/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/escoffier-labs/cloche/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/escoffier-labs/cloche/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/escoffier-labs/cloche/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/escoffier-labs/cloche/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/escoffier-labs/cloche/releases/tag/v0.1.0

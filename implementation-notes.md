# Implementation Notes

Running log of decisions and tradeoffs not captured in commit messages or the spec.

## Repo hygiene (2026-06-01)

- Added `.github/workflows/ci.yml` running fmt/clippy/test on Linux + Windows for
  pushes to `master` and all PRs. Clippy runs with `-D warnings`; verified the tree
  is already warning-clean so CI starts green. Mirrors `release.yml` conventions
  (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`).
- Added `LICENSE` (Apache-2.0) to match the `license` field already declared in
  `Cargo.toml`. Copyright holder: Solomon Neas, 2026.

## Gallery HTML export (2026-06-01)

- New `src/html.rs`: pure, dependency-free helpers (`base64_encode`, `escape_html`,
  `render`) plus a `GalleryItem` view struct. Kept it a leaf module so `cli.rs`
  depends on `html`, not the reverse.
- Decision: embed images as base64 `data:` URIs so the exported file is a single
  self-contained, shareable artifact (matches the roadmap "sharing batches" goal).
  Tradeoff: larger HTML files vs. zero external file dependencies. Self-containment
  wins for a share-and-send use case.
- Decision: hand-rolled base64 encoder instead of pulling the `base64` crate. The
  repo keeps deps minimal (all `default-features = false`); the encoder is ~20 lines
  with RFC 4648 test vectors, so a new dependency was not worth it.
- `gallery --html <PATH>` writes the file and still prints the normal gallery JSON,
  now with an added `htmlPath` field. `--open` opens the result via the existing
  `open_path` helper. Image selection prefers `presentation_image`, falls back to the
  raw `image`, matching `preview` semantics.

## MCP wrapper (2026-06-01)

- New `src/mcp.rs`: a minimal stdio JSON-RPC 2.0 server (`cloche mcp`) implementing
  `initialize`, `ping`, `tools/list`, `tools/call`, and notification handling.
- Decision: tool calls shell out to the Cloche binary itself (`current_exe`) rather
  than calling internal functions. This keeps the capture contract in exactly one
  place, guarantees byte-identical output to the CLI, and matches the roadmap phrasing
  ("MCP wrapper around the CLI contract"). Tradeoff: one subprocess per tool call;
  negligible next to the cost of a screen capture.
- Dispatch is split into a pure `dispatch()` function (testable with a mock tool
  runner) and `run_tool_via_subprocess()` (the side-effecting part). `tool_command_args`
  is also pure and unit-tested.
- Protocol version reported: `2025-06-18`.

## Polish command (2026-06-11)

- New `cloche polish <INPUT>` styles an existing image into the presentation card
  (rounded window, layered shadows, gradient backdrop) without a live capture.
  Motivation: agent sessions kept being asked to "clean up this screenshot with
  Cloche", which previously had no real implementation, so agents improvised
  ImageMagick recreations with flat white or transparent canvases.
- Output is a single PNG (default `<input>-card.png` next to the input). `--out`
  must end in `.png`; the card always carries alpha for its rounded canvas
  corners, so silently re-encoding to JPEG would corrupt the look.
- `--palette <name>` pins one of the named palettes while every other style
  parameter still derives from the seed (`style_with_palette` in `polish.rs`).
  Palette names come from the `PALETTES` table via `palette_names()` so clap
  help, MCP schema enum, and validation share one source of truth.
- No `metadata.json` and no frame-extent cropping: polish is a single-file
  transform, not a capture. `gallery`/`latest` only scan capture dirs and that
  stays true.
- Result contract is a new `PolishResult` (camelCase wire keys) rather than
  reusing `AppshotResult`, which is capture-shaped (target, backend, window).

## Dependency feature trim (2026-06-12)

- All deps now declare `default-features = false` with explicit feature lists,
  per the AGENTS.md rule. Net effect: clap drops `color`/`suggestions`
  (anstream, colorchoice, strsim gone from the tree; anstyle remains because
  clap_builder depends on it unconditionally). Help and error output verified
  readable without them.
- `image` gained `jpeg` and `webp` decode features: `polish` documents JPEG and
  WebP inputs, and png-only silently broke that contract (caught by a failing
  test before the fix). Encode surface is still PNG-only by design.
- `rand` features: `std`, `std_rng`, `thread_rng`, `os_rng` are exactly what
  `rand::rng()` + `StdRng::seed_from_u64` need. `schemars` keeps `derive` and
  `chrono` (DateTime fields in the contract derive JsonSchema).

## Region capture and clipboard (2026-06-12)

- `capture --target region` reuses the whole capture contract instead of being
  a separate `grab` verb: gallery/latest/preview, metadata.json, and the MCP
  capture tool all picked it up for free. Spec: docs/specs/2026-06-12-region-capture.md.
- Flameshot is driven with `gui --accept-on-select --path <file>`. `--raw` is a
  trap: a running flameshot daemon takes over the capture and prints the PNG on
  ITS stdout, not the client's. Discovered live; encoded as an AGENTS.md gotcha.
- Clipboard copy shells out (wl-copy / xclip) per the no-new-deps rule, in
  src/clipboard.rs with the selection logic split out pure for unit tests.
  Failures are capture warnings, not errors: the shot on disk is still good.
- Clipboard publication runs before best-effort text extraction. AT-SPI can use
  its full three-second timeout, and making clipboard copy wait for it caused
  immediate pastes to retrieve the preceding capture on X11.

## Setup onboarding (2026-06-17)

- `cloche setup` automates the previously manual onboarding: install cloche-grab,
  bind Print (GNOME), register the MCP server with agents, then verify. Spec:
  docs/specs/2026-06-17-setup-onboarding.md, plan: ...-setup-onboarding-plan.md.
- No new deps. JSON configs (.claude.json, openclaw.json) are edited with the
  existing serde_json. Codex's config.toml is edited by guarded text-append
  (append the static `[mcp_servers.cloche]` block only when absent) rather than
  pulling in a TOML crate; the block is static so presence == configured.
- OpenClaw MCP schema confirmed against the live config: top-level
  `mcp.servers.<name> = {command, args}`. Resolved the spec's print-only fallback.
- Pure/side-effect split for testability: desktop classification, slot
  selection, gsettings array parsing, JSON upsert, codex-block detection, and
  the tools/list parser are all unit-tested without a desktop or subprocess.
- The MCP verify check is a REAL end-to-end handshake: it spawns `cloche mcp`,
  writes initialize + tools/list, and asserts capture+polish are listed. This is
  the actual proof an LLM can call cloche, not just a config-file presence check.
- GNOME idempotency keys on the binding command via `is_cloche_command`, which
  matches both the bare `cloche-grab` and any `*/cloche-grab` absolute path.
  Found live: an exact-string match created a duplicate "Cloche Grab" binding
  next to a pre-existing one that used an absolute path. Basename match fixes it.
- `gsettings` ignores HOME (talks to dbus), so sandbox-HOME testing still hits
  the real GNOME session. Test the binding path via dry-run (`bind_gnome(false)`)
  or expect to revert real keybindings afterward.
- `util::env_var` falls back to scraping desktop-process environ, so clearing
  XDG_CURRENT_DESKTOP in the child does NOT force a non-GNOME code path.

## Clean-box onboarding test (2026-06-17)

Tested cloche + brigade as a brand-new user on a pristine Ubuntu 24.04 LXC to
catch "works on my machine" gaps. Findings:

- MSRV vs distro Rust: Ubuntu 24.04 `apt install cargo` is rustc 1.75; cloche
  needs 1.88. `cargo install cloche` fails for apt-Rust users. README now warns
  and points at rustup. (rustup gave 1.96, built clean in ~40s; only extra dep
  was build-essential for the linker.)
- crates.io still serves cloche 0.3.0, NOT 0.4.0, despite the v0.4.0 release
  commit + CHANGELOG. 0.4.0 (reels) was never published. ACTION: publish.
- `--format json` was not machine-safe: generic MCP snippet, non-GNOME hotkey
  steps, abort notice, and the confirm prompt all went to stdout and corrupted
  JSON. Only surfaced on a clean box (no agent clients installed -> generic-print
  path fires). Fixed: all human/prompt output -> stderr; decline still emits a
  valid report. Caught by clean-box test + Codex review.
- Headless coverage achieved: polish/mcp/doctor need no display; capture works
  under Xvfb (import -window root); the GNOME gsettings bind works under
  `dbus-run-session` + XDG_CURRENT_DESKTOP=GNOME + libglib2.0-bin
  (gnome-settings-daemon-common provides the schema, libglib2.0-bin the
  gsettings binary). Full `cloche setup --yes` goes all-green there.
- AT-SPI missing dumps a raw Python traceback into capture warnings instead of a
  clean message (cosmetic; text extraction is best-effort). Candidate cleanup.
- cloche degrades correctly when gsettings is absent: warning + manual fallback,
  no crash.

Brigade (pipx install brigade-cli 0.12.0): full happy path clean -- repo and
workspace quickstart, doctor, tools list, verify-harness, handoff
draft/lint/doctor all OK. Only friction: README "60 seconds" block runs
`brigade ...` right after `pipx install` in the same shell, but pipx warns
`~/.local/bin` is not yet on PATH -> new user hits command-not-found. Suggest a
`pipx ensurepath` + new-shell note between install and first command.

## HyperFrames reel engine (2026-06-19)

Added a second `cloche reels render` engine alongside Remotion, plus the
palette -> DESIGN.md bridge so a reel shares the still `shot-card` identity.

- **Engine shape.** Reused the existing `ReelRenderEngine` enum + dispatch seam
  rather than a new command. `engine` in `ReelRenderResult` was hardcoded
  `"remotion"`; now it reflects the chosen engine via `ReelRenderEngine::name`.
- **No vendored node project.** Unlike the Remotion engine (which ships a
  `remotion/` package in the crate), HyperFrames is invoked through
  `npx hyperframes`. `CLOCHE_HYPERFRAMES_CMD` overrides the launcher
  (whitespace-split) for non-standard setups. This keeps the crate small and
  matches how HyperFrames is meant to be run.
- **Composition generator is a pure fn** (`composition_html`) so it is unit
  tested without I/O. It emits a standalone `index.html` (no `<template>`),
  registers `window.__timelines`, is deterministic (no `Math.random`/`Date.now`/
  `repeat: -1`), and HTML-escapes all user text.
- **Lint-driven fixes (caught by `npx hyperframes lint`):**
  1. Timed `<div>`s MUST have `class="clip"` or the runtime shows them for the
     whole composition (caption/title/outro would never hide). This was a real
     correctness bug, not cosmetic.
  2. System-font keywords (`-apple-system`, `ui-sans-serif`, `Segoe UI`) are not
     auto-resolvable and hard-fail the render. Font stack reduced to
     `Inter, sans-serif` (HyperFrames fetches Inter from Google Fonts).
  3. Switched CSS from `[data-composition-id="..."]` to `#cloche-reel` (added an
     `id` to the root) to clear the self-attribute-selector warnings.
  Final composition: 0 lint errors. Remaining warnings (self-id styling,
  gsap-studio-edit) are benign for headless rendering.
- **Multi-worker encode failure.** On this environment, parallel frame capture
  (`--workers >= 2`) corrupts a frame and the ffmpeg image2 encode dies with
  `Could not find codec parameters ... unspecified size` (HyperFrames suggests
  `--docker`). Isolated to worker count, NOT quality. So the bridge exposes
  `--workers` and defaults it to **1** for reliability; users raise it on stable
  setups. Doctor reports all-green including Docker, so it is a parallel-capture
  race, not a missing dependency.
- **Palette bridge (`src/design.rs`).** `design_md(style, title)` converts a
  `PresentationStyle` (the same struct behind `cloche polish`/`capture`) into a
  HyperFrames `DESIGN.md` with the 4 Visual-Identity-Gate sections. The engine
  both writes that DESIGN.md into the staged project AND uses the exact hex
  values in the composition CSS, so still and motion share one brand.
  `--palette`/`--style-seed` on `reels render` resolve the style the same way
  capture does (`resolve_reel_style`).
- **Verified end-to-end:** `cloche reels render --engine hyperframes` produced a
  valid h264 1080x1920 ~4s MP4; extracted frames confirm the title card,
  bottom caption, and aurora-teal palette render at their scheduled times.

## Space-themed backdrops (2026-07-17)

`src/space.rs` renders procedural deep-space scenes behind shot-cards:
fbm value-noise nebula in the palette's two glow tints, thresholded so most
of the frame stays black sky (real astro frames carry color on a fraction of
the field), a dust-lane darkening pass, three star layers (dense faint, mid,
a few spiked hero stars, warm-dominated color mix), and 0-2 corner-anchored
bodies per seed (shaded planet with optional rings, cratered moon, galaxy
smudge, edge sun). Bodies anchor to corners because the capture window covers
the canvas center; only the padding band shows.

Tradeoffs made:

- The palette table went from tuples to a struct with a `BackdropKind`; the
  8 space palettes are color-sampled from astrophotography of the named
  objects (Orion, Carina, Pleiades, Rho Ophiuchi, Milky Way core, Andromeda,
  Horsehead/Flame, Lagoon/Trifid).
- Random rotation (`style_from_seed`) now picks space palettes only, per
  owner preference; the 5 legacy gradients remain reachable via `--palette`.
  Reels pinned to a gradient name keep working.
- Scene randomness derives from `style.seed` xor a salt, so `--style-seed`
  reproduces the exact scene and the same-seed determinism test still holds.
- `PresentationStyleInfo` (JSON contract) is untouched: the palette name
  already identifies the backdrop kind, so no schema change.
- Everything is hand-rolled on `image` + `rand` (no new dependencies).

## Planet/moon refinement from measured Hubble data (2026-07-17)

A brigade run scraped 30 ESA Hubble Top 100 images and median-cut quantized
the actual pixels (artifacts in untracked `run_artifacts/`). Changes grounded
in the measurements:

- Planet disc/band colors are now the measured warm cream/tan/gold pairs from
  the Jupiter and Saturn portraits (plus one Neptune-blue for variety),
  replacing palette-glow tinting that made planets look dyed.
- Limb darkening strengthened to ~3x (measured limb vs disc-center ratio);
  the blue atmosphere rim was replaced with a faint whitened echo of the body
  color (measured limbs darken and desaturate, they do not glow blue).
- Terminator uses a smoothstep S-curve (matches measured scanline falloff).
- Gas-giant bands are 3D-projected latitude with an axis tilt so they curve
  near the limb; rings gained a cylindrical globe shadow on the night side.
- Craters are directional relief (sun-facing wall shadowed, far rim
  highlighted) instead of flat dark dots; moons got maria mottling and
  surface roughness noise so they stop reading as smooth clay balls.

## Nebula-first rewrite, planets removed (2026-07-17)

Owner verdict: procedural planets/moons never looked good and the Hubble
reference frames are all about the gas. `src/space.rs` dropped the entire
body system (sphere/ring/crater rendering) and became nebula-first:

- Domain-warped fbm (a noise field offsets the sample point of another)
  turns soft blobs into curled filaments; three decorrelated cloud systems
  carry glow_a, glow_b, and stops[2] so one frame holds multiple hues.
- The base sky stays near-black; brightness lives inside the gas. A
  presence-based void multiplier pulls no-cloud regions toward black.
- Key transfer-curve lesson: fbm output effectively spans ~0.3-0.7, so the
  wisp() shaping must remap that band to 0..1 before powf, or cloud cores
  never saturate and every frame reads as grey haze regardless of palette.
- New features per Hubble refs: ridge ionization fronts (thin whitened
  crests), pink star-forming knots, star clusters at the ionization core,
  spikes on most mid-bright stars, galaxy color variety (spiral/elliptical/
  lenticular), and a 1-in-8 ultra-deep-field seed: black sky with 35-70 tiny
  galaxies and almost no gas.

## Scene-seed caveat (2026-07-17)

Scene generation draws a canvas-area-dependent number of RNG values while
placing field stars, so everything rolled after the starfield (cores, hero
galaxies, planetary nebulae) depends on canvas dimensions as well as the
seed. `--style-seed` reproduces a scene exactly only for the same input
size. Fine for the actual contract (same input, same card), but seed lists
computed at one canvas size do not transfer to another.

## JWST look (2026-07-17)

Roughly 45% of seeds now render a JWST variant (`scene.jwst`), driven by the
last reference batch of James Webb frames:

- 6-point snowflake diffraction spikes (`jwst_spike`): six primary rays at
  30-degree spacing plus two fainter half-length horizontal struts, matching
  the hexagonal-mirror + secondary-strut signature. Hubble scenes keep the
  4-point cross. The whole frame commits to one instrument look.
- Clumpy dust: a high-frequency lump field multiplies the cloud values down in
  the gaps, breaking the smooth fbm haze into cauliflower globules like Webb's
  elephant-trunk pillars.
- Inverted hero-galaxy palette: JWST mid-IR spirals read as red/orange PAH-dust
  arms around an electric blue-white old-star core, the reverse of the Hubble
  warm-core/blue-arm scheme.

The reachability test now also asserts both spike styles occur and at least one
JWST-palette hero appears across 400 seeds.

## Telescope cameo pass (2026-07-18)

Four more instrument looks joined the pool alongside Hubble/JWST:

- ALMA protoplanetary disc (HL Tau): tilted copper disc with dark concentric
  gap rings and a hot core, in the focal-object slot with the ring nebula and
  butterfly.
- SDO extreme-UV sun: 40% of suns render as a hard-limbed granulated gold
  disc with limb brightening, a short corona, and 2-4 coronal loop arcs,
  instead of the smooth glow.
- Chandra remnant (Cas A): new scene kind - a fragmented shell of teal/gold/
  red shards with a faint blue synchrotron haze inside.
- Planck CMB easter egg: ~1 in 24 seeds render the pure blue-to-orange
  temperature mottle with nothing else drawn.

The reachability test asserts all four appear across 400 seeds.

## Scene-pick flag (2026-07-18)

`cloche polish --scene <name>` pins the space scene look instead of leaving it
to the seed. Names: nebula, jwst, hubble, galaxy, alma, ring, butterfly, sun,
sdo, cluster, deep-field, veil, remnant, cmb (also exposed on the MCP `polish`
tool). `SceneKind` lives in `space.rs`; `PresentationStyle` carries an optional
`scene` the CLI sets after building the style.

Design note: `Scene::generate` honors the pin at each decision point, and the
unpinned (`None`) path draws the RNG in exactly the original order so existing
seeds render identically. Where a roll was previously behind a short-circuit
gate (hero, focal object), the pinned override stays inside that gate so it
never consumes an extra draw on the unpinned path. `--scene` on a gradient
palette is a no-op warning, since gradients have no scene.

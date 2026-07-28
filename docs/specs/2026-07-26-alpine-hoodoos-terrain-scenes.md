# Alpine and Hoodoos Terrain Scene Spike

## Status

**Completed (2026-07-26).** Both `alpine` and `hoodoos` were cut after three failed 4% visual reviews. The shipping change retains only `dunes`, `mesa`, `badlands`, and `glacier`. Cut reasons and work-verify receipt IDs are recorded in `implementation-notes.md`.

## Goal

Test two additions to Cloche's procedural terrain family:

- `alpine`: layered angular mountain ridges with sparse snow highlights.
- `hoodoos`: irregular clusters of capped rock spires with a continuous bedrock floor.

Only scenes that remain legible around a centered screenshot at finished-card padding width will ship.

## Why these two

The screenshot covers the center of a finished card and leaves roughly 4% of the backdrop visible on each side. Both candidates can carry their identifying structure through the outer bands:

- Alpine ridges form continuous silhouettes across the canvas. The existing dune horizon, badlands face lighting, and glacier highlights provide most of the required rendering vocabulary.
- Hoodoos form recognizable capped spires and layered protrusions. The implementation can reuse mesa cap and shoulder ideas plus badlands strata, but must avoid an evenly spaced fence.

Reference material:

- [Bryce Canyon hoodoo geology](https://www.nps.gov/brca/learn/nature/hoodoos.htm)
- [USGS description of alpine glacial peaks, cirques, and arêtes](https://pubs.usgs.gov/bul/1467a-d/report.pdf)

## Alternatives considered

### Alpine plus hoodoos

Recommended and approved. The pair adds one broad continuous silhouette and one sparse vertical silhouette, giving the spike two meaningfully different visual tests.

### Alpine plus canyon

Lower risk to render, but a canyon's identifying negative space would sit beneath the centered screenshot. The visible side strips could read as generic strata.

### Basalt columns or salt flats

Cut from this spike. Basalt columns risk repeating the badlands fence failure and overlapping the pattern family. Salt polygons would be confined mostly to the bottom strip and could read as a geometric motif rather than terrain.

## Existing contract

Terrain palettes own color and `TerrainKind` owns structure. Every accepted scene must work with all four existing terrain palettes:

- `dunes`
- `mesa`
- `badlands`
- `glacier`

The style seed controls every free parameter and must reproduce the same pixels. Rendering remains hand-rolled on the existing `image` and `rand` dependencies.

## Design

### Alpine

Generate three full-width ridge profiles:

1. A low-contrast far ridge near the horizon.
2. A mid ridge with larger angular peaks.
3. A darker near ridge that reaches both outer padding bands.

Each profile combines a small set of seeded peak anchors with low-amplitude warped noise. Piecewise slopes keep the skyline mountainous rather than cloud-like. Adjacent profiles use different salts and vertical ranges so they do not collapse into one band.

Face shading follows the local silhouette slope. Snow appears only near sufficiently high crests and on the lit side, using the palette highlight color. Snow is a sparse accent, not a white horizontal cap.

### Hoodoos

Generate two edge-anchored clusters with an optional smaller center cluster. Each cluster contains a few seeded spires with:

- a tapered shaft.
- an irregular resistant cap wider than the shaft.
- two or three horizontal erosion bands.
- small width and height differences between neighbors.

A low continuous bedrock profile connects the scene across the bottom. Spires must overlap or vary enough that their gaps and widths do not form a repeating picket pattern. The two outer clusters are mandatory because the center is normally hidden.

Lighting darkens one shaft face and lifts cap rims. Strata use the horizon color at low opacity.

## Code boundaries

Primary implementation stays in `src/terrain.rs`:

- add `Alpine` and `Hoodoos` to `TerrainKind`.
- add both public names to `TerrainKind::NAMES` and `from_name`.
- add generation ranges for horizon, coverage, feature scale, and light.
- add isolated profile helpers and structure painters for each scene.
- route the new variants through `base_layer` and `apply_structure`.

Existing public surfaces already derive terrain choices from `TerrainKind::NAMES`, so no separate CLI, MCP, config, or Studio enumeration should be introduced.

If either scene is accepted:

- update the terrain documentation in `README.md`.
- record the visual tradeoffs and iteration count in `implementation-notes.md`.

No new dependency, palette, command, configuration field, or rendering family is in scope.

## Data flow

1. `PresentationStyle` supplies the palette, style seed, and optional pinned terrain.
2. `Terrain::generate` derives scene parameters once from the salted seed.
3. `base_layer` paints the shared sky and ground ramps.
4. A scene-specific profile supplies the local horizon where required.
5. A scene-specific structure painter adds faces, strata, snow, or caps.
6. Shared grain and quantization produce the final opaque pixels.

Profile helpers remain pure functions so silhouette behavior can be tested without image comparison.

## Automated acceptance

Tests are written before implementation and observed failing.

Shared requirements:

- both names parse and appear in `TerrainKind::NAMES`.
- rendering is deterministic and seed-sensitive.
- zero-width and zero-height rendering remains safe.
- every outer edge band and corner contains variation.
- existing dunes, mesa, badlands, and glacier tests remain unchanged and green.

Alpine profile requirements:

- relief reaches both outer bands at 440×300.
- the near and middle ridges differ by at least 0.025 canvas height at 25% or more of sampled positions.
- adjacent samples cannot contain single-column cliffs.
- snow covers 1% to 12% of the ground pixels and never appears below the scene's elevation threshold.

Hoodoo profile requirements:

- the first and last 12.5% of the canvas each contain a spire rising at least 0.05 canvas height above bedrock.
- shaft widths exceed one pixel at 440×300.
- visible spire-center gaps vary by at least 0.04 canvas width between the smallest and largest gap.
- the bedrock profile stays at least 0.04 canvas height across the width.
- every cap is at least 1.25 times its shaft width.

## Visual acceptance

Render full backdrops and finished cards for:

- scenes: `alpine`, `hoodoos`.
- palettes: all four terrain palettes.
- seeds: 1, 7, 42, and 99.
- review size: 440×300.

For each scene, inspect all 16 combinations for:

- recognition from the visible side, top, and bottom strips.
- no dependence on a centered subject.
- no flat outer band.
- no fence, barcode, smoke, or generic noise reading.
- enough contrast behind both light and dark screenshots.
- clear distinction from the existing four terrain scenes.

Allow at most three implementation-and-render iterations per scene. An iteration means one code adjustment followed by the full 16-render visual sheet. If a scene still fails after its third sheet, remove that scene and its tests from the shipping change, record why, and continue with any scene that passed.

## Verification and delivery

After the last tracked edit, run:

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

An accepted change must have:

- the full verification entrypoint green.
- accepted visual sheets for every retained scene.
- no ignored tests, lint allowances, debug output, or new dependency.
- a memory handoff recording durable profile and visual-QA lessons.

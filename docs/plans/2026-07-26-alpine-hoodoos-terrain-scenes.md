# Alpine and Hoodoos Terrain Scenes Implementation Plan

> **HISTORICAL RECORD: DO NOT RE-RUN.** This plan documents a completed spike that executed 2026-07-26. Both scenes were cut after three failed 4% visual reviews. Do not re-execute Tasks 0–7, re-export visual sheets, or re-open the keep branches below.

## Final outcome (2026-07-26)

Both `alpine` and `hoodoos` were **CUT**. Retained kinds: `dunes`, `mesa`, `badlands`, `glacier` (`TerrainKind::NAMES` length 4).

| Scene | Iteration 1 | Iteration 2 | Iteration 3 | Decision |
|---|---|---|---|---|
| `alpine` | `20260726-214654-work-verify-3489fc`: flat horizontal bands and snow speckles | `20260726-215339-work-verify-79cf17`: repeated sawtooth silhouette and window-like snow patches | `20260726-221354-work-verify-d2ffbe`: rolling bands with a snow rim, still indistinct from glacier and mesa | **CUT** |
| `hoodoos` | `20260726-221945-work-verify-b63dec`: tiny T-shaped posts and striped bedrock | `20260726-222329-work-verify-015ad1`: squat mesa-like forms with pale caps | `20260726-222618-work-verify-c96372`: repeated fence-like pillars with caps too small around cards | **CUT** |

- Temporary visual exporter removed before the branch was squashed.
- Final code verification: `20260727-001059-work-verify-dedfb3` (`./scripts/verify` green on the four-kind baseline).
- No README terrain section because neither scene shipped. Details are in `implementation-notes.md`.

---

> **For agentic workers (historical):** Implement task-by-task in order. Each behavior task adds failing tests, runs RED, implements the minimum production code to go GREEN, then commits **once**. Never commit RED tests, compiling stubs, `todo!()`, `panic!` placeholders, or a partially rendered public variant. The first visual sheet evaluates that test-green commit. After a visual failure, keep every adjustment uncommitted until the scene passes or is cut. Never commit a known-bad visual state.

**Goal (historical):** Ship `alpine` and `hoodoos` as two new `TerrainKind` variants in `src/terrain.rs`, each legible in the ~4% outer padding band of a finished card across all four existing terrain palettes (`dunes`, `mesa`, `badlands`, `glacier`).

**Architecture:** Terrain palettes own color. `TerrainKind` owns structure. `PresentationStyle` supplies palette, seed, and optional pinned terrain. `Terrain::generate` rolls scene parameters once. `base_layer` paints sky/ground ramps and calls a per-kind horizon/profile. `apply_structure` dispatches to `structure_alpine` or `structure_hoodoos`. CLI, config, and Studio derive terrain names from `TerrainKind::NAMES`. MCP tool schemas use `polish::terrain_names()` for the enum but hard-code human descriptions that must be updated.

**Tech stack:** Rust 2024, `image`, `rand` (existing deps only). Verification through Brigade.

**Spec:** `docs/specs/2026-07-26-alpine-hoodoos-terrain-scenes.md`

**GraphTrail impact set (must stay green):** `TerrainKind`, `TerrainKind::from_name`, `Terrain::generate`, `terrain_from_name`, `resolve_style`, `style_from_query`, `capture`, `run_polish`, `backdrop_png`, `card_png`, `render`, `apply_structure`, shared terrain enum/render tests.

**`NAMES` length progression:** 4 (baseline) → 5 after Alpine commit → 6 after Hoodoos commit. Each scene task commits only when its variant is fully rendered and the whole suite compiles.

---

## File map

| File | Action | What changes |
|---|---|---|
| `src/terrain.rs` | Modify | Per-scene enum extension, `NAMES`, `from_name`, `Terrain::generate`, `base_layer`, `apply_structure`, alpine/hoodoo profile helpers, structure painters, profile tests, visual export test |
| `src/polish.rs` | Modify | Keep terrain-kind expectations aligned as `NAMES` grows. Decouple the four terrain palette assertions from the terrain structure axis |
| `src/mcp.rs` | Modify | Terrain `description` strings on capture and polish tools. extend `capture_tool_advertises_the_terrain_enum` to assert enum entries and both description strings |
| `README.md` | Modify | Terrain backdrop section: add retained kind names |
| `implementation-notes.md` | Modify | Visual iteration counts, keep/cut decisions, exact tradeoff sentences |
| `.claude/memory-handoffs/` | Create | Durable profile + visual-QA lessons (maintainer-local. skip if directory absent) |

**Files that must NOT change:** `src/cli.rs`, `src/config.rs`, `src/studio.rs`, `Cargo.toml`, palette table in `polish.rs`. New kinds flow through `TerrainKind::NAMES` automatically.

**Visual artifacts:** `/tmp/cloche-terrain-scenes/` (never committed).

---

## Brigade verification contract

Run every check through Brigade from the repo root:

```bash
brigade work brief --target .
brigade work verify run --target . --command "<command>" --capture brigade-work
```

Brigade runs `--command` with `shell=False`. pass **one** executable and its arguments. For `cargo test`, use **one** substring filter per invocation (no space-separated test names).

| When | Command | RED reason | GREEN reason |
|---|---|---|---|
| Task 1 complete | `cargo test terrain::tests::alpine_ -- --nocapture` | `Alpine` missing, `NAMES.len() != 5`, or alpine helper/profile/snow/layer assertions fail | All `alpine_*` tests pass. `NAMES` has five entries. Alpine fully rendered |
| Task 2 complete | `cargo test terrain::tests::hoodoo_ -- --nocapture` | `Hoodoos` missing, `NAMES.len() != 6`, or hoodoo geometry/material assertions fail | All `hoodoo_*` tests pass. `NAMES` has six entries. Hoodoos fully rendered |
| Task 3 complete | `cargo test terrain_names_lists -- --nocapture` | Expected vec still has 4 entries | `terrain_names()` returns six kinds in menu order |
| Task 3 MCP | `cargo test capture_tool_advertises_the_terrain_enum -- --nocapture` | Enum length, missing names, or description strings omit `alpine`/`hoodoos` | Enum length 6. both names present. both descriptions mention both names |
| Task 3 shared | `cargo test every_kind -- --nocapture` | Any `every_kind_*` test fails for a new variant | All six kinds pass edge/corner/opacity/variation tests |
| Task 3 distinction | `cargo test kinds_render_differently_from_each_other -- --nocapture` | New kind matches dunes at seed 5 | Each kind differs from dunes |
| Final delivery | `./scripts/verify` | fmt, clippy, or any test fails | Full CI entrypoint green |

Full verify (final task only):

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

---

## Visual sheet protocol

**Review size:** 440×300. **Seeds:** 1, 7, 42, 99. **Palettes:** `dunes`, `mesa`, `badlands`, `glacier`. **Scenes:** one per run via `CLOCHE_VISUAL_SCENE` (`alpine` or `hoodoos`). **Iteration:** `CLOCHE_VISUAL_ITER` (`1`, `2`, or `3`).

Each scene gets **at most three iterations**. One iteration = one code adjustment + one export run producing:

- **16 individual backdrop PNGs** (`{palette}-seed{seed}-backdrop.png`)
- **32 individual finished-card PNGs** (`{palette}-seed{seed}-card-light.png`, `{palette}-seed{seed}-card-dark.png` - two cards per palette/seed)
- 1 backdrop contact sheet (`contact-backdrops.png`, 4×4 grid of normalized 440×300 tiles)
- 1 light-card contact sheet (`contact-cards-light.png`, 4×4 grid)
- 1 dark-card contact sheet (`contact-cards-dark.png`, 4×4 grid)

Individual card PNGs keep `compose_card` output dimensions. Contact-sheet tiles are resized to 440×300 before stitching because style padding changes card size.

Output root: `/tmp/cloche-terrain-scenes/{scene}/iter{iteration}/`.

### Export helper (add in Task 3)

Add to `src/terrain.rs` `#[cfg(test)]` module:

```rust
use image::DynamicImage;
use image::Rgba;
use image::RgbaImage;

fn mock_screenshot(light: bool) -> DynamicImage {
    let rgb = if light {
        [245_u8, 245, 248]
    } else {
        [28, 28, 32]
    };
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(400, 300, Rgba([rgb[0], rgb[1], rgb[2], 255])))
}

fn normalize_contact_tile(tile: &RgbaImage) -> RgbaImage {
    image::imageops::resize(tile, 440, 300, image::imageops::FilterType::Triangle)
}

fn stitch_contact_sheet(tiles: &[RgbaImage], cols: u32) -> RgbaImage {
    let normalized: Vec<RgbaImage> = tiles.iter().map(normalize_contact_tile).collect();
    let tile_w = 440_u32;
    let tile_h = 300_u32;
    let rows = (normalized.len() as u32 + cols - 1) / cols;
    let mut sheet = RgbaImage::new(tile_w * cols, tile_h * rows);
    for (index, tile) in normalized.iter().enumerate() {
        let col = (index as u32) % cols;
        let row = (index as u32) / cols;
        image::imageops::overlay(
            &mut sheet,
            tile,
            (col * tile_w) as i64,
            (row * tile_h) as i64,
        );
    }
    sheet
}

fn write_visual_sheet() {
    let scene = match std::env::var("CLOCHE_VISUAL_SCENE").ok().as_deref() {
        Some("alpine") | Some("hoodoos") => std::env::var("CLOCHE_VISUAL_SCENE").unwrap(),
        _ => return,
    };
    let iteration: u32 = std::env::var("CLOCHE_VISUAL_ITER")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let root = std::path::PathBuf::from("/tmp/cloche-terrain-scenes")
        .join(&scene)
        .join(format!("iter{iteration}"));
    let palettes = ["dunes", "mesa", "badlands", "glacier"];
    let seeds = [1_u64, 7, 42, 99];
    let mut backdrops = Vec::new();
    let mut cards_light = Vec::new();
    let mut cards_dark = Vec::new();
    for palette in palettes {
        for seed in seeds {
            let mut style = polish::style_with_palette(seed, palette).expect("palette");
            style.terrain = TerrainKind::from_name(&scene);
            let backdrop = render(440, 300, &style);
            let card_light = polish::compose_card(&mock_screenshot(true), &style);
            let card_dark = polish::compose_card(&mock_screenshot(false), &style);
            std::fs::create_dir_all(&root).unwrap();
            backdrop
                .save(root.join(format!("{palette}-seed{seed}-backdrop.png")))
                .unwrap();
            card_light
                .save(root.join(format!("{palette}-seed{seed}-card-light.png")))
                .unwrap();
            card_dark
                .save(root.join(format!("{palette}-seed{seed}-card-dark.png")))
                .unwrap();
            backdrops.push(backdrop);
            cards_light.push(card_light);
            cards_dark.push(card_dark);
        }
    }
    stitch_contact_sheet(&backdrops, 4)
        .save(root.join("contact-backdrops.png"))
        .unwrap();
    stitch_contact_sheet(&cards_light, 4)
        .save(root.join("contact-cards-light.png"))
        .unwrap();
    stitch_contact_sheet(&cards_dark, 4)
        .save(root.join("contact-cards-dark.png"))
        .unwrap();
}

#[test]
fn export_visual_sheet_when_env_set() {
    write_visual_sheet();
}
```

Run a sheet (no code edit between iterations - only env vars and terrain adjustments):

```bash
CLOCHE_VISUAL_SCENE=alpine CLOCHE_VISUAL_ITER=1 \
  brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

```bash
CLOCHE_VISUAL_SCENE=hoodoos CLOCHE_VISUAL_ITER=1 \
  brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

Inspect `/tmp/cloche-terrain-scenes/{scene}/iter{N}/contact-*.png` first, then spot-check individual PNGs.

### Inspection checklist (every sheet)

- [x] Recognizable from visible side, top, and bottom strips without a centered subject: **failed** (alpine cut)
- [x] No flat outer band (edge pixels vary): **failed** (iter 1 flat bands)
- [x] No fence, barcode, smoke, or generic noise reading: **failed** (iter 2 sawtooth and iter 3 rolling bands)
- [x] Enough contrast behind both light and dark screenshot content: marginal, not decisive
- [x] Clearly distinct from dunes, mesa, badlands, and glacier at the same seed/palette: **failed** (iter 3 indistinct from glacier/mesa)
- [x] Alpine: three visibly separated ridge layers (far atmospheric, mid angular, near dominant): **failed** (all three iterations)
- [x] Hoodoos: capped spires with wider caps and narrower shafts above continuous bedrock: **failed** (hoodoos cut)

### Keep-or-cut rules

| Outcome | Action |
|---|---|
| Sheet passes checklist on iteration 1 | **Keep** the existing test-green implementation commit. record iteration count in `implementation-notes.md`. do not create an empty visual-fix commit |
| Sheet passes checklist on iteration 2 or 3 | **Keep** scene. commit the accepted visual adjustments once. record iteration count in `implementation-notes.md` |
| Sheet fails after iteration 3 | **Cut** scene: apply explicit patches removing the variant, all scene-specific helpers/tests, and `base_layer`/`apply_structure`/`generate` arms. restore `NAMES` length. document cut reason. commit `revert(terrain): cut alpine after failed visual QA` or `revert(terrain): cut hoodoos after failed visual QA` **only** with documentation updates |
| Both scenes pass | Ship both. README lists six terrain kinds |
| One scene passes, one cut | Ship the survivor only. README lists five terrain kinds |
| Both cut | Revert the spike. do not merge enum extension. document both failures |

**Iteration discipline:** The first sheet evaluates the test-green implementation commit. Once a sheet fails, keep all adjustments **uncommitted**. If a later sheet passes, make one commit with the accepted adjustments. Never commit intermediate visual attempts.

---

## Task 1: Alpine - enum, profiles, layer paint, and snow mask (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/polish.rs`

Alpine uses **piecewise-linear angular ridge profiles** (seeded anchor points connected by straight segments, plus tiny warped noise). Ridge y-order uses `far_y < mid_y < near_y` (smaller normalized `v` is higher on the canvas). This is **screen-space silhouette stacking** so each band stays visible in the card padding strip - not a physical elevation model. `structure_alpine` must **paint** far, mid, and near ridge layers with distinct masks - not merely compute profiles for the horizon line.

- [x] **Step 1: Add failing alpine tests** (prefix `alpine_` for the single-filter verify command)

```rust
fn alpine_test_terrain(seed: u64) -> Terrain {
    let mut rng = StdRng::seed_from_u64(seed);
    Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::Alpine))
}

fn sample_alpine_ridges(terrain: &Terrain, samples: usize) -> Vec<(f32, f32, f32)> {
    (0..samples)
        .map(|index| {
            let fx = index as f32 * terrain.width / samples as f32;
            alpine_ridge_profiles(fx, terrain)
        })
        .collect()
}

fn alpine_near_ridge_material_sample_count(terrain: &Terrain, samples: usize) -> usize {
    let mut count = 0usize;
    for row in 0..samples {
        for col in 0..samples {
            let fx = col as f32 * terrain.width / samples as f32;
            let v = row as f32 / samples as f32;
            let (near, _, _) = alpine_ridge_profiles(fx, terrain);
            if v >= near {
                count += 1;
            }
        }
    }
    count
}

fn alpine_snow_sample_count(terrain: &Terrain, samples: usize) -> usize {
    let mut count = 0usize;
    for row in 0..samples {
        for col in 0..samples {
            let fx = col as f32 * terrain.width / samples as f32;
            let v = row as f32 / samples as f32;
            if alpine_snow_mask(fx, v, terrain) > 0.5 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn alpine_name_parses_and_names_len_five() {
    assert_eq!(TerrainKind::from_name("alpine"), Some(TerrainKind::Alpine));
    assert!(TerrainKind::NAMES.contains(&"alpine"));
    assert_eq!(TerrainKind::NAMES.len(), 5);
}

#[test]
fn alpine_profile_reaches_both_outer_bands() {
    let terrain = alpine_test_terrain(7);
    let samples = 440usize;
    let mut left_peak = 0.0_f32;
    let mut right_peak = 0.0_f32;
    for index in 0..samples {
        let fx = index as f32;
        let near = alpine_horizon(fx, &terrain);
        let relief = terrain.horizon - near;
        if index < 55 {
            left_peak = left_peak.max(relief);
        }
        if index >= 385 {
            right_peak = right_peak.max(relief);
        }
    }
    let max_relief = (0..samples)
        .map(|i| terrain.horizon - alpine_horizon(i as f32, &terrain))
        .fold(0.0_f32, f32::max);
    let threshold = max_relief * 0.40;
    assert!(left_peak >= threshold, "left {left_peak} < {threshold}");
    assert!(right_peak >= threshold, "right {right_peak} < {threshold}");
}

#[test]
fn alpine_profile_ridges_ordered_far_mid_near() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = alpine_test_terrain(seed);
        let ridges = sample_alpine_ridges(&terrain, 160);
        let ordered = ridges.iter().all(|(near, mid, far)| far < mid && mid < near);
        let differing = ridges
            .iter()
            .filter(|(near, mid, _)| (near - mid).abs() >= 0.025)
            .count();
        assert!(ordered, "seed {seed}: ridge y-order violated");
        assert!(
            differing as f32 / ridges.len() as f32 >= 0.25,
            "seed {seed}: only {differing}/{} samples separate near/mid",
            ridges.len()
        );
    }
}

#[test]
fn alpine_profile_avoids_single_column_cliffs() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = alpine_test_terrain(seed);
        let crests: Vec<f32> = (0..240)
            .map(|i| alpine_horizon(i as f32 * terrain.width / 240.0, &terrain))
            .collect();
        let relief = terrain.horizon - crests.iter().copied().fold(f32::INFINITY, f32::min);
        let max_drop = max_adjacent_drop(&crests);
        assert!(
            max_drop < relief * 0.80,
            "seed {seed}: cliff drop {max_drop} for relief {relief}"
        );
    }
}

#[test]
fn alpine_layer_classifier_separates_ridge_bands() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = alpine_test_terrain(seed);
        let fx = terrain.width * 0.5;
        let (near, mid, far) = alpine_ridge_profiles(fx, &terrain);
        assert_eq!(alpine_layer_at(fx, far - 0.02, &terrain), AlpineLayer::Air);
        assert_eq!(
            alpine_layer_at(fx, (far + mid) * 0.5, &terrain),
            AlpineLayer::FarRidge
        );
        assert_eq!(
            alpine_layer_at(fx, (mid + near) * 0.5, &terrain),
            AlpineLayer::MidRidge
        );
        assert_eq!(
            alpine_layer_at(fx, near + 0.02, &terrain),
            AlpineLayer::NearMountain
        );
        assert_eq!(
            alpine_layer_at(fx, terrain.horizon + 0.05, &terrain),
            AlpineLayer::Ground
        );
    }
}

#[test]
fn alpine_snow_mask_coverage_is_sparse() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = alpine_test_terrain(seed);
        let samples = 80usize;
        let material = alpine_near_ridge_material_sample_count(&terrain, samples);
        let snow = alpine_snow_sample_count(&terrain, samples);
        assert!(material > 0, "seed {seed}: no near-ridge material samples");
        let pct = snow as f32 / material as f32;
        assert!((0.01..=0.12).contains(&pct), "seed {seed}: snow {pct:.3}");
    }
}

#[test]
fn alpine_snow_mask_zero_in_air_and_below_crest_band() {
    const MAX_SNOW_DEPTH: f32 = 0.14;
    for seed in [1_u64, 7, 42, 99] {
        let terrain = alpine_test_terrain(seed);
        let samples = 80usize;
        for row in 0..samples {
            for col in 0..samples {
                let fx = col as f32 * terrain.width / samples as f32;
                let v = row as f32 / samples as f32;
                let (near, _, _) = alpine_ridge_profiles(fx, &terrain);
                let mask = alpine_snow_mask(fx, v, &terrain);
                if v < near && mask > 0.5 {
                    panic!("seed {seed}: snow in air above near crest at v={v}");
                }
                if v >= near && (v - near) > MAX_SNOW_DEPTH && mask > 0.5 {
                    panic!("seed {seed}: snow below allowed crest band at v={v}");
                }
            }
        }
    }
}
```

- [x] **Step 2: Verify RED**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::alpine_ -- --nocapture" \
  --capture brigade-work
```

Expected: **FAIL** - compile error (`Alpine` variant missing), `NAMES.len() == 4`, or missing `alpine_ridge_profiles` / `alpine_snow_mask` / `alpine_layer_at`.

- [x] **Step 3: Implement alpine production code**

Add the `Alpine` variant and extend `NAMES` to five entries (do **not** add `Hoodoos` yet):

```rust
pub enum TerrainKind {
    Dunes,
    Mesa,
    Badlands,
    Glacier,
    /// Layered angular mountain ridges with sparse snow highlights.
    Alpine,
}

impl TerrainKind {
    pub const NAMES: [&'static str; 5] =
        ["dunes", "mesa", "badlands", "glacier", "alpine"];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "dunes" => Self::Dunes,
            "mesa" => Self::Mesa,
            "badlands" => Self::Badlands,
            "glacier" => Self::Glacier,
            "alpine" => Self::Alpine,
            _ => return None,
        })
    }
}
```

In `Terrain::generate`:

```rust
TerrainKind::Alpine => (rng.random_range(0.38..=0.52), 0.72, 5.5),
```

Glacier-style light for Alpine (same as Glacier arm).

Piecewise-linear ridge helper and profiles:

```rust
const ALPINE_RIDGE_SALTS: [u64; 3] = [0x414C_5046, 0x414C_504D, 0x414C_504E];

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlpineLayer {
    Air,
    FarRidge,
    MidRidge,
    NearMountain,
    Ground,
}

fn alpine_peak_anchors(terrain: &Terrain, layer: usize) -> [(f32, f32); 5] {
    let salt = ALPINE_RIDGE_SALTS[layer];
    std::array::from_fn(|index| {
        let u = (index as f32 + 0.5) / 5.0;
        let amp = 0.55 + cell_hash(index as i64, layer as i64, terrain.noise_seed ^ salt) * 0.45;
        (u, amp)
    })
}

fn alpine_piecewise_peak(u: f32, anchors: &[(f32, f32)]) -> f32 {
    if u <= anchors[0].0 {
        return anchors[0].1;
    }
    for window in anchors.windows(2) {
        let (u0, a0) = window[0];
        let (u1, a1) = window[1];
        if u <= u1 {
            let t = (u - u0) / (u1 - u0).max(0.001);
            return a0 + (a1 - a0) * t;
        }
    }
    anchors.last().copied().map(|(_, a)| a).unwrap_or(0.0)
}

fn alpine_ridge_height(fx: f32, terrain: &Terrain, layer: usize, base: f32, amp: f32) -> f32 {
    let u = fx / terrain.width.max(1.0);
    let anchors = alpine_peak_anchors(terrain, layer);
    let peak = alpine_piecewise_peak(u, &anchors);
    let noise = warped_fbm(
        u * 2.4 + layer as f32 * 1.7,
        0.41,
        terrain.noise_seed ^ ALPINE_RIDGE_SALTS[layer],
        3,
    );
    (base - amp * (0.40 + peak * 0.60) - noise * amp * 0.06).clamp(0.20, 0.78)
}

fn alpine_ridge_profiles(fx: f32, terrain: &Terrain) -> (f32, f32, f32) {
    let far = alpine_ridge_height(fx, terrain, 0, terrain.horizon - 0.20, 0.05);
    let mid = alpine_ridge_height(fx, terrain, 1, terrain.horizon - 0.12, 0.07);
    let near = alpine_ridge_height(fx, terrain, 2, terrain.horizon - 0.02, 0.05);
    (near, mid, far)
}

fn alpine_horizon(fx: f32, terrain: &Terrain) -> f32 {
    alpine_ridge_profiles(fx, terrain).0
}

#[cfg(test)]
fn alpine_layer_at(fx: f32, v: f32, terrain: &Terrain) -> AlpineLayer {
    let (near, mid, far) = alpine_ridge_profiles(fx, terrain);
    if v < far {
        AlpineLayer::Air
    } else if v < mid {
        AlpineLayer::FarRidge
    } else if v < near {
        AlpineLayer::MidRidge
    } else if v < terrain.horizon + 0.03 {
        AlpineLayer::NearMountain
    } else {
        AlpineLayer::Ground
    }
}

/// Pure snow accent mask in 0..1. Tests sample this directly - never infer snow from output RGB.
/// Mountain material starts at v >= near. Snow only in a shallow band immediately below the near crest.
/// v < near is air - always zero.
fn alpine_snow_mask(fx: f32, v: f32, terrain: &Terrain) -> f32 {
    let (near, mid, _far) = alpine_ridge_profiles(fx, terrain);
    if v < near {
        return 0.0;
    }
    let elevation_below_crest = v - near;
    const MAX_SNOW_DEPTH: f32 = 0.14;
    if elevation_below_crest > MAX_SNOW_DEPTH {
        return 0.0;
    }
    let crest_proximity = smoothstep_range(elevation_below_crest, 0.0, 0.06);
    let mid_separation = smoothstep_range((near - mid).abs(), 0.012, 0.030);
    let sample = terrain.width * 0.020;
    let left = alpine_horizon(fx - sample, terrain);
    let right = alpine_horizon(fx + sample, terrain);
    let slope = ((right - left) * 8.0).clamp(-1.0, 1.0);
    let lit_face = if terrain.light.0 >= 0.5 {
        slope.max(0.0)
    } else {
        (-slope).max(0.0)
    };
    let patch = cell_hash(
        (fx * 0.37) as i64,
        ((v - near) * 640.0) as i64,
        terrain.noise_seed ^ 0x534E_4F57,
    );
    let patch_gate = smoothstep_range(patch, 0.82, 0.94);
    let exposure = lit_face.max(slope.abs() * 0.45).max(patch_gate);
    (crest_proximity * mid_separation * exposure * terrain.coverage * 1.35).clamp(0.0, 1.0)
}
```

`structure_alpine` paints three ridge layers then applies `alpine_snow_mask`:

```rust
fn structure_alpine(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let light = ctx.light;
    let shade = ctx.shade;
    let (near, mid, far) = alpine_ridge_profiles(fx, terrain);

    let far_body = smoothstep_range(v, far - 0.008, far + 0.018)
        * (1.0 - smoothstep_range(v, mid - 0.006, mid));
    let mid_body = smoothstep_range(v, mid - 0.010, mid + 0.022)
        * (1.0 - smoothstep_range(v, near - 0.008, near));
    let near_ground = smoothstep_range(v, near - 0.016, near + 0.030);

    let sample = terrain.width * 0.020;
    let left = alpine_horizon(fx - sample, terrain);
    let right = alpine_horizon(fx + sample, terrain);
    let slope = ((right - left) * 8.0).clamp(-1.0, 1.0);
    let lit_face = if terrain.light.0 >= 0.5 {
        slope.max(0.0)
    } else {
        (-slope).max(0.0)
    };

    let mut color = base;
    color = mix3(color, shade, far_body * terrain.coverage * 0.10);
    color = mix3(color, shade, mid_body * terrain.coverage * 0.16);
    let near_shaded = mix3(color, shade, near_ground * terrain.coverage * 0.22);
    let faced = mix3(near_shaded, shade, lit_face * near_ground * 0.26);
    let lit = mix3(faced, light, lit_face * near_ground * 0.20);
    let snow = alpine_snow_mask(fx, v, terrain) * near_ground;
    mix3(lit, light, (snow * 0.55).min(0.48))
}
```

Wire `base_layer` and `apply_structure`:

```rust
TerrainKind::Alpine => alpine_horizon(fx, terrain),
TerrainKind::Alpine => structure_alpine(base, ctx),
```

In `src/polish.rs`, rename `terrain_names_lists_the_four_kinds` to
`terrain_names_lists_every_kind` and expect the five current structure names:

```rust
#[test]
fn terrain_names_lists_every_kind() {
    assert_eq!(
        terrain_names(),
        vec!["dunes", "mesa", "badlands", "glacier", "alpine"]
    );
}
```

`terrain_palettes_catalog_as_terrain` tests palette metadata, not the structure
axis. Keep it restricted to the four existing terrain palettes:

```rust
#[test]
fn terrain_palettes_catalog_as_terrain() {
    let catalog = palette_catalog();
    for name in ["dunes", "mesa", "badlands", "glacier"] {
        let entry = catalog
            .iter()
            .find(|(palette, _)| *palette == name)
            .unwrap_or_else(|| panic!("terrain palette {name} missing from catalog"));
        assert_eq!(entry.1, "terrain", "palette {name} cataloged wrong");
    }
}
```

Do not add an `alpine` palette. Alpine is a structure that composes with all
four existing terrain palettes.

- [x] **Step 4: Verify GREEN**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::alpine_ -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test every_name_round_trips -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test every_kind -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - all `alpine_*` tests green. five kinds compile. Alpine fully rendered in shared tests.

- [x] **Step 5: Commit once**

```bash
git add src/terrain.rs src/polish.rs
git commit -m "feat(terrain): add alpine ridge layers and snow mask"
```

Completed during the discarded spike history. RED: `20260726-213241-work-verify-ece720`.
Focused GREEN: `20260726-213406-work-verify-413c05`,
`20260726-213406-work-verify-da9e26`, and
`20260726-213407-work-verify-6332bc`. Full verification:
`20260726-213753-work-verify-00e428`.

---

## Task 2: Hoodoos - enum, spires, and material query (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/polish.rs`

Keep **normalized** coordinates (`center_u`, `shaft_half_u`, `height`, `cap_scale`) separate from pixel widths (`shaft_half_u * terrain.width`). `hoodoo_material_at(u, v, terrain)` classifies each point. do **not** fold cap width into a single triangular horizon that spans the full shaft.

- [x] **Step 1: Add failing hoodoo tests** (prefix `hoodoo_`)

```rust
fn hoodoo_test_terrain(seed: u64) -> Terrain {
    let mut rng = StdRng::seed_from_u64(seed);
    Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::Hoodoos))
}

#[test]
fn hoodoo_name_parses_and_names_len_six() {
    assert_eq!(TerrainKind::from_name("hoodoos"), Some(TerrainKind::Hoodoos));
    assert!(TerrainKind::NAMES.contains(&"hoodoos"));
    assert_eq!(TerrainKind::NAMES.len(), 6);
}

#[test]
fn hoodoo_profile_spires_anchor_both_outer_bands() {
    let terrain = hoodoo_test_terrain(7);
    let w = terrain.width;
    let mut left_peak = 0.0_f32;
    let mut right_peak = 0.0_f32;
    for index in 0..55 {
        let u = index as f32 / w;
        let crest_v = hoodoo_spire_crest_v(u, &terrain);
        left_peak = left_peak.max(hoodoo_bedrock_y(u, &terrain) - crest_v);
    }
    for index in 385..440 {
        let u = index as f32 / w;
        let crest_v = hoodoo_spire_crest_v(u, &terrain);
        right_peak = right_peak.max(hoodoo_bedrock_y(u, &terrain) - crest_v);
    }
    assert!(left_peak >= 0.05, "left spire relief {left_peak}");
    assert!(right_peak >= 0.05, "right spire relief {right_peak}");
}

#[test]
fn hoodoo_shaft_widths_exceed_one_pixel() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = hoodoo_test_terrain(seed);
        let min_shaft = hoodoo_spires(&terrain)
            .iter()
            .map(|spire| spire.shaft_half_u * 2.0 * terrain.width)
            .fold(f32::INFINITY, f32::min);
        assert!(min_shaft > 1.0, "seed {seed}: min shaft {min_shaft}px");
    }
}

#[test]
fn hoodoo_gap_variation_at_least_004_canvas() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = hoodoo_test_terrain(seed);
        let mut centers: Vec<f32> = hoodoo_spires(&terrain)
            .iter()
            .map(|spire| spire.center_u)
            .collect();
        centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let gaps: Vec<f32> = centers
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs() * terrain.width)
            .filter(|gap| *gap > 1.0)
            .collect();
        let spread = gaps.iter().copied().fold(0.0_f32, f32::max)
            - gaps.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            spread >= terrain.width * 0.04,
            "seed {seed}: gap spread {spread}"
        );
    }
}

#[test]
fn hoodoo_bedrock_stays_continuous() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = hoodoo_test_terrain(seed);
        let profile: Vec<f32> = (0..440)
            .map(|i| {
                let u = i as f32 / terrain.width;
                terrain.horizon - hoodoo_bedrock_y(u, &terrain)
            })
            .collect();
        let peak = profile.iter().copied().fold(0.0_f32, f32::max);
        let floor = peak * 0.18;
        let min_body = profile.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(min_body >= floor, "seed {seed}: bedrock dip {min_body}");
    }
}

#[test]
fn hoodoo_caps_are_wider_than_shafts() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = hoodoo_test_terrain(seed);
        for spire in hoodoo_spires(&terrain) {
            let shaft_px = spire.shaft_half_u * terrain.width;
            let cap_px = spire.shaft_half_u * spire.cap_scale * terrain.width;
            assert!(
                cap_px >= shaft_px * 1.25,
                "seed {seed}: cap {cap_px} < 1.25 * shaft {shaft_px}"
            );
        }
    }
}

#[test]
fn hoodoo_material_at_separates_cap_and_shaft() {
    let terrain = hoodoo_test_terrain(7);
    let spire = hoodoo_spires(&terrain)
        .into_iter()
        .find(|s| s.center_u > 0.1 && s.center_u < 0.9)
        .expect("center spire");
    let u = spire.center_u;
    let bed = hoodoo_bedrock_y(u, &terrain);
    let crest = bed - spire.height * terrain.coverage;
    let cap_band_top = crest + 0.012;
    let shaft_mid = (bed + crest) * 0.5;
    assert_eq!(hoodoo_material_at(u, cap_band_top, &terrain), HoodooMaterial::Cap);
    assert_eq!(hoodoo_material_at(u, shaft_mid, &terrain), HoodooMaterial::Shaft);
    assert_eq!(hoodoo_material_at(u, bed + 0.001, &terrain), HoodooMaterial::Bedrock);
}
```

Add helper used by outer-band test:

```rust
fn hoodoo_spire_crest_v(u: f32, terrain: &Terrain) -> f32 {
    let mut crest = terrain.horizon;
    for spire in hoodoo_spires(terrain) {
        let dx_u = (u - spire.center_u).abs();
        let cap_half_u = spire.shaft_half_u * spire.cap_scale;
        if dx_u > cap_half_u {
            continue;
        }
        let taper = 1.0 - (dx_u / cap_half_u.max(0.001)).clamp(0.0, 1.0);
        let candidate = hoodoo_bedrock_y(u, terrain) - spire.height * taper * terrain.coverage;
        crest = crest.min(candidate);
    }
    crest
}
```

- [x] **Step 2: Verify RED**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::hoodoo_ -- --nocapture" \
  --capture brigade-work
```

Expected: **FAIL** - compile error (`Hoodoos` variant missing), `NAMES.len() == 5`, empty clusters, or material query returns only bedrock.

- [x] **Step 3: Implement hoodoo production code**

Extend enum and `NAMES` to six entries:

```rust
    /// Capped rock spire clusters on a continuous bedrock floor.
    Hoodoos,

    pub const NAMES: [&'static str; 6] =
        ["dunes", "mesa", "badlands", "glacier", "alpine", "hoodoos"];
```

Add `from_name` arm and `generate` arm:

```rust
"hoodoos" => Self::Hoodoos,
TerrainKind::Hoodoos => (rng.random_range(0.48..=0.62), 0.68, 4.5),
```

Side light for Hoodoos (same as Dunes/Mesa/Badlands `_` arm).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoodooMaterial {
    Air,
    Bedrock,
    Shaft,
    Cap,
}

#[derive(Debug, Clone, Copy)]
struct HoodooSpire {
    center_u: f32,
    shaft_half_u: f32,
    height: f32,
    cap_scale: f32,
}

fn hoodoo_spires(terrain: &Terrain) -> Vec<HoodooSpire> {
    let (left, center, right) = hoodoo_clusters(terrain);
    left.into_iter()
        .chain(center)
        .chain(right)
        .collect()
}

fn hoodoo_clusters(
    terrain: &Terrain,
) -> (
    Vec<HoodooSpire>,
    Vec<HoodooSpire>,
    Vec<HoodooSpire>,
) {
    let seed = terrain.noise_seed;
    let left_count = 2 + (cell_hash(0, 0, seed ^ 0x484F_4F4C) * 2.0).floor() as usize;
    let right_count = 2 + (cell_hash(1, 0, seed ^ 0x484F_4F52) * 2.0).floor() as usize;
    let center_count = (cell_hash(2, 0, seed ^ 0x484F_4F43) * 3.0).floor() as usize;
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut center = Vec::new();
    for index in 0..left_count {
        left.push(HoodooSpire {
            center_u: 0.04 + index as f32 * 0.06
                + (cell_hash(index as i64, 0, seed ^ 0x4C45_4654) - 0.5) * 0.03,
            shaft_half_u: 0.018 + cell_hash(index as i64, 1, seed ^ 0x5348_4146) * 0.014,
            height: 0.14 + cell_hash(index as i64, 2, seed ^ 0x4845_4947) * 0.10,
            cap_scale: 1.25 + cell_hash(index as i64, 3, seed ^ 0x4341_5053) * 0.35,
        });
    }
    for index in 0..right_count {
        right.push(HoodooSpire {
            center_u: 0.82 + index as f32 * 0.06
                + (cell_hash(index as i64, 0, seed ^ 0x5249_4748) - 0.5) * 0.03,
            shaft_half_u: 0.018 + cell_hash(index as i64, 1, seed ^ 0x5348_4146) * 0.014,
            height: 0.14 + cell_hash(index as i64, 2, seed ^ 0x4845_4947) * 0.10,
            cap_scale: 1.25 + cell_hash(index as i64, 3, seed ^ 0x4341_5053) * 0.35,
        });
    }
    for index in 0..center_count {
        center.push(HoodooSpire {
            center_u: 0.44 + index as f32 * 0.08
                + (cell_hash(index as i64, 0, seed ^ 0x4345_4E54) - 0.5) * 0.04,
            shaft_half_u: 0.014 + cell_hash(index as i64, 1, seed ^ 0x434E_5452) * 0.010,
            height: 0.10 + cell_hash(index as i64, 2, seed ^ 0x434E_5448) * 0.08,
            cap_scale: 1.25 + cell_hash(index as i64, 3, seed ^ 0x434E_5443) * 0.30,
        });
    }
    (left, center, right)
}

fn hoodoo_bedrock_y(u: f32, terrain: &Terrain) -> f32 {
    let floor = 0.05
        + warped_fbm(u * 3.0, 0.2, terrain.noise_seed ^ 0x4245_4452, 3) * 0.03;
    (terrain.horizon - floor * terrain.coverage * 0.95).clamp(0.20, 0.80)
}

fn hoodoo_material_at(u: f32, v: f32, terrain: &Terrain) -> HoodooMaterial {
    let bed = hoodoo_bedrock_y(u, terrain);
    for spire in hoodoo_spires(terrain) {
        let dx_u = (u - spire.center_u).abs();
        let shaft_half_u = spire.shaft_half_u;
        let cap_half_u = shaft_half_u * spire.cap_scale;
        let crest = bed - spire.height * terrain.coverage;
        let cap_bottom = crest + 0.012;
        if (crest..=cap_bottom).contains(&v) && dx_u <= cap_half_u {
            return HoodooMaterial::Cap;
        }
        if v > cap_bottom && v < bed {
            let down = ((v - cap_bottom) / (bed - cap_bottom).max(0.001)).clamp(0.0, 1.0);
            let tapered_half_u = shaft_half_u * (0.72 + down * 0.28);
            if dx_u <= tapered_half_u {
                return HoodooMaterial::Shaft;
            }
        }
    }
    if v >= bed {
        HoodooMaterial::Bedrock
    } else {
        HoodooMaterial::Air
    }
}
```

`structure_hoodoos` shades by `hoodoo_material_at` (cap rim lift, shaft face, bedrock strata):

```rust
fn structure_hoodoos(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let fy = ctx.fy;
    let scale = ctx.scale;
    let light = ctx.light;
    let shade = ctx.shade;
    let horizon_color = ctx.horizon_color;
    let seed = ctx.seed;
    let u = fx / terrain.width.max(1.0);
    let material = hoodoo_material_at(u, v, terrain);
    if matches!(material, HoodooMaterial::Air) {
        return base;
    }
    let ground_mask = match material {
        HoodooMaterial::Bedrock => smoothstep_range(v, hoodoo_bedrock_y(u, terrain), terrain.horizon + 0.028),
        HoodooMaterial::Shaft | HoodooMaterial::Cap => 1.0,
        HoodooMaterial::Air => return base,
    };
    let tau = std::f32::consts::TAU;
    let band = ((fy / (scale * 0.20)) * tau
        + value_noise(fx / (scale * 0.30), fy / (scale * 0.30), seed ^ 0x424E_4453) * 0.8)
        .sin();
    let strata_amt = smoothstep_range(band, 0.10, 0.88) * 0.14 * ground_mask;
    let shaft_face = if terrain.light.0 >= 0.5 { 1.0 } else { 0.65 };
    let body = mix3(base, shade, ground_mask * terrain.coverage * 0.30);
    let faced = mix3(
        body,
        shade,
        if matches!(material, HoodooMaterial::Shaft) {
            shaft_face * ground_mask * 0.22
        } else {
            0.0
        },
    );
    let rim = if matches!(material, HoodooMaterial::Cap) {
        0.35 * ground_mask
    } else {
        0.0
    };
    let capped = mix3(faced, light, rim);
    mix3(capped, horizon_color, strata_amt)
}
```

Wire:

```rust
TerrainKind::Hoodoos => hoodoo_bedrock_y(fx / terrain.width.max(1.0), terrain),
TerrainKind::Hoodoos => structure_hoodoos(base, ctx),
```

Update `terrain_names_lists_every_kind` in `src/polish.rs` to expect all six
structure names:

```rust
#[test]
fn terrain_names_lists_every_kind() {
    assert_eq!(
        terrain_names(),
        vec!["dunes", "mesa", "badlands", "glacier", "alpine", "hoodoos"]
    );
}
```

- [x] **Step 4: Verify GREEN**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::hoodoo_ -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test kinds_render_differently_from_each_other -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - all `hoodoo_*` tests green. hoodoos ≠ dunes at seed 5.

- [x] **Step 5: Commit once**

```bash
git add src/terrain.rs src/polish.rs
git commit -m "feat(terrain): add hoodoo spire geometry and material shading"
```

Completed during the discarded spike history. The worker's first pre-test filter
(`20260726-214048-work-verify-464829`) matched zero tests, so it was not valid
RED evidence. A reversible parser mutation then proved the committed focused
tests fail in `20260726-214324-work-verify-e7cc6a`. Restored focused GREEN:
`20260726-214336-work-verify-a54305`. Restored full verification:
`20260726-214337-work-verify-f3242a`.

---

## Task 3: Integration, MCP descriptions, visual export (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/mcp.rs`

- [x] **Step 1: Update module doc** (`src/terrain.rs` line 1):

```rust
//! Procedural terrain backdrops: dunes, mesa, badlands, glacier, alpine, hoodoos.
```

- [x] **Step 2: Update MCP terrain descriptions** (`src/mcp.rs` lines 128 and 148):

```rust
"description": "Pin the terrain kind (dunes, mesa, badlands, glacier, alpine, hoodoos). Only applies to terrain palettes."
```

```rust
"description": "Pin the terrain kind (dunes, mesa, badlands, glacier, alpine, hoodoos); random when omitted. Only applies to terrain palettes."
```

- [x] **Step 3: Extend MCP enum test**

In `capture_tool_advertises_the_terrain_enum`:

```rust
let tools = tool_definitions();
let capture = tools
    .as_array()
    .expect("tools")
    .iter()
    .find(|tool| tool["name"] == "capture")
    .expect("capture tool");
let properties = &capture["inputSchema"]["properties"];
let names = properties["terrain"]["enum"].as_array().expect("terrain enum");
assert_eq!(names.len(), crate::polish::terrain_names().len());
assert!(names.contains(&json!("alpine")), "alpine missing from MCP terrain enum");
assert!(names.contains(&json!("hoodoos")), "hoodoos missing from MCP terrain enum");
let capture_desc = properties["terrain"]["description"]
    .as_str()
    .expect("capture terrain description");
assert!(capture_desc.contains("alpine"), "capture description missing alpine");
assert!(capture_desc.contains("hoodoos"), "capture description missing hoodoos");

let polish = tools
    .as_array()
    .expect("tools")
    .iter()
    .find(|tool| tool["name"] == "polish")
    .expect("polish tool");
let polish_props = &polish["inputSchema"]["properties"];
let polish_desc = polish_props["terrain"]["description"]
    .as_str()
    .expect("polish terrain description");
assert!(polish_desc.contains("alpine"), "polish description missing alpine");
assert!(polish_desc.contains("hoodoos"), "polish description missing hoodoos");
```

- [x] **Step 4: Add `write_visual_sheet` export test** (see Visual sheet protocol)

- [x] **Step 5: Verify GREEN**

```bash
brigade work verify run --target . \
  --command "cargo test terrain_names_lists -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test capture_tool_advertises_the_terrain_enum -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test every_kind -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test fresh_profiles -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - six-kind integration. mesa/badlands profile tests unchanged. MCP enum and both description strings mention `alpine` and `hoodoos`.

- [x] **Step 6: Commit once**

```bash
git add src/terrain.rs src/mcp.rs
git commit -m "test(terrain): wire six-kind integration, MCP docs, and visual export"
```

Completed during the discarded spike history. MCP description RED:
`20260726-214438-work-verify-8abf55`. Fresh MCP and no-env export GREEN:
`20260726-214612-work-verify-6ad926` and
`20260726-214613-work-verify-e36ca4`. Fresh full verification:
`20260726-214613-work-verify-2cdd3d`.

---

## Task 4: Visual QA - alpine (max 3 iterations, commit only on pass): **CUT**

**Outcome:** Alpine cut after iteration 3 failed visual QA. Commit:
`revert(terrain): cut alpine after failed visual QA`.

**Export receipts:**
- Iteration 1: `20260726-214654-work-verify-3489fc`
- Iteration 2: `20260726-215339-work-verify-79cf17`
- Iteration 3: `20260726-221354-work-verify-d2ffbe`

**Files:** `src/terrain.rs` (adjustments only while failing), `implementation-notes.md` (after keep/cut)

- [x] **Iteration 1 (uncommitted):**

```bash
CLOCHE_VISUAL_SCENE=alpine CLOCHE_VISUAL_ITER=1 \
  brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

Inspect `/tmp/cloche-terrain-scenes/alpine/iter1/`. Apply checklist. **Failed:** flat bands plus snow speckles.

- [x] **If iteration 1 fails:** Adjust `alpine_peak_anchors`, ridge amplitudes, layer mix weights, or `alpine_snow_mask` thresholds in the working tree. **Do not commit.** Re-run with `CLOCHE_VISUAL_ITER=2`, then `3` if needed.

- [x] **If iteration 3 still fails - cut alpine:**

Apply explicit patches removing `TerrainKind::Alpine`, all `alpine_*` functions/tests, and related match arms. Restore `NAMES` to five entries (or four if hoodoos also cut later). Commit:

```bash
git add src/terrain.rs
git commit -m "revert(terrain): cut alpine after failed visual QA"
```

Append to `implementation-notes.md`:

```markdown
## Alpine terrain cut (2026-07-26)
- Visual QA failed after 3 iterations (seeds 1/7/42/99, palettes dunes/mesa/badlands/glacier, 440×300).
- Failure mode: outer bands read as flat sky or ridges collapse to one band.
- Decision: variant removed; shipping change retains dunes, mesa, badlands, glacier, and hoodoos.
```

- [ ] **If alpine passes on iteration 1:** Keep the Task 1 implementation commit. Do not create another commit when the working tree has no renderer changes.

- [ ] **If alpine passes on iteration 2 or 3:** Commit the accepted working-tree adjustments once:

```bash
git add src/terrain.rs
git commit -m "fix(terrain): alpine visual QA iteration N"
```

Append to `implementation-notes.md`:

```markdown
## Alpine terrain kept (2026-07-26)
- Passed visual sheet on iteration N (seeds 1/7/42/99, palettes dunes/mesa/badlands/glacier, 440×300).
- Ridge separation: near/mid profiles differ by ≥0.025 at ≥25% of samples; far layer visible in contact sheet outer columns.
- Snow density: `alpine_snow_mask` coverage held in 1%–12% of near-ridge material pixels via crest band 0.0–0.06 below near crest and max depth 0.14.
- Export command: `CLOCHE_VISUAL_SCENE=alpine CLOCHE_VISUAL_ITER=N brigade work verify run --target . --command "cargo test export_visual_sheet_when_env_set -- --nocapture" --capture brigade-work`.
```

---

## Task 5: Visual QA - hoodoos (max 3 iterations, commit only on pass): **CUT**

**Outcome:** Hoodoos cut after iteration 3 failed visual QA. Commit:
`revert(terrain): cut hoodoos after failed visual QA`.

**Export receipts:**
- Iteration 1: `20260726-221945-work-verify-b63dec`
- Iteration 2: `20260726-222329-work-verify-015ad1`
- Iteration 3: `20260726-222618-work-verify-c96372`

**Failure modes:**
- Iteration 1: tiny T-shaped posts plus striped waterline bedrock.
- Iteration 2: squat mesa-like forms with pale frosting caps.
- Iteration 3: tall repeated pillars/fence reading with caps too small around cards.

**Files:** `src/terrain.rs`, `src/polish.rs`, `src/mcp.rs`, `implementation-notes.md`

- [x] **Iteration 1 (uncommitted):** failed (see receipts above).
- [x] **Iterations 2–3 (uncommitted):** failed (see receipts above).
- [x] **Cut hoodoos:** variant, helpers, tests, visual export, and match arms removed. `NAMES` restored to four baseline kinds.

---

## Task 6: Documentation: **complete (notes-only)**

**Files:** `implementation-notes.md` (no README terrain-kind change)

Both alpine and hoodoos were cut after failed visual QA. Cut reasons and Brigade
export receipts are recorded in `implementation-notes.md`. README terrain-kind
documentation was not updated because no new kinds ship.

- [x] **Step 1:** Cut records appended to `implementation-notes.md` (alpine and hoodoos).
- [x] **Step 2:** No README change (both scenes cut).
- [x] **Step 3:** Notes-only. No separate docs commit beyond the hoodoos cut revert.

---

## Task 7: Final verification and handoff

- [x] **Step 1: Full verify**

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

Expected: **PASS** - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all green. No ignored tests, no `#[allow]`, no new dependencies.

Final receipt: `20260726-223150-work-verify-8ee2f7`.

- [x] **Step 2: Brigade operator checkup**

```bash
brigade operator checkup --target .
```

The code loop is green. Operator readiness remains blocked by pre-existing local
Brigade setup: dogfood config or Codex binary missing, security not initialized,
release-readiness receipt missing, and `.brigade/tools.toml` missing.

- [x] **Step 3: Memory handoff**

Record: piecewise ridge anchor counts, `alpine_snow_mask` threshold values that held 1%–12%, hoodoo `cap_scale` ranges, visual failures that drove iteration, keep/cut outcome, and the exact `CLOCHE_VISUAL_SCENE` / `CLOCHE_VISUAL_ITER` Brigade export commands.

---

## Delivery checklist

- [x] `TerrainKind::NAMES` contains every retained scene name. `from_name` round-trips (four baseline kinds after both cuts)
- [x] `terrain_from_name`, `resolve_style`, `style_from_query`, `capture`, `run_polish`, `backdrop_png`, `card_png`, `render`, and `apply_structure` work without consumer logic changes. only MCP description text changes outside the renderer and its tests
- [x] MCP capture/polish tool descriptions list all retained kinds. `capture_tool_advertises_the_terrain_enum` asserts all four names and rejects cut scene names
- [x] Alpine cut: variant, helpers, tests, and match arms removed. Cut reasons are in `implementation-notes.md`
- [x] Hoodoos cut: variant, helpers, tests, visual export, and match arms removed. Cut reasons are in `implementation-notes.md`
- [x] Shared tests green for all retained kinds. dunes/mesa/badlands/glacier profile tests unchanged
- [x] No accepted visual contact sheets (both scenes cut after ≤3 iterations each)
- [x] `./scripts/verify` green via Brigade (`20260726-223150-work-verify-8ee2f7`)
- [x] Memory handoff written (maintainer-local)

---

## Task commit summary

| Task | Commit message | When |
|---|---|---|
| 1 | `feat(terrain): add alpine ridge layers and snow mask` | After all `alpine_*` tests GREEN. `NAMES` length 5. Alpine fully rendered |
| 2 | `feat(terrain): add hoodoo spire geometry and material shading` | After all `hoodoo_*` tests GREEN. `NAMES` length 6. Hoodoos fully rendered |
| 3 | `test(terrain): wire six-kind integration, MCP docs, and visual export` | After integration + MCP tests GREEN |
| 4 | `revert(terrain): cut alpine after failed visual QA` | Alpine cut after iteration 3 |
| 5 | `revert(terrain): cut hoodoos after failed visual QA` | Hoodoos cut after iteration 3 |
| 6 | (notes-only, no README change) | Both scenes cut. Cut records are in `implementation-notes.md` |
| 7 | (verify only. no commit unless docs tweak) | Final `./scripts/verify` green |

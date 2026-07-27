# Slot Canyon and Cave Mouth Terrain Scenes Implementation Plan

> **HISTORICAL RECORD: DO NOT RE-RUN.** This plan documents a completed spike that executed 2026-07-26. Both scenes were cut after three failed 4% visual reviews. Do not re-execute Tasks 0–8, re-export visual sheets, or re-open the keep branches below.

## Final outcome (2026-07-26)

Both `slot-canyon` and `cave-mouth` were **CUT**. Retained kinds: `dunes`, `mesa`, `badlands`, `glacier` (`TerrainKind::NAMES` length 4).

| Scene | Iter 1 evidence | Iter 1 failure | Iter 2 evidence | Iter 2 failure | Iter 3 evidence | Iter 3 failure (final) | Decision |
|---|---|---|---|---|---|---|---|
| `slot-canyon` | `20260726-232114-bad3b350` (Brigade run) | Soft edge hills. 4% mask is a smooth gradient with no legible wall face or cross-bedding | `20260726-232411-54a7f5ed` (Brigade run) | Rounded hills. 4% mask lacks visible strata | `20260726-234207-work-verify-ddb2bc` (work-verify) | Mesa-like slabs and full-width striped ramps. Masked card lost the canyon silhouette | **CUT** |
| `cave-mouth` | `20260726-233103-work-verify-d460f7` (work-verify) | Thin evenly spaced vertical strokes like a teeth fence. 4% mask lacks a legible enclosing ceiling/side frame | `20260726-233210-87e24148` (Brigade run) | Separate rectangular hanging blocks. 4% mask is a smooth dark frame without rock/rib texture | `20260726-234054-work-verify-c08c54` (work-verify) | Full sheet still read as a row of stalactite teeth. 4% masked card reduced to a rectangular textured border with none of the 3–4 forms visible | **CUT** |

- Temporary visual exporter removed before the branch was squashed.
- Final code verification: `20260727-000219-work-verify-b548c4` (`./scripts/verify` green on four-kind baseline).
- No README terrain section (both scenes cut). Details in `implementation-notes.md`.

---

> **For agentic workers (historical):** Implement task-by-task in order. Each behavior task adds failing tests, runs RED, implements the minimum production code to go GREEN, then commits **once**. Never commit RED tests, compiling stubs, `todo!()`, `panic!` placeholders, or a partially rendered public variant. The first visual sheet evaluates the test-green integration commit. After a visual failure, keep every adjustment uncommitted until the scene passes or is cut. Never commit a known-bad visual state.

**Goal (historical):** Ship `slot-canyon` and `cave-mouth` as two new `TerrainKind` variants in `src/terrain.rs`, each legible in the ~4% outer padding band of a finished card across all four existing terrain palettes (`dunes`, `mesa`, `badlands`, `glacier`).

**Architecture:** Terrain palettes own color. `TerrainKind` owns structure. `PresentationStyle` supplies palette, seed, and optional pinned terrain. `Terrain::generate` rolls scene parameters once. `base_layer` paints sky/ground ramps. Only `SlotCanyon` overrides the horizon profile via `slot_canyon_horizon`. `CaveMouth` keeps the shared `ramp_horizon` branch. `cave_mouth_enclosure_mask` supplies cave identity. `apply_structure` dispatches to `structure_slot_canyon` or `structure_cave_mouth`. CLI, config, and Studio derive terrain names from `TerrainKind::NAMES`. MCP tool schemas use `polish::terrain_names()` for the enum but hard-code human descriptions that must list every retained kind.

**Task count:** Nine tasks (Tasks 0 through 8).

**Tech stack:** Rust 2024, `image`, `rand` (existing deps only). Verification through Brigade.

**Spec:** `docs/specs/2026-07-26-slot-canyon-cave-mouth-terrain-scenes.md`

**GraphTrail impact set (must stay green):**

| Symbol | Callees / consumers |
|---|---|
| `TerrainKind::from_name` | `Terrain::generate`, `tests::kind_name`, `tests::terrain_style`, `polish::terrain_from_name`, `resolve_style`, `style_from_query`, `tests::terrain_pool_pins_only_terrain_kinds`, `capture`, `run_polish`, `backdrop_png`, `card_png` |
| `terrain::render` | `tests::same_seed_renders_identically`, `tests::different_seeds_render_differently`, `tests::every_kind_is_opaque_everywhere`, `tests::every_kind_varies_across_the_canvas`, `tests::kinds_render_differently_from_each_other`, `tests::every_kind_carries_texture_at_the_edges_and_corners` |
| `terrain::apply_structure` | `base_layer`, dispatches `structure_dunes`, `structure_mesa`, `structure_badlands`, `structure_glacier`, plus new scene painters |

**`NAMES` length progression:** 4 (baseline) → 5 after Slot Canyon commit → 6 after Cave Mouth commit. Integration, CLI, MCP, README, and the temporary visual exporter land only after both geometry tasks are GREEN.

---

## File map

| File | Action | What changes |
|---|---|---|
| `src/terrain.rs` | Modify | Per-scene enum extension, `NAMES`, `from_name`, `Terrain::generate`, `base_layer` (`SlotCanyon` horizon only), `apply_structure`, slot-canyon/cave-mouth profile helpers, structure painters, profile tests, temporary `#[cfg(test)]` visual exporter (removed before final delivery) |
| `src/polish.rs` | Modify | `terrain_names_lists_every_kind` expects current `NAMES` length after each geometry task. palette catalog test stays four palettes only |
| `src/cli.rs` | Modify | Hard-coded `--terrain` help examples list every **retained** kind (integration task only) |
| `src/mcp.rs` | Modify | Terrain `description` strings on capture and polish tools. extend `capture_tool_advertises_the_terrain_enum` for retained names (integration task only) |
| `README.md` | Modify | Add terrain backdrop section listing retained kinds only if at least one scene passes visual QA |
| `implementation-notes.md` | Modify | Visual iteration counts, keep/cut decisions, Brigade receipt ids |
| `.claude/memory-handoffs/` | Create | Durable profile + visual-QA lessons (maintainer-local, skip if directory absent) |

**Files that must NOT change:** `src/config.rs`, `src/studio.rs`, `Cargo.toml`, palette table in `polish.rs`. No new dependencies, CLI flags, or MCP surfaces.

**Visual artifacts:** `/tmp/cloche-terrain-scenes/` (never committed).

---

## Brigade verification contract

Run every check through Brigade from the repo root:

```bash
brigade work brief --target .
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

Brigade runs `--command` with `shell=False`. Pass **one** executable and its arguments. For `cargo test`, use **one** substring filter per invocation (no space-separated test names, no `&&`, no pipes).

| When | Command | RED reason | GREEN reason |
|---|---|---|---|
| Task 0 | `./scripts/verify` | fmt, clippy, or any test fails | Full CI entrypoint green on baseline branch |
| Task 1 | `cargo test terrain::tests::slot_canyon_ -- --nocapture` | `SlotCanyon` missing, `NAMES.len() != 5`, or slot-canyon profile assertions fail | All `slot_canyon_*` tests pass. `NAMES` has five entries. slot canyon fully rendered |
| Task 2 | `cargo test terrain::tests::cave_mouth_ -- --nocapture` | `CaveMouth` missing, `NAMES.len() != 6`, or cave-mouth profile assertions fail | All `cave_mouth_*` tests pass. `NAMES` has six entries. cave mouth fully rendered |
| Task 3 | `cargo test terrain_names_lists -- --nocapture` | Expected vec length or names wrong | `terrain_names()` returns six kinds in menu order |
| Task 3 MCP RED | `cargo test capture_tool_advertises_the_terrain_enum -- --nocapture` | Description strings still list four baseline kinds | Test fails before MCP description update |
| Task 3 MCP GREEN | `cargo test capture_tool_advertises_the_terrain_enum -- --nocapture` | Enum length, missing names, or description strings omit retained kinds | Enum length 6. both new names present when retained. descriptions mention them |
| Task 3 shared | `cargo test every_kind -- --nocapture` | Any `every_kind_*` test fails for a new variant | All six kinds pass edge/corner/opacity/variation tests |
| Task 3 distinction | `cargo test kinds_render_differently_from_each_other -- --nocapture` | New kind matches dunes at seed 5 | Each kind differs from dunes |
| Visual export | `cargo test export_visual_sheet_when_env_set -- --nocapture` | Compile error in exporter | PNGs written under `/tmp/cloche-terrain-scenes/{scene}/iter1/` (or `iter2`, `iter3`) |
| Final delivery | `./scripts/verify` | fmt, clippy, or any test fails | Full CI entrypoint green. temporary exporter removed |

---

## Visual sheet protocol

**Review size:** 440×300. **Seeds:** 1, 7, 42, 99. **Palettes:** `dunes`, `mesa`, `badlands`, `glacier`. **Scenes:** one per run via `CLOCHE_VISUAL_SCENE` (`slot-canyon` or `cave-mouth`). **Iteration:** `CLOCHE_VISUAL_ITER` (`1`, `2`, or `3`).

Each scene gets **at most three iterations**. One iteration = one code adjustment + one export run producing:

- **16 individual backdrop PNGs** (`{palette}-seed{seed}-backdrop.png`)
- **48 masked backdrop PNGs** (16 per mask width at 4%, 6%, and 10%: `{palette}-seed{seed}-backdrop-mask4pct.png`, `mask6pct`, `mask10pct`)
- **32 individual finished-card PNGs** (`{palette}-seed{seed}-card-light.png`, `{palette}-seed{seed}-card-dark.png`)
- **96 masked card PNGs** (32 per mask width: `{palette}-seed{seed}-card-light-mask4pct.png`, `{palette}-seed{seed}-card-dark-mask4pct.png`, etc.)
- **12 contact sheets:** 1 backdrop grid, 3 masked backdrop grids, 1 light-card grid, 3 masked light-card grids, 1 dark-card grid, 3 masked dark-card grids (`contact-backdrops.png`, `contact-backdrops-mask4pct.png`, and siblings)

**204 PNG files per iteration** (192 individual tiles plus 12 contact sheets).

Individual card PNGs keep `compose_card` output dimensions. Contact-sheet tiles are resized to 440×300 before stitching because style padding changes card size.

Output root: `/tmp/cloche-terrain-scenes/{scene}/iter{iteration}/`.

### Mask rule (matches spec table)

| Visible perimeter width | Keep pixels |
|---|---|
| 4% each side | outer `round(width * 0.04)` left/right and top/bottom |
| 6% each side | outer `round(width * 0.06)` |
| 10% each side | outer `round(width * 0.10)` |

Center pixels become opaque black `[0, 0, 0, 255]` so contact sheets show only the perimeter band.

### Inspection checklist (every sheet)

- [ ] Recognizable from visible side, top, and bottom strips at **4%** mask without a centered subject
- [ ] No flat outer band (edge pixels vary)
- [ ] No fence, barcode, smoke, icicle teeth, or generic noise reading
- [ ] Enough contrast behind both light and dark screenshot content
- [ ] Clearly distinct from dunes, mesa, badlands, glacier, and the other new scene at the same seed/palette
- [ ] Slot canyon: continuous sandstone wall mass in side bands with curved cross-bedding, not a centered river void
- [ ] Cave mouth: enclosing rock mass on top, sides, and lower corners with 3-4 large drapery/stalactite forms, not evenly spaced teeth

### Keep-or-cut rules (independent per scene)

| Outcome | Action |
|---|---|
| Sheet passes checklist on iteration 1 | **Keep** the existing test-green commit. Record iteration count in `implementation-notes.md`. Do not create an empty visual-fix commit |
| Sheet passes checklist on iteration 2 or 3 | **Keep** scene. Commit the accepted visual adjustments once. Record iteration count in `implementation-notes.md` |
| Sheet fails after iteration 3 | **Cut** scene: remove variant, helpers, tests, match arms, CLI/MCP/README mentions. Restore `NAMES` length. Document cut reason with Brigade receipt ids. Commit `revert(terrain): cut slot-canyon after failed visual QA` or `revert(terrain): cut cave-mouth after failed visual QA` |
| Both scenes pass | Ship both. README lists six terrain kinds |
| One scene passes, one cut | Ship the survivor only. README lists five terrain kinds |
| Both cut | Revert the spike. `NAMES` returns to four baseline kinds. No README terrain section. **Actual outcome (2026-07-26).** |

**Iteration discipline:** Forward-only tuning. Never revert to an earlier renderer state. Keep failed adjustments uncommitted until pass or cut.

---

## Task 0: Green baseline

**Files:** none (verify only)

- [ ] Read Brigade brief:

```bash
brigade work brief --target .
```

- [ ] Run full verify and capture receipt:

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

Expected: **PASS** - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all green on the four-kind baseline.

- [ ] Record the receipt id in your session notes. Do not commit.

---

## Task 1: Slot Canyon geometry (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/polish.rs`

Slot canyon uses **positive full-height wall masses** anchored partly off-frame (mesa lane pattern). `slot_canyon_rock_mask` and `slot_canyon_bedding_signal` are pure query functions tests sample directly. `structure_slot_canyon` paints wall faces, cross-bedding tints, and slope lighting. Do **not** add `CaveMouth`, CLI help, MCP strings, README, or the visual exporter in this task.

- [ ] **Step 1: Add failing slot-canyon tests** (prefix `slot_canyon_` for the single-filter verify command)

Add to `src/terrain.rs` `#[cfg(test)] mod tests`:

```rust
fn slot_canyon_test_terrain(seed: u64) -> Terrain {
    let mut rng = StdRng::seed_from_u64(seed);
    Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::SlotCanyon))
}

fn dominant_band_indices(terrain: &Terrain, dominant_left: bool) -> (usize, usize) {
    let w = terrain.width as usize;
    if dominant_left {
        (0, (w as f32 * 0.125).ceil() as usize)
    } else {
        ((w as f32 * 0.875).floor() as usize, w)
    }
}

fn sample_vertical_rock_mask(terrain: &Terrain, u: f32, samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|row| {
            let v = row as f32 / samples as f32;
            slot_canyon_rock_mask(u, v, terrain)
        })
        .collect()
}

#[test]
fn slot_canyon_name_parses_and_names_len_five() {
    assert_eq!(
        TerrainKind::from_name("slot-canyon"),
        Some(TerrainKind::SlotCanyon)
    );
    assert!(TerrainKind::NAMES.contains(&"slot-canyon"));
    assert_eq!(TerrainKind::NAMES.len(), 5);
}

#[test]
fn slot_canyon_outer_band_wall_mass_reaches_040_height() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let (_, _, dominant_left) = slot_canyon_walls(&terrain);
        let (start, end) = dominant_band_indices(&terrain, dominant_left);
        let mut peak_span = 0_usize;
        for x in start..end {
            let u = x as f32 / terrain.width;
            let column = sample_vertical_rock_mask(&terrain, u, 64);
            let active = column.iter().filter(|&&m| m > 0.20).count();
            peak_span = peak_span.max(active);
        }
        let min_active = (64.0_f32 * 0.40).ceil() as usize;
        assert!(
            peak_span >= min_active,
            "seed {seed}: dominant wall vertical span {peak_span} < {min_active}"
        );
    }
}

#[test]
fn slot_canyon_walls_are_asymmetric() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let (dom, sec, _) = slot_canyon_walls(&terrain);
        let delta = (dom.relief - sec.relief).abs();
        assert!(
            delta >= 0.03,
            "seed {seed}: dominant/secondary relief delta {delta} < 0.03"
        );
    }
}

#[test]
fn slot_canyon_dominant_wall_covers_sky_to_ground() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let (dom, _, dominant_left) = slot_canyon_walls(&terrain);
        let u = dom.center_u.clamp(0.02, 0.98);
        let column = sample_vertical_rock_mask(&terrain, u, 50);
        let covered = column.iter().filter(|&&m| m > 0.25).count();
        assert!(
            covered as f32 / column.len() as f32 >= 0.60,
            "seed {seed}: only {covered}/{} samples > 0.25 in dominant band",
            column.len()
        );
        let _ = dominant_left;
    }
}

#[test]
fn slot_canyon_cross_bedding_varies_on_dominant_face() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let (dom, _, _) = slot_canyon_walls(&terrain);
        let u = dom.center_u.clamp(0.04, 0.96);
        let samples: Vec<f32> = (0..16)
            .map(|i| {
                let v = 0.20 + i as f32 * 0.45 / 15.0;
                slot_canyon_bedding_signal(u, v, &terrain)
            })
            .collect();
        let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
        let max = samples.iter().copied().fold(0.0_f32, f32::max);
        assert!(
            max - min >= 0.02,
            "seed {seed}: bedding spread {} < 0.02",
            max - min
        );
    }
}

#[test]
fn slot_canyon_avoids_single_column_cliffs_on_wall_top() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let (_, _, dominant_left) = slot_canyon_walls(&terrain);
        let (start, end) = dominant_band_indices(&terrain, dominant_left);
        let crests: Vec<f32> = (start..end)
            .map(|x| slot_canyon_wall_top(x as f32, &terrain))
            .collect();
        let relief = crests.iter().copied().fold(f32::INFINITY, f32::min);
        let local_relief = terrain.horizon - relief;
        let max_drop = max_adjacent_drop(&crests);
        assert!(
            max_drop < local_relief * 0.80,
            "seed {seed}: cliff drop {max_drop} for relief {local_relief}"
        );
    }
}

#[test]
fn slot_canyon_center_stays_open() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = slot_canyon_test_terrain(seed);
        let start = (terrain.width * 0.25) as usize;
        let end = (terrain.width * 0.75) as usize;
        let mut sum = 0.0_f32;
        let mut count = 0_usize;
        for x in start..end {
            for row in 0..48 {
                let u = x as f32 / terrain.width;
                let v = row as f32 / 48.0;
                sum += slot_canyon_rock_mask(u, v, &terrain);
                count += 1;
            }
        }
        let avg = sum / count as f32;
        assert!(avg < 0.35, "seed {seed}: center band average mask {avg}");
    }
}
```

- [ ] **Step 2: Verify RED**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::slot_canyon_ -- --nocapture" \
  --capture brigade-work
```

Expected: **FAIL** - compile error (`SlotCanyon` variant missing), `NAMES.len() == 4`, or missing `slot_canyon_walls` / `slot_canyon_rock_mask` / `slot_canyon_bedding_signal`.

- [ ] **Step 3: Implement slot-canyon production code**

Extend enum and `NAMES` to five entries (do **not** add `CaveMouth` yet):

```rust
    /// Narrow canyon cross-section with asymmetric full-height sandstone walls.
    SlotCanyon,

    pub const NAMES: [&'static str; 5] =
        ["dunes", "mesa", "badlands", "glacier", "slot-canyon"];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "dunes" => Self::Dunes,
            "mesa" => Self::Mesa,
            "badlands" => Self::Badlands,
            "glacier" => Self::Glacier,
            "slot-canyon" => Self::SlotCanyon,
            _ => return None,
        })
    }
```

In `Terrain::generate`:

```rust
            TerrainKind::SlotCanyon => (rng.random_range(0.46..=0.58), 0.74, 4.5),
```

Side light for Slot Canyon (same `_` arm as Dunes/Mesa/Badlands).

Pinned helpers (place above `structure_slot_canyon`):

```rust
const SLOT_WALL_SALTS: [u64; 2] = [0x534C_5744, 0x534C_5742];

#[derive(Debug, Clone, Copy)]
struct SlotWall {
    center_u: f32,
    half_width_u: f32,
    relief: f32,
    bedding_phase: f32,
}

fn slot_canyon_walls(terrain: &Terrain) -> (SlotWall, SlotWall, bool) {
    let seed = terrain.noise_seed;
    let dominant_left = cell_hash(0, 0, seed ^ 0x534C_5349) < 0.5;
    let dom_relief = 0.24 + cell_hash(1, 0, seed ^ 0x5245_4C46) * 0.10;
    let sec_relief = (dom_relief - 0.035 - cell_hash(2, 0, seed ^ 0x5345_435F) * 0.04)
        .clamp(0.12, dom_relief - 0.03);
    let dom_half = 0.19 + cell_hash(3, 0, seed ^ 0x5749_4454) * 0.07;
    let sec_half = 0.14 + cell_hash(4, 0, seed ^ 0x5749_4453) * 0.06;
    let dom_phase = cell_hash(5, 0, seed ^ 0x4245_4447) * std::f32::consts::TAU;
    let sec_phase = cell_hash(6, 0, seed ^ 0x4245_4442) * std::f32::consts::TAU;
    let (dom_center, sec_center) = if dominant_left {
        (
            -0.05 + cell_hash(7, 0, seed ^ 0x4C45_4654) * 0.04,
            0.90 + cell_hash(8, 0, seed ^ 0x5249_4748) * 0.05,
        )
    } else {
        (
            1.05 + cell_hash(7, 0, seed ^ 0x5249_4748) * 0.04,
            0.06 + cell_hash(8, 0, seed ^ 0x4C45_4654) * 0.05,
        )
    };
    let dominant = SlotWall {
        center_u: dom_center,
        half_width_u: dom_half,
        relief: dom_relief,
        bedding_phase: dom_phase,
    };
    let secondary = SlotWall {
        center_u: sec_center,
        half_width_u: sec_half,
        relief: sec_relief,
        bedding_phase: sec_phase,
    };
    (dominant, secondary, dominant_left)
}

fn slot_canyon_wall_body(u: f32, v: f32, wall: &SlotWall, terrain: &Terrain) -> f32 {
    let dx = (u - wall.center_u).abs();
    if dx >= wall.half_width_u {
        return 0.0;
    }
    let t = 1.0 - dx / wall.half_width_u.max(0.001);
    let face = t * smoothstep(t);
    let top = terrain.horizon - wall.relief * terrain.coverage * face;
    let bottom = (terrain.horizon + 0.08).min(0.96);
    smoothstep_range(v, top, bottom) * face
}

fn slot_canyon_floor_mask(u: f32, v: f32, terrain: &Terrain) -> f32 {
    if v < terrain.horizon {
        return 0.0;
    }
    let ripple = warped_fbm(u * 3.2, 0.18, terrain.noise_seed ^ SLOT_WALL_SALTS[1], 3);
    let floor_top = terrain.horizon + 0.02 + ripple * 0.015;
    let floor_bot = (terrain.horizon + 0.09).min(0.97);
    smoothstep_range(v, floor_top, floor_bot) * 0.35 * terrain.coverage
}

/// Rock occupancy mask in 0..1. Tests sample this directly.
fn slot_canyon_rock_mask(u: f32, v: f32, terrain: &Terrain) -> f32 {
    let (dom, sec, _) = slot_canyon_walls(terrain);
    slot_canyon_wall_body(u, v, &dom, terrain)
        .max(slot_canyon_wall_body(u, v, &sec, terrain))
        .max(slot_canyon_floor_mask(u, v, terrain))
        .clamp(0.0, 1.0)
}

fn slot_canyon_wall_top(fx: f32, terrain: &Terrain) -> f32 {
    let u = fx / terrain.width.max(1.0);
    let (dom, sec, _) = slot_canyon_walls(terrain);
    let dom_top = terrain.horizon
        - dom.relief * terrain.coverage
            * smoothstep_range(
                1.0 - (u - dom.center_u).abs() / dom.half_width_u.max(0.001),
                0.0,
                1.0,
            );
    let sec_top = terrain.horizon
        - sec.relief * terrain.coverage
            * smoothstep_range(
                1.0 - (u - sec.center_u).abs() / sec.half_width_u.max(0.001),
                0.0,
                1.0,
            );
    dom_top.min(sec_top)
}

/// Normalized bedding modulation 0..1 on the wall face. Tests sample this directly.
fn slot_canyon_bedding_signal(u: f32, v: f32, terrain: &Terrain) -> f32 {
    let (dom, _, _) = slot_canyon_walls(terrain);
    let body = slot_canyon_wall_body(u, v, &dom, terrain);
    if body < 0.15 {
        return 0.0;
    }
    let bend = (u * 4.8 + v * 1.6 + dom.bedding_phase).sin() * 0.5 + 0.5;
    let warp = warped_fbm(u * 2.1, v * 1.3, terrain.noise_seed ^ SLOT_WALL_SALTS[0], 3);
    (bend * 0.55 + warp * 0.45).clamp(0.0, 1.0)
}

fn slot_canyon_horizon(fx: f32, terrain: &Terrain) -> f32 {
    slot_canyon_wall_top(fx, terrain)
}

fn structure_slot_canyon(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let light = ctx.light;
    let shade = ctx.shade;
    let horizon_color = ctx.horizon_color;
    let u = fx / terrain.width.max(1.0);
    let mask = slot_canyon_rock_mask(u, v, terrain);
    if mask <= 0.0 {
        return base;
    }
    let sample = terrain.width * 0.020;
    let left = slot_canyon_wall_top(fx - sample, terrain);
    let right = slot_canyon_wall_top(fx + sample, terrain);
    let slope = ((right - left) * 8.0).clamp(-1.0, 1.0);
    let lit_face = if terrain.light.0 >= 0.5 {
        slope.max(0.0)
    } else {
        (-slope).max(0.0)
    };
    let bedding = slot_canyon_bedding_signal(u, v, terrain);
    let body = mix3(base, shade, mask * terrain.coverage * 0.28);
    let faced = mix3(body, shade, lit_face * mask * 0.22);
    let rim = mix3(faced, light, lit_face * mask * 0.18);
    let bedded = mix3(rim, horizon_color, bedding * mask * 0.12);
    mix3(bedded, shade, (1.0 - lit_face) * mask * 0.10)
}
```

Wire `base_layer` and `apply_structure`:

```rust
            TerrainKind::SlotCanyon => slot_canyon_horizon(fx, terrain),
            // ramp_horizon arm unchanged: SlotCanyon uses default local_horizon branch
```

```rust
        TerrainKind::SlotCanyon => structure_slot_canyon(base, ctx),
```

Update `src/polish.rs`:

```rust
    #[test]
    fn terrain_names_lists_every_kind() {
        assert_eq!(
            terrain_names(),
            vec!["dunes", "mesa", "badlands", "glacier", "slot-canyon"]
        );
    }
```

Keep `terrain_palettes_catalog_as_terrain` restricted to the four existing terrain palettes.

- [ ] **Step 4: Verify GREEN**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::slot_canyon_ -- --nocapture" \
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

Expected: **PASS** - all `slot_canyon_*` tests green. five kinds compile. slot canyon fully rendered in shared tests.

- [ ] **Step 5: Commit once**

```bash
git add src/terrain.rs src/polish.rs
git commit -m "feat(terrain): add slot-canyon wall geometry and cross-bedding"
```

---

## Task 2: Cave Mouth geometry (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/polish.rs`

Cave mouth models an **enclosing rock mass** with 3-4 large dripstone features. `cave_mouth_rock_mask` and `cave_mouth_features` are pure query functions. Do **not** add a `CaveMouth` arm to `base_layer` (no `cave_mouth_horizon`). keep the shared `ramp_horizon` branch. Do **not** touch CLI, MCP, README, or the visual exporter in this task.

- [ ] **Step 1: Add failing cave-mouth tests** (prefix `cave_mouth_`)

```rust
fn cave_mouth_test_terrain(seed: u64) -> Terrain {
    let mut rng = StdRng::seed_from_u64(seed);
    Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::CaveMouth))
}

fn sample_horizontal_band_mask(terrain: &Terrain, v0: f32, v1: f32, samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|col| {
            let u = col as f32 / samples as f32;
            let mut peak = 0.0_f32;
            let steps = 8_usize;
            for step in 0..steps {
                let v = v0 + (v1 - v0) * step as f32 / steps as f32;
                peak = peak.max(cave_mouth_rock_mask(u, v, terrain));
            }
            peak
        })
        .collect()
}

#[test]
fn cave_mouth_name_parses_and_names_len_six() {
    assert_eq!(
        TerrainKind::from_name("cave-mouth"),
        Some(TerrainKind::CaveMouth)
    );
    assert!(TerrainKind::NAMES.contains(&"cave-mouth"));
    assert_eq!(TerrainKind::NAMES.len(), 6);
}

#[test]
fn cave_mouth_top_band_enclosure_reaches_030_average() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let band = sample_horizontal_band_mask(&terrain, 0.0, 0.08, 88);
        let avg = band.iter().sum::<f32>() / band.len() as f32;
        assert!(avg >= 0.30, "seed {seed}: top band average {avg}");
    }
}

#[test]
fn cave_mouth_side_band_peak_reaches_035() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let w = terrain.width;
        let mut left_peak = 0.0_f32;
        let mut right_peak = 0.0_f32;
        for x in 0..(w * 0.125).ceil() as usize {
            let u = x as f32 / w;
            for row in 0..64 {
                let v = row as f32 / 64.0;
                left_peak = left_peak.max(cave_mouth_rock_mask(u, v, &terrain));
            }
        }
        for x in (w as f32 * 0.875).floor() as usize..w as usize {
            let u = x as f32 / w;
            for row in 0..64 {
                let v = row as f32 / 64.0;
                right_peak = right_peak.max(cave_mouth_rock_mask(u, v, &terrain));
            }
        }
        let side_peak = left_peak.max(right_peak);
        assert!(
            side_peak >= 0.35,
            "seed {seed}: side peak {side_peak} < 0.35"
        );
    }
}

#[test]
fn cave_mouth_lower_corners_carry_mass() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let w = terrain.width;
        let h = 300.0_f32;
        let corners = [
            (0.06_f32, 0.92_f32),
            (0.94_f32, 0.92_f32),
        ];
        for (u, v) in corners {
            let x = (u * w) as usize;
            let mut peak = 0.0_f32;
            for dx in 0..3 {
                for dy in 0..4 {
                    let sample_u = (x + dx) as f32 / w;
                    let sample_v = v + dy as f32 / h;
                    peak = peak.max(cave_mouth_rock_mask(sample_u, sample_v, &terrain));
                }
            }
            assert!(peak >= 0.30, "seed {seed}: corner ({u},{v}) peak {peak}");
        }
    }
}

#[test]
fn cave_mouth_feature_count_is_three_or_four() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let features = cave_mouth_features(&terrain);
        assert!(
            (3..=4).contains(&features.len()),
            "seed {seed}: feature count {}",
            features.len()
        );
        for feature in &features {
            assert!(
                feature.width_u >= 0.02,
                "seed {seed}: feature width {} < 0.02 canvas",
                feature.width_u
            );
            let peak = cave_mouth_feature_peak_mask(feature, &terrain);
            assert!(peak >= 0.20, "seed {seed}: feature peak {peak}");
        }
    }
}

#[test]
fn cave_mouth_gap_variation_at_least_004_canvas() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let mut centers: Vec<f32> = cave_mouth_features(&terrain)
            .iter()
            .map(|f| f.center_u)
            .collect();
        centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let gaps: Vec<f32> = centers
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs() * terrain.width)
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
fn cave_mouth_avoids_teeth_fence_in_perimeter_bands() {
    for seed in [1_u64, 7, 42, 99] {
        let terrain = cave_mouth_test_terrain(seed);
        let features = cave_mouth_features(&terrain);
        let perimeter: Vec<_> = features
            .iter()
            .filter(|f| f.anchor_v <= 0.22 || f.center_u <= 0.14 || f.center_u >= 0.86)
            .collect();
        let mut bins = [0_u8; 13];
        for feature in perimeter {
            let bin = ((feature.center_u * 12.0).round() as usize).min(12);
            bins[bin] += 1;
        }
        assert!(
            bins.iter().all(|&count| count <= 1),
            "seed {seed}: fence bins {bins:?}"
        );
    }
}
```

- [ ] **Step 2: Verify RED**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::cave_mouth_ -- --nocapture" \
  --capture brigade-work
```

Expected: **FAIL** - compile error (`CaveMouth` variant missing), `NAMES.len() == 5`, or empty feature list.

- [ ] **Step 3: Implement cave-mouth production code**

Extend enum and `NAMES` to six entries:

```rust
    /// Enclosing rock mass with broad drapery and stalactite forms on the perimeter.
    CaveMouth,

    pub const NAMES: [&'static str; 6] = [
        "dunes",
        "mesa",
        "badlands",
        "glacier",
        "slot-canyon",
        "cave-mouth",
    ];
```

Add `from_name` arm and `generate` arm:

```rust
            "cave-mouth" => Self::CaveMouth,
            TerrainKind::CaveMouth => (rng.random_range(0.44..=0.56), 0.70, 5.0),
```

Side light for Cave Mouth (same `_` arm).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaveFeatureKind {
    Drapery,
    Stalactite,
}

#[derive(Debug, Clone, Copy)]
struct CaveFeature {
    center_u: f32,
    anchor_v: f32,
    width_u: f32,
    length_v: f32,
    kind: CaveFeatureKind,
}

fn cave_mouth_feature_count(seed: u64) -> usize {
    3 + (cell_hash(0, 0, seed ^ 0x4341_5645) * 2.0).floor() as usize
}

fn cave_mouth_feature_bases(count: usize) -> &'static [f32] {
    match count {
        3 => &[0.16, 0.42, 0.80],
        _ => &[0.10, 0.30, 0.58, 0.88],
    }
}

fn cave_mouth_features(terrain: &Terrain) -> Vec<CaveFeature> {
    let seed = terrain.noise_seed;
    let count = cave_mouth_feature_count(seed);
    let bases = cave_mouth_feature_bases(count);
    // The 3-feature nominal gap spread is 0.12 and center jitter can shrink it by at most
    // 0.03, leaving 0.09. The 4-feature nominal spread is 0.10 and jitter can shrink it
    // by at most 0.025, leaving 0.075. Both remain above the 0.04 profile threshold.
    let jitter_span = if count == 3 { 0.03 } else { 0.025 };
    let mut features = Vec::with_capacity(count);
    for (index, &base_u) in bases.iter().enumerate() {
        let kind = if cell_hash(index as i64, 0, seed ^ 0x4452_4150) < 0.5 {
            CaveFeatureKind::Drapery
        } else {
            CaveFeatureKind::Stalactite
        };
        let jitter = (cell_hash(index as i64, 1, seed ^ 0x4345_4E54) - 0.5) * jitter_span;
        let center_u = (base_u + jitter).clamp(0.04, 0.96);
        let anchor_v = 0.04 + cell_hash(index as i64, 2, seed ^ 0x414E_4348) * 0.14;
        let width_u = 0.024 + cell_hash(index as i64, 3, seed ^ 0x5749_4454) * 0.028;
        let length_v = 0.10 + cell_hash(index as i64, 4, seed ^ 0x4C45_4E47) * 0.16;
        features.push(CaveFeature {
            center_u,
            anchor_v,
            width_u,
            length_v,
            kind,
        });
    }
    features
}

fn cave_mouth_enclosure_mask(u: f32, v: f32, terrain: &Terrain) -> f32 {
    let seed = terrain.noise_seed;
    // Full-width ceiling: strongest at v=0, seeded width asymmetry via warped_fbm.
    let top_falloff = 1.0 - smoothstep_range(v, 0.0, 0.14);
    let width_ripple = 0.82 + warped_fbm(u * 2.4, 0.08, seed ^ 0x4345_494C, 3) * 0.36;
    let ceiling = top_falloff * width_ripple * 0.62;
    let side_left = smoothstep_range(1.0 - u / 0.14, 0.0, 1.0) * 0.44;
    let side_right = smoothstep_range(1.0 - (1.0 - u) / 0.14, 0.0, 1.0) * 0.44;
    let floor = smoothstep_range(v, terrain.horizon + 0.02, 0.96) * 0.32;
    (ceiling + side_left.max(side_right) + floor)
        .clamp(0.0, 1.0)
        * terrain.coverage
}

fn cave_mouth_feature_mask(u: f32, v: f32, feature: &CaveFeature, terrain: &Terrain) -> f32 {
    let dx = (u - feature.center_u).abs();
    let half = feature.width_u * 0.5;
    if dx > half {
        return 0.0;
    }
    let taper = 1.0 - dx / half.max(0.001);
    let tip = feature.anchor_v + feature.length_v;
    match feature.kind {
        CaveFeatureKind::Drapery => {
            smoothstep_range(v, feature.anchor_v, tip) * taper * terrain.coverage
        }
        CaveFeatureKind::Stalactite => {
            smoothstep_range(v, feature.anchor_v, tip)
                * taper
                * smoothstep_range(tip - v, 0.0, feature.length_v * 0.25)
                * terrain.coverage
        }
    }
}

fn cave_mouth_feature_peak_mask(feature: &CaveFeature, terrain: &Terrain) -> f32 {
    let u = feature.center_u;
    let v = feature.anchor_v + feature.length_v * 0.5;
    cave_mouth_feature_mask(u, v, feature, terrain)
}

/// Rock occupancy mask in 0..1. Tests sample this directly.
fn cave_mouth_rock_mask(u: f32, v: f32, terrain: &Terrain) -> f32 {
    let mut mask = cave_mouth_enclosure_mask(u, v, terrain);
    for feature in cave_mouth_features(terrain) {
        mask = mask.max(cave_mouth_feature_mask(u, v, &feature, terrain));
    }
    let fracture = warped_fbm(u * 2.8, v * 2.2, terrain.noise_seed ^ 0x4352_4F43, 3);
    mask + fracture * 0.04 * mask
}

fn structure_cave_mouth(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let light = ctx.light;
    let shade = ctx.shade;
    let u = fx / terrain.width.max(1.0);
    let mask = cave_mouth_rock_mask(u, v, terrain).clamp(0.0, 1.0);
    if mask <= 0.0 {
        return base;
    }
    let rim = if terrain.light.1 < 0.35 {
        smoothstep_range(v, 0.0, 0.12)
    } else if terrain.light.0 >= 0.5 {
        smoothstep_range(u, 0.86, 0.98)
    } else {
        smoothstep_range(1.0 - u, 0.86, 0.98)
    };
    let tip_highlight = cave_mouth_features(terrain)
        .iter()
        .map(|f| cave_mouth_feature_mask(u, v, f, terrain))
        .fold(0.0_f32, f32::max);
    let body = mix3(base, shade, mask * 0.32);
    let faced = mix3(body, shade, (1.0 - rim) * mask * 0.14);
    let lit = mix3(faced, light, rim * mask * 0.22 + tip_highlight * 0.28);
    mix3(lit, shade, tip_highlight * 0.12)
}
```

Wire `apply_structure` only (do **not** add a `CaveMouth` arm to `base_layer`. keep the shared `ramp_horizon` branch):

```rust
        TerrainKind::CaveMouth => structure_cave_mouth(base, ctx),
```

Update `src/polish.rs`:

```rust
    #[test]
    fn terrain_names_lists_every_kind() {
        assert_eq!(
            terrain_names(),
            vec![
                "dunes",
                "mesa",
                "badlands",
                "glacier",
                "slot-canyon",
                "cave-mouth"
            ]
        );
    }
```

- [ ] **Step 4: Verify GREEN**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::cave_mouth_ -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test kinds_render_differently_from_each_other -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - all `cave_mouth_*` tests green. six kinds compile. cave mouth differs from dunes at seed 5.

- [ ] **Step 5: Commit once**

```bash
git add src/terrain.rs src/polish.rs
git commit -m "feat(terrain): add cave-mouth enclosure and dripstone geometry"
```

---

## Task 3: Integration, CLI, MCP, temporary visual exporter (RED → GREEN → commit once)

**Files:** `src/terrain.rs`, `src/cli.rs`, `src/mcp.rs`

Run this task only after Tasks 1 and 2 are GREEN. **RED first:** extend MCP assertions while capture/polish descriptions still list four kinds. **GREEN second:** update module doc, CLI help, MCP description strings, and the temporary exporter, then re-run the same tests.

- [ ] **Step 1: Extend MCP enum test only** (`src/mcp.rs`)

Replace the body of `capture_tool_advertises_the_terrain_enum` with:

```rust
    fn capture_tool_advertises_the_terrain_enum() {
        let tools = tool_definitions();
        let capture = tools
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "capture")
            .expect("capture tool");
        let properties = &capture["inputSchema"]["properties"];
        let names = properties["terrain"]["enum"]
            .as_array()
            .expect("terrain enum");
        assert_eq!(names.len(), crate::polish::terrain_names().len());
        assert!(
            names.contains(&json!("slot-canyon")),
            "slot-canyon missing from MCP terrain enum"
        );
        assert!(
            names.contains(&json!("cave-mouth")),
            "cave-mouth missing from MCP terrain enum"
        );
        let capture_desc = properties["terrain"]["description"]
            .as_str()
            .expect("capture terrain description");
        assert!(
            capture_desc.contains("slot-canyon"),
            "capture description missing slot-canyon"
        );
        assert!(
            capture_desc.contains("cave-mouth"),
            "capture description missing cave-mouth"
        );

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
        assert!(
            polish_desc.contains("slot-canyon"),
            "polish description missing slot-canyon"
        );
        assert!(
            polish_desc.contains("cave-mouth"),
            "polish description missing cave-mouth"
        );
    }
```

Do **not** change module doc, CLI help, MCP description strings, or the visual exporter yet.

- [ ] **Step 2: Verify RED**

```bash
brigade work verify run --target . \
  --command "cargo test capture_tool_advertises_the_terrain_enum -- --nocapture" \
  --capture brigade-work
```

Expected: **FAIL** - enum length is 6 but capture/polish `description` strings still list only the four baseline kinds (`dunes`, `mesa`, `badlands`, `glacier`). Record the RED receipt id.

- [ ] **Step 3: Update module doc, CLI help, MCP descriptions, and temporary visual exporter**

Module doc (`src/terrain.rs` line 1):

```rust
//! Procedural terrain backdrops: dunes, mesa, badlands, glacier, slot-canyon, cave-mouth.
```

CLI help examples (`src/cli.rs`):

Line 85-86 (`capture`):

```rust
    /// Pin the terrain kind (e.g. `dunes`, `mesa`, `glacier`, `slot-canyon`); random when
```

Line 232-233 (`polish`):

```rust
    /// Pin the terrain kind (e.g. `dunes`, `mesa`, `glacier`, `cave-mouth`). Only applies to
```

MCP terrain descriptions (`src/mcp.rs` lines 128 and 148):

```rust
"description": "Pin the terrain kind (dunes, mesa, badlands, glacier, slot-canyon, cave-mouth). Only applies to terrain palettes."
```

```rust
"description": "Pin the terrain kind (dunes, mesa, badlands, glacier, slot-canyon, cave-mouth); random when omitted. Only applies to terrain palettes."
```

Add temporary visual exporter to `src/terrain.rs` `#[cfg(test)] mod tests` (reuses existing `use crate::polish.` at module scope. no `#[ignore]`, no `#[allow]`, no new dependency):

```rust
    use image::DynamicImage;

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

    fn apply_perimeter_mask(tile: &RgbaImage, keep_fraction: f32) -> RgbaImage {
        let (w, h) = tile.dimensions();
        let mut masked = tile.clone();
        let x_band = (w as f32 * keep_fraction).round() as u32;
        let y_band = (h as f32 * keep_fraction).round() as u32;
        let black = Rgba([0, 0, 0, 255]);
        for y in 0..h {
            for x in 0..w {
                let keep = x < x_band
                    || x >= w - x_band
                    || y < y_band
                    || y >= h - y_band;
                if !keep {
                    masked.put_pixel(x, y, black);
                }
            }
        }
        masked
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
            Some("slot-canyon") | Some("cave-mouth") => {
                std::env::var("CLOCHE_VISUAL_SCENE").unwrap()
            }
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
        let mask_fractions = [0.04_f32, 0.06, 0.10];
        let mut backdrops = Vec::new();
        let mut cards_light = Vec::new();
        let mut cards_dark = Vec::new();
        let mut masked_backdrops = [(); 3].map(|_| Vec::<RgbaImage>::new());
        let mut masked_cards_light = [(); 3].map(|_| Vec::<RgbaImage>::new());
        let mut masked_cards_dark = [(); 3].map(|_| Vec::<RgbaImage>::new());
        for palette in palettes {
            for seed in seeds {
                let mut style = polish::style_with_palette(seed, palette).expect("palette");
                style.terrain = TerrainKind::from_name(&scene).expect("visual scene");
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
                backdrops.push(backdrop.clone());
                cards_light.push(card_light.clone());
                cards_dark.push(card_dark.clone());
                for (index, fraction) in mask_fractions.iter().enumerate() {
                    let mask_label = match *fraction {
                        0.04 => "mask4pct",
                        0.06 => "mask6pct",
                        _ => "mask10pct",
                    };
                    let masked_backdrop = apply_perimeter_mask(&backdrop, *fraction);
                    masked_backdrop
                        .save(root.join(format!(
                            "{palette}-seed{seed}-backdrop-{mask_label}.png"
                        )))
                        .unwrap();
                    let masked_light = apply_perimeter_mask(&card_light, *fraction);
                    masked_light
                        .save(root.join(format!(
                            "{palette}-seed{seed}-card-light-{mask_label}.png"
                        )))
                        .unwrap();
                    let masked_dark = apply_perimeter_mask(&card_dark, *fraction);
                    masked_dark
                        .save(root.join(format!(
                            "{palette}-seed{seed}-card-dark-{mask_label}.png"
                        )))
                        .unwrap();
                    masked_backdrops[index].push(masked_backdrop);
                    masked_cards_light[index].push(masked_light);
                    masked_cards_dark[index].push(masked_dark);
                }
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
        for (index, fraction) in mask_fractions.iter().enumerate() {
            let mask_label = match *fraction {
                0.04 => "mask4pct",
                0.06 => "mask6pct",
                _ => "mask10pct",
            };
            stitch_contact_sheet(&masked_backdrops[index], 4)
                .save(root.join(format!("contact-backdrops-{mask_label}.png")))
                .unwrap();
            stitch_contact_sheet(&masked_cards_light[index], 4)
                .save(root.join(format!("contact-cards-light-{mask_label}.png")))
                .unwrap();
            stitch_contact_sheet(&masked_cards_dark[index], 4)
                .save(root.join(format!("contact-cards-dark-{mask_label}.png")))
                .unwrap();
        }
    }

    #[test]
    fn export_visual_sheet_when_env_set() {
        write_visual_sheet();
    }
```

- [ ] **Step 4: Verify GREEN**

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

```bash
brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - six-kind integration. mesa/badlands profile tests unchanged. MCP enum and both description strings mention `slot-canyon` and `cave-mouth`. exporter compiles and no-ops without env vars.

- [ ] **Step 5: Commit once**

```bash
git add src/terrain.rs src/cli.rs src/mcp.rs
git commit -m "test(terrain): wire six-kind integration, MCP docs, and visual export"
```

---

## Task 4: Visual QA - slot canyon (max 3 iterations, commit only on pass)

**Files:** `src/terrain.rs` (renderer adjustments only while failing), `implementation-notes.md` (after keep/cut)

- [ ] **Iteration 1 export (receipt required):**

```bash
CLOCHE_VISUAL_SCENE=slot-canyon CLOCHE_VISUAL_ITER=1 \
  brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

Inspect `/tmp/cloche-terrain-scenes/slot-canyon/iter1/contact-backdrops-mask4pct.png` first, then full backdrops and card sheets. Apply the inspection checklist.

- [ ] **If iteration 1 fails:** Adjust `slot_canyon_walls`, `slot_canyon_bedding_signal`, or `structure_slot_canyon` mix weights in the working tree. **Do not commit.** Re-run with `CLOCHE_VISUAL_ITER=2`, then `3` if needed. Capture a Brigade receipt on every export.

- [x] **If iteration 3 still fails - cut slot canyon:**

Remove `TerrainKind::SlotCanyon`, all `slot_canyon_*` functions/tests, `base_layer`/`apply_structure`/`generate` arms, CLI/MCP/README mentions of `slot-canyon`, and the `Some("slot-canyon")` arm from the temporary exporter scene match. Restore `NAMES` to five entries if cave mouth is kept, or four if cave mouth is also cut later. Commit:

```bash
git add src/terrain.rs src/polish.rs src/cli.rs src/mcp.rs implementation-notes.md
git commit -m "revert(terrain): cut slot-canyon after failed visual QA"
```

Append a `## Slot canyon terrain cut (2026-07-26)` section to `implementation-notes.md`. Record the fixed seed, palette, and 440×300 matrix. the one-sentence defect observed on iteration 3. the three export receipt ids copied verbatim. and the exact retained-kind list after the cut.

If slot canyon alone is cut and cave mouth is kept, the retained-kind list is: `dunes`, `mesa`, `badlands`, `glacier`, `cave-mouth`.

**Done:** cut after the iteration 3 failure above. Cave mouth was cut later in the same spike. Final retained kinds are four.

- [ ] **If slot canyon passes on iteration 1:** Keep the Task 1 implementation commit. Do not create another code commit when the working tree has no renderer changes. Append a `## Slot canyon terrain kept (2026-07-26)` section to `implementation-notes.md` recording iteration 1, successful 4% outer-band wall readability, and the passing export receipt id copied verbatim.

- [ ] **If slot canyon passes on iteration 2:** Commit the accepted working-tree adjustments once:

```bash
git add src/terrain.rs
git commit -m "fix(terrain): slot-canyon visual QA iteration 2"
```

- [ ] **If slot canyon passes on iteration 3:** Commit the accepted working-tree adjustments once:

```bash
git add src/terrain.rs
git commit -m "fix(terrain): slot-canyon visual QA iteration 3"
```

After an iteration 2 pass, append a `## Slot canyon terrain kept (2026-07-26)` section to `implementation-notes.md` recording iteration 2, successful 4% outer-band wall readability, and the passing export receipt id copied verbatim. After an iteration 3 pass, record the same facts with iteration 3 and its passing receipt.

---

## Task 5: Visual QA - cave mouth (max 3 iterations, commit only on pass)

**Files:** `src/terrain.rs`, `src/polish.rs`, `src/cli.rs`, `src/mcp.rs`, `implementation-notes.md`

Independent of Task 4 outcome. If slot canyon was cut, cave mouth QA still runs on the remaining five- or six-kind tree.

- [ ] **Iteration 1 export:**

```bash
CLOCHE_VISUAL_SCENE=cave-mouth CLOCHE_VISUAL_ITER=1 \
  brigade work verify run --target . \
  --command "cargo test export_visual_sheet_when_env_set -- --nocapture" \
  --capture brigade-work
```

Inspect `/tmp/cloche-terrain-scenes/cave-mouth/iter1/contact-backdrops-mask4pct.png` first.

- [ ] **If iteration 1 fails:** Adjust `cave_mouth_enclosure_mask`, `cave_mouth_features` placement, or `structure_cave_mouth` rim/tip weights. **Do not commit** until pass or cut. Maximum three iterations with receipt on each export.

- [x] **If iteration 3 still fails - cut cave mouth:**

Remove `TerrainKind::CaveMouth`, all `cave_mouth_*` functions/tests, match arms, `cave-mouth` from CLI/MCP/README, and the `Some("cave-mouth")` arm from the temporary exporter scene match. Restore `NAMES` to the correct retained length. Commit:

```bash
git add src/terrain.rs src/polish.rs src/cli.rs src/mcp.rs implementation-notes.md
git commit -m "revert(terrain): cut cave-mouth after failed visual QA"
```

Append a `## Cave mouth terrain cut (2026-07-26)` section to `implementation-notes.md`. Record the fixed seed, palette, and 440×300 matrix. the one-sentence defect observed on iteration 3. the three export receipt ids copied verbatim. and the exact retained-kind list after the cut.

If cave mouth alone is cut and slot canyon is kept, the retained-kind list is: `dunes`, `mesa`, `badlands`, `glacier`, `slot-canyon`.

**Done:** cut after the iteration 3 failure above. Final retained kinds: `dunes`, `mesa`, `badlands`, `glacier`.

- [ ] **If cave mouth passes on iteration 1:** Keep the Task 2 implementation commit. Do not create another code commit when the working tree has no renderer changes. Append a `## Cave mouth terrain kept (2026-07-26)` section to `implementation-notes.md` recording iteration 1, successful 4% enclosure readability with 3-4 irregular broad dripstone forms, and the passing export receipt id copied verbatim.

- [ ] **If cave mouth passes on iteration 2:** Commit once:

```bash
git add src/terrain.rs
git commit -m "fix(terrain): cave-mouth visual QA iteration 2"
```

Append a `## Cave mouth terrain kept (2026-07-26)` section to `implementation-notes.md` recording iteration 2, successful 4% enclosure readability with 3-4 irregular broad dripstone forms, and the passing export receipt id copied verbatim.

- [ ] **If cave mouth passes on iteration 3:** Commit once:

```bash
git add src/terrain.rs
git commit -m "fix(terrain): cave-mouth visual QA iteration 3"
```

Append a `## Cave mouth terrain kept (2026-07-26)` section to `implementation-notes.md` recording iteration 3, successful 4% enclosure readability with 3-4 irregular broad dripstone forms, and the passing export receipt id copied verbatim.

---

## Task 6: Documentation for retained scenes only

**Files:** `README.md`, `implementation-notes.md`

Skip README edits when both scenes are cut. **Done (2026-07-26):** both scenes cut. No README terrain section. Cut notes in `implementation-notes.md` only.

- [ ] **If both scenes are kept:** Add this `## Terrain backdrops` section to `README.md`:

````markdown
## Terrain backdrops

The fifth backdrop family: procedural landforms drawn from the style seed. Pin a structure with `--terrain` (or `terrain` in MCP JSON). Terrain palettes (`dunes`, `mesa`, `badlands`, `glacier`) supply color; the terrain kind supplies structure.

Kinds: `dunes`, `mesa`, `badlands`, `glacier`, `slot-canyon`, `cave-mouth`.

Example:

```bash
cloche polish --palette dunes --terrain slot-canyon --style-seed 7 shot.png
```
````

- [ ] **If only slot canyon is kept:** Add this section instead:

````markdown
## Terrain backdrops

The fifth backdrop family: procedural landforms drawn from the style seed. Pin a structure with `--terrain` (or `terrain` in MCP JSON). Terrain palettes (`dunes`, `mesa`, `badlands`, `glacier`) supply color; the terrain kind supplies structure.

Kinds: `dunes`, `mesa`, `badlands`, `glacier`, `slot-canyon`.

Example:

```bash
cloche polish --palette dunes --terrain slot-canyon --style-seed 7 shot.png
```
````

- [ ] **If only cave mouth is kept:** Add this section instead:

````markdown
## Terrain backdrops

The fifth backdrop family: procedural landforms drawn from the style seed. Pin a structure with `--terrain` (or `terrain` in MCP JSON). Terrain palettes (`dunes`, `mesa`, `badlands`, `glacier`) supply color; the terrain kind supplies structure.

Kinds: `dunes`, `mesa`, `badlands`, `glacier`, `cave-mouth`.

Example:

```bash
cloche polish --palette mesa --terrain cave-mouth --style-seed 7 shot.png
```
````

- [ ] **Update `implementation-notes.md`** with final keep/cut summary and cross-reference profile test names (`slot_canyon_*`, `cave_mouth_*`).

- [ ] **Commit docs only when README changed or notes need a standalone commit:**

```bash
git add README.md implementation-notes.md
git commit -m "docs(terrain): record slot-canyon and cave-mouth visual QA outcome"
```

If both scenes are cut, append cut sections only (no README commit).

---

## Task 7: Remove temporary visual exporter (before final delivery)

**Files:** `src/terrain.rs`

The exporter is test-only scaffolding. Remove it after visual QA completes (pass or cut) and before final `./scripts/verify`.

- [x] **Delete** `mock_screenshot`, `normalize_contact_tile`, `apply_perimeter_mask`, `stitch_contact_sheet`, `write_visual_sheet`, and `export_visual_sheet_when_env_set` from `src/terrain.rs`.

- [ ] **Verify GREEN:**

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::slot_canyon_ -- --nocapture" \
  --capture brigade-work
```

```bash
brigade work verify run --target . \
  --command "cargo test terrain::tests::cave_mouth_ -- --nocapture" \
  --capture brigade-work
```

If a scene was cut, run the focused filter that still applies, then shared tests:

```bash
brigade work verify run --target . \
  --command "cargo test every_kind -- --nocapture" \
  --capture brigade-work
```

Expected: **PASS** - no `export_visual_sheet_when_env_set` test remains. all retained geometry tests green.

- [x] **Commit once:**

```bash
git add src/terrain.rs
git commit -m "chore(terrain): remove temporary visual export helper"
```

**Done:** removed before the branch was squashed.

---

## Task 8: Final verification, Vale, handoff, clean status

**Files:** `.claude/memory-handoffs/` (if present), session notes

- [x] **Full verify:**

```bash
brigade work verify run --target . --command "./scripts/verify" --capture brigade-work
```

Expected: **PASS** - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all green. No ignored tests, no `#[allow]`, no new dependencies, no visual exporter test.

**Done:** receipt `20260727-000219-work-verify-b548c4`.

- [ ] **Vale on public prose** (README and any new `implementation-notes.md` sentences intended for contributors):

```bash
brigade work verify run --target . --command "vale README.md implementation-notes.md" --capture brigade-work
```

Expected: **0 errors** on edited sections. Fix Slop violations before finishing.

- [ ] **Whitespace trap check:**

```bash
brigade work verify run --target . --command "git diff --check" --capture brigade-work
```

Expected: no conflict markers or trailing whitespace on touched lines.

- [ ] **Brigade operator checkup:**

```bash
brigade operator checkup --target .
```

- [ ] **Memory handoff** (skip if `.claude/memory-handoffs/` absent). Record:

- `slot_canyon_walls` relief deltas and off-frame anchor centers that passed 4% mask review
- `cave_mouth_features` count/spacing that avoided teeth-fence failure
- Visual failures that drove each iteration
- Independent keep/cut outcome per scene
- Exact `CLOCHE_VISUAL_SCENE` / `CLOCHE_VISUAL_ITER` Brigade export commands and receipt ids

- [ ] **Clean status check:**

```bash
git status --short
```

Expected: clean worktree, or only maintainer-local `.claude/memory-handoffs/` files untracked.

Do **not** push, open a PR, or merge.

---

## Task commit summary

| Task | Commit message | When |
|---|---|---|
| 0 | (verify only) | Baseline green receipt recorded |
| 1 | `feat(terrain): add slot-canyon wall geometry and cross-bedding` | After all `slot_canyon_*` tests GREEN. `NAMES` length 5 |
| 2 | `feat(terrain): add cave-mouth enclosure and dripstone geometry` | After all `cave_mouth_*` tests GREEN. `NAMES` length 6 |
| 3 | `test(terrain): wire six-kind integration, MCP docs, and visual export` | After integration + MCP tests GREEN |
| 4 | `fix(terrain): slot-canyon visual QA iteration 2`, `fix(terrain): slot-canyon visual QA iteration 3`, or `revert(terrain): cut slot-canyon after failed visual QA` | After slot-canyon visual decision |
| 5 | `fix(terrain): cave-mouth visual QA iteration 2`, `fix(terrain): cave-mouth visual QA iteration 3`, or `revert(terrain): cut cave-mouth after failed visual QA` | After cave-mouth visual decision |
| 6 | `docs(terrain): record slot-canyon and cave-mouth visual QA outcome` | When README or standalone notes commit is needed |
| 7 | `chore(terrain): remove temporary visual export helper` | After visual QA. before final verify |
| 8 | (verify only) | Final `./scripts/verify` green |

---

## Delivery checklist

- [x] `TerrainKind::NAMES` contains every **retained** scene name. `from_name` round-trips. Four kinds: `dunes`, `mesa`, `badlands`, `glacier`
- [x] `terrain_from_name`, `resolve_style`, `style_from_query`, `capture`, `run_polish`, `backdrop_png`, `card_png`, `render`, and `apply_structure` work without consumer logic changes outside description text
- [x] MCP capture/polish descriptions list every retained kind. `capture_tool_advertises_the_terrain_enum` asserts enum length and both new names when retained. Four kinds only after both cuts
- [x] Slot canyon and cave mouth profile tests removed with cut variants
- [x] Visual sheets and 4%/6%/10% masked contact sheets produced for each scene (max 3 iterations each). All six iteration exports captured. All failed 4% gate
- [x] Temporary visual exporter removed before the branch was squashed
- [x] `./scripts/verify` green via Brigade (`20260727-000219-work-verify-b548c4`)
- [x] Vale clean on public prose edits (via Brigade). Receipt `20260727-000703-work-verify-d812ed`
- [ ] `git diff --check` clean (via Brigade)
- [ ] Memory handoff written (maintainer-local)
- [ ] `git status --short` clean

---

## Plan self-review (completed before handoff)

- **Task count:** Nine tasks (Tasks 0 through 8).
- **Spec coverage:** Every automated and visual acceptance bullet in `docs/specs/2026-07-26-slot-canyon-cave-mouth-terrain-scenes.md` maps to Tasks 0-8 (profile thresholds, mask widths, iteration cap, independent cut, README/notes gating, no new deps).
- **Placeholder scan:** No TBD, TODO, angle-bracket templates, or "implement later" steps. Visual cut/keep notes instruct the executor to quote exact observed defects and exact Brigade receipt ids. Every behavior task includes full test and production Rust blocks.
- **Name consistency:** `slot-canyon`, `cave-mouth`, `SlotCanyon`, `CaveMouth`, helper prefixes, and `NAMES` order match across tasks.
- **Cave mouth horizon:** `CaveMouth` uses the shared `ramp_horizon` branch in `base_layer`. no `cave_mouth_horizon` helper. Identity comes from `cave_mouth_enclosure_mask` and `cave_mouth_features`.
- **Profile math:** `cave_mouth_enclosure_mask` full-width ceiling is strongest at `v=0` with seeded width ripple. `cave_mouth_feature_bases` plus bounded jitter guarantees gap spread >= 0.04 canvas width for 3 and 4 features. Side-band test binds `side_peak` once.
- **SLOT_WALL_SALTS:** Both array entries used (`[0]` bedding warp, `[1]` floor ripple).
- **Task 3 RED/GREEN:** MCP assertions land before description updates. RED observed while descriptions list four kinds.
- **Exporter:** Reuses `use crate::polish.`, `polish::compose_card`, and `TerrainKind::from_name(&scene).expect("visual scene")`. Each iteration writes 204 PNG files (192 individual tiles plus 12 contact sheets. see Visual sheet protocol).
- **Brigade commands:** No `&&`, pipes, or env-prefix chains inside `--command`. env vars are set in the shell before `brigade work verify run`. Vale and `git diff --check` route through Brigade verify.

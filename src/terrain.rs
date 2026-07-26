//! Procedural terrain backdrops: dunes, mesa, badlands, glacier.
//!
//! The fifth backdrop family, alongside the gradients, the deep-space
//! scenes, the geometric patterns, and the skies. It mirrors the space,
//! pattern, and sky models exactly: a terrain palette carries the colors and
//! a [`TerrainKind`] carries the structure, so `--palette` and `--terrain`
//! compose the same way `--palette` and `--sky` do.
//!
//! Everything is drawn from the style seed at render time. Nothing is a stored
//! photograph, so a terrain resolves cleanly at any card size and the same
//! `--style-seed` reproduces one exactly.
//!
//! Terrain reads as a vertical cross-section: a sky ramp above a horizon line
//! and a ground ramp below it, with one structural motif per kind. A finished
//! card is only about 4% backdrop by width and the capture window covers the
//! center, so every kind is judged on texture at the edges and corners, never
//! on a centered subject. The horizon itself is cross-faded with a smoothstep
//! band rather than a bare comparison, which would draw a hard seam straight
//! across the card.
//!
//! All drawing is hand-rolled on `image` + `rand` (no new dependencies, per
//! repo policy). The helpers (`to_f32`, `mix3`, `smoothstep`, `quantize`,
//! grain, `warped_fbm`) are private copies kept here on purpose, matching the
//! convention in `polish.rs`, `space.rs`, `pattern.rs`, and `sky.rs`.

use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::polish::PresentationStyle;

/// Decorrelates the terrain RNG from the style RNG, which already consumed
/// the raw seed in `style_from_seed`. Distinct from the sky salt so the two
/// families never share a noise stream.
const TERRAIN_SEED_SALT: u64 = 0x5452_4E20_4C41_4E44; // "TRN LAND"

/// Matches the gradient, space, pattern, and sky backdrops so film feel stays
/// consistent across all five families.
const GRAIN_STRENGTH: f32 = 2.4;

/// Ridge and dune silhouettes are fbm; five octaves is where the silhouettes
/// stop reading as noise and start reading as landform.
const RIDGE_OCTAVES: u32 = 5;

/// A specific terrain the caller can pin instead of the seed's random pick.
/// The seed still drives every free parameter (horizon, coverage, ridge
/// geometry, light direction); this only forces which landform appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainKind {
    /// Overlapping warped dune ridges, wind ripples, leeward shading.
    Dunes,
    /// Off-center stepped table silhouettes, stratified bands, talus.
    Mesa,
    /// Repeated eroded ridges and gullies across the frame, layered sediment.
    Badlands,
    /// Fractured ice field, crevasse bands, blue shadow and snow highlights.
    Glacier,
}

impl TerrainKind {
    /// All names accepted by [`TerrainKind::from_name`], in menu order.
    pub const NAMES: [&'static str; 4] = ["dunes", "mesa", "badlands", "glacier"];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "dunes" => Self::Dunes,
            "mesa" => Self::Mesa,
            "badlands" => Self::Badlands,
            "glacier" => Self::Glacier,
            _ => return None,
        })
    }
}

/// Everything the renderer needs, rolled once so the per-pixel loop stays a
/// pure function of the scene.
struct Terrain {
    kind: TerrainKind,
    noise_seed: u64,
    /// Vertical position of the horizon line, 0 at the top.
    horizon: f32,
    /// Broad ground coverage, 0 bare to 1 solid feature.
    coverage: f32,
    /// Feature scale read as "features across the short edge"; a larger value
    /// means smaller structure. `scale` in pixels per noise unit is derived
    /// from this as `short_edge / features_across`.
    features_across: f32,
    /// Where the light comes from, in canvas fractions. Off-frame is fine and
    /// usually better: it puts the lit edge in the padding band.
    light: (f32, f32),
    /// Canvas width in pixels; anchored profiles use horizontal fractions of this.
    width: f32,
}

pub fn render(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    let mut rng = StdRng::seed_from_u64(style.seed ^ TERRAIN_SEED_SALT);
    let terrain = Terrain::generate(&mut rng, width, height, style.terrain);
    base_layer(width, height, style, &terrain)
}

impl Terrain {
    fn generate(
        rng: &mut StdRng,
        _width: u32,
        _height: u32,
        pinned: Option<TerrainKind>,
    ) -> Terrain {
        let kind = pinned.unwrap_or_else(|| {
            let index = rng.random_range(0..TerrainKind::NAMES.len());
            TerrainKind::from_name(TerrainKind::NAMES[index]).unwrap_or(TerrainKind::Dunes)
        });
        let noise_seed = rng.random();

        let (horizon, coverage, features_across) = match kind {
            TerrainKind::Dunes => (rng.random_range(0.42..=0.58), 0.70, 5.0),
            TerrainKind::Mesa => (rng.random_range(0.50..=0.65), 0.62, 4.0),
            TerrainKind::Badlands => (rng.random_range(0.45..=0.60), 0.78, 6.0),
            TerrainKind::Glacier => (rng.random_range(0.45..=0.60), 0.66, 5.0),
        };

        // Light from the side, usually off-frame. Glacier keeps the sun high
        // and behind so the ice reads as lit from above rather than rimmed.
        let light = match kind {
            TerrainKind::Glacier => (
                rng.random_range(-0.20..=0.20),
                rng.random_range(-0.30..=-0.05),
            ),
            _ => (rng.random_range(0.05..=0.95), rng.random_range(0.70..=1.05)),
        };

        Terrain {
            kind,
            noise_seed,
            horizon,
            coverage,
            features_across,
            light,
            width: _width.max(1) as f32,
        }
    }
}

/// Paint the sky/ground ramp and cross-fade the horizon, then layer the
/// per-kind structure on top.
fn base_layer(width: u32, height: u32, style: &PresentationStyle, terrain: &Terrain) -> RgbaImage {
    let sky_top = to_f32(style.stops[0]);
    let horizon_color = to_f32(style.stops[1]);
    let ground = to_f32(style.stops[2]);
    let light = to_f32(style.glow_a);
    let shade = to_f32(style.glow_b);

    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    // `scale` is pixels per noise unit, derived from the short edge so a wide
    // card does not stretch terrain structure into streaks. `features_across`
    // reads as "features across the short edge", so a larger value means
    // smaller structure. Inverting this yields ~one noise cell per pixel and
    // the result is speckle.
    let scale = (w.min(h).max(1.0) / terrain.features_across.max(0.1)).max(1.0);

    ImageBuffer::from_fn(width.max(1), height.max(1), |px, py| {
        let fx = px as f32;
        let fy = py as f32;
        let v = fy / h;

        // A soft but straight split still reads as a seam. Dunes move the
        // split itself with a broad x-only field so their skyline rolls.
        let local_horizon = match terrain.kind {
            TerrainKind::Dunes => dune_horizon(fx, scale, terrain),
            TerrainKind::Mesa => mesa_profile(fx, terrain).0,
            TerrainKind::Badlands => badlands_horizon(fx, terrain),
            _ => terrain.horizon,
        };

        // Mesa steps move only the silhouette. Letting each table also change
        // the ramp normalization paints a vertical column through the sky.
        let ramp_horizon = match terrain.kind {
            TerrainKind::Mesa => terrain.horizon,
            _ => local_horizon,
        };

        // Two-segment vertical ramp: sky above the horizon, ground below it.
        let sky_progress = (v / ramp_horizon.max(0.001)).clamp(0.0, 1.0);
        let ground_progress =
            ((v - ramp_horizon) / (1.0 - ramp_horizon).max(0.001)).clamp(0.0, 1.0);
        let sky_col = mix3(sky_top, horizon_color, smoothstep(sky_progress));
        let ground_col = mix3(horizon_color, ground, smoothstep(ground_progress));

        // Cross-fade the horizon with a smoothstep band rather than a bare
        // comparison, which would draw a hard seam straight across the card.
        let band = match terrain.kind {
            TerrainKind::Mesa => 0.012,
            _ => 0.025,
        };
        let sky_mask = 1.0 - smoothstep_range(v, local_horizon - band, local_horizon + band);
        let mut color = mix3(ground_col, sky_col, sky_mask);

        // Atmospheric haze bands across the sky. Without this the top edge is a
        // flat sky-top band, which reads as padding on a finished card; a low
        // frequency warped field laid across the whole sky puts gentle
        // horizontal variation into the top edge and the top corners without
        // introducing a centered subject. The amplitude stays small so the sky
        // still reads as a ramp, not as cloud cover.
        let haze = warped_fbm(
            fx / (scale * 2.5),
            fy / (scale * 2.5),
            terrain.noise_seed ^ 0x5D4D_534B,
            4,
        );
        let haze_amt = smoothstep_range(haze, 0.35, 0.65) * sky_mask * 0.10;
        color = mix3(color, horizon_color, haze_amt);

        let ctx = StructureCtx {
            terrain,
            v,
            fx,
            fy,
            scale,
            light,
            shade,
            horizon_color,
            seed: terrain.noise_seed,
        };
        color = apply_structure(color, &ctx);

        let grain = grain_noise(px, py, terrain.noise_seed) * GRAIN_STRENGTH;
        Rgba([
            quantize(color[0] + grain),
            quantize(color[1] + grain),
            quantize(color[2] + grain),
            255,
        ])
    })
}

/// Everything the per-kind structure pass needs, bundled so the per-kind
/// functions stay under clippy's argument-count threshold without silencing
/// the lint. The codebase keeps helpers duplicated per backdrop module on
/// purpose; this struct is the terrain module's own small convenience.
struct StructureCtx<'a> {
    terrain: &'a Terrain,
    v: f32,
    fx: f32,
    fy: f32,
    scale: f32,
    light: [f32; 3],
    shade: [f32; 3],
    horizon_color: [f32; 3],
    seed: u64,
}

/// The per-kind structure pass. Split out so `base_layer` stays a ramp plus a
/// grain, which is what every terrain has in common.
fn apply_structure(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    match ctx.terrain.kind {
        TerrainKind::Dunes => structure_dunes(base, ctx),
        TerrainKind::Mesa => structure_mesa(base, ctx),
        TerrainKind::Badlands => structure_badlands(base, ctx),
        TerrainKind::Glacier => structure_glacier(base, ctx),
    }
}

/// Dunes: overlapping warped ridges, wind ripples, leeward shading. The
/// structure runs across the whole ground band, so the padding edges carry
/// ridge texture rather than a centered dune.
fn structure_dunes(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let fy = ctx.fy;
    let scale = ctx.scale;
    let light = ctx.light;
    let shade = ctx.shade;
    let seed = ctx.seed;
    let crest = dune_horizon(fx, scale, terrain);
    let ground_mask = smoothstep_range(v, crest - 0.018, crest + 0.035);
    if ground_mask <= 0.0 {
        return base;
    }
    // Shade coherent faces from the silhouette slope. Two-dimensional density
    // here produced smoky blobs instead of a dune body.
    let left = dune_horizon(fx - scale * 0.12, scale, terrain);
    let right = dune_horizon(fx + scale * 0.12, scale, terrain);
    let slope = ((right - left) * 9.0).clamp(-1.0, 1.0);
    let light_from_right = terrain.light.0 >= 0.5;
    let windward = if light_from_right {
        slope.max(0.0)
    } else {
        (-slope).max(0.0)
    };
    let leeward = if light_from_right {
        (-slope).max(0.0)
    } else {
        slope.max(0.0)
    };

    // Diagonal wind ripples. Multiplying by TAU makes the stated cell size one
    // full period, and the jitter varies at the ripple cell scale.
    let tau = std::f32::consts::TAU;
    let jitter = (value_noise(fx / (scale * 0.22), fy / (scale * 0.22), seed ^ 0x8111) - 0.5) * 1.4;
    let ripple = ((fx / (scale * 0.20)) * tau + (fy / (scale * 0.075)) * tau + jitter).sin();
    let ripple_amt = smoothstep_range(ripple, 0.25, 0.95) * 0.10 * ground_mask * terrain.coverage;

    let body = mix3(base, shade, ground_mask * terrain.coverage * 0.20);
    let lit = mix3(body, light, windward * ground_mask * 0.32);
    let shaded = mix3(lit, shade, leeward * ground_mask * 0.28);
    mix3(shaded, light, ripple_amt)
}

fn dune_horizon(fx: f32, scale: f32, terrain: &Terrain) -> f32 {
    let broad = warped_fbm(
        fx / (scale * 1.8),
        0.37,
        terrain.noise_seed ^ 0x4455_4E45,
        4,
    );
    let ridge = warped_fbm(
        fx / (scale * 0.72) + 3.1,
        1.9,
        terrain.noise_seed ^ 0x5249_4447,
        3,
    );
    (terrain.horizon - 0.025 - broad * 0.13 - ridge * 0.035).clamp(0.20, 0.78)
}

/// Mesa: two or three anchored horizontal table lanes with stepped shoulders,
/// stratified bands, and darker cliff-face shading between open ground.
fn structure_mesa(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let fy = ctx.fy;
    let scale = ctx.scale;
    let light = ctx.light;
    let shade = ctx.shade;
    let horizon_color = ctx.horizon_color;
    let seed = ctx.seed;
    let (top, best_dist, on_shoulder) = mesa_profile(fx, terrain);
    let ground_mask = smoothstep_range(v, top - 0.015, top + 0.025);
    if ground_mask <= 0.0 {
        return base;
    }
    let strata = fbm(fx / (scale * 0.25), fy / (scale * 0.04), seed ^ 0x6A6A, 4);
    let strata_amt = smoothstep_range(strata, 0.42, 0.62) * 0.18 * ground_mask;
    let talus = warped_fbm(fx / (scale * 0.2), fy / (scale * 0.2), seed ^ 0x7A11, 3);
    let talus_amt = smoothstep_range(talus, 0.50, 0.70) * 0.12 * ground_mask * terrain.coverage;

    let above_ground = 1.0 - smoothstep_range(v, terrain.horizon - 0.015, terrain.horizon + 0.035);
    let table = ground_mask * above_ground * terrain.coverage;
    let body = mix3(base, shade, (table * 0.58).min(0.68));
    // Stepped shoulders and cliff faces read darker than the flat cap.
    let cliff = on_shoulder * table;
    let faced = mix3(body, shade, (cliff * 0.55).min(0.62));
    let rim = (1.0 - best_dist).max(0.0) * table * (1.0 - on_shoulder * 0.85);
    let lit = mix3(faced, light, (rim * 0.5).min(0.4));
    let banded = mix3(lit, horizon_color, strata_amt);
    mix3(banded, shade, talus_amt)
}

fn mesa_lane_count(seed: u64) -> usize {
    2 + (cell_hash(0, 0, seed ^ 0x4D45_5341) * 2.0).floor() as usize
}

fn mesa_lanes(seed: u64) -> [(f32, f32, f32, f32); 3] {
    let count = mesa_lane_count(seed);
    let mut lanes = [(0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32); 3];

    // Left and right edge plateaus are anchored partly off-frame so the
    // capture window's center padding still reads as mesa shoulders and caps.
    let left_jitter = (cell_hash(0, 0, seed ^ 0x4C41_4E45) - 0.5) * 0.05;
    let left_center = -0.03 + left_jitter;
    let left_half = 0.21 + cell_hash(0, 1, seed ^ 0x5749_4454) * 0.08;
    let left_height = 0.30 + cell_hash(0, 2, seed ^ 0x4845_4947) * 0.14;
    let left_shoulder = 0.25 + cell_hash(0, 3, seed ^ 0x5348_4F55) * 0.18;
    lanes[0] = (left_center, left_half, left_height, left_shoulder);

    let right_jitter = (cell_hash(1, 0, seed ^ 0x5249_4748) - 0.5) * 0.05;
    let right_center = 1.03 + right_jitter;
    let right_half = 0.21 + cell_hash(1, 1, seed ^ 0x5749_4454) * 0.08;
    let right_height = 0.30 + cell_hash(1, 2, seed ^ 0x4845_4947) * 0.14;
    let right_shoulder = 0.25 + cell_hash(1, 3, seed ^ 0x5348_4F55) * 0.18;
    lanes[1] = (right_center, right_half, right_height, right_shoulder);

    if count >= 3 {
        let center_jitter = (cell_hash(2, 0, seed ^ 0x4345_4E54) - 0.5) * 0.08;
        let center = 0.50 + center_jitter;
        let half_width = 0.07 + cell_hash(2, 1, seed ^ 0x434E_5452) * 0.05;
        let height = 0.16 + cell_hash(2, 2, seed ^ 0x434E_5448) * 0.10;
        let shoulder_frac = 0.28 + cell_hash(2, 3, seed ^ 0x434E_5453) * 0.15;
        lanes[2] = (center, half_width, height, shoulder_frac);
    }
    lanes
}

fn mesa_lane_height(
    u: f32,
    center: f32,
    half_width: f32,
    height: f32,
    shoulder_frac: f32,
) -> (f32, f32) {
    let dx = (u - center).abs();
    if dx >= half_width {
        return (0.0, 0.0);
    }
    let cap_end = half_width * (1.0 - shoulder_frac.max(0.25));
    if dx <= cap_end {
        let dist = dx / half_width.max(0.001);
        return (height, dist);
    }
    let shoulder_w = (half_width - cap_end).max(0.001);
    let step = (dx - cap_end) / shoulder_w;
    let terrace = if step < 0.5 {
        height * 0.68
    } else {
        height * 0.34
    };
    let dist = dx / half_width.max(0.001);
    (terrace, dist)
}

fn mesa_profile(fx: f32, terrain: &Terrain) -> (f32, f32, f32) {
    let u = fx / terrain.width.max(1.0);
    let lanes = mesa_lanes(terrain.noise_seed);
    let count = mesa_lane_count(terrain.noise_seed);
    let mut best_height = 0.0_f32;
    let mut best_dist = 1.0_f32;
    let mut on_shoulder = 0.0_f32;
    for &(center, half_width, height, shoulder_frac) in &lanes[..count] {
        let (terrace, dist) = mesa_lane_height(u, center, half_width, height, shoulder_frac);
        if terrace > best_height {
            best_height = terrace;
            best_dist = dist;
            let cap_end = half_width * (1.0 - shoulder_frac.max(0.25));
            on_shoulder = if (u - center).abs() > cap_end && terrace > 0.0 {
                1.0
            } else {
                0.0
            };
        }
    }
    let relief = best_height * terrain.coverage;
    let top = terrain.horizon - relief * 0.36;
    (top.clamp(0.20, terrain.horizon), best_dist, on_shoulder)
}

/// Badlands: anchored eroded ridge profiles, slope-lit faces, horizontal
/// strata, and localized diagonal washes without a periodic full-height mask.
fn structure_badlands(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let fy = ctx.fy;
    let scale = ctx.scale;
    let light = ctx.light;
    let shade = ctx.shade;
    let horizon_color = ctx.horizon_color;
    let seed = ctx.seed;
    let crest = badlands_horizon(fx, terrain);
    let ground_mask = smoothstep_range(v, crest - 0.014, crest + 0.028);
    if ground_mask <= 0.0 {
        return base;
    }

    let u = fx / terrain.width.max(1.0);
    let ridge_body = badlands_ridge_body(u, terrain);
    let localized_wash = value_noise(
        fx / (scale * 0.55) + ridge_body * 4.0,
        fy / (scale * 0.65),
        seed ^ 0x5741_5348,
    );
    let wash = smoothstep_range(localized_wash, 0.62, 0.88)
        * ridge_body
        * ground_mask
        * terrain.coverage
        * 0.22;

    let tau = std::f32::consts::TAU;
    let strata_jitter =
        (value_noise(fx / (scale * 0.35), fy / (scale * 0.35), seed ^ 0x6A6A) - 0.5) * 0.9;
    let strata = ((fy / (scale * 0.24)) * tau + strata_jitter).sin();
    let strata_amt = smoothstep_range(strata, 0.05, 0.92) * 0.11 * ground_mask * terrain.coverage;

    let sample = terrain.width * 0.022;
    let left = badlands_horizon(fx - sample, terrain);
    let right = badlands_horizon(fx + sample, terrain);
    let slope = ((right - left) * 7.0).clamp(-1.0, 1.0);
    let lit_face = if terrain.light.0 >= 0.5 {
        slope.max(0.0)
    } else {
        (-slope).max(0.0)
    };
    let lit_face = lit_face * lit_face;

    let body = mix3(
        base,
        shade,
        (ground_mask * terrain.coverage * 0.42).min(0.55),
    );
    let ridged = mix3(body, shade, (ridge_body * ground_mask * 0.28).min(0.38));
    let lit = mix3(ridged, light, lit_face * ground_mask * 0.22);
    let washed = mix3(lit, shade, wash);
    mix3(washed, horizon_color, strata_amt)
}

fn badlands_ridge_count(seed: u64) -> usize {
    4 + (cell_hash(0, 1, seed ^ 0x4241_444C) * 2.0).floor() as usize
}

fn badlands_ridges(seed: u64) -> [(f32, f32, f32); 5] {
    let count = badlands_ridge_count(seed);
    let mut ridges = [(0.0_f32, 0.0_f32, 0.0_f32); 5];
    const ANCHORS: [(f32, f32, f32); 5] = [
        (-0.05, 0.34, 0.18),
        (0.20, 0.30, 0.16),
        (0.50, 0.32, 0.15),
        (0.80, 0.30, 0.16),
        (1.05, 0.34, 0.18),
    ];
    for (index, ridge) in ridges.iter_mut().enumerate().take(count) {
        let (base_center, base_half, base_peak) = ANCHORS[index];
        let center_jitter = (cell_hash(index as i64, 0, seed ^ 0x5249_4447) - 0.5) * 0.06;
        let half_jitter = cell_hash(index as i64, 1, seed ^ 0x4552_4F44) * 0.06;
        let peak_jitter = cell_hash(index as i64, 2, seed ^ 0x5045_414B) * 0.06;
        *ridge = (
            base_center + center_jitter,
            base_half + half_jitter,
            base_peak + peak_jitter,
        );
    }
    ridges
}

fn badlands_ridge_height(u: f32, center: f32, half_width: f32, peak: f32) -> f32 {
    let dx = (u - center).abs();
    if dx >= half_width {
        return 0.0;
    }
    let t = 1.0 - dx / half_width.max(0.001);
    let eroded = t * smoothstep(t);
    peak * eroded
}

fn badlands_ridge_body(u: f32, terrain: &Terrain) -> f32 {
    let ridges = badlands_ridges(terrain.noise_seed);
    let count = badlands_ridge_count(terrain.noise_seed);
    let mut body = 0.0_f32;
    for &(center, half_width, peak) in &ridges[..count] {
        body = body.max(badlands_ridge_height(u, center, half_width, peak));
    }
    let floor = 0.05 + cell_hash(0, 2, terrain.noise_seed ^ 0x464C_4F52) * 0.03;
    body = body.max(floor);
    body * terrain.coverage
}

fn badlands_horizon(fx: f32, terrain: &Terrain) -> f32 {
    let u = fx / terrain.width.max(1.0);
    let relief = badlands_ridge_body(u, terrain);
    (terrain.horizon - relief * 0.95).clamp(0.20, 0.80)
}

/// Glacier: a fractured ice field, crevasse bands, blue shadow in the
/// crevasses and snow highlights on the ridges.
fn structure_glacier(base: [f32; 3], ctx: &StructureCtx) -> [f32; 3] {
    let terrain = ctx.terrain;
    let v = ctx.v;
    let fx = ctx.fx;
    let fy = ctx.fy;
    let scale = ctx.scale;
    let light = ctx.light;
    let shade = ctx.shade;
    let seed = ctx.seed;
    let ground_mask = smoothstep_range(v, terrain.horizon - 0.06, terrain.horizon + 0.04);
    if ground_mask <= 0.0 {
        return base;
    }
    // Ice relief: warped fbm gives rolling ice hummocks across the field.
    let relief = warped_fbm(fx / scale, fy / (scale * 0.9), seed, RIDGE_OCTAVES);
    let edge = 0.50 - terrain.coverage * 0.28;
    let ice_mass = smoothstep_range(relief, edge - 0.10, edge + 0.18) * ground_mask;

    // Crevasse bands: high-frequency warped fractures. Where the fracture
    // field is low, the ice opens into a blue crevasse.
    let fracture = warped_fbm(fx / (scale * 0.22), fy / (scale * 0.22), seed ^ 0xC4A5, 4);
    let crevasse = (1.0 - smoothstep_range(fracture, 0.30, 0.50)) * ground_mask * terrain.coverage;

    // Snow highlights on the high ridges, blue shadow in the crevasses.
    let snow = ice_mass * (0.5 + relief * 0.5);
    let body = mix3(base, light, (snow * 0.45).min(0.5));
    mix3(body, shade, (crevasse * 0.7).min(0.6))
}

/// Domain-warped fbm. Plain value noise is built on a grid, and at the low
/// frequencies a landform silhouette needs (only a few cells across the whole
/// canvas) that grid shows through as visible interpolated quadrilaterals.
/// Offsetting the sample point by a second noise field breaks the alignment.
fn warped_fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let wx = fbm(x * 0.5, y * 0.5, seed ^ 0x5741_5250, 3) - 0.5;
    let wy = fbm(x * 0.5 + 5.2, y * 0.5 + 1.3, seed ^ 0x5750_5259, 3) - 0.5;
    fbm(x + wx * 1.8, y + wy * 1.8, seed, octaves)
}

fn fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut normal = 0.0;
    for octave in 0..octaves {
        total += value_noise(x * frequency, y * frequency, seed ^ (octave as u64)) * amplitude;
        normal += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if normal <= 0.0 { 0.0 } else { total / normal }
}

fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let tx = smoothstep(x - x0);
    let ty = smoothstep(y - y0);
    let (ix, iy) = (x0 as i64, y0 as i64);
    let c00 = cell_hash(ix, iy, seed);
    let c10 = cell_hash(ix + 1, iy, seed);
    let c01 = cell_hash(ix, iy + 1, seed);
    let c11 = cell_hash(ix + 1, iy + 1, seed);
    let top = c00 + (c10 - c00) * tx;
    let bottom = c01 + (c11 - c01) * tx;
    top + (bottom - top) * ty
}

fn cell_hash(x: i64, y: i64, seed: u64) -> f32 {
    let mut hash = seed ^ 0x9E37_79B9_7F4A_7C15;
    hash ^= (x as u64).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    hash ^= (y as u64).wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    hash ^= hash >> 29;
    (hash >> 40) as f32 / 16_777_216.0
}

fn grain_noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut hash = seed ^ 0x2545_F491_4F6C_DD1D;
    hash ^= (x as u64).wrapping_mul(0x9E37_79B1);
    hash ^= (y as u64).wrapping_mul(0x85EB_CA6B);
    hash ^= hash >> 31;
    hash = hash.wrapping_mul(0xC2B2_AE35);
    hash ^= hash >> 27;
    ((hash >> 40) as f32 / 16_777_216.0) - 0.5
}

fn to_f32(color: [u8; 3]) -> [f32; 3] {
    [color[0] as f32, color[1] as f32, color[2] as f32]
}

fn mix3(start: [f32; 3], end: [f32; 3], amount: f32) -> [f32; 3] {
    let t = amount.clamp(0.0, 1.0);
    [
        start[0] + (end[0] - start[0]) * t,
        start[1] + (end[1] - start[1]) * t,
        start[2] + (end[2] - start[2]) * t,
    ]
}

/// Smoothstep between two edges. Soft thresholds are the difference between a
/// silhouette and a field of speckles, so almost every kind uses this instead
/// of a bare comparison.
fn smoothstep_range(value: f32, edge0: f32, edge1: f32) -> f32 {
    if edge1 <= edge0 {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    smoothstep((value - edge0) / (edge1 - edge0))
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn quantize(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polish;

    fn terrain_style(seed: u64, kind: &str) -> PresentationStyle {
        let mut style = polish::style_with_palette(seed, "dunes").expect("terrain palette");
        style.terrain = TerrainKind::from_name(kind);
        style
    }

    #[test]
    fn every_name_round_trips() {
        for name in TerrainKind::NAMES {
            assert!(
                TerrainKind::from_name(name).is_some(),
                "{name} did not parse"
            );
        }
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert_eq!(TerrainKind::from_name("butte"), None);
        assert_eq!(TerrainKind::from_name(""), None);
    }

    #[test]
    fn names_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for name in TerrainKind::NAMES {
            assert!(!seen.contains(&name), "duplicate name {name}");
            seen.push(name);
        }
    }

    #[test]
    fn same_seed_renders_identically() {
        for name in TerrainKind::NAMES {
            let style = terrain_style(7, name);
            let a = render(96, 72, &style);
            let b = render(96, 72, &style);
            assert_eq!(a.as_raw(), b.as_raw(), "{name} was not reproducible");
        }
    }

    #[test]
    fn different_seeds_render_differently() {
        for name in TerrainKind::NAMES {
            let a = render(80, 60, &terrain_style(3, name));
            let b = render(80, 60, &terrain_style(4, name));
            assert_ne!(a.as_raw(), b.as_raw(), "{name} was seed-insensitive");
        }
    }

    #[test]
    fn every_kind_is_opaque_everywhere() {
        for name in TerrainKind::NAMES {
            let style = terrain_style(3, name);
            let canvas = render(80, 60, &style);
            assert!(
                canvas.pixels().all(|pixel| pixel[3] == 255),
                "{name} left a transparent pixel"
            );
        }
    }

    #[test]
    fn every_kind_varies_across_the_canvas() {
        for name in TerrainKind::NAMES {
            let style = terrain_style(11, name);
            let canvas = render(80, 60, &style);
            let first = canvas.pixels().next().copied().expect("a pixel");
            assert!(
                canvas.pixels().any(|pixel| pixel != &first),
                "{name} rendered flat"
            );
        }
    }

    #[test]
    fn kinds_render_differently_from_each_other() {
        let base = render(64, 48, &terrain_style(5, "dunes"));
        for name in TerrainKind::NAMES.iter().skip(1) {
            let other = render(64, 48, &terrain_style(5, name));
            assert_ne!(base.as_raw(), other.as_raw(), "{name} matched dunes");
        }
    }

    #[test]
    fn survives_a_zero_dimension() {
        let style = terrain_style(1, "dunes");
        assert_eq!(render(0, 0, &style).dimensions(), (1, 1));
        assert_eq!(render(0, 40, &style).dimensions(), (1, 40));
        assert_eq!(render(40, 0, &style).dimensions(), (40, 1));
    }

    /// The whole point of the terrain family: the capture window covers the
    /// center, so a scene built around a centered subject is invisible. This
    /// asserts that every edge band and every corner carries texture (more
    /// than one distinct pixel), not a flat fill.
    #[test]
    fn every_kind_carries_texture_at_the_edges_and_corners() {
        for name in TerrainKind::NAMES {
            let canvas = render(120, 90, &terrain_style(9, name));
            let (w, h) = canvas.dimensions();
            // Each edge band: a strip of pixels along one edge must vary.
            for &(xs, ys, dx, dy, label) in &[
                (0, 0, 1, 0, "top"),
                (0, (h - 1) as i32, 1, 0, "bottom"),
                (0, 0, 0, 1, "left"),
                ((w - 1) as i32, 0, 0, 1, "right"),
            ] {
                let mut distinct = std::collections::HashSet::new();
                for i in 0..16 {
                    let x = (xs + dx * i).max(0).min((w - 1) as i32) as u32;
                    let y = (ys + dy * i).max(0).min((h - 1) as i32) as u32;
                    let p = canvas.get_pixel(x, y).0;
                    distinct.insert((p[0], p[1], p[2]));
                }
                assert!(
                    distinct.len() > 1,
                    "{name} {label} edge was flat ({distinct:?})"
                );
            }
            // The four corners themselves must not all be the same color.
            let mut corners = std::collections::HashSet::new();
            for &(x, y) in &[(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
                let p = canvas.get_pixel(x, y).0;
                corners.insert((p[0], p[1], p[2]));
            }
            assert!(
                corners.len() > 1,
                "{name} all four corners were identical ({corners:?})"
            );
        }
    }

    #[test]
    fn a_pinned_kind_is_honoured() {
        let mut rng = StdRng::seed_from_u64(2);
        let terrain = Terrain::generate(&mut rng, 100, 100, Some(TerrainKind::Mesa));
        assert_eq!(terrain.kind, TerrainKind::Mesa);
    }

    #[test]
    fn an_unpinned_kind_still_picks_something() {
        for seed in 0..24 {
            let mut rng = StdRng::seed_from_u64(seed);
            let terrain = Terrain::generate(&mut rng, 64, 64, None);
            assert!(TerrainKind::NAMES.contains(&kind_name(terrain.kind)));
        }
    }

    fn kind_name(kind: TerrainKind) -> &'static str {
        TerrainKind::NAMES
            .iter()
            .copied()
            .find(|name| TerrainKind::from_name(name) == Some(kind))
            .expect("every kind has a name")
    }

    #[test]
    fn value_noise_is_bounded() {
        for x in 0..40 {
            for y in 0..40 {
                let value = value_noise(x as f32 * 0.37, y as f32 * 0.37, 99);
                assert!((0.0..=1.0).contains(&value), "{value} out of range");
            }
        }
    }

    #[test]
    fn fbm_is_bounded() {
        for x in 0..30 {
            for y in 0..30 {
                let value = fbm(x as f32 * 0.5, y as f32 * 0.5, 7, RIDGE_OCTAVES);
                assert!((0.0..=1.0).contains(&value), "{value} out of range");
            }
        }
    }

    fn mesa_test_terrain(seed: u64) -> Terrain {
        let mut rng = StdRng::seed_from_u64(seed);
        Terrain::generate(&mut rng, 240, 180, Some(TerrainKind::Mesa))
    }

    fn badlands_test_terrain(seed: u64) -> Terrain {
        let mut rng = StdRng::seed_from_u64(seed);
        Terrain::generate(&mut rng, 240, 180, Some(TerrainKind::Badlands))
    }

    fn sample_mesa_tops(terrain: &Terrain, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| {
                let fx = index as f32 * terrain.width / samples as f32;
                mesa_profile(fx, terrain).0
            })
            .collect()
    }

    fn sample_badlands_crests(terrain: &Terrain, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| {
                let fx = index as f32 * terrain.width / samples as f32;
                badlands_horizon(fx, terrain)
            })
            .collect()
    }

    fn count_elevated_components(elevation: &[f32], threshold: f32) -> usize {
        let mut count = 0;
        let mut in_component = false;
        for &value in elevation {
            if value >= threshold {
                if !in_component {
                    count += 1;
                    in_component = true;
                }
            } else {
                in_component = false;
            }
        }
        count
    }

    fn count_ridge_peaks(profile: &[f32]) -> usize {
        if profile.len() < 3 {
            return 0;
        }
        let mut peaks = 0;
        for index in 1..profile.len() - 1 {
            if profile[index] < profile[index - 1] && profile[index] < profile[index + 1] {
                peaks += 1;
            }
        }
        peaks
    }

    fn max_adjacent_drop(values: &[f32]) -> f32 {
        values
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn mesa_profile_has_two_or_three_separated_components() {
        for seed in [1_u64, 7, 42, 99, 2026] {
            let terrain = mesa_test_terrain(seed);
            let tops = sample_mesa_tops(&terrain, 240);
            let elevation: Vec<f32> = tops.iter().map(|top| terrain.horizon - top).collect();
            let relief = elevation.iter().copied().fold(0.0_f32, f32::max);
            let threshold = relief * 0.35;
            let components = count_elevated_components(&elevation, threshold);
            assert!(
                (2..=3).contains(&components),
                "seed {seed}: expected 2-3 mesa components, got {components} (relief {relief})"
            );
        }
    }

    #[test]
    fn mesa_profile_avoids_single_column_drops() {
        for seed in [1_u64, 7, 42, 99, 2026] {
            let terrain = mesa_test_terrain(seed);
            let tops = sample_mesa_tops(&terrain, 240);
            let relief = terrain.horizon - tops.iter().copied().fold(f32::INFINITY, f32::min);
            let max_drop = max_adjacent_drop(&tops);
            assert!(
                max_drop < relief * 0.80,
                "seed {seed}: column drop {max_drop} too large for relief {relief}"
            );
        }
    }

    #[test]
    fn badlands_profile_has_multiple_separated_ridge_peaks() {
        for seed in [2_u64, 11, 37, 88, 2026] {
            let terrain = badlands_test_terrain(seed);
            let crests = sample_badlands_crests(&terrain, 240);
            let peaks = count_ridge_peaks(&crests);
            assert!(
                peaks >= 3,
                "seed {seed}: expected multiple ridge peaks, got {peaks}"
            );
        }
    }

    #[test]
    fn badlands_profile_carries_material_relief() {
        for seed in [2_u64, 11, 37, 88, 2026] {
            let terrain = badlands_test_terrain(seed);
            let crests = sample_badlands_crests(&terrain, 240);
            let relief = terrain.horizon - crests.iter().copied().fold(f32::INFINITY, f32::min);
            assert!(
                (0.10..=0.26).contains(&relief),
                "seed {seed}: relief {relief} outside expected badlands range"
            );
        }
    }

    fn sample_mesa_relief(terrain: &Terrain, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| {
                let fx = index as f32 * terrain.width / samples as f32;
                terrain.horizon - mesa_profile(fx, terrain).0
            })
            .collect()
    }

    fn sample_badlands_ridge_body(terrain: &Terrain, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| {
                let u = index as f32 / samples as f32;
                badlands_ridge_body(u, terrain)
            })
            .collect()
    }

    #[test]
    fn fresh_profiles_mesa_reaches_both_outer_bands() {
        let mut rng = StdRng::seed_from_u64(7);
        let terrain = Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::Mesa));
        let relief = sample_mesa_relief(&terrain, 440);
        let max_relief = relief.iter().copied().fold(0.0_f32, f32::max);
        let left_band = &relief[..55];
        let right_band = &relief[385..];
        let left_peak = left_band.iter().copied().fold(0.0_f32, f32::max);
        let right_peak = right_band.iter().copied().fold(0.0_f32, f32::max);
        let threshold = max_relief * 0.40;
        assert!(
            left_peak >= threshold,
            "seed 7: left outer band peak {left_peak} below {threshold} (max {max_relief})"
        );
        assert!(
            right_peak >= threshold,
            "seed 7: right outer band peak {right_peak} below {threshold} (max {max_relief})"
        );
    }

    #[test]
    fn fresh_profiles_badlands_stays_continuous() {
        let mut rng = StdRng::seed_from_u64(7);
        let terrain = Terrain::generate(&mut rng, 440, 300, Some(TerrainKind::Badlands));
        let body = sample_badlands_ridge_body(&terrain, 440);
        let peak = body.iter().copied().fold(0.0_f32, f32::max);
        let floor = peak * 0.18;
        let min_body = body.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            min_body >= floor,
            "seed 7: ridge body dipped to {min_body}, floor {floor} (peak {peak})"
        );
    }
}

//! Procedural sky backdrops: cloud decks, storms, lightning, and twilight.
//!
//! The fourth backdrop family, alongside the gradients, the deep-space scenes,
//! and the geometric patterns. It mirrors the space and pattern models exactly:
//! a sky palette carries the colors and a [`SkyKind`] carries the structure, so
//! `--palette` and `--sky` compose the same way `--palette` and `--scene` do.
//!
//! Everything is drawn from the style seed at render time. Nothing is a stored
//! photograph, so a sky resolves cleanly at any card size and the same
//! `--style-seed` reproduces one exactly.
//!
//! Palette colors are sampled from photographs of the named conditions the way
//! the space palettes are sampled from astrophotography: supercell navy and
//! underlit anvil gold, mammatus gray-violet, blue-hour navy, aurora teal.
//!
//! All drawing is hand-rolled on `image` + `rand` (no new dependencies, per
//! repo policy). Sky reads as a vertical problem: almost every condition is a
//! banded ramp plus one structural motif plus grain, so the shared base layer
//! does the ramp and each kind contributes only its own structure.

use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::polish::PresentationStyle;

/// Decorrelates the sky RNG from the style RNG, which already consumed the raw
/// seed in `style_from_seed`.
const SKY_SEED_SALT: u64 = 0x534B_5920_4C41_5952; // "SKY LAYR"

/// Matches the gradient, space, and pattern backdrops so film feel stays
/// consistent across all four families.
const GRAIN_STRENGTH: f32 = 2.4;

/// Cloud structure is fbm; five octaves is where the turret detail stops
/// reading as noise and starts reading as cloud.
const CLOUD_OCTAVES: u32 = 5;

/// A specific sky the caller can pin instead of the seed's random pick. The
/// seed still drives every free parameter (coverage, placement, bolt geometry,
/// curtain phase); this only forces which condition appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyKind {
    /// Smooth twilight ramp, a few high bars, first stars near the zenith.
    BlueHour,
    /// Cloud mass with the sun low and behind it, lighting only the edges.
    GoldenHour,
    /// Featureless stratus deck; the quietest sky in the set.
    Overcast,
    /// High wispy ice streaks on open blue.
    Cirrus,
    /// Cirrocumulus cell rows shrinking toward the horizon: a mackerel sky.
    Mackerel,
    /// Pouches hanging from a storm ceiling, lit from above.
    Mammatus,
    /// Heavy base with rain curtains falling out of it.
    Storm,
    /// Storm base with branching cloud-to-ground lightning.
    Bolt,
    /// Auroral curtains over a star field.
    Aurora,
    /// Sun beams fanning through a break in the deck.
    Crepuscular,
}

impl SkyKind {
    /// All names accepted by [`SkyKind::from_name`], in menu order.
    pub const NAMES: [&'static str; 10] = [
        "blue-hour",
        "golden-hour",
        "overcast",
        "cirrus",
        "mackerel",
        "mammatus",
        "storm",
        "bolt",
        "aurora",
        "crepuscular",
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "blue-hour" => Self::BlueHour,
            "golden-hour" => Self::GoldenHour,
            "overcast" => Self::Overcast,
            "cirrus" => Self::Cirrus,
            "mackerel" => Self::Mackerel,
            "mammatus" => Self::Mammatus,
            "storm" => Self::Storm,
            "bolt" => Self::Bolt,
            "aurora" => Self::Aurora,
            "crepuscular" => Self::Crepuscular,
            _ => return None,
        })
    }
}

/// Everything the renderer needs, rolled once so the per-pixel loop stays a
/// pure function of the scene.
struct Sky {
    kind: SkyKind,
    noise_seed: u64,
    /// Vertical position of the cloud base or curtain foot, 0 at the top.
    horizon: f32,
    /// Broad cloud coverage, 0 clear to 1 solid.
    coverage: f32,
    /// Feature scale multiplier; small means bigger cloud masses.
    scale: f32,
    /// Where the light comes from, in canvas fractions. Off-frame is fine and
    /// usually better: it puts the flare outside the capture window.
    light: (f32, f32),
    /// Lightning channels, empty for every kind but [`SkyKind::Bolt`].
    bolts: Vec<Bolt>,
    /// Star positions and brightness, empty unless the kind shows stars.
    stars: Vec<Star>,
}

struct Star {
    x: f32,
    y: f32,
    brightness: f32,
}

/// One lightning channel, already walked into a polyline so the draw pass does
/// no branching logic of its own.
struct Bolt {
    /// Every segment as `(x0, y0, x1, y1, intensity)`, canvas pixels.
    segments: Vec<(f32, f32, f32, f32, f32)>,
}

pub fn render(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    let mut rng = StdRng::seed_from_u64(style.seed ^ SKY_SEED_SALT);
    let sky = Sky::generate(&mut rng, width, height, style.sky);
    let mut canvas = base_layer(width, height, style, &sky);
    draw_stars(&mut canvas, &sky.stars);
    for bolt in &sky.bolts {
        draw_bolt(&mut canvas, bolt, style.glow_a);
    }
    canvas
}

impl Sky {
    fn generate(rng: &mut StdRng, width: u32, height: u32, pinned: Option<SkyKind>) -> Sky {
        let kind = pinned.unwrap_or_else(|| {
            let index = rng.random_range(0..SkyKind::NAMES.len());
            SkyKind::from_name(SkyKind::NAMES[index]).unwrap_or(SkyKind::BlueHour)
        });
        let noise_seed = rng.random();

        // Each condition wants its own coverage and feature scale; rolling
        // inside the match keeps the per-kind tuning in one readable place.
        let (horizon, coverage, scale) = match kind {
            SkyKind::BlueHour => (rng.random_range(0.62..=0.78), 0.18, 3.0),
            SkyKind::GoldenHour => (rng.random_range(0.55..=0.72), 0.62, 2.4),
            SkyKind::Overcast => (rng.random_range(0.50..=0.70), 0.88, 1.8),
            SkyKind::Cirrus => (rng.random_range(0.60..=0.80), 0.30, 2.2),
            SkyKind::Mackerel => (rng.random_range(0.58..=0.76), 0.55, 9.0),
            SkyKind::Mammatus => (rng.random_range(0.42..=0.58), 0.80, 6.5),
            SkyKind::Storm => (rng.random_range(0.46..=0.62), 0.85, 2.2),
            SkyKind::Bolt => (rng.random_range(0.40..=0.55), 0.88, 2.2),
            SkyKind::Aurora => (rng.random_range(0.68..=0.86), 0.10, 3.2),
            SkyKind::Crepuscular => (rng.random_range(0.55..=0.72), 0.58, 2.4),
        };

        // The capture window covers the middle of the canvas, so the sun is
        // pushed toward a corner and usually just off-frame.
        let light = match kind {
            // Kept well above the top edge: a sun inside the frame renders as
            // a symmetrical starburst, which reads as a logo, not weather.
            SkyKind::Crepuscular => (
                rng.random_range(0.10..=0.90),
                rng.random_range(-0.45..=-0.12),
            ),
            _ => (rng.random_range(0.05..=0.95), rng.random_range(0.60..=0.95)),
        };

        let stars = match kind {
            SkyKind::BlueHour => generate_stars(rng, width, height, 70, horizon * 0.8),
            SkyKind::Aurora => generate_stars(rng, width, height, 220, 1.0),
            _ => Vec::new(),
        };

        let bolts = if matches!(kind, SkyKind::Bolt) {
            let count = rng.random_range(1..=3);
            (0..count)
                .map(|_| generate_bolt(rng, width, height, horizon))
                .collect()
        } else {
            Vec::new()
        };

        Sky {
            kind,
            noise_seed,
            horizon,
            coverage,
            scale,
            light,
            bolts,
            stars,
        }
    }
}

fn generate_stars(
    rng: &mut StdRng,
    width: u32,
    height: u32,
    count: usize,
    max_depth: f32,
) -> Vec<Star> {
    (0..count)
        .map(|_| Star {
            x: rng.random_range(0.0..=width as f32),
            // Stars thin out toward the horizon, so bias the draw upward.
            y: rng.random_range(0.0..=(height as f32 * max_depth).max(1.0)),
            brightness: rng.random_range(0.25..=1.0),
        })
        .collect()
}

/// Walk a lightning channel from the cloud base toward the ground, forking as
/// it goes. Real bolts vary enormously in how much they branch, so the branch
/// budget is rolled per bolt rather than fixed.
fn generate_bolt(rng: &mut StdRng, width: u32, height: u32, horizon: f32) -> Bolt {
    let w = width as f32;
    let h = height as f32;
    let mut segments = Vec::new();
    let start_x = rng.random_range(w * 0.1..=w * 0.9);
    let start_y = h * horizon * rng.random_range(0.75..=1.0);
    let ground = h * rng.random_range(0.90..=1.0);
    let branch_budget = rng.random_range(0..=9);

    walk_channel(
        rng,
        &mut segments,
        (start_x, start_y),
        ground,
        w,
        1.0,
        branch_budget,
        0,
    );
    Bolt { segments }
}

/// One channel and its forks. `depth` guards the recursion; three levels is as
/// deep as a bolt reads at card sizes.
#[allow(clippy::too_many_arguments)]
fn walk_channel(
    rng: &mut StdRng,
    segments: &mut Vec<(f32, f32, f32, f32, f32)>,
    start: (f32, f32),
    ground: f32,
    width: f32,
    intensity: f32,
    branch_budget: i32,
    depth: u32,
) {
    let (mut x, mut y) = start;
    let mut budget = branch_budget;
    // Segment length scales with the drop so a short branch is not chopped
    // into more pieces than the main channel.
    let step = ((ground - y) / 14.0).max(3.0);

    while y < ground {
        // Deviation per segment sits near 15 to 30 degrees off vertical in the
        // reference frames, with the occasional hard kink.
        let lateral = if rng.random_bool(0.12) {
            rng.random_range(-1.10..=1.10)
        } else {
            rng.random_range(-0.45..=0.45)
        };
        let next_x = (x + lateral * step).clamp(0.0, width);
        let next_y = y + step * rng.random_range(0.7..=1.3);
        segments.push((x, y, next_x, next_y, intensity));

        // Forks head outward and down, taper hard, and never fork back up.
        if depth < 3 && budget > 0 && rng.random_bool(0.22) {
            budget -= 1;
            let spread =
                rng.random_range(0.6..=1.6) * if rng.random_bool(0.5) { -1.0 } else { 1.0 };
            let branch_end = ground - (ground - next_y) * rng.random_range(0.25..=0.75);
            // Rolled before the call: `rng` cannot be borrowed twice in one
            // argument list.
            let branch_intensity = intensity * rng.random_range(0.35..=0.6);
            walk_channel(
                rng,
                segments,
                (next_x, next_y),
                branch_end.max(next_y + step),
                width,
                branch_intensity,
                budget.min(2),
                depth + 1,
            );
            // Nudge the main channel away from the fork it just shed.
            x = (next_x + spread * step * 0.3).clamp(0.0, width);
            y = next_y;
            continue;
        }

        x = next_x;
        y = next_y;
    }
}

/// Paint the ramp and whichever structure the kind calls for.
fn base_layer(width: u32, height: u32, style: &PresentationStyle, sky: &Sky) -> RgbaImage {
    let zenith = to_f32(style.stops[0]);
    let middle = to_f32(style.stops[1]);
    let horizon_color = to_f32(style.stops[2]);
    let light = to_f32(style.glow_a);
    let shade = to_f32(style.glow_b);

    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    // `scale` is how many pixels one noise unit spans, derived from the short
    // edge so a wide card does not stretch cloud structure into streaks.
    // `sky.scale` reads as "features across the short edge", so a larger value
    // means smaller structure. Getting this backwards is what turns a cloud
    // deck into speckle: a mass needs to span a good fraction of the canvas,
    // not a handful of pixels.
    let scale = (w.min(h).max(1.0) / sky.scale.max(0.1)).max(1.0);

    ImageBuffer::from_fn(width.max(1), height.max(1), |px, py| {
        let fx = px as f32;
        let fy = py as f32;
        let u = fx / w;
        let v = fy / h;

        // Two-segment vertical ramp: zenith to middle above the horizon,
        // middle to horizon colour below it.
        let mut color = if v < sky.horizon {
            let t = smoothstep((v / sky.horizon.max(0.001)).clamp(0.0, 1.0));
            mix3(zenith, middle, t)
        } else {
            let t =
                smoothstep(((v - sky.horizon) / (1.0 - sky.horizon).max(0.001)).clamp(0.0, 1.0));
            mix3(middle, horizon_color, t)
        };

        color = apply_structure(color, sky, u, v, fx, fy, scale, light, shade);

        let grain = grain_noise(px, py, sky.noise_seed) * GRAIN_STRENGTH;
        Rgba([
            quantize(color[0] + grain),
            quantize(color[1] + grain),
            quantize(color[2] + grain),
            255,
        ])
    })
}

/// The per-kind structure pass. Split out so `base_layer` stays a ramp plus a
/// grain, which is what every sky has in common.
#[allow(clippy::too_many_arguments)]
fn apply_structure(
    base: [f32; 3],
    sky: &Sky,
    u: f32,
    v: f32,
    fx: f32,
    fy: f32,
    scale: f32,
    light: [f32; 3],
    shade: [f32; 3],
) -> [f32; 3] {
    let seed = sky.noise_seed;
    match sky.kind {
        SkyKind::BlueHour => {
            // Thin horizontal bars low down, nothing else. The ramp is the star.
            let bar = fbm(fx / (scale * 1.6), fy / (scale * 0.18), seed, 3);
            let band = ((v - sky.horizon * 0.75) * 3.0).clamp(0.0, 1.0);
            let amount = smoothstep_range(bar, 0.52, 0.68) * 0.4 * band;
            mix3(base, shade, amount)
        }
        SkyKind::GoldenHour => {
            let density = cloud_density(fx, fy, scale, seed, sky.coverage);
            if density <= 0.0 {
                return base;
            }
            // Rim light: sample the density a short step toward the sun.
            // Thinner that way means this pixel sits on the lit edge of the
            // mass. The step is a fraction of the feature size, not a fixed
            // pixel count, so the rim stays proportional at any card size.
            let (lx, ly) = sky.light;
            let dx = lx - u;
            let dy = ly - v;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let step = scale * 0.30;
            let toward = cloud_density(
                fx + dx / len * step,
                fy + dy / len * step,
                scale,
                seed,
                sky.coverage,
            );
            let rim = (density - toward).max(0.0);
            let body = mix3(base, shade, (density * 0.70).min(0.85));
            mix3(body, light, (rim * 1.8).min(0.85))
        }
        SkyKind::Overcast => {
            let density = warped_fbm(fx / scale, fy / scale, seed, 4);
            mix3(base, shade, smoothstep_range(density, 0.30, 0.72) * 0.5)
        }
        SkyKind::Cirrus => {
            // Ice streaks: stretch the noise hard along x and keep it high.
            let streak = fbm(fx / (scale * 2.6), fy / (scale * 0.30), seed, 4);
            let height_falloff = (1.0 - (v / sky.horizon.max(0.001))).clamp(0.0, 1.0);
            let amount = smoothstep_range(streak, 0.48, 0.70) * 0.75 * height_falloff;
            mix3(base, light, amount)
        }
        SkyKind::Mackerel => {
            // Cells shrink toward the horizon, which is what sells the
            // perspective. Guard the divisor so the horizon row cannot blow up.
            // Perspective is kept mild on purpose. Driving the cell size to
            // near zero at the horizon aliases the sine lattice into a fisheye
            // fan, which looks like a lens artifact rather than a sky.
            let depth = ((sky.horizon - v) / sky.horizon.max(0.001)).clamp(0.0, 1.0);
            let cell = scale * (0.55 + depth * 0.45);
            // The jitter has to vary at roughly cell scale. A smooth
            // low-frequency field only shifts the whole lattice's phase, which
            // leaves it looking like halftone dots.
            let jitter = (fbm(fx / (scale * 0.7), fy / (scale * 0.7), seed, 4) - 0.5) * 6.0;
            // One cell must be one full sine period. Dividing by the cell size
            // alone leaves a period of `TAU * cell`, which is why the cloudlets
            // came out several times too big.
            let tau = std::f32::consts::TAU;
            let rows = ((fy / cell.max(1.0)) * tau + jitter).sin();
            let cols = ((fx / cell.max(1.0)) * tau + jitter * 1.4).sin();
            let cellular = (rows * cols).max(0.0);
            // A bare sin*sin product is a perfect lattice and reads as halftone
            // rather than cloud. Patchiness gates whole groups of cells in and
            // out so the rows break up the way real cirrocumulus does.
            let patchiness = warped_fbm(fx / (scale * 5.0), fy / (scale * 5.0), seed ^ 0x2C7A, 3);
            let mask = smoothstep_range(patchiness, 0.34, 0.64);
            // Fade out at the horizon rather than cutting, which would leave a
            // hard seam straight across the card.
            let band = 1.0 - smoothstep_range(v, sky.horizon - 0.10, sky.horizon);
            let amount = smoothstep_range(cellular, 0.10, 0.70) * mask * band * 0.6;
            mix3(base, light, amount)
        }
        SkyKind::Mammatus => {
            // A field of hanging hemispheres. The lobe grid is jittered so the
            // spacing reads irregular, and each lobe is shaded from above,
            // which is the whole reason mammatus looks like mammatus.
            let ceiling = 1.0 - smoothstep_range(v, sky.horizon - 0.28, sky.horizon + 0.05);
            if ceiling <= 0.0 {
                return base;
            }
            let lobe = scale;
            let gx = fx / lobe.max(1.0);
            let gy = fy / lobe.max(1.0);
            let base_cx = gx.floor();
            let base_cy = gy.floor();
            // A lobe wider than its own cell used to get clipped at the cell
            // border, which turned it into a rounded square. Walk the
            // neighbours too and keep whichever lobe covers this pixel best.
            let mut dome = 0.0_f32;
            let mut lit = 0.0_f32;
            for oy in -1..=1 {
                for ox in -1..=1 {
                    let cx = base_cx + ox as f32;
                    let cy = base_cy + oy as f32;
                    let (ix, iy) = (cx as i64, cy as i64);
                    // Jitter has to be centred and wide, or the lobes land on
                    // a visible lattice and read as polka dots.
                    let jx = (cell_hash(ix, iy, seed) - 0.5) * 0.7;
                    let jy = (cell_hash(ix, iy, seed ^ 0x9E37) - 0.5) * 0.7;
                    let dx = gx - cx - 0.5 - jx;
                    let dy = gy - cy - 0.5 - jy;
                    let r = (dx * dx + dy * dy).sqrt();
                    // Lobes vary a lot in size, and the bigger ones overlap.
                    let radius = 0.30 + cell_hash(ix, iy, seed ^ 0x51ED) * 0.38;
                    if r > radius {
                        continue;
                    }
                    let this_dome = (1.0 - (r / radius).powi(2)).max(0.0).sqrt();
                    if this_dome > dome {
                        dome = this_dome;
                        // Shade by vertical position within the lobe: bright
                        // on top, dark underneath. Contrast stays low.
                        lit = (0.5 - dy / radius * 0.5).clamp(0.0, 1.0);
                    }
                }
            }
            if dome <= 0.0 {
                return base;
            }
            let body = mix3(base, shade, (dome * 0.55 * ceiling).min(0.6));
            mix3(body, light, (dome * lit * 0.30 * ceiling).min(0.4))
        }
        SkyKind::Storm => structure_storm(base, sky, v, fx, fy, scale, shade),
        SkyKind::Bolt => structure_storm(base, sky, v, fx, fy, scale, shade),
        SkyKind::Aurora => {
            // Curtains: a horizontal intensity walk, a soft top, a ragged foot,
            // and vertical striations inside the band.
            let walk = fbm(fx / (scale * 1.4), 0.0, seed, 4);
            let curtain = smoothstep_range(walk, 0.40, 0.60);
            if curtain <= 0.0 {
                return base;
            }
            let foot = sky.horizon * (0.72 + walk * 0.3);
            let top = foot * 0.18;
            let vertical = if v < top {
                (v / top.max(0.001)).clamp(0.0, 1.0)
            } else if v < foot {
                1.0 - ((v - top) / (foot - top).max(0.001)).clamp(0.0, 1.0) * 0.35
            } else {
                0.0
            };
            let striation = 0.75 + fbm(fx / (scale * 0.16), fy / (scale * 4.0), seed, 3) * 0.5;
            let amount = (curtain * vertical * striation * 0.85).clamp(0.0, 1.0);
            mix3(base, light, amount)
        }
        SkyKind::Crepuscular => {
            // Beams fan from an off-frame sun. Working in angle space makes
            // them straight and convergent for free.
            let (lx, ly) = sky.light;
            let dx = u - lx;
            let dy = v - ly;
            let angle = dy.atan2(dx);
            let distance = (dx * dx + dy * dy).sqrt();
            let beam = ((angle * 9.0).sin() * 0.5 + 0.5).powi(2);
            let ray_noise = fbm(angle * 5.0, 0.0, seed, 3);
            let falloff = (1.0 - distance * 0.9).clamp(0.0, 1.0);
            let deck = cloud_density(fx, fy, scale, seed, sky.coverage);
            let body = mix3(base, shade, (deck * 0.5).min(0.6));
            mix3(body, light, (beam * ray_noise * falloff * 0.55).min(0.6))
        }
    }
}

/// Shared between [`SkyKind::Storm`] and [`SkyKind::Bolt`]: a heavy deck with
/// rain curtains under it. Bolt adds channels on top in a later pass.
fn structure_storm(
    base: [f32; 3],
    sky: &Sky,
    v: f32,
    fx: f32,
    fy: f32,
    scale: f32,
    shade: [f32; 3],
) -> [f32; 3] {
    let seed = sky.noise_seed;
    let density = warped_fbm(fx / scale, fy / (scale * 1.4), seed, CLOUD_OCTAVES);
    // Coverage has to reach the deck, or a storm leaves clear patches that
    // expose the palette's horizon stop as a flat pale block.
    let edge = 0.62 - sky.coverage * 0.34;
    let above = smoothstep_range(density, edge - 0.10, edge + 0.20);
    // Rain falls in soft vertical curtains out of the base.
    let curtain = fbm(fx / (scale * 0.8), fy / (scale * 4.0), seed ^ 0x7A11, 3);
    // Rain shafts under a storm base are close to uniformly dark; the curtain
    // noise is texture on top, not the difference between dark and clear. A
    // low floor here lets the palette's light horizon stop flood the lower
    // half of the card.
    let below = (0.52 + smoothstep_range(curtain, 0.38, 0.68) * 0.34).min(1.0);
    // Cross-fade the two rather than switching at the horizon, which would
    // draw a hard line straight across the card.
    let blend = smoothstep_range(v, sky.horizon - 0.10, sky.horizon + 0.04);
    let deck = above + (below - above) * blend;
    // Capped short of full shade: storm palettes put near-black in `glow_b`, so
    // mixing all the way crushes the whole card to one flat dark tone.
    mix3(base, shade, (deck * 0.74).clamp(0.0, 0.80))
}

/// Cloud coverage as a 0..1 mass. Coverage shifts the threshold rather than
/// scaling the result, so a thin sky keeps hard-edged cloud instead of turning
/// into a uniform haze.
/// Domain-warped fbm. Plain value noise is built on a grid, and at the low
/// frequencies a cloud mass needs (only a few cells across the whole canvas)
/// that grid shows through as visible interpolated quadrilaterals. Offsetting
/// the sample point by a second noise field breaks the alignment.
fn warped_fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let wx = fbm(x * 0.5, y * 0.5, seed ^ 0x5741_5250, 3) - 0.5;
    let wy = fbm(x * 0.5 + 5.2, y * 0.5 + 1.3, seed ^ 0x5750_5259, 3) - 0.5;
    fbm(x + wx * 1.8, y + wy * 1.8, seed, octaves)
}

fn cloud_density(fx: f32, fy: f32, scale: f32, seed: u64, coverage: f32) -> f32 {
    let raw = warped_fbm(fx / scale, fy / (scale * 0.85), seed, CLOUD_OCTAVES);
    // Coverage slides the edge rather than scaling the result, so a thin sky
    // keeps hard-edged cloud instead of turning into uniform haze. The band
    // around the edge is what keeps the mass soft instead of speckled.
    let edge = 0.62 - coverage * 0.28;
    smoothstep_range(raw, edge - 0.10, edge + 0.14)
}

fn draw_stars(canvas: &mut RgbaImage, stars: &[Star]) {
    for star in stars {
        let x = star.x as i32;
        let y = star.y as i32;
        add_light(canvas, x, y, [255.0, 255.0, 255.0], star.brightness);
        // A single lit pixel disappears at card scale, so bleed the bright
        // ones into their neighbours.
        if star.brightness > 0.7 {
            let halo = star.brightness * 0.3;
            add_light(canvas, x + 1, y, [255.0, 255.0, 255.0], halo);
            add_light(canvas, x - 1, y, [255.0, 255.0, 255.0], halo);
            add_light(canvas, x, y + 1, [255.0, 255.0, 255.0], halo);
            add_light(canvas, x, y - 1, [255.0, 255.0, 255.0], halo);
        }
    }
}

/// Stamp the channels. The core runs near white and the glow takes the
/// palette's light colour, which is what keeps a bolt tied to its palette
/// instead of looking pasted on.
fn draw_bolt(canvas: &mut RgbaImage, bolt: &Bolt, glow: [u8; 3]) {
    let glow = to_f32(glow);
    for &(x0, y0, x1, y1, intensity) in &bolt.segments {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = (dx.abs().max(dy.abs()).ceil() as i32).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = (x0 + dx * t) as i32;
            let y = (y0 + dy * t) as i32;
            add_light(canvas, x, y, [255.0, 255.0, 255.0], intensity);
            // Glow falls off either side of the channel.
            for offset in 1..=3 {
                let amount = intensity * 0.34 / offset as f32;
                add_light(canvas, x + offset, y, glow, amount);
                add_light(canvas, x - offset, y, glow, amount);
                add_light(canvas, x, y + offset, glow, amount * 0.6);
                add_light(canvas, x, y - offset, glow, amount * 0.6);
            }
        }
    }
}

fn add_light(canvas: &mut RgbaImage, x: i32, y: i32, color: [f32; 3], amount: f32) {
    if amount <= 0.0 {
        return;
    }
    if let Some(pixel) = pixel_mut(canvas, x, y) {
        for channel in 0..3 {
            let current = pixel[channel] as f32;
            pixel[channel] = quantize(current + (color[channel] - current) * amount.min(1.0));
        }
    }
}

fn pixel_mut(canvas: &mut RgbaImage, x: i32, y: i32) -> Option<&mut Rgba<u8>> {
    if x < 0 || y < 0 {
        return None;
    }
    let (width, height) = canvas.dimensions();
    let (x, y) = (x as u32, y as u32);
    if x >= width || y >= height {
        return None;
    }
    Some(canvas.get_pixel_mut(x, y))
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
/// cloud and a field of speckles, so almost every kind uses this instead of a
/// bare comparison.
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

    fn sky_style(seed: u64, kind: &str) -> PresentationStyle {
        let mut style = polish::style_with_palette(seed, "blue-hour").expect("sky palette");
        style.sky = SkyKind::from_name(kind);
        style
    }

    #[test]
    fn every_name_round_trips() {
        for name in SkyKind::NAMES {
            assert!(SkyKind::from_name(name).is_some(), "{name} did not parse");
        }
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert_eq!(SkyKind::from_name("mackeral"), None);
        assert_eq!(SkyKind::from_name(""), None);
    }

    #[test]
    fn names_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for name in SkyKind::NAMES {
            assert!(!seen.contains(&name), "duplicate name {name}");
            seen.push(name);
        }
    }

    #[test]
    fn same_seed_renders_identically() {
        for name in SkyKind::NAMES {
            let style = sky_style(7, name);
            let a = render(96, 72, &style);
            let b = render(96, 72, &style);
            assert_eq!(a.as_raw(), b.as_raw(), "{name} was not reproducible");
        }
    }

    #[test]
    fn every_kind_is_opaque_everywhere() {
        for name in SkyKind::NAMES {
            let style = sky_style(3, name);
            let canvas = render(80, 60, &style);
            assert!(
                canvas.pixels().all(|pixel| pixel[3] == 255),
                "{name} left a transparent pixel"
            );
        }
    }

    #[test]
    fn every_kind_varies_across_the_canvas() {
        // A sky that renders one flat colour is a bug, not a style.
        for name in SkyKind::NAMES {
            let style = sky_style(11, name);
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
        let base = render(64, 48, &sky_style(5, "blue-hour"));
        for name in SkyKind::NAMES.iter().skip(1) {
            let other = render(64, 48, &sky_style(5, name));
            assert_ne!(base.as_raw(), other.as_raw(), "{name} matched blue-hour");
        }
    }

    #[test]
    fn survives_a_zero_dimension() {
        let style = sky_style(1, "storm");
        assert_eq!(render(0, 0, &style).dimensions(), (1, 1));
        assert_eq!(render(0, 40, &style).dimensions(), (1, 40));
        assert_eq!(render(40, 0, &style).dimensions(), (40, 1));
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
                let value = fbm(x as f32 * 0.5, y as f32 * 0.5, 7, CLOUD_OCTAVES);
                assert!((0.0..=1.0).contains(&value), "{value} out of range");
            }
        }
    }

    #[test]
    fn a_bolt_reaches_the_ground() {
        let mut rng = StdRng::seed_from_u64(4);
        let bolt = generate_bolt(&mut rng, 400, 300, 0.45);
        let lowest = bolt
            .segments
            .iter()
            .map(|segment| segment.3)
            .fold(0.0_f32, f32::max);
        assert!(lowest > 300.0 * 0.85, "bolt stopped at {lowest}");
    }

    #[test]
    fn bolt_segments_stay_on_canvas_horizontally() {
        let mut rng = StdRng::seed_from_u64(12);
        let bolt = generate_bolt(&mut rng, 200, 160, 0.5);
        for segment in &bolt.segments {
            assert!(
                (0.0..=200.0).contains(&segment.0) && (0.0..=200.0).contains(&segment.2),
                "segment left the canvas: {segment:?}"
            );
        }
    }

    #[test]
    fn bolt_renders_brighter_than_plain_storm() {
        // The channels must actually add light, not just exist in the struct.
        let storm: u64 = render(120, 90, &sky_style(21, "storm"))
            .pixels()
            .map(|pixel| pixel[0] as u64 + pixel[1] as u64 + pixel[2] as u64)
            .sum();
        let bolt: u64 = render(120, 90, &sky_style(21, "bolt"))
            .pixels()
            .map(|pixel| pixel[0] as u64 + pixel[1] as u64 + pixel[2] as u64)
            .sum();
        assert!(
            bolt > storm,
            "bolt {bolt} was not brighter than storm {storm}"
        );
    }

    #[test]
    fn a_pinned_kind_is_honoured() {
        let mut rng = StdRng::seed_from_u64(2);
        let sky = Sky::generate(&mut rng, 100, 100, Some(SkyKind::Mammatus));
        assert_eq!(sky.kind, SkyKind::Mammatus);
    }

    #[test]
    fn an_unpinned_kind_still_picks_something() {
        for seed in 0..24 {
            let mut rng = StdRng::seed_from_u64(seed);
            let sky = Sky::generate(&mut rng, 64, 64, None);
            assert!(SkyKind::NAMES.contains(&kind_name(sky.kind)));
        }
    }

    fn kind_name(kind: SkyKind) -> &'static str {
        SkyKind::NAMES
            .iter()
            .copied()
            .find(|name| SkyKind::from_name(name) == Some(kind))
            .expect("every kind has a name")
    }
}

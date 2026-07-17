//! Procedural deep-space backdrops for shot-cards, nebula-first: domain-warped
//! fbm gas with ridge highlights and dust lanes, layered spiked starfields,
//! ionization cores with embedded star clusters, star-forming knots, galaxy
//! smudges, occasional suns, and rare ultra-deep-field seeds. Everything
//! derives from the style seed so `--style-seed` reproduces the exact scene,
//! and the whole canvas is opaque like the gradient backdrop.
//!
//! All drawing is hand-rolled on `image` + `rand` (no new dependencies, per
//! repo policy). Focal features anchor to corners and edges because the
//! capture window covers the center of the canvas; only the padding band
//! shows.

use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::polish::PresentationStyle;

/// Decorrelates the scene RNG from the style RNG, which already consumed the
/// raw seed in `style_from_seed`.
const SCENE_SEED_SALT: u64 = 0x5354_4152_4649_454C; // "STARFIEL"

const NEBULA_OCTAVES: u32 = 5;
/// Grain matches the gradient backdrop's strength so film feel is consistent.
const GRAIN_STRENGTH: f32 = 2.4;

pub fn render(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    let mut rng = StdRng::seed_from_u64(style.seed ^ SCENE_SEED_SALT);
    let scene = Scene::generate(&mut rng, width, height);
    let mut canvas = base_layer(width, height, style, &scene);
    if let Some(hero) = &scene.hero {
        draw_hero_galaxy(&mut canvas, hero);
    }
    for galaxy in &scene.galaxies {
        draw_galaxy(&mut canvas, galaxy);
    }
    for &(x, y, radius) in &scene.knots {
        draw_knot(&mut canvas, x, y, radius);
    }
    if let Some((x, y, radius)) = scene.ring_nebula {
        draw_ring_nebula(&mut canvas, x, y, radius, scene.noise_seed);
    }
    draw_stars(&mut canvas, &scene.stars);
    if let Some(sun) = &scene.sun {
        draw_sun(&mut canvas, sun);
    }
    canvas
}

/// Ring-Nebula-style planetary nebula: blue interior haze, teal-white middle
/// shell, and a clumpy orange rim whose radius wobbles with noise so the
/// donut reads organic instead of drafted.
fn draw_ring_nebula(canvas: &mut RgbaImage, x: f32, y: f32, radius: f32, seed: u64) {
    let reach = (radius * 1.5).ceil() as i32;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let raw = ((dx * dx + dy * dy) as f32).sqrt() / radius;
            if raw > 1.45 {
                continue;
            }
            let angle = (dy as f32).atan2(dx as f32);
            // Wobble the shell radius around the ring.
            let wobble = fbm(angle.cos() * 2.1 + 7.7, angle.sin() * 2.1 + 3.3, seed, 3) - 0.5;
            let distance = raw * (1.0 + wobble * 0.18);
            let px = x as i32 + dx;
            let py = y as i32 + dy;
            // Blue interior glow.
            let interior = (1.0 - (distance / 0.72).powi(2)).clamp(0.0, 1.0);
            add_light(canvas, px, py, [76.0, 128.0, 224.0], interior * 0.5);
            // Teal-white middle shell.
            let middle = (-((distance - 0.78) / 0.14).powi(2)).exp();
            add_light(canvas, px, py, [168.0, 232.0, 222.0], middle * 0.55);
            // Clumpy orange rim.
            let rim = (-((distance - 1.0) / 0.13).powi(2)).exp();
            let clump = fbm(
                dx as f32 / radius * 4.0,
                dy as f32 / radius * 4.0,
                seed ^ 0xF00D,
                3,
            );
            add_light(
                canvas,
                px,
                py,
                [255.0, 138.0, 58.0],
                rim * (0.45 + 0.55 * clump) * 0.8,
            );
        }
    }
}

/// Star-forming knot: a small saturated pink clump with a hot center, like
/// the star-birth regions strung along the Antennae galaxies.
fn draw_knot(canvas: &mut RgbaImage, x: f32, y: f32, radius: f32) {
    let reach = (radius * 2.4).ceil() as i32;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let distance = ((dx * dx + dy * dy) as f32).sqrt() / radius;
            if distance > 2.4 {
                continue;
            }
            let px = x as i32 + dx;
            let py = y as i32 + dy;
            let halo = (-(distance * 1.3).powi(2)).exp();
            let hot = (-(distance * 3.0).powi(2)).exp();
            add_light(canvas, px, py, [255.0, 130.0, 190.0], halo * 0.55);
            add_light(canvas, px, py, [255.0, 232.0, 240.0], hot * 0.6);
        }
    }
}

struct Scene {
    noise_seed: u64,
    /// Domain offsets so two seeds never sample the same nebula slice.
    nebula_offset: (f32, f32),
    nebula_scale: f32,
    /// 0..1 how much nebula tint shows at all (some seeds are mostly starfield).
    nebula_strength: f32,
    /// Domain-warp amount: what turns soft blobs into curled filaments.
    warp_strength: f32,
    dust_strength: f32,
    /// Bright ionization heart of the nebula: (x, y, strength).
    core: Option<(f32, f32, f32)>,
    /// Star-forming knots: (x, y, radius) pink clumps in the gas.
    knots: Vec<(f32, f32, f32)>,
    /// Concentric dusty shells around the core, V838-style; ring spacing px.
    shell_spacing: Option<f32>,
    stars: Vec<Star>,
    galaxies: Vec<Galaxy>,
    hero: Option<HeroGalaxy>,
    sun: Option<Sun>,
    /// Ring-Nebula-style planetary nebula: (x, y, radius).
    ring_nebula: Option<(f32, f32, f32)>,
    /// Veil-remnant ribbon: (axis angle, fractional offset from center).
    veil: Option<(f32, f32)>,
}

struct Star {
    x: f32,
    y: f32,
    radius: f32,
    brightness: f32,
    color: [f32; 3],
    /// Diffraction spike half-length in pixels; 0 for plain stars.
    spike: f32,
}

struct Galaxy {
    x: f32,
    y: f32,
    radius: f32,
    angle: f32,
    /// Minor/major axis ratio: low = edge-on sliver, high = face-on disc.
    flatten: f32,
    core: [f32; 3],
    arms: [f32; 3],
}

struct Sun {
    x: f32,
    y: f32,
    radius: f32,
    color: [f32; 3],
}

/// A large foreground spiral, Whirlpool/M106 style: log-spiral blue arms laced
/// with dust filaments, pink star-forming knots, and a warm core.
struct HeroGalaxy {
    x: f32,
    y: f32,
    radius: f32,
    angle: f32,
    /// Minor/major axis ratio (viewing tilt).
    flatten: f32,
    /// Log-spiral winding; sign flips the rotation sense.
    twist: f32,
    seed: u64,
}

impl Scene {
    fn generate(rng: &mut StdRng, width: u32, height: u32) -> Self {
        let min_side = width.min(height) as f32;
        // Scene kinds: mostly nebula; sometimes an ultra-deep-field (black sky
        // packed with tiny galaxies) or a bare open-cluster starfield.
        let kind_roll = rng.random_range(0..10u32);
        let deep_field = kind_roll == 0;
        let cluster_field = kind_roll == 1;
        // Veil-style supernova remnant: braided rainbow filaments over black.
        let veil = kind_roll == 2;
        let mut stars = generate_stars(rng, width, height);
        let galaxies = if deep_field {
            generate_deep_field(rng, width, height, min_side)
        } else {
            generate_galaxies(rng, width, height)
        };
        let sun = if deep_field || cluster_field || veil {
            None
        } else {
            generate_sun(rng, width, height)
        };
        if cluster_field {
            // Open-cluster field over black sky: one dense colorful cluster
            // plus a doubled sprinkle of loose field stars, per the Hubble
            // globular/open cluster portraits.
            let (cx, cy) = corner_anchor(rng, width, height, 0.12);
            let strength = rng.random_range(1.0..=1.6);
            stars.extend(generate_cluster(rng, cx, cy, min_side, strength));
            let extra = generate_stars(rng, width, height);
            stars.extend(extra);
        }
        // Bright ionization core, Orion/Westerlund style: a hot white-pink
        // heart in the nebula with a young star cluster embedded in it. Kept
        // near an edge so the capture window doesn't cover it.
        let core = if !deep_field && !cluster_field && !veil && rng.random_bool(0.65) {
            let (cx, cy) = corner_anchor(rng, width, height, 0.08);
            Some((cx, cy, rng.random_range(0.5..=1.0)))
        } else {
            None
        };
        if let Some((cx, cy, strength)) = core {
            stars.extend(generate_cluster(rng, cx, cy, min_side, strength));
        }
        // Star-forming knots: small saturated pink clumps scattered through
        // the gas, like the Antennae's star-birth regions.
        let knots = if deep_field || cluster_field || veil {
            Vec::new()
        } else {
            (0..rng.random_range(2..=6))
                .map(|_| {
                    (
                        rng.random_range(0.0..width as f32),
                        rng.random_range(0.0..height as f32),
                        min_side * rng.random_range(0.008..=0.028),
                    )
                })
                .collect()
        };
        let nebula_strength = if deep_field || cluster_field || veil {
            rng.random_range(0.05..=0.16)
        } else {
            rng.random_range(0.75..=1.0)
        };
        // Foreground hero spiral, corner-anchored so the window doesn't
        // swallow it.
        let hero = if !deep_field && !cluster_field && !veil && rng.random_bool(0.3) {
            let (hx, hy) = corner_anchor(rng, width, height, 0.1);
            Some(HeroGalaxy {
                x: hx,
                y: hy,
                radius: min_side * rng.random_range(0.16..=0.3),
                angle: rng.random_range(0.0..std::f32::consts::PI),
                flatten: rng.random_range(0.3..=0.75),
                twist: rng.random_range(1.8..=3.2) * if rng.random_bool(0.5) { 1.0 } else { -1.0 },
                seed: rng.random(),
            })
        } else {
            None
        };
        let shell_spacing = if core.is_some() && rng.random_bool(0.5) {
            Some(min_side * rng.random_range(0.05..=0.1))
        } else {
            None
        };
        // Ring-Nebula-style planetary nebula: a small donut with a blue
        // heart, teal-white middle, and wobbling orange rim.
        let ring_nebula =
            if !deep_field && !cluster_field && !veil && hero.is_none() && rng.random_bool(0.2) {
                let (rx, ry) = corner_anchor(rng, width, height, 0.1);
                Some((rx, ry, min_side * rng.random_range(0.06..=0.12)))
            } else {
                None
            };
        Scene {
            noise_seed: rng.random(),
            nebula_offset: (rng.random_range(0.0..64.0), rng.random_range(0.0..64.0)),
            nebula_scale: rng.random_range(2.4..=4.6),
            nebula_strength,
            warp_strength: rng.random_range(0.6..=1.4),
            dust_strength: rng.random_range(0.3..=0.7),
            core,
            knots,
            shell_spacing,
            stars,
            galaxies,
            hero,
            sun,
            ring_nebula,
            veil: if veil {
                Some((rng.random_range(0.3..=1.3), rng.random_range(-0.25..=0.25)))
            } else {
                None
            },
        }
    }
}

fn generate_stars(rng: &mut StdRng, width: u32, height: u32) -> Vec<Star> {
    let area = (width as f32 * height as f32) / 1_000_000.0;
    let mut stars = Vec::new();
    // Dense faint dust of stars.
    for _ in 0..((area * rng.random_range(550.0..=950.0)) as u32).max(90) {
        stars.push(Star {
            x: rng.random_range(0.0..width as f32),
            y: rng.random_range(0.0..height as f32),
            radius: rng.random_range(0.5..=0.9),
            brightness: rng.random_range(0.12..=0.4),
            color: star_color(rng),
            spike: 0.0,
        });
    }
    // Mid stars with a visible soft disc.
    for _ in 0..((area * rng.random_range(80.0..=130.0)) as u32).max(18) {
        stars.push(Star {
            x: rng.random_range(0.0..width as f32),
            y: rng.random_range(0.0..height as f32),
            radius: rng.random_range(0.9..=1.8),
            brightness: rng.random_range(0.4..=0.8),
            color: star_color(rng),
            spike: 0.0,
        });
    }
    // Mid-bright stars with small diffraction spikes; Hubble frames spike
    // nearly every star above the noise floor.
    for _ in 0..((area * rng.random_range(18.0..=34.0)) as u32).max(6) {
        let radius = rng.random_range(1.1..=1.9);
        stars.push(Star {
            x: rng.random_range(0.0..width as f32),
            y: rng.random_range(0.0..height as f32),
            radius,
            brightness: rng.random_range(0.6..=0.9),
            color: star_color(rng),
            spike: radius * rng.random_range(2.0..=3.5),
        });
    }
    // A handful of hero stars with long diffraction spikes.
    for _ in 0..rng.random_range(6..=14) {
        let radius = rng.random_range(1.6..=2.8);
        stars.push(Star {
            x: rng.random_range(0.0..width as f32),
            y: rng.random_range(0.0..height as f32),
            radius,
            brightness: rng.random_range(0.85..=1.0),
            color: star_color(rng),
            spike: radius * rng.random_range(3.5..=6.5),
        });
    }
    stars
}

/// Temperature mix per real populations: mostly near-white with subtle warm
/// tints, a scattering of clearly orange stars, rare saturated blue-white.
fn star_color(rng: &mut StdRng) -> [f32; 3] {
    match rng.random_range(0..12) {
        0 => [168.0, 196.0, 255.0],     // rare saturated blue-white giant
        1..=3 => [255.0, 206.0, 158.0], // orange K/M dwarfs
        4..=6 => [252.0, 240.0, 224.0], // subtle warm white
        _ => [244.0, 244.0, 252.0],     // near-white
    }
}

/// Young star cluster embedded at the nebula core: dense gaussian sprinkle of
/// hot blue-white and red points, like Westerlund 2 / the Trapezium.
fn generate_cluster(rng: &mut StdRng, cx: f32, cy: f32, min_side: f32, strength: f32) -> Vec<Star> {
    let spread = min_side * rng.random_range(0.09..=0.2);
    // Tarantula/NGC 346 density: hundreds of members, brightest with spikes.
    let count = (strength * rng.random_range(150.0..=300.0)) as u32;
    (0..count)
        .map(|_| {
            // Sum of two uniforms approximates a gaussian falloff cheaply.
            let radius =
                spread * (rng.random_range(0.0..1.0f32) + rng.random_range(0.0..1.0)) / 2.0;
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let color = if rng.random_bool(0.3) {
                [255.0, 150.0, 140.0] // young reddened members
            } else {
                [190.0, 210.0, 255.0] // hot blue-white
            };
            let star_radius = rng.random_range(0.5..=1.6);
            Star {
                x: cx + angle.cos() * radius,
                y: cy + angle.sin() * radius,
                radius: star_radius,
                brightness: rng.random_range(0.3..=1.0),
                color,
                spike: if rng.random_bool(0.06) {
                    star_radius * 3.0
                } else {
                    0.0
                },
            }
        })
        .collect()
}

fn generate_galaxies(rng: &mut StdRng, width: u32, height: u32) -> Vec<Galaxy> {
    let count = match rng.random_range(0..10) {
        0..=4 => 0,
        5..=8 => 1,
        _ => 2,
    };
    let min_side = width.min(height) as f32;
    (0..count)
        .map(|_| {
            let (core, arms) = galaxy_colors(rng);
            Galaxy {
                x: rng.random_range(0.0..width as f32),
                y: rng.random_range(0.0..height as f32),
                radius: min_side * rng.random_range(0.03..=0.07),
                angle: rng.random_range(0.0..std::f32::consts::PI),
                flatten: rng.random_range(0.22..=0.6),
                core,
                arms,
            }
        })
        .collect()
}

/// Warm-core/cool-arm spirals dominate, with golden ellipticals and pale
/// lenticulars mixed in, per the deep-field population.
fn galaxy_colors(rng: &mut StdRng) -> ([f32; 3], [f32; 3]) {
    match rng.random_range(0..6) {
        0..=2 => ([255.0, 236.0, 200.0], [176.0, 196.0, 244.0]), // spiral
        3..=4 => ([255.0, 206.0, 130.0], [235.0, 190.0, 140.0]), // gold elliptical
        _ => ([244.0, 236.0, 224.0], [190.0, 200.0, 230.0]),     // pale lenticular
    }
}

/// Dozens of tiny distant galaxies scattered over black sky.
fn generate_deep_field(rng: &mut StdRng, width: u32, height: u32, min_side: f32) -> Vec<Galaxy> {
    (0..rng.random_range(35..=70))
        .map(|_| {
            let (core, arms) = galaxy_colors(rng);
            Galaxy {
                x: rng.random_range(0.0..width as f32),
                y: rng.random_range(0.0..height as f32),
                radius: min_side * rng.random_range(0.006..=0.022),
                angle: rng.random_range(0.0..std::f32::consts::PI),
                flatten: rng.random_range(0.2..=0.85),
                core,
                arms,
            }
        })
        .collect()
}

fn generate_sun(rng: &mut StdRng, width: u32, height: u32) -> Option<Sun> {
    if rng.random_range(0..10) < 7 {
        return None;
    }
    let (x, y) = corner_anchor(rng, width, height, 0.02);
    Some(Sun {
        x,
        y,
        radius: width.min(height) as f32 * rng.random_range(0.16..=0.28),
        color: if rng.random_bool(0.5) {
            [255.0, 238.0, 200.0] // yellow-white main sequence
        } else {
            [255.0, 176.0, 120.0] // red giant warmth
        },
    })
}

/// A point near one of the four canvas corners (the visible padding band),
/// pushed `inset` (fraction of the short side) in from the exact corner.
fn corner_anchor(rng: &mut StdRng, width: u32, height: u32, inset: f32) -> (f32, f32) {
    let min_side = width.min(height) as f32;
    let margin = min_side * (inset + rng.random_range(0.0..=0.06));
    let x = if rng.random_bool(0.5) {
        margin
    } else {
        width as f32 - margin
    };
    let y = if rng.random_bool(0.5) {
        margin
    } else {
        height as f32 - margin
    };
    (x, y)
}

/// Base pass: tinted near-black gradient, nebula clouds in the two glow tints,
/// dark dust lanes, and film grain. One pixel at a time like the gradient
/// backdrop, so the two paths share their cost profile.
fn base_layer(width: u32, height: u32, style: &PresentationStyle, scene: &Scene) -> RgbaImage {
    let stops = style.stops.map(to_f32);
    let glow_a = to_f32(style.glow_a);
    let glow_b = to_f32(style.glow_b);
    let (gradient_cos, gradient_sin) = (style.gradient_angle.cos(), style.gradient_angle.sin());
    let gradient_norm = (gradient_cos + gradient_sin).max(f32::EPSILON);
    let scale = scene.nebula_scale;
    ImageBuffer::from_fn(width, height, |x, y| {
        let fx = x as f32 / width.max(1) as f32;
        let fy = y as f32 / height.max(1) as f32;
        // Keep the base near-black: real astro frames spend their brightness
        // inside the gas, not on the sky. The mid/bright stops belong to the
        // clouds below.
        let t = ((fx * gradient_cos + fy * gradient_sin) / gradient_norm).clamp(0.0, 1.0);
        let mut color = mix3(stops[0], stops[1], smoothstep(t) * 0.35);

        let bx = fx * scale + scene.nebula_offset.0;
        let by = fy * scale * (height as f32 / width.max(1) as f32) + scene.nebula_offset.1;
        // Domain warp: offset the sample point by another noise field so the
        // clouds curl into filaments instead of resting as soft blobs.
        let warp_x = fbm(bx + 31.4, by + 47.2, scene.noise_seed ^ 0x77, 3) - 0.5;
        let warp_y = fbm(bx + 12.9, by + 91.1, scene.noise_seed ^ 0x99, 3) - 0.5;
        let nx = bx + warp_x * scene.warp_strength;
        let ny = by + warp_y * scene.warp_strength;
        // Two decorrelated cloud fields, thresholded so the clouds stay wispy
        // with large dark gaps instead of an even haze.
        let cloud_a = wisp(fbm(nx, ny, scene.noise_seed, NEBULA_OCTAVES));
        let cloud_b = wisp(fbm(
            nx + 19.7,
            ny + 7.3,
            scene.noise_seed ^ 0xA5A5,
            NEBULA_OCTAVES,
        ));
        // A third field lets the palette's bright stop live in its own cloud
        // system, so one frame carries multiple hues like the Hubble frames.
        let cloud_c = wisp(fbm(
            nx * 1.6 + 41.3,
            ny * 1.6 + 27.9,
            scene.noise_seed ^ 0xC3C3,
            NEBULA_OCTAVES,
        ));
        color = mix3(color, glow_a, cloud_a * 0.95 * scene.nebula_strength);
        color = mix3(color, glow_b, cloud_b * 0.8 * scene.nebula_strength);
        color = mix3(color, stops[2], cloud_c * 0.5 * scene.nebula_strength);
        // Ridge highlights: bright ionization fronts along filament crests,
        // whitened toward the hot edge like the rims in Hubble frames.
        let ridge_raw = fbm(nx * 1.4 + 5.1, ny * 1.4 + 2.7, scene.noise_seed ^ 0x3C3C, 4);
        let ridge = (1.0 - (ridge_raw * 2.0 - 1.0).abs()).powi(6) * cloud_a.max(cloud_b);
        let ridge_tint = mix3(glow_a, [255.0, 250.0, 240.0], 0.55);
        color = mix3(color, ridge_tint, ridge * 0.5 * scene.nebula_strength);
        // Deepen the voids: where no cloud lives, pull hard toward black so
        // the gas floats on real darkness instead of haze.
        let presence = cloud_a.max(cloud_b).max(cloud_c);
        let void = 0.35 + 0.65 * presence.min(1.0);
        color = [color[0] * void, color[1] * void, color[2] * void];
        // Dust lanes: a further field darkens where it runs dense, carving the
        // brown-black filaments real nebulae have. Carve hardest over bright
        // gas so the silhouettes actually read.
        let dust = fbm(
            nx * 2.1 + 4.2,
            ny * 2.1 + 11.8,
            scene.noise_seed ^ 0x5A5A,
            4,
        );
        let presence_ab = cloud_a.max(cloud_b);
        let mut dim = 1.0 - dust.powi(3) * scene.dust_strength * (0.6 + 0.4 * presence_ab);
        // Hard silhouettes: only where dense dust crosses BRIGHT gas, carving
        // Cone/Eagle-style black pillars. Gating on cloud presence keeps the
        // empty sky from turning into marble.
        let silhouette = ((dust - 0.72) / 0.1).clamp(0.0, 1.0) * presence_ab;
        dim *= 1.0 - silhouette * 0.75;
        color = [color[0] * dim, color[1] * dim, color[2] * dim];
        // Hot white-pink glow around the ionization core, if this seed has one.
        if let Some((cx, cy, strength)) = scene.core {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let pixel_distance = (dx * dx + dy * dy).sqrt();
            let falloff = (1.0 - pixel_distance / (width.max(1) as f32 * 0.32)).clamp(0.0, 1.0);
            color = mix3(
                color,
                [255.0, 226.0, 224.0],
                falloff.powi(2) * 0.65 * strength,
            );
            // Concentric dusty shells thrown off the core, V838-style: golden
            // rings that fade with distance and ripple through the gas.
            if let Some(spacing) = scene.shell_spacing {
                let ring = ((pixel_distance / spacing) * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                color = mix3(
                    color,
                    [212.0, 176.0, 128.0],
                    ring.powi(4) * falloff * 0.22 * strength,
                );
            }
        }

        // Veil-remnant ribbon: three braided ridge-noise strands in electric
        // blue, pink, and gold, confined to a soft diagonal band.
        if let Some((veil_angle, veil_offset)) = scene.veil {
            let (band_sin, band_cos) = veil_angle.sin_cos();
            let across = (fx - 0.5) * -band_sin + (fy - 0.5 - veil_offset) * band_cos;
            let envelope = (-(across / 0.22).powi(2)).exp();
            if envelope > 0.02 {
                let strand_colors: [[f32; 3]; 3] = [
                    [96.0, 140.0, 255.0],
                    [255.0, 110.0, 180.0],
                    [235.0, 190.0, 90.0],
                ];
                for (strand, tint) in strand_colors.iter().enumerate() {
                    let braid = fbm(
                        nx * 2.2 + strand as f32 * 13.7,
                        ny * 2.2 + strand as f32 * 5.9,
                        scene.noise_seed ^ (0xBEEF + strand as u64),
                        4,
                    );
                    let filament = (1.0 - (braid * 2.0 - 1.0).abs()).powi(8) * envelope;
                    color = [
                        (color[0] + tint[0] * filament * 0.9).min(255.0),
                        (color[1] + tint[1] * filament * 0.9).min(255.0),
                        (color[2] + tint[2] * filament * 0.9).min(255.0),
                    ];
                }
            }
        }

        let grain = grain_noise(x, y, scene.noise_seed) * GRAIN_STRENGTH;
        Rgba([
            quantize(color[0] + grain),
            quantize(color[1] + grain),
            quantize(color[2] + grain),
            255,
        ])
    })
}

fn draw_stars(canvas: &mut RgbaImage, stars: &[Star]) {
    for star in stars {
        let reach = (star.radius * 3.0).max(star.spike).ceil() as i32;
        let cx = star.x;
        let cy = star.y;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                // Gaussian-ish core.
                let mut amount = (-((distance / star.radius).powi(2))).exp() * star.brightness;
                // Diffraction spikes: thin horizontal and vertical rays.
                if star.spike > 0.0 {
                    let along = dx.abs().max(dy.abs()) as f32;
                    if (dx == 0 || dy == 0) && along <= star.spike {
                        amount =
                            amount.max((1.0 - along / star.spike).powi(2) * star.brightness * 0.7);
                    }
                }
                if amount > 0.01 {
                    add_light(canvas, px, py, star.color, amount);
                }
            }
        }
    }
}

/// The big foreground spiral: pale disc glow, two log-spiral blue arms cut by
/// dust filaments, single-pixel blue speckle and pink knots along the arms,
/// and a warm bright core, per the Whirlpool/M106/M74 portraits.
fn draw_hero_galaxy(canvas: &mut RgbaImage, hero: &HeroGalaxy) {
    let reach = (hero.radius * 2.0).ceil() as i32;
    let (sin, cos) = hero.angle.sin_cos();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let major = dx as f32 * cos + dy as f32 * sin;
            let minor = (-(dx as f32) * sin + dy as f32 * cos) / hero.flatten;
            let distance = (major * major + minor * minor).sqrt() / hero.radius;
            if distance > 1.8 {
                continue;
            }
            let px = hero.x as i32 + dx;
            let py = hero.y as i32 + dy;
            // Two arms traced in log-spiral phase space.
            let phase = minor.atan2(major) + hero.twist * distance.max(0.04).ln();
            let arm = ((phase * 2.0).sin() * 0.5 + 0.5).powi(2);
            let disc = (-(distance * 1.5).powi(2)).exp();
            let core = (-(distance * 5.0).powi(2)).exp();
            // Dust filaments riding the disc.
            let filaments = fbm(
                major / hero.radius * 3.2,
                minor / hero.radius * 3.2,
                hero.seed,
                4,
            );
            let dust = ((filaments - 0.48) / 0.22).clamp(0.0, 1.0);
            add_light(
                canvas,
                px,
                py,
                [225.0, 215.0, 205.0],
                disc * 0.45 * (1.0 - dust * 0.8),
            );
            let arm_light = arm * disc * (1.0 - dust * 0.85);
            add_light(canvas, px, py, [150.0, 175.0, 235.0], arm_light * 0.5);
            // Per-pixel sparkle: blue cluster speckle and pink knots.
            let sparkle = cell_hash(px as i64, py as i64, hero.seed);
            if sparkle > 0.982 && arm_light > 0.06 {
                add_light(canvas, px, py, [205.0, 222.0, 255.0], 0.75);
            } else if sparkle < 0.005 && arm_light > 0.08 {
                add_light(canvas, px, py, [255.0, 140.0, 190.0], 0.7);
            }
            add_light(canvas, px, py, [255.0, 226.0, 185.0], core * 0.95);
        }
    }
}

fn draw_galaxy(canvas: &mut RgbaImage, galaxy: &Galaxy) {
    let reach = (galaxy.radius * 2.4).ceil() as i32;
    let (sin, cos) = galaxy.angle.sin_cos();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let major = dx as f32 * cos + dy as f32 * sin;
            let minor = (-(dx as f32) * sin + dy as f32 * cos) / galaxy.flatten;
            let distance = (major * major + minor * minor).sqrt() / galaxy.radius;
            if distance > 2.2 {
                continue;
            }
            // Warm bright core falling off into cool blue arms.
            let core = (-(distance * 3.2).powi(2)).exp();
            let halo = (-(distance * 1.1).powi(2)).exp() * 0.35;
            let px = galaxy.x as i32 + dx;
            let py = galaxy.y as i32 + dy;
            add_light(canvas, px, py, galaxy.core, core * 0.9);
            add_light(canvas, px, py, galaxy.arms, halo);
        }
    }
}

fn draw_sun(canvas: &mut RgbaImage, sun: &Sun) {
    let reach = (sun.radius * 2.2).ceil() as i32;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let distance = ((dx * dx + dy * dy) as f32).sqrt() / sun.radius;
            if distance > 2.2 {
                continue;
            }
            // Hot white core, colored corona, long soft falloff.
            let core = (-(distance * 3.4).powi(2)).exp();
            let corona = (-(distance * 1.15).powi(2)).exp() * 0.7;
            let px = sun.x as i32 + dx;
            let py = sun.y as i32 + dy;
            add_light(canvas, px, py, [255.0, 255.0, 248.0], core);
            add_light(canvas, px, py, sun.color, corona);
        }
    }
}

/// Additive light blending (stars, glows) clamped to opaque white.
fn add_light(canvas: &mut RgbaImage, x: i32, y: i32, color: [f32; 3], amount: f32) {
    let Some(pixel) = pixel_mut(canvas, x, y) else {
        return;
    };
    for (channel, tint) in pixel.0.iter_mut().zip(color) {
        let value = f32::from(*channel) + tint * amount;
        *channel = value.min(255.0) as u8;
    }
}

fn pixel_mut(canvas: &mut RgbaImage, x: i32, y: i32) -> Option<&mut Rgba<u8>> {
    if x < 0 || y < 0 || x >= canvas.width() as i32 || y >= canvas.height() as i32 {
        return None;
    }
    Some(canvas.get_pixel_mut(x as u32, y as u32))
}

/// Thresholds cloud noise so most of the sky stays black and only the upper
/// range reads as nebula; real astro frames carry color on maybe a quarter of
/// the field, not an even haze.
fn wisp(cloud: f32) -> f32 {
    // fbm output effectively lives in ~0.3..0.7, so remap that band to the
    // full range before shaping: dense cloud hearts must reach ~1.0 or the
    // palette never saturates and every frame reads as grey haze.
    let cloud = ((cloud - 0.34) / 0.36).clamp(0.0, 1.0);
    cloud.powf(1.7)
}

/// Fractal brownian motion over hash-based value noise, normalized to 0..1.
fn fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut norm = 0.0;
    for octave in 0..octaves {
        total += value_noise(
            x * frequency,
            y * frequency,
            seed.wrapping_add(octave as u64),
        ) * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    (total / norm).clamp(0.0, 1.0)
}

/// Smooth value noise: hash the four cell corners, bilinear with smoothstep.
fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let tx = smoothstep(x - x0);
    let ty = smoothstep(y - y0);
    let x0 = x0 as i64;
    let y0 = y0 as i64;
    let a = cell_hash(x0, y0, seed);
    let b = cell_hash(x0 + 1, y0, seed);
    let c = cell_hash(x0, y0 + 1, seed);
    let d = cell_hash(x0 + 1, y0 + 1, seed);
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}

fn cell_hash(x: i64, y: i64, seed: u64) -> f32 {
    let mut hash = (x as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add(seed);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hash ^= hash >> 32;
    (hash & 0xFFFF_FFFF) as f32 / u32::MAX as f32
}

fn grain_noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut hash = x
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(y.wrapping_mul(0x85EB_CA77))
        .wrapping_add(seed as u32);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7FEB_352D);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846C_A68B);
    hash ^= hash >> 16;
    (hash as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn to_f32(color: [u8; 3]) -> [f32; 3] {
    [
        f32::from(color[0]),
        f32::from(color[1]),
        f32::from(color[2]),
    ]
}

fn mix3(start: [f32; 3], end: [f32; 3], amount: f32) -> [f32; 3] {
    let amount = amount.clamp(0.0, 1.0);
    [
        start[0] + (end[0] - start[0]) * amount,
        start[1] + (end[1] - start[1]) * amount,
        start[2] + (end[2] - start[2]) * amount,
    ]
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn quantize(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polish;

    fn space_style(seed: u64) -> PresentationStyle {
        polish::style_with_palette(seed, "orion-emission").expect("known space palette")
    }

    #[test]
    fn space_backdrop_is_fully_opaque() {
        let canvas = render(320, 240, &space_style(3));
        for pixel in canvas.pixels() {
            assert_eq!(pixel.0[3], 255);
        }
    }

    #[test]
    fn same_seed_renders_identical_scene() {
        let style = space_style(42);
        assert_eq!(
            render(320, 240, &style).as_raw(),
            render(320, 240, &style).as_raw()
        );
    }

    #[test]
    fn different_seeds_render_different_scenes() {
        assert_ne!(
            render(320, 240, &space_style(1)).as_raw(),
            render(320, 240, &space_style(2)).as_raw()
        );
    }

    #[test]
    fn scene_contains_stars_brighter_than_the_base() {
        // Star splats guarantee bright pixels well above the near-black base.
        let canvas = render(640, 480, &space_style(7));
        let brightest = canvas
            .pixels()
            .map(|p| u32::from(p.0[0]) + u32::from(p.0[1]) + u32::from(p.0[2]))
            .max()
            .unwrap();
        assert!(brightest > 400, "expected bright stars, got {brightest}");
    }

    #[test]
    fn base_stays_dark_overall() {
        // Space cards must read as black sky: the median pixel stays dark even
        // with nebula tint at full strength.
        let canvas = render(320, 240, &space_style(11));
        let mut sums: Vec<u32> = canvas
            .pixels()
            .map(|p| u32::from(p.0[0]) + u32::from(p.0[1]) + u32::from(p.0[2]))
            .collect();
        sums.sort_unstable();
        let median = sums[sums.len() / 2];
        assert!(median < 240, "median brightness {median} is not a dark sky");
    }

    #[test]
    fn value_noise_is_bounded() {
        for i in 0..200 {
            let v = fbm(i as f32 * 0.37, i as f32 * 0.71, 99, 5);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}

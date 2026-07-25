//! Procedural geometric backdrops: plaids, stripes, rules, weaves.
//!
//! The third backdrop family, alongside the gradients and the deep-space
//! scenes. It mirrors the space model exactly: a pattern palette carries the
//! colors, and a [`MotifKind`] carries the structure, so `--palette` and
//! `--motif` compose the same way `--palette` and `--scene` do.
//!
//! Everything is drawn from the style seed at render time. Nothing is a tiled
//! bitmap, so a pattern resolves cleanly at any card size and the same
//! `--style-seed` reproduces one exactly.

use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::polish::PresentationStyle;

/// Matches the gradient and space backdrops so film feel stays consistent.
const GRAIN_STRENGTH: f32 = 2.4;
/// Motif cells are sized against this so a pattern looks the same density on a
/// thumbnail and on a full card.
const REFERENCE_SIZE: f32 = 900.0;
/// Projecting onto a 45-degree axis: `(x + y) / sqrt(2)`.
const DIAGONAL: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// A specific pattern the caller can pin instead of the seed's random pick.
/// The seed still drives every free parameter (cell size, band widths, phase);
/// this only forces which pattern appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifKind {
    /// Asymmetric tartan sett, mirrored, crossed both ways with a twill tooth.
    Plaid,
    /// Even two-color checks; overlaps read darker, like the tablecloth.
    Gingham,
    /// Vertical bands from a seeded sett.
    Stripe,
    /// Hairlines on a flat ground, like ruled paper.
    Rule,
    /// Hairlines both ways: graph paper.
    Grid,
    /// The stripe sett turned 45 degrees.
    Diagonal,
    /// Zigzag bands.
    Chevron,
    /// Dots on a staggered grid.
    Dot,
    /// Opposed diagonal hairlines.
    Crosshatch,
    /// Alternating horizontal and vertical bars, basket style.
    Weave,
    /// Broken twill: diagonal blocks that reverse direction each column.
    Herringbone,
    /// The four-step dogtooth tessellation.
    Houndstooth,
}

impl MotifKind {
    /// All names accepted by [`MotifKind::from_name`], in menu order.
    pub const NAMES: [&'static str; 12] = [
        "plaid",
        "gingham",
        "stripe",
        "rule",
        "grid",
        "diagonal",
        "chevron",
        "dot",
        "crosshatch",
        "weave",
        "herringbone",
        "houndstooth",
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "plaid" => Self::Plaid,
            "gingham" => Self::Gingham,
            "stripe" => Self::Stripe,
            "rule" => Self::Rule,
            "grid" => Self::Grid,
            "diagonal" => Self::Diagonal,
            "chevron" => Self::Chevron,
            "dot" => Self::Dot,
            "crosshatch" => Self::Crosshatch,
            "weave" => Self::Weave,
            "herringbone" => Self::Herringbone,
            "houndstooth" => Self::Houndstooth,
            _ => return None,
        })
    }

    fn all() -> [Self; 12] {
        [
            Self::Plaid,
            Self::Gingham,
            Self::Stripe,
            Self::Rule,
            Self::Grid,
            Self::Diagonal,
            Self::Chevron,
            Self::Dot,
            Self::Crosshatch,
            Self::Weave,
            Self::Herringbone,
            Self::Houndstooth,
        ]
    }
}

/// One entry in a tartan sett: a band of `width` cells in `color` at `alpha`.
struct Band {
    width: f32,
    color: [f32; 3],
    alpha: f32,
}

/// Paint the pattern backdrop for `style` at this size.
pub fn render(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    let mut rng = StdRng::seed_from_u64(style.seed ^ 0x9e37_79b9_7f4a_7c15);
    let motif = style
        .motif
        .unwrap_or_else(|| MotifKind::all()[rng.random_range(0..12)]);

    let min_side = width.min(height).max(1) as f32;
    let scale = (min_side / REFERENCE_SIZE).clamp(0.25, 3.0);
    // Cell size in pixels, kept generous so a motif still reads on a swatch.
    let cell = (rng.random_range(46.0..=92.0) * scale).max(7.0);
    let phase_x = rng.random_range(0.0..1.0);
    let phase_y = rng.random_range(0.0..1.0);

    let ground = to_f32(style.stops[0]);
    let sett = build_sett(&mut rng, style);
    let sett_span: f32 = sett.iter().map(|band| band.width).sum::<f32>().max(0.001);
    let ink = to_f32(style.stops[2]);
    let accent = to_f32(style.glow_a);
    let second = to_f32(style.glow_b);
    let line_weight = (rng.random_range(0.05..=0.12) * cell).max(1.0);

    ImageBuffer::from_fn(width, height, |x, y| {
        let fx = x as f32 / cell + phase_x;
        let fy = y as f32 / cell + phase_y;
        let mut color = ground;

        match motif {
            MotifKind::Plaid => {
                // Horizontal pass then vertical, so the crossings compound and
                // the sett reads as woven rather than printed.
                let (h_color, h_alpha) = sample_sett(&sett, sett_span, fy);
                color = mix(color, h_color, h_alpha);
                let (v_color, v_alpha) = sample_sett(&sett, sett_span, fx);
                color = mix(color, v_color, v_alpha * 0.82);
                // Twill tooth: a faint diagonal running through the cloth.
                let twill = ((fx + fy) * 6.0).sin() * 0.5 + 0.5;
                color = mix(color, ink, 0.05 * twill);
            }
            MotifKind::Gingham => {
                let cx = fract(fx * 0.5) < 0.5;
                let cy = fract(fy * 0.5) < 0.5;
                color = match (cx, cy) {
                    (true, true) => mix(ground, ink, 0.85),
                    (true, false) | (false, true) => mix(ground, ink, 0.42),
                    (false, false) => ground,
                };
            }
            MotifKind::Stripe => {
                let (band, alpha) = sample_sett(&sett, sett_span, fx);
                color = mix(color, band, alpha);
            }
            MotifKind::Rule => {
                color = mix(color, ink, line(fy, cell, line_weight));
                // Every fifth rule is heavier, the way ledger paper is set.
                if (fy.floor() as i64).rem_euclid(5) == 0 {
                    color = mix(color, accent, line(fy, cell, line_weight) * 0.7);
                }
            }
            MotifKind::Grid => {
                let h = line(fy, cell, line_weight);
                let v = line(fx, cell, line_weight);
                color = mix(color, ink, h.max(v));
                if (fx.floor() as i64).rem_euclid(5) == 0 || (fy.floor() as i64).rem_euclid(5) == 0
                {
                    color = mix(color, accent, h.max(v) * 0.6);
                }
            }
            MotifKind::Diagonal => {
                let (band, alpha) = sample_sett(&sett, sett_span, (fx + fy) * DIAGONAL);
                color = mix(color, band, alpha);
            }
            MotifKind::Chevron => {
                // Triangle wave along x turns the stripe into a zigzag.
                let zig = (fract(fx * 0.5) - 0.5).abs() * 4.0;
                let (band, alpha) = sample_sett(&sett, sett_span, fy + zig);
                color = mix(color, band, alpha);
            }
            MotifKind::Dot => {
                // Stagger every other row so the field reads as polka, not grid.
                let row = fy.floor() as i64;
                let offset = if row.rem_euclid(2) == 0 { 0.0 } else { 0.5 };
                let dx = fract(fx + offset) - 0.5;
                let dy = fract(fy) - 0.5;
                let distance = (dx * dx + dy * dy).sqrt();
                let radius = 0.3;
                let edge = 1.2 / cell;
                let inside = 1.0 - smoothstep_range(distance, radius - edge, radius + edge);
                let tone = if (row + fx.floor() as i64).rem_euclid(3) == 0 {
                    accent
                } else {
                    ink
                };
                color = mix(color, tone, inside);
            }
            MotifKind::Crosshatch => {
                let a = line((fx + fy) * DIAGONAL, cell, line_weight);
                let b = line((fx - fy) * DIAGONAL, cell, line_weight);
                color = mix(color, ink, a.max(b) * 0.9);
                color = mix(color, second, (a * b) * 0.5);
            }
            MotifKind::Weave => {
                // Basket: alternating blocks run with the warp or the weft.
                let bx = (fx * 0.5).floor() as i64;
                let by = (fy * 0.5).floor() as i64;
                let vertical = (bx + by).rem_euclid(2) == 0;
                let across = if vertical { fx } else { fy };
                let shade = 0.5 + 0.5 * (fract(across) * std::f32::consts::TAU).sin();
                let base = if vertical { ink } else { second };
                color = mix(color, base, 0.55 + 0.35 * shade);
            }
            MotifKind::Herringbone => {
                // Diagonal runs that reverse direction column by column.
                let column = (fx * 0.5).floor() as i64;
                let rising = column.rem_euclid(2) == 0;
                let along = if rising { fx + fy } else { fx - fy };
                let stripe = line(along * DIAGONAL, cell * 0.5, line_weight * 1.6);
                color = mix(color, ink, 0.28 + 0.62 * stripe);
            }
            MotifKind::Houndstooth => {
                // Threads are finer than motif cells, so a swatch shows
                // several repeats instead of one giant tooth.
                let tone = if houndstooth(fx * 2.0, fy * 2.0) {
                    ink
                } else {
                    ground
                };
                color = mix(color, tone, 0.92);
            }
        }

        // Broad tonal drift so a large field is not perfectly flat, then the
        // shared grain.
        let drift = 0.5
            + 0.5
                * ((x as f32 / width.max(1) as f32) * style.gradient_angle.cos()
                    + (y as f32 / height.max(1) as f32) * style.gradient_angle.sin());
        color = mix(color, to_f32(style.stops[1]), 0.10 * drift);

        let grain = hash_noise(x, y, style.seed) * GRAIN_STRENGTH;
        Rgba([
            clamp_u8(color[0] + grain),
            clamp_u8(color[1] + grain),
            clamp_u8(color[2] + grain),
            255,
        ])
    })
}

/// Build a symmetric tartan sett from the seed. Real setts mirror around a
/// pivot, which is what stops a plaid reading as arbitrary stripes.
fn build_sett(rng: &mut StdRng, style: &PresentationStyle) -> Vec<Band> {
    let palette = [
        (to_f32(style.stops[1]), 0.85),
        (to_f32(style.stops[2]), 0.7),
        (to_f32(style.glow_a), 0.55),
        (to_f32(style.glow_b), 0.5),
    ];
    let half = rng.random_range(3..=5);
    let mut bands: Vec<Band> = Vec::with_capacity(half * 2);
    for index in 0..half {
        let (color, alpha) = palette[index % palette.len()];
        // Alternate wide grounds with narrow overcheck lines.
        let width = if index % 2 == 0 {
            rng.random_range(0.55..=1.25)
        } else {
            rng.random_range(0.10..=0.28)
        };
        bands.push(Band {
            width,
            color,
            alpha: alpha * rng.random_range(0.75..=1.0),
        });
    }
    // Mirror for the pivot.
    for index in (0..half).rev() {
        bands.push(Band {
            width: bands[index].width,
            color: bands[index].color,
            alpha: bands[index].alpha,
        });
    }
    bands
}

/// Colour and coverage at position `t` (in cells) through a repeating sett.
fn sample_sett(sett: &[Band], span: f32, t: f32) -> ([f32; 3], f32) {
    let mut position = t.rem_euclid(span);
    for band in sett {
        if position < band.width {
            return (band.color, band.alpha);
        }
        position -= band.width;
    }
    match sett.last() {
        Some(band) => (band.color, band.alpha),
        None => ([0.0, 0.0, 0.0], 0.0),
    }
}

/// Antialiased hairline coverage: 1.0 on the line, 0.0 off it.
fn line(t: f32, cell: f32, weight: f32) -> f32 {
    let distance = (fract(t) - 0.5).abs() * 2.0;
    let half = (weight / cell).clamp(0.01, 0.5);
    1.0 - smoothstep_range(1.0 - distance, half, half * 2.0)
}

/// The dogtooth tessellation.
///
/// Houndstooth is not a checkerboard with corners added: it is what a 2/2 twill
/// produces when warp and weft each carry a four-thread colour repeat. Deriving
/// it from the weave is what grows the teeth. Treating it as a check plus an
/// anti-diagonal split just yields diagonal bands.
fn houndstooth(warp: f32, weft: f32) -> bool {
    let x = warp.floor() as i64;
    let y = weft.floor() as i64;
    // 2/2 twill: the float steps two threads and shifts one each pick.
    let shows_warp = (x - y).rem_euclid(4) < 2;
    let warp_dark = x.rem_euclid(8) < 4;
    let weft_dark = y.rem_euclid(8) < 4;
    if shows_warp { warp_dark } else { weft_dark }
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

fn smoothstep_range(value: f32, edge0: f32, edge1: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn to_f32(color: [u8; 3]) -> [f32; 3] {
    [color[0] as f32, color[1] as f32, color[2] as f32]
}

fn clamp_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0) as u8
}

/// Cheap deterministic per-pixel noise for grain.
fn hash_noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut hash = seed ^ ((x as u64) << 32) ^ (y as u64).wrapping_mul(0x9e37_79b9);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    ((hash & 0xffff) as f32 / 65535.0) - 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polish;

    fn pattern_style(seed: u64, motif: Option<MotifKind>) -> PresentationStyle {
        let mut style = polish::style_with_palette(seed, "tartan-moss").expect("pattern palette");
        style.motif = motif;
        style
    }

    #[test]
    fn every_name_round_trips() {
        assert_eq!(MotifKind::NAMES.len(), MotifKind::all().len());
        for name in MotifKind::NAMES {
            assert!(MotifKind::from_name(name).is_some(), "{name}");
        }
        assert!(MotifKind::from_name("houndstooth-ish").is_none());
    }

    #[test]
    fn every_motif_paints_something_distinct() {
        // Two motifs that render identically would be a dead menu entry.
        let mut seen: Vec<(MotifKind, Vec<u8>)> = Vec::new();
        for motif in MotifKind::all() {
            let image = render(64, 48, &pattern_style(7, Some(motif)));
            let raw = image.as_raw().clone();
            for (other, previous) in &seen {
                assert_ne!(&raw, previous, "{motif:?} renders the same as {other:?}");
            }
            seen.push((motif, raw));
        }
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn a_pinned_motif_is_deterministic() {
        let a = render(48, 36, &pattern_style(11, Some(MotifKind::Plaid)));
        let b = render(48, 36, &pattern_style(11, Some(MotifKind::Plaid)));
        assert_eq!(a.as_raw(), b.as_raw());
    }

    #[test]
    fn an_unpinned_motif_still_follows_the_seed() {
        let a = render(48, 36, &pattern_style(11, None));
        let b = render(48, 36, &pattern_style(11, None));
        assert_eq!(a.as_raw(), b.as_raw(), "same seed must repeat");
        let c = render(48, 36, &pattern_style(12, None));
        assert_ne!(a.as_raw(), c.as_raw(), "a different seed must differ");
    }

    #[test]
    fn patterns_are_not_flat_fields() {
        // A motif that collapsed to one color would still pass the determinism
        // tests, so check there is actual structure.
        for motif in MotifKind::all() {
            let image = render(80, 60, &pattern_style(5, Some(motif)));
            let mut min = 255u8;
            let mut max = 0u8;
            for pixel in image.pixels() {
                min = min.min(pixel[0]);
                max = max.max(pixel[0]);
            }
            assert!(
                max.saturating_sub(min) > 12,
                "{motif:?} is nearly flat (red spread {})",
                max - min
            );
        }
    }

    #[test]
    fn a_pattern_resolves_at_swatch_and_card_size() {
        // Cell size scales with the canvas, so a swatch should not turn into a
        // single band.
        for (w, h) in [(64u32, 48u32), (300, 225), (1044, 724)] {
            let image = render(w, h, &pattern_style(3, Some(MotifKind::Gingham)));
            assert_eq!(image.dimensions(), (w, h));
            let mut min = 255u8;
            let mut max = 0u8;
            for pixel in image.pixels() {
                min = min.min(pixel[0]);
                max = max.max(pixel[0]);
            }
            assert!(max.saturating_sub(min) > 12, "flat at {w}x{h}");
        }
    }

    #[test]
    fn houndstooth_tessellates_into_both_tones() {
        let mut light = 0;
        let mut dark = 0;
        for step in 0..400 {
            let u = step as f32 * 0.037;
            let v = step as f32 * 0.061;
            if houndstooth(u, v) {
                dark += 1
            } else {
                light += 1
            }
        }
        assert!(light > 40 && dark > 40, "light {light} dark {dark}");
    }
}

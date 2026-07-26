use std::path::Path;

use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use image::imageops;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::backends::FrameExtents;
use crate::contract::PresentationStyleInfo;
use crate::util::AppError;

const AMBIENT_SHADOW_ALPHA: u8 = 52;
const KEY_SHADOW_ALPHA: u8 = 96;
const RIM_STRENGTH: f32 = 0.24;
const STREAK_STRENGTH: f32 = 0.09;
const GRAIN_STRENGTH: f32 = 2.4;
const REFERENCE_SIZE: f32 = 900.0;
const SHADOW_DOWNSCALE: u32 = 4;

/// How the backdrop behind the card is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropKind {
    /// The original soft gradient with glows and light streaks.
    Gradient,
    /// Procedural deep-space scene: starfield, nebula dust, celestial bodies.
    Space,
    /// Procedural geometric cloth or paper: plaids, stripes, rules, weaves.
    Pattern,
    /// Procedural sky: cloud decks, storms, lightning, twilight, aurora.
    Sky,
    /// Procedural terrain: dunes, mesa, badlands, glacier.
    Terrain,
}

struct Palette {
    name: &'static str,
    kind: BackdropKind,
    stops: [[u8; 3]; 3],
    glow_a: [u8; 3],
    glow_b: [u8; 3],
}

const fn gradient(
    name: &'static str,
    stops: [[u8; 3]; 3],
    glow_a: [u8; 3],
    glow_b: [u8; 3],
) -> Palette {
    Palette {
        name,
        kind: BackdropKind::Gradient,
        stops,
        glow_a,
        glow_b,
    }
}

const fn space(
    name: &'static str,
    stops: [[u8; 3]; 3],
    glow_a: [u8; 3],
    glow_b: [u8; 3],
) -> Palette {
    Palette {
        name,
        kind: BackdropKind::Space,
        stops,
        glow_a,
        glow_b,
    }
}

/// Pattern palette stops read differently from the other two families:
/// stop 0 is the ground the cloth is woven on, stop 1 a broad tonal drift, and
/// stop 2 the ink the motif is drawn in. The glows are the two overcheck
/// accents.
const fn pattern(
    name: &'static str,
    stops: [[u8; 3]; 3],
    glow_a: [u8; 3],
    glow_b: [u8; 3],
) -> Palette {
    Palette {
        name,
        kind: BackdropKind::Pattern,
        stops,
        glow_a,
        glow_b,
    }
}

/// Sky palette stops read as altitude, not as a generic ramp: stop 0 is the
/// zenith, stop 1 the middle of the sky, stop 2 the horizon. `glow_a` is the
/// light source (sun, bolt core, auroral emission) and `glow_b` is the cloud
/// shadow. `sky.rs` relies on that ordering.
const fn sky(name: &'static str, stops: [[u8; 3]; 3], glow_a: [u8; 3], glow_b: [u8; 3]) -> Palette {
    Palette {
        name,
        kind: BackdropKind::Sky,
        stops,
        glow_a,
        glow_b,
    }
}

/// Terrain palette stops read as a vertical cross-section, like the sky
/// palettes: stop 0 is the sky above the horizon, stop 1 the horizon line, and
/// stop 2 the ground. `glow_a` is the sun/snow highlight and `glow_b` is the
/// shadow. `terrain.rs` relies on that ordering.
const fn terrain(
    name: &'static str,
    stops: [[u8; 3]; 3],
    glow_a: [u8; 3],
    glow_b: [u8; 3],
) -> Palette {
    Palette {
        name,
        kind: BackdropKind::Terrain,
        stops,
        glow_a,
        glow_b,
    }
}

/// Space palette colors are sampled from astrophotography of the named
/// objects (emission pinks, reflection blues, dust golds); stops run
/// dark-to-mid so `design.rs` still reads stop 0 as the canvas base.
///
/// Sky palette colors are sampled the same way, from photographs of the named
/// conditions: supercell navy over underlit anvil gold, mammatus gray-violet,
/// blue-hour navy, aurora teal.
const PALETTES: [Palette; 38] = [
    gradient(
        "violet-haze",
        [[49, 29, 130], [112, 52, 224], [44, 84, 228]],
        [235, 96, 190],
        [132, 196, 255],
    ),
    gradient(
        "ember-glow",
        [[224, 158, 64], [214, 98, 40], [128, 46, 24]],
        [252, 214, 140],
        [170, 50, 30],
    ),
    gradient(
        "aurora-teal",
        [[10, 72, 92], [18, 152, 128], [88, 206, 196]],
        [150, 235, 190],
        [36, 96, 176],
    ),
    gradient(
        "rose-noir",
        [[40, 26, 50], [150, 38, 94], [236, 96, 122]],
        [250, 152, 100],
        [124, 72, 200],
    ),
    gradient(
        "midnight-sky",
        [[14, 24, 58], [40, 78, 198], [92, 170, 248]],
        [142, 102, 248],
        [132, 228, 250],
    ),
    gradient(
        "sea-glass",
        [[12, 58, 62], [24, 120, 120], [120, 206, 190]],
        [180, 240, 220],
        [40, 120, 180],
    ),
    gradient(
        "peach-dusk",
        [[60, 26, 44], [184, 80, 86], [248, 158, 120]],
        [255, 206, 150],
        [140, 74, 170],
    ),
    gradient(
        "ink-wash",
        [[16, 18, 26], [48, 56, 78], [120, 132, 164]],
        [190, 204, 236],
        [70, 90, 140],
    ),
    gradient(
        "citrus-noon",
        [[92, 72, 10], [206, 158, 24], [248, 214, 92]],
        [255, 246, 180],
        [200, 96, 40],
    ),
    space(
        "orion-emission",
        [[8, 6, 18], [58, 22, 48], [128, 52, 84]],
        [255, 140, 160],
        [120, 200, 210],
    ),
    space(
        "carina-hubble",
        [[10, 10, 14], [40, 58, 52], [96, 120, 88]],
        [230, 190, 110],
        [90, 180, 190],
    ),
    space(
        "pleiades-reflection",
        [[6, 8, 20], [24, 40, 86], [70, 110, 180]],
        [160, 200, 255],
        [210, 230, 255],
    ),
    space(
        "rho-ophiuchi",
        [[14, 8, 10], [70, 50, 30], [130, 90, 120]],
        [255, 190, 90],
        [110, 150, 230],
    ),
    space(
        "milkyway-core",
        [[10, 8, 8], [50, 38, 28], [140, 110, 80]],
        [255, 220, 170],
        [255, 170, 120],
    ),
    space(
        "andromeda-haze",
        [[8, 8, 14], [40, 36, 50], [110, 100, 110]],
        [255, 225, 180],
        [130, 160, 220],
    ),
    space(
        "horsehead-flame",
        [[12, 6, 8], [60, 16, 28], [140, 40, 60]],
        [235, 90, 110],
        [255, 180, 110],
    ),
    space(
        "lagoon-trifid",
        [[10, 6, 14], [64, 24, 54], [150, 70, 110]],
        [255, 150, 190],
        [130, 170, 240],
    ),
    space(
        "eagle-pillars",
        [[10, 8, 16], [52, 40, 36], [124, 96, 70]],
        [240, 200, 140],
        [110, 170, 200],
    ),
    space(
        "crab-remnant",
        [[10, 8, 14], [54, 30, 58], [126, 64, 96]],
        [220, 120, 230],
        [120, 220, 200],
    ),
    space(
        "tarantula-web",
        [[8, 10, 16], [38, 52, 66], [96, 120, 140]],
        [255, 170, 140],
        [150, 200, 255],
    ),
    space(
        "sombrero-dust",
        [[12, 10, 10], [48, 42, 36], [128, 112, 88]],
        [255, 232, 190],
        [150, 170, 210],
    ),
    pattern(
        "tartan-moss",
        [[18, 26, 20], [34, 48, 36], [96, 124, 86]],
        [214, 178, 96],
        [140, 60, 52],
    ),
    pattern(
        "oxford-navy",
        [[16, 22, 38], [28, 38, 62], [78, 100, 150]],
        [220, 228, 240],
        [180, 60, 70],
    ),
    pattern(
        "blueprint",
        [[10, 32, 64], [16, 46, 88], [140, 190, 240]],
        [235, 245, 255],
        [90, 150, 210],
    ),
    pattern(
        "ledger-cream",
        [[238, 232, 214], [228, 220, 198], [70, 80, 96]],
        [176, 70, 60],
        [110, 140, 120],
    ),
    pattern(
        "picnic-red",
        [[236, 226, 214], [226, 212, 196], [178, 44, 44]],
        [250, 244, 236],
        [120, 30, 30],
    ),
    pattern(
        "workshop-ochre",
        [[40, 32, 22], [58, 46, 30], [162, 120, 54]],
        [232, 196, 120],
        [96, 112, 88],
    ),
    // Twilight, straight off a blue-hour frame: navy at both ends with a
    // lighter band through the middle. The quietest sky palette, and the one
    // that sits behind a screenshot most politely.
    sky(
        "blue-hour",
        [[0, 33, 87], [1, 79, 123], [0, 43, 88]],
        [127, 168, 196],
        [0, 22, 58],
    ),
    // Warm light rimming a dark cloud mass. The rim is the effect, so `glow_a`
    // runs much brighter than the stops.
    sky(
        "golden-hour",
        [[38, 67, 78], [136, 129, 115], [233, 200, 157]],
        [250, 219, 168],
        [43, 46, 37],
    ),
    // Deep navy crown over an underlit anvil, base near black.
    sky(
        "supercell",
        [[13, 34, 62], [23, 48, 88], [216, 193, 169]],
        [247, 228, 193],
        [2, 3, 1],
    ),
    // Gray-violet pouches. The narrowest range in the table on purpose:
    // mammatus is a low-contrast condition and reads wrong if pushed.
    sky(
        "mammatus",
        [[156, 152, 164], [136, 129, 143], [111, 106, 118]],
        [187, 176, 182],
        [85, 80, 92],
    ),
    // Teal emission with a green-white fringe and deep blue on the flank.
    sky(
        "aurora-curtain",
        [[18, 49, 79], [1, 127, 138], [5, 114, 139]],
        [100, 212, 185],
        [10, 62, 87],
    ),
    // High ice on open blue; the only sky palette that runs light at the base.
    sky(
        "cirrus-blue",
        [[94, 141, 194], [131, 178, 226], [201, 227, 248]],
        [255, 255, 255],
        [127, 160, 189],
    ),
    // Magenta alpenglow in a narrow mid band against near black. Kept narrow
    // deliberately: widen the band and it turns garish.
    sky(
        "alpenglow",
        [[23, 31, 48], [116, 38, 69], [12, 13, 17]],
        [160, 36, 71],
        [15, 20, 27],
    ),
    // Dawn desert: pale sky over warm sand, deepening to rust in the dune
    // shadow. The quietest terrain palette and the one that sits behind a
    // screenshot most politely.
    terrain(
        "dunes",
        [[196, 168, 120], [168, 122, 64], [74, 50, 28]],
        [248, 214, 150],
        [60, 36, 20],
    ),
    // Colorado plateau: dusty sky over mesa rock, talus shadow at the foot.
    terrain(
        "mesa",
        [[180, 150, 130], [150, 86, 54], [92, 56, 40]],
        [240, 180, 120],
        [70, 40, 30],
    ),
    // Eroded sediment: pale sky over banded ochre and gray, gully shadow.
    terrain(
        "badlands",
        [[170, 150, 120], [120, 96, 72], [86, 70, 60]],
        [220, 190, 140],
        [60, 48, 40],
    ),
    // Ice field: cold sky over bright ice, crevasse blue in the shadow.
    terrain(
        "glacier",
        [[150, 178, 200], [200, 222, 240], [70, 110, 150]],
        [255, 255, 255],
        [40, 80, 120],
    ),
];

#[derive(Debug, Clone)]
pub struct PresentationStyle {
    pub seed: u64,
    pub palette_name: String,
    pub backdrop: BackdropKind,
    pub stops: [[u8; 3]; 3],
    pub glow_a: [u8; 3],
    pub glow_b: [u8; 3],
    /// Base values tuned for a `REFERENCE_SIZE` capture; scaled at render time.
    pub padding: u32,
    pub corner_radius: u32,
    pub shadow_blur: f32,
    pub shadow_offset_y: i32,
    pub gradient_angle: f32,
    pub streak_angle: f32,
    pub streak_phase: f32,
    pub glow_a_pos: (f32, f32),
    pub glow_b_pos: (f32, f32),
    /// Optional pinned space scene; `None` lets the seed pick at random.
    pub scene: Option<crate::space::SceneKind>,
    /// Optional pinned pattern motif; `None` lets the seed pick at random.
    pub motif: Option<crate::pattern::MotifKind>,
    /// Optional pinned sky condition; `None` lets the seed pick at random.
    pub sky: Option<crate::sky::SkyKind>,
    /// Optional pinned terrain kind; `None` lets the seed pick at random.
    pub terrain: Option<crate::terrain::TerrainKind>,
}

pub fn random_style() -> PresentationStyle {
    style_from_seed(random_seed())
}

/// All scene names accepted by `--scene` / [`scene_from_name`], in menu order.
pub fn scene_names() -> Vec<&'static str> {
    crate::space::SceneKind::NAMES.to_vec()
}

/// Parse a `--scene` value into a [`crate::space::SceneKind`].
pub fn scene_from_name(name: &str) -> Option<crate::space::SceneKind> {
    crate::space::SceneKind::from_name(name)
}

/// All motif names accepted by `--motif` / [`motif_from_name`], in menu order.
pub fn motif_names() -> Vec<&'static str> {
    crate::pattern::MotifKind::NAMES.to_vec()
}

/// Parse a `--motif` value into a [`crate::pattern::MotifKind`].
pub fn motif_from_name(name: &str) -> Option<crate::pattern::MotifKind> {
    crate::pattern::MotifKind::from_name(name)
}

/// All sky names accepted by `--sky` / [`sky_from_name`], in menu order.
pub fn sky_names() -> Vec<&'static str> {
    crate::sky::SkyKind::NAMES.to_vec()
}

/// Parse a `--sky` value into a [`crate::sky::SkyKind`].
pub fn sky_from_name(name: &str) -> Option<crate::sky::SkyKind> {
    crate::sky::SkyKind::from_name(name)
}

/// All terrain names accepted by `--terrain` / [`terrain_from_name`], in menu
/// order.
pub fn terrain_names() -> Vec<&'static str> {
    crate::terrain::TerrainKind::NAMES.to_vec()
}

/// Parse a `--terrain` value into a [`crate::terrain::TerrainKind`].
pub fn terrain_from_name(name: &str) -> Option<crate::terrain::TerrainKind> {
    crate::terrain::TerrainKind::from_name(name)
}

impl PresentationStyle {
    /// Whether this style paints a procedural space scene.
    pub fn is_space(&self) -> bool {
        self.backdrop == BackdropKind::Space
    }

    /// Whether this style paints a geometric pattern.
    pub fn is_pattern(&self) -> bool {
        self.backdrop == BackdropKind::Pattern
    }

    /// Whether this style paints a procedural sky.
    pub fn is_sky(&self) -> bool {
        self.backdrop == BackdropKind::Sky
    }

    /// Whether this style paints a procedural terrain scene.
    pub fn is_terrain(&self) -> bool {
        self.backdrop == BackdropKind::Terrain
    }
}

pub fn random_seed() -> u64 {
    rand::rng().random()
}

/// All palette names accepted by `style_with_palette`, in table order.
pub fn palette_names() -> Vec<&'static str> {
    PALETTES.iter().map(|palette| palette.name).collect()
}

/// Every palette paired with its backdrop kind (`gradient` or `space`), for
/// menus that need to group the two families.
pub fn palette_catalog() -> Vec<(&'static str, &'static str)> {
    PALETTES
        .iter()
        .map(|palette| {
            let kind = match palette.kind {
                BackdropKind::Gradient => "gradient",
                BackdropKind::Space => "space",
                BackdropKind::Pattern => "pattern",
                BackdropKind::Sky => "sky",
                BackdropKind::Terrain => "terrain",
            };
            (palette.name, kind)
        })
        .collect()
}

/// Seeded style with the gradient palette pinned to `palette_name` instead of
/// the seed's random pick. Returns `None` for an unknown palette name.
pub fn style_with_palette(seed: u64, palette_name: &str) -> Option<PresentationStyle> {
    let palette = PALETTES
        .iter()
        .find(|palette| palette.name == palette_name)?;
    let mut style = style_from_seed(seed);
    style.palette_name = palette.name.to_string();
    style.backdrop = palette.kind;
    style.stops = palette.stops;
    style.glow_a = palette.glow_a;
    style.glow_b = palette.glow_b;
    Some(style)
}

pub fn style_from_seed(seed: u64) -> PresentationStyle {
    style_from_seed_in_pool(seed, &[], &[], &[], &[], &[])
}

/// FNV-1a, hand-rolled because `DefaultHasher` is explicitly not stable across
/// Rust releases and a backdrop that changes when the toolchain updates is not
/// reproducible.
fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// Score a candidate name against a seed. Highest score wins.
fn affinity(seed: u64, name: &str) -> u64 {
    fnv1a(name.as_bytes(), fnv1a(&seed.to_le_bytes(), FNV_OFFSET))
}

/// Rendezvous selection: score every candidate against the seed and take the
/// best. Indexing by `rng.random_range(0..pool.len())` makes the pool length
/// the modulus, so adding one palette reshuffles what every existing seed
/// renders. Scoring by name means a newcomer only takes the seeds it actually
/// wins, leaving the rest of the table alone, which is what makes the palette
/// list safe to grow. Ties break on the name so the result is total.
fn pick_by_affinity<'a>(seed: u64, names: &[&'a str]) -> Option<&'a str> {
    names
        .iter()
        .copied()
        .max_by_key(|name| (affinity(seed, name), *name))
}

/// Seeded style whose random picks are confined to the caller's allow-lists.
///
/// An empty list means "no constraint", so `style_from_seed_in_pool(seed, &[],
/// &[])` reproduces the pre-pool style for that seed byte for byte. Names the
/// tables do not know are ignored here; validating and reporting them belongs
/// to the caller that owns the user-facing warnings (see `config.rs`). If that
/// leaves no palette at all, the built-in space pool is used rather than
/// failing a capture over a preferences typo.
pub fn style_from_seed_in_pool(
    seed: u64,
    palettes: &[String],
    scenes: &[String],
    motifs: &[String],
    skies: &[String],
    terrains: &[String],
) -> PresentationStyle {
    let allowed: Vec<&Palette> = PALETTES
        .iter()
        .filter(|palette| palettes.iter().any(|name| name == palette.name))
        .collect();
    // Random rotation stays space-only by default; the gradient and pattern
    // palettes are reachable by explicit `--palette` name or by naming them in
    // the pool.
    let pool: Vec<&Palette> = if allowed.is_empty() {
        PALETTES
            .iter()
            .filter(|palette| palette.kind == BackdropKind::Space)
            .collect()
    } else {
        allowed
    };
    let names: Vec<&str> = pool.iter().map(|palette| palette.name).collect();
    let chosen = pick_by_affinity(seed, &names).unwrap_or(PALETTES[0].name);
    let palette = PALETTES
        .iter()
        .find(|palette| palette.name == chosen)
        .unwrap_or(&PALETTES[0]);

    // The palette pick no longer draws from this rng, so the style fields are
    // whatever the seed produces on a fresh stream.
    let mut rng = StdRng::seed_from_u64(seed);
    let mut style = PresentationStyle {
        seed,
        palette_name: palette.name.to_string(),
        backdrop: palette.kind,
        stops: palette.stops,
        glow_a: palette.glow_a,
        glow_b: palette.glow_b,
        padding: rng.random_range(58..=78),
        corner_radius: rng.random_range(18..=26),
        shadow_blur: rng.random_range(22.0..=34.0),
        shadow_offset_y: rng.random_range(16..=26),
        gradient_angle: rng.random_range(0.35..=1.15),
        streak_angle: rng.random_range(0.5..=1.05),
        streak_phase: rng.random_range(0.0..=std::f32::consts::TAU),
        glow_a_pos: (rng.random_range(0.55..=0.95), rng.random_range(0.0..=0.22)),
        glow_b_pos: (rng.random_range(0.05..=0.45), rng.random_range(0.72..=1.0)),
        scene: None,
        motif: None,
        sky: None,
        terrain: None,
    };

    // A pooled second axis is pinned here rather than filtered inside the
    // renderer: both `space.rs` and `pattern.rs` blend their own rolls, so
    // filtering there would bias the pool instead of confining it.
    if style.is_space() && !scenes.is_empty() {
        let allowed: Vec<&str> = crate::space::SceneKind::NAMES
            .iter()
            .copied()
            .filter(|known| scenes.iter().any(|name| name == known))
            .collect();
        style.scene = pick_by_affinity(seed, &allowed).and_then(scene_from_name);
    }
    if style.is_pattern() && !motifs.is_empty() {
        let allowed: Vec<&str> = crate::pattern::MotifKind::NAMES
            .iter()
            .copied()
            .filter(|known| motifs.iter().any(|name| name == known))
            .collect();
        style.motif = pick_by_affinity(seed, &allowed).and_then(motif_from_name);
    }
    if style.is_sky() && !skies.is_empty() {
        let allowed: Vec<&str> = crate::sky::SkyKind::NAMES
            .iter()
            .copied()
            .filter(|known| skies.iter().any(|name| name == known))
            .collect();
        style.sky = pick_by_affinity(seed, &allowed).and_then(sky_from_name);
    }
    if style.is_terrain() && !terrains.is_empty() {
        let allowed: Vec<&str> = crate::terrain::TerrainKind::NAMES
            .iter()
            .copied()
            .filter(|known| terrains.iter().any(|name| name == known))
            .collect();
        style.terrain = pick_by_affinity(seed, &allowed).and_then(terrain_from_name);
    }
    style
}

pub fn render_codex_card(
    input_path: &Path,
    output_path: &Path,
    frame_extents: Option<FrameExtents>,
    style: &PresentationStyle,
) -> Result<(), AppError> {
    let mut input = image::open(input_path).map_err(|source| AppError::Image {
        path: input_path.to_path_buf(),
        source,
    })?;
    if let Some(extents) = frame_extents {
        input = crop_frame_extents(input, extents);
    }
    let canvas = compose_card(&input, style);
    canvas.save(output_path).map_err(|source| AppError::Image {
        path: output_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Pure composition step: rounded window on a styled backdrop with layered
/// shadows and a rim highlight. The outer canvas is a fully opaque rectangle
/// (like a Codex appshot) so the card survives JPEG re-encoding and pasting
/// into apps that flatten alpha on white; only the inner window is rounded.
pub fn compose_card(input: &DynamicImage, style: &PresentationStyle) -> RgbaImage {
    let (window_width, window_height) = input.dimensions();
    let scale = (window_width.min(window_height) as f32 / REFERENCE_SIZE).clamp(0.7, 3.0);
    let padding = (style.padding as f32 * scale).round() as u32;
    let card_radius = ((style.corner_radius as f32 * scale).round() as u32).max(6);
    let rim_width = (1.6 * scale).clamp(1.0, 4.0);

    let window = rounded_window(input, card_radius, rim_width);
    let canvas_width = window_width + padding * 2;
    let canvas_height = window_height + padding * 2;
    let mut canvas = backdrop(canvas_width, canvas_height, style);

    let key_offset = (style.shadow_offset_y as f32 * scale).round() as i32;
    let ambient_offset = (style.shadow_offset_y as f32 * 1.8 * scale).round() as i32;
    let key_sigma = style.shadow_blur * scale;
    let ambient_sigma = style.shadow_blur * 2.4 * scale;

    let ambient = soft_shadow_layer(
        canvas_width,
        canvas_height,
        padding as i32,
        padding as i32 + ambient_offset,
        window_width,
        window_height,
        card_radius,
        ambient_sigma,
        AMBIENT_SHADOW_ALPHA,
    );
    alpha_composite(&mut canvas, &ambient, 0, 0);
    let key = soft_shadow_layer(
        canvas_width,
        canvas_height,
        padding as i32,
        padding as i32 + key_offset,
        window_width,
        window_height,
        card_radius,
        key_sigma,
        KEY_SHADOW_ALPHA,
    );
    alpha_composite(&mut canvas, &key, 0, 0);
    alpha_composite(&mut canvas, &window, padding as i32, padding as i32);
    canvas
}

impl PresentationStyle {
    pub fn info(&self) -> PresentationStyleInfo {
        PresentationStyleInfo {
            seed: self.seed,
            palette: self.palette_name.clone(),
            padding: self.padding,
            corner_radius: self.corner_radius,
            shadow_blur: self.shadow_blur,
            shadow_offset_y: self.shadow_offset_y,
        }
    }
}

fn crop_frame_extents(input: DynamicImage, extents: FrameExtents) -> DynamicImage {
    let (width, height) = input.dimensions();
    let horizontal = extents.left.saturating_add(extents.right);
    let vertical = extents.top.saturating_add(extents.bottom);
    if horizontal >= width || vertical >= height {
        return input;
    }
    input.crop_imm(
        extents.left,
        extents.top,
        width - horizontal,
        height - vertical,
    )
}

fn rounded_window(input: &DynamicImage, radius: u32, rim_width: f32) -> RgbaImage {
    let mut image = input.to_rgba8();
    let (width, height) = image.dimensions();
    for y in 0..height {
        for x in 0..width {
            let distance = inner_distance(x, y, width, height, radius);
            let coverage = (distance + 0.5).clamp(0.0, 1.0);
            let pixel = image.get_pixel_mut(x, y);
            if coverage < 1.0 {
                pixel.0[3] = ((f32::from(pixel.0[3]) * coverage).round()) as u8;
            }
            // Subtle rim highlight just inside the card edge.
            if distance > -0.5 && distance < rim_width + 1.0 {
                let band = if distance <= rim_width {
                    coverage
                } else {
                    (rim_width + 1.0 - distance).clamp(0.0, 1.0) * coverage
                };
                let amount = RIM_STRENGTH * band;
                for channel in 0..3 {
                    let value = f32::from(pixel.0[channel]);
                    pixel.0[channel] = (value + (255.0 - value) * amount).round() as u8;
                }
            }
        }
    }
    image
}

/// Signed distance to the inside of a rounded rectangle covering the full
/// `width` x `height` area. Positive inside, negative outside.
fn inner_distance(x: u32, y: u32, width: u32, height: u32, radius: u32) -> f32 {
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;
    let radius = (radius as f32).min(half_width).min(half_height);
    let px = (x as f32 + 0.5 - half_width).abs();
    let py = (y as f32 + 0.5 - half_height).abs();
    let qx = px - (half_width - radius);
    let qy = py - (half_height - radius);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius;
    -outside
}

fn rounded_alpha(x: u32, y: u32, width: u32, height: u32, radius: u32) -> u8 {
    let coverage = (inner_distance(x, y, width, height, radius) + 0.5).clamp(0.0, 1.0);
    (coverage * 255.0).round() as u8
}

#[allow(clippy::too_many_arguments)]
fn soft_shadow_layer(
    canvas_width: u32,
    canvas_height: u32,
    rect_x: i32,
    rect_y: i32,
    rect_width: u32,
    rect_height: u32,
    radius: u32,
    sigma: f32,
    alpha: u8,
) -> RgbaImage {
    // Blur at reduced resolution, then upscale: visually identical for a
    // soft shadow and far cheaper than a full-resolution gaussian pass.
    let small_width = canvas_width.div_ceil(SHADOW_DOWNSCALE).max(1);
    let small_height = canvas_height.div_ceil(SHADOW_DOWNSCALE).max(1);
    let mut mask = RgbaImage::from_pixel(small_width, small_height, Rgba([0, 0, 0, 0]));
    for y in 0..small_height {
        for x in 0..small_width {
            let full_x = (x * SHADOW_DOWNSCALE) as i32 - rect_x;
            let full_y = (y * SHADOW_DOWNSCALE) as i32 - rect_y;
            if full_x < 0
                || full_y < 0
                || full_x >= rect_width as i32
                || full_y >= rect_height as i32
            {
                continue;
            }
            let coverage = rounded_alpha(
                full_x as u32,
                full_y as u32,
                rect_width,
                rect_height,
                radius,
            );
            if coverage == 0 {
                continue;
            }
            let shadow_alpha = ((u16::from(coverage) * u16::from(alpha)) / 255) as u8;
            mask.put_pixel(x, y, Rgba([0, 0, 0, shadow_alpha]));
        }
    }
    let blurred = imageops::blur(&mask, (sigma / SHADOW_DOWNSCALE as f32).max(0.5));
    imageops::resize(
        &blurred,
        canvas_width,
        canvas_height,
        imageops::FilterType::Triangle,
    )
}

/// Paint just the backdrop at an arbitrary size, with no card on top.
///
/// A finished card is only about 4% backdrop by width (padding scales with the
/// input, so that ratio holds at every size), which makes whole cards useless
/// as swatches in a picker. This is the same painting `compose_card` sits a
/// card on, so a swatch and the real thing agree.
pub fn render_backdrop(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    backdrop(width.max(1), height.max(1), style)
}

fn backdrop(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    match style.backdrop {
        BackdropKind::Space => return crate::space::render(width, height, style),
        BackdropKind::Pattern => return crate::pattern::render(width, height, style),
        BackdropKind::Sky => return crate::sky::render(width, height, style),
        BackdropKind::Terrain => return crate::terrain::render(width, height, style),
        BackdropKind::Gradient => {}
    }
    let stops = style.stops.map(to_f32);
    let glow_a = to_f32(style.glow_a);
    let glow_b = to_f32(style.glow_b);
    let (gradient_cos, gradient_sin) = (style.gradient_angle.cos(), style.gradient_angle.sin());
    let gradient_norm = (gradient_cos + gradient_sin).max(f32::EPSILON);
    let (streak_cos, streak_sin) = (style.streak_angle.cos(), style.streak_angle.sin());
    ImageBuffer::from_fn(width, height, |x, y| {
        let fx = x as f32 / width.max(1) as f32;
        let fy = y as f32 / height.max(1) as f32;
        let t = ((fx * gradient_cos + fy * gradient_sin) / gradient_norm).clamp(0.0, 1.0);
        let mut color = if t < 0.5 {
            mix3(stops[0], stops[1], smoothstep(t * 2.0))
        } else {
            mix3(stops[1], stops[2], smoothstep((t - 0.5) * 2.0))
        };

        let glow_a_distance =
            ((fx - style.glow_a_pos.0).powi(2) + (fy - style.glow_a_pos.1).powi(2)).sqrt();
        color = mix3(
            color,
            glow_a,
            (1.0 - glow_a_distance / 0.85).clamp(0.0, 1.0).powi(2) * 0.55,
        );
        let glow_b_distance =
            ((fx - style.glow_b_pos.0).powi(2) + (fy - style.glow_b_pos.1).powi(2)).sqrt();
        color = mix3(
            color,
            glow_b,
            (1.0 - glow_b_distance / 0.9).clamp(0.0, 1.0).powi(2) * 0.48,
        );

        // Broad diagonal light streaks, like soft window light.
        let band = fx * streak_cos + fy * streak_sin;
        let streak = (band * 17.0 + style.streak_phase).sin() * 0.62
            + (band * 29.0 + style.streak_phase * 1.7).sin() * 0.38;
        if streak > 0.0 {
            color = mix3(color, [255.0, 255.0, 255.0], streak * STREAK_STRENGTH);
        } else {
            let dim = 1.0 + streak * 0.05;
            color = [color[0] * dim, color[1] * dim, color[2] * dim];
        }

        // Fine grain breaks up gradient banding.
        let grain = grain_noise(x, y, style.seed) * GRAIN_STRENGTH;
        Rgba([
            quantize(color[0] + grain),
            quantize(color[1] + grain),
            quantize(color[2] + grain),
            255,
        ])
    })
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

fn alpha_composite(base: &mut RgbaImage, overlay: &RgbaImage, offset_x: i32, offset_y: i32) {
    let (base_width, base_height) = base.dimensions();
    for y in 0..overlay.height() {
        for x in 0..overlay.width() {
            let target_x = offset_x + x as i32;
            let target_y = offset_y + y as i32;
            if target_x < 0 || target_y < 0 {
                continue;
            }
            let target_x = target_x as u32;
            let target_y = target_y as u32;
            if target_x >= base_width || target_y >= base_height {
                continue;
            }
            let src = overlay.get_pixel(x, y);
            let alpha = f32::from(src.0[3]) / 255.0;
            if alpha == 0.0 {
                continue;
            }
            let dst = base.get_pixel(target_x, target_y);
            let inv_alpha = 1.0 - alpha;
            let out = Rgba([
                (f32::from(src.0[0]) * alpha + f32::from(dst.0[0]) * inv_alpha).round() as u8,
                (f32::from(src.0[1]) * alpha + f32::from(dst.0[1]) * inv_alpha).round() as u8,
                (f32::from(src.0[2]) * alpha + f32::from(dst.0[2]) * inv_alpha).round() as u8,
                255,
            ]);
            base.put_pixel(target_x, target_y, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_input(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([200, 200, 200, 255]),
        ))
    }

    #[test]
    fn card_is_fully_opaque_everywhere() {
        // Every pixel must be opaque so the card survives JPEG re-encoding and
        // pasting into apps that composite alpha on white (Discord, LinkedIn).
        // The four outer corners were the regression: a rounded transparent
        // canvas rendered white the moment alpha was flattened.
        let canvas = compose_card(&test_input(400, 300), &style_from_seed(7));
        let (width, height) = canvas.dimensions();
        for &(x, y) in &[
            (0, 0),
            (width - 1, 0),
            (0, height - 1),
            (width - 1, height - 1),
            (width / 2, height / 2),
        ] {
            assert_eq!(
                canvas.get_pixel(x, y).0[3],
                255,
                "pixel ({x},{y}) must be opaque"
            );
        }
    }

    #[test]
    fn canvas_adds_scaled_padding() {
        let style = style_from_seed(7);
        let canvas = compose_card(&test_input(400, 300), &style);
        let scale = (300.0_f32 / REFERENCE_SIZE).clamp(0.7, 3.0);
        let padding = (style.padding as f32 * scale).round() as u32;
        assert_eq!(canvas.dimensions(), (400 + padding * 2, 300 + padding * 2));
    }

    #[test]
    fn backdrop_varies_across_canvas() {
        let style = style_from_seed(11);
        let canvas = backdrop(320, 240, &style);
        let a = canvas.get_pixel(8, 8);
        let b = canvas.get_pixel(311, 231);
        assert_ne!(a.0[..3], b.0[..3]);
    }

    #[test]
    fn same_seed_renders_identically() {
        let input = test_input(200, 160);
        let first = compose_card(&input, &style_from_seed(42));
        let second = compose_card(&input, &style_from_seed(42));
        assert_eq!(first.as_raw(), second.as_raw());
    }

    #[test]
    fn random_rotation_only_picks_space_palettes() {
        for seed in 0..64 {
            assert_eq!(
                style_from_seed(seed).backdrop,
                BackdropKind::Space,
                "seed {seed} left the space rotation"
            );
        }
    }

    #[test]
    fn every_seed_palette_is_known() {
        for seed in 0..32 {
            let style = style_from_seed(seed);
            assert!(
                PALETTES
                    .iter()
                    .any(|palette| palette.name == style.palette_name)
            );
        }
    }

    #[test]
    fn palette_names_match_palette_table() {
        let names = palette_names();
        assert_eq!(names.len(), PALETTES.len());
        for palette in &PALETTES {
            assert!(names.contains(&palette.name));
        }
    }

    #[test]
    fn style_with_palette_pins_the_named_palette() {
        let palette = &PALETTES[1];
        let style = style_with_palette(9, "ember-glow").expect("known palette");
        assert_eq!(style.palette_name, "ember-glow");
        assert_eq!(style.stops, palette.stops);
        assert_eq!(style.glow_a, palette.glow_a);
        assert_eq!(style.glow_b, palette.glow_b);
        // Everything except the palette still derives from the seed.
        let base = style_from_seed(9);
        assert_eq!(style.padding, base.padding);
        assert_eq!(style.gradient_angle, base.gradient_angle);
    }

    #[test]
    fn style_with_palette_is_deterministic() {
        let first = style_with_palette(42, "midnight-sky").expect("known palette");
        let second = style_with_palette(42, "midnight-sky").expect("known palette");
        let input = test_input(200, 160);
        assert_eq!(
            compose_card(&input, &first).as_raw(),
            compose_card(&input, &second).as_raw()
        );
    }

    #[test]
    fn style_with_unknown_palette_returns_none() {
        assert!(style_with_palette(1, "hotdog-stand").is_none());
    }

    #[test]
    fn empty_pools_reproduce_the_unconstrained_style() {
        // The pool argument must not change the result for a given seed.
        let input = test_input(160, 120);
        for seed in 0..16 {
            let base = style_from_seed_in_pool(seed, &[], &[], &[], &[], &[]);
            let pooled = style_from_seed(seed);
            assert_eq!(pooled.palette_name, base.palette_name, "seed {seed}");
            assert_eq!(pooled.scene, None, "seed {seed}");
            assert_eq!(
                compose_card(&input, &pooled).as_raw(),
                compose_card(&input, &base).as_raw(),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn palette_pool_confines_the_random_pick() {
        let pool = vec!["aurora-teal".to_string(), "lagoon-trifid".to_string()];
        let mut seen: Vec<String> = Vec::new();
        for seed in 0..64 {
            let style = style_from_seed_in_pool(seed, &pool, &[], &[], &[], &[]);
            assert!(
                pool.contains(&style.palette_name),
                "seed {seed} escaped the pool with {}",
                style.palette_name
            );
            if !seen.contains(&style.palette_name) {
                seen.push(style.palette_name.clone());
            }
        }
        // Both entries should turn up, otherwise the pool is not really random.
        assert_eq!(seen.len(), 2, "seen: {seen:?}");
    }

    #[test]
    fn palette_pool_can_reach_gradient_palettes() {
        // The default rotation is space-only, so an explicit gradient pool is
        // the only way back to the legacy look without naming it per run.
        let pool = vec!["ember-glow".to_string()];
        let style = style_from_seed_in_pool(5, &pool, &[], &[], &[], &[]);
        assert_eq!(style.palette_name, "ember-glow");
        assert!(!style.is_space());
    }

    #[test]
    fn scene_pool_pins_a_scene_from_the_pool() {
        let scenes = vec!["alma".to_string(), "veil".to_string()];
        let allowed = [scene_from_name("alma"), scene_from_name("veil")];
        for seed in 0..32 {
            let style = style_from_seed_in_pool(seed, &[], &scenes, &[], &[], &[]);
            assert!(
                allowed.contains(&style.scene),
                "seed {seed} produced {:?}",
                style.scene
            );
        }
    }

    #[test]
    fn scene_pool_is_ignored_for_gradient_palettes() {
        let palettes = vec!["ember-glow".to_string()];
        let scenes = vec!["alma".to_string()];
        let style = style_from_seed_in_pool(3, &palettes, &scenes, &[], &[], &[]);
        assert_eq!(style.scene, None);
    }

    #[test]
    fn unknown_pool_names_fall_back_to_the_default_pool() {
        let palettes = vec!["hotdog-stand".to_string()];
        let style = style_from_seed_in_pool(11, &palettes, &[], &[], &[], &[]);
        assert_eq!(style.palette_name, style_from_seed(11).palette_name);
        assert!(style.is_space());
    }

    #[test]
    fn render_backdrop_matches_the_card_it_would_sit_under() {
        // A swatch that drifts from the real card is worse than no swatch.
        let style = style_with_palette(9, "orion-emission").expect("known palette");
        let swatch = render_backdrop(120, 90, &style);
        let same = backdrop(120, 90, &style);
        assert_eq!(swatch.as_raw(), same.as_raw());
    }

    #[test]
    fn render_backdrop_is_deterministic_and_seed_sensitive() {
        let a = style_with_palette(9, "carina-hubble").expect("known palette");
        let b = style_with_palette(10, "carina-hubble").expect("known palette");
        assert_eq!(
            render_backdrop(96, 72, &a).as_raw(),
            render_backdrop(96, 72, &a).as_raw(),
            "same seed must repeat"
        );
        assert_ne!(
            render_backdrop(96, 72, &a).as_raw(),
            render_backdrop(96, 72, &b).as_raw(),
            "a different seed must paint a different sky"
        );
    }

    #[test]
    fn render_backdrop_survives_a_zero_dimension() {
        // Width and height come off a query string in the studio server.
        let style = style_from_seed(4);
        let image = render_backdrop(0, 0, &style);
        assert_eq!(image.dimensions(), (1, 1));
    }

    #[test]
    fn render_backdrop_distinguishes_palettes_where_whole_cards_do_not() {
        // The point of the whole function: at swatch size, two palettes must
        // not look alike.
        let a = render_backdrop(
            64,
            48,
            &style_with_palette(3, "ember-glow").expect("palette"),
        );
        let b = render_backdrop(
            64,
            48,
            &style_with_palette(3, "aurora-teal").expect("palette"),
        );
        assert_ne!(a.as_raw(), b.as_raw());
    }

    #[test]
    fn growing_the_table_leaves_most_seeds_alone() {
        // The point of scoring by name instead of indexing by position: adding
        // a palette must only take the seeds it actually wins. With modulo
        // indexing this number would be near zero.
        let current: Vec<String> = PALETTES
            .iter()
            .filter(|palette| palette.kind == BackdropKind::Space)
            .map(|palette| palette.name.to_string())
            .collect();
        let mut grown = current.clone();
        grown.push("a-brand-new-sky".to_string());

        let mut unchanged = 0;
        let total = 600;
        for seed in 0..total {
            let before = pick_by_affinity(
                seed,
                &current.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            let after =
                pick_by_affinity(seed, &grown.iter().map(String::as_str).collect::<Vec<_>>());
            if before == after {
                unchanged += 1;
            }
        }
        // One new entry among 13 should disturb roughly 1/13 of seeds.
        assert!(
            unchanged as f64 / total as f64 > 0.85,
            "only {unchanged}/{total} seeds kept their palette"
        );
    }

    #[test]
    fn affinity_selection_spreads_across_the_pool() {
        // A hash that favoured one name would make the picker look broken.
        let names: Vec<&str> = PALETTES
            .iter()
            .filter(|palette| palette.kind == BackdropKind::Space)
            .map(|palette| palette.name)
            .collect();
        let mut seen: Vec<&str> = Vec::new();
        for seed in 0..400 {
            let pick = pick_by_affinity(seed, &names).expect("pool");
            if !seen.contains(&pick) {
                seen.push(pick);
            }
        }
        assert_eq!(
            seen.len(),
            names.len(),
            "unreached: {:?}",
            names.len() - seen.len()
        );
    }

    #[test]
    fn every_space_palette_is_reachable_by_the_default_picker() {
        // "all the space palettes are available" as an assertion, not a hope.
        let expected: Vec<&str> = PALETTES
            .iter()
            .filter(|palette| palette.kind == BackdropKind::Space)
            .map(|palette| palette.name)
            .collect();
        let mut seen: Vec<String> = Vec::new();
        for seed in 0..500 {
            let name = style_from_seed(seed).palette_name;
            if !seen.contains(&name) {
                seen.push(name);
            }
        }
        assert_eq!(seen.len(), expected.len(), "reached {seen:?}");
    }

    #[test]
    fn pattern_palettes_render_as_patterns() {
        let style = style_with_palette(4, "blueprint").expect("pattern palette");
        assert!(style.is_pattern());
        assert!(!style.is_space());
        let a = render_backdrop(64, 48, &style);
        let b = render_backdrop(
            64,
            48,
            &style_with_palette(4, "midnight-sky").expect("gradient"),
        );
        assert_ne!(a.as_raw(), b.as_raw());
    }

    #[test]
    fn a_motif_pool_confines_the_pick() {
        let motifs = vec!["plaid".to_string(), "grid".to_string()];
        let palettes = vec!["tartan-moss".to_string()];
        let allowed = [motif_from_name("plaid"), motif_from_name("grid")];
        for seed in 0..40 {
            let style = style_from_seed_in_pool(seed, &palettes, &[], &motifs, &[], &[]);
            assert!(
                allowed.contains(&style.motif),
                "seed {seed}: {:?}",
                style.motif
            );
        }
    }

    #[test]
    fn a_motif_pool_is_ignored_for_non_pattern_palettes() {
        let motifs = vec!["plaid".to_string()];
        let palettes = vec!["orion-emission".to_string()];
        assert_eq!(
            style_from_seed_in_pool(3, &palettes, &[], &motifs, &[], &[]).motif,
            None
        );
    }

    #[test]
    fn the_table_has_all_three_families() {
        let catalog = palette_catalog();
        for kind in ["gradient", "space", "pattern"] {
            let count = catalog.iter().filter(|(_, k)| *k == kind).count();
            assert!(count >= 5, "{kind} has only {count} palettes");
        }
    }

    #[test]
    fn palette_catalog_labels_every_palette() {
        let catalog = palette_catalog();
        assert_eq!(catalog.len(), PALETTES.len());
        for (name, kind) in &catalog {
            let palette = PALETTES
                .iter()
                .find(|palette| palette.name == *name)
                .expect("catalog name is in the table");
            let expected = match palette.kind {
                BackdropKind::Gradient => "gradient",
                BackdropKind::Space => "space",
                BackdropKind::Pattern => "pattern",
                BackdropKind::Sky => "sky",
                BackdropKind::Terrain => "terrain",
            };
            assert_eq!(kind, &expected, "{name}");
        }
    }

    #[test]
    fn inner_distance_sign_matches_geometry() {
        // Center of a 100x100 rect is deep inside.
        assert!(inner_distance(50, 50, 100, 100, 20) > 30.0);
        // The exact corner pixel is outside the rounded corner.
        assert!(inner_distance(0, 0, 100, 100, 20) < 0.0);
        // Edge midpoints sit on the border.
        assert!(inner_distance(50, 0, 100, 100, 20) < 1.0);
    }

    #[test]
    fn terrain_names_lists_the_four_kinds() {
        assert_eq!(
            terrain_names(),
            vec!["dunes", "mesa", "badlands", "glacier"]
        );
    }

    #[test]
    fn terrain_palettes_catalog_as_terrain() {
        let catalog = palette_catalog();
        for name in terrain_names() {
            let entry = catalog
                .iter()
                .find(|(palette, _)| *palette == name)
                .unwrap_or_else(|| panic!("terrain palette {name} missing from catalog"));
            assert_eq!(entry.1, "terrain", "palette {name} cataloged wrong");
        }
    }

    #[test]
    fn random_rotation_never_picks_a_terrain_palette() {
        // The default rotation stays space-only, so terrain palettes are
        // reachable only by explicit `--palette` or by naming them in the pool.
        for seed in 0..64 {
            let style = style_from_seed(seed);
            assert!(
                !style.is_terrain(),
                "seed {seed} leaked a terrain palette into the space rotation"
            );
        }
    }

    #[test]
    fn terrain_pool_pins_only_terrain_kinds() {
        let terrains = vec!["dunes".to_string(), "mesa".to_string()];
        let palettes = vec!["dunes".to_string(), "mesa".to_string()];
        let allowed = [terrain_from_name("dunes"), terrain_from_name("mesa")];
        for seed in 0..40 {
            let style = style_from_seed_in_pool(seed, &palettes, &[], &[], &[], &terrains);
            assert!(
                allowed.contains(&style.terrain),
                "seed {seed} produced {:?}",
                style.terrain
            );
        }
    }

    #[test]
    fn terrain_pool_is_ignored_for_non_terrain_palettes() {
        let palettes = vec!["ember-glow".to_string()];
        let terrains = vec!["dunes".to_string()];
        let style = style_from_seed_in_pool(3, &palettes, &[], &[], &[], &terrains);
        assert_eq!(style.terrain, None);
    }
}

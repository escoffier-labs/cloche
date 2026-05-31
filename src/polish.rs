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

const SHADOW_ALPHA: u8 = 95;

#[derive(Debug, Clone)]
pub struct PresentationStyle {
    pub seed: u64,
    pub palette_name: String,
    pub start: [u8; 3],
    pub end: [u8; 3],
    pub accent: [u8; 3],
    pub padding: u32,
    pub corner_radius: u32,
    pub shadow_blur: f32,
    pub shadow_offset_y: i32,
}

pub fn random_style() -> PresentationStyle {
    let seed = rand::rng().random();
    style_from_seed(seed)
}

pub fn style_from_seed(seed: u64) -> PresentationStyle {
    let mut rng = StdRng::seed_from_u64(seed);
    let palettes = [
        ("dusk-berry", [34, 40, 78], [178, 48, 104], [118, 79, 178]),
        ("aurora-teal", [15, 77, 87], [62, 148, 126], [165, 212, 141]),
        (
            "graphite-rose",
            [38, 42, 49],
            [158, 64, 91],
            [222, 134, 113],
        ),
        (
            "indigo-copper",
            [31, 45, 92],
            [190, 104, 62],
            [240, 167, 92],
        ),
        ("forest-slate", [23, 65, 55], [73, 88, 103], [129, 160, 126]),
    ];
    let (name, start, end, accent) = palettes[rng.random_range(0..palettes.len())];
    PresentationStyle {
        seed,
        palette_name: name.to_string(),
        start,
        end,
        accent,
        padding: rng.random_range(56..=88),
        corner_radius: rng.random_range(14..=22),
        shadow_blur: rng.random_range(18.0..=30.0),
        shadow_offset_y: rng.random_range(14..=28),
    }
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
    let window = rounded_window(input, style.corner_radius);
    let (window_width, window_height) = window.dimensions();
    let canvas_width = window_width + style.padding * 2;
    let canvas_height = window_height + style.padding * 2 + style.shadow_offset_y as u32;

    let mut canvas = backdrop(canvas_width, canvas_height, style);
    let shadow = shadow_layer(
        window_width,
        window_height,
        canvas_width,
        canvas_height,
        style,
    );
    alpha_composite(&mut canvas, &shadow, 0, 0);
    alpha_composite(
        &mut canvas,
        &window,
        style.padding as i32,
        style.padding as i32,
    );

    canvas.save(output_path).map_err(|source| AppError::Image {
        path: output_path.to_path_buf(),
        source,
    })?;
    Ok(())
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

fn rounded_window(input: DynamicImage, radius: u32) -> RgbaImage {
    let mut image = input.to_rgba8();
    let (width, height) = image.dimensions();
    for y in 0..height {
        for x in 0..width {
            let alpha = rounded_alpha(x, y, width, height, radius);
            if alpha < 255 {
                let pixel = image.get_pixel_mut(x, y);
                pixel.0[3] = ((u16::from(pixel.0[3]) * u16::from(alpha)) / 255) as u8;
            }
        }
    }
    image
}

fn rounded_alpha(x: u32, y: u32, width: u32, height: u32, radius: u32) -> u8 {
    if radius == 0 || width <= radius * 2 || height <= radius * 2 {
        return 255;
    }

    let cx = if x < radius {
        Some(radius as i32)
    } else if x >= width - radius {
        Some((width - radius - 1) as i32)
    } else {
        None
    };
    let cy = if y < radius {
        Some(radius as i32)
    } else if y >= height - radius {
        Some((height - radius - 1) as i32)
    } else {
        None
    };
    let (Some(cx), Some(cy)) = (cx, cy) else {
        return 255;
    };

    let dx = x as i32 - cx;
    let dy = y as i32 - cy;
    let distance = ((dx * dx + dy * dy) as f32).sqrt();
    let edge = radius as f32;
    if distance <= edge - 1.0 {
        255
    } else if distance >= edge {
        0
    } else {
        ((edge - distance) * 255.0).round() as u8
    }
}

fn shadow_layer(
    window_width: u32,
    window_height: u32,
    canvas_width: u32,
    canvas_height: u32,
    style: &PresentationStyle,
) -> RgbaImage {
    let mut mask = RgbaImage::from_pixel(canvas_width, canvas_height, Rgba([0, 0, 0, 0]));
    let shadow_x = style.padding as i32;
    let shadow_y = style.padding as i32 + style.shadow_offset_y;
    for y in 0..window_height {
        for x in 0..window_width {
            let alpha = rounded_alpha(x, y, window_width, window_height, style.corner_radius);
            if alpha == 0 {
                continue;
            }
            let target_x = shadow_x + x as i32;
            let target_y = shadow_y + y as i32;
            if target_x < 0 || target_y < 0 {
                continue;
            }
            let target_x = target_x as u32;
            let target_y = target_y as u32;
            if target_x < canvas_width && target_y < canvas_height {
                let shadow_alpha = ((u16::from(alpha) * u16::from(SHADOW_ALPHA)) / 255) as u8;
                mask.put_pixel(target_x, target_y, Rgba([0, 0, 0, shadow_alpha]));
            }
        }
    }
    imageops::blur(&mask, style.shadow_blur)
}

fn backdrop(width: u32, height: u32, style: &PresentationStyle) -> RgbaImage {
    ImageBuffer::from_fn(width, height, |x, y| {
        let fx = x as f32 / width.max(1) as f32;
        let fy = y as f32 / height.max(1) as f32;
        let diagonal = fx * 0.62 + fy * 0.38;
        let radial = ((fx - 0.78).powi(2) + (fy - 0.18).powi(2)).sqrt();
        let accent_mix = (1.0 - radial * 1.6).clamp(0.0, 0.45);
        let base = mix_rgb(style.start, style.end, diagonal);
        let mixed = mix_rgb(base, style.accent, accent_mix);
        let vignette = 1.0 - (((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt() * 0.18);
        let r = (f32::from(mixed[0]) * vignette).round().clamp(0.0, 255.0) as u8;
        let g = (f32::from(mixed[1]) * vignette).round().clamp(0.0, 255.0) as u8;
        let b = (f32::from(mixed[2]) * vignette).round().clamp(0.0, 255.0) as u8;
        Rgba([r, g, b, 255])
    })
}

fn mix_rgb(start: [u8; 3], end: [u8; 3], amount: f32) -> [u8; 3] {
    [
        lerp(f32::from(start[0]), f32::from(end[0]), amount) as u8,
        lerp(f32::from(start[1]), f32::from(end[1]), amount) as u8,
        lerp(f32::from(start[2]), f32::from(end[2]), amount) as u8,
    ]
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount.clamp(0.0, 1.0)
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

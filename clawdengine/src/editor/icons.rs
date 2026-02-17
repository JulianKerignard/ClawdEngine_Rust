use std::path::Path;

use egui::{ColorImage, Vec2};
use image::{ImageBuffer, Rgba};

const ICON_SIZE: u32 = 48;
const ICONS_DIR: &str = "icons";

pub struct EditorIcons {
    pub folder: egui::TextureHandle,
    pub script: egui::TextureHandle,
    pub mesh: egui::TextureHandle,
    pub file: egui::TextureHandle,
    pub material: egui::TextureHandle,
    pub texture: egui::TextureHandle,
    pub skeleton: egui::TextureHandle,
    pub clip: egui::TextureHandle,
    pub audio: egui::TextureHandle,
    pub scene: egui::TextureHandle,
    pub prefab: egui::TextureHandle,
}

impl EditorIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        ensure_icons_dir();

        Self {
            folder: load_or_generate(ctx, "folder", generate_folder),
            script: load_or_generate(ctx, "script", generate_script),
            mesh: load_or_generate(ctx, "mesh", generate_mesh),
            file: load_or_generate(ctx, "file", generate_file),
            material: load_or_generate(ctx, "material", generate_material),
            texture: load_or_generate(ctx, "texture", generate_texture),
            skeleton: load_or_generate(ctx, "skeleton", generate_skeleton),
            clip: load_or_generate(ctx, "clip", generate_clip),
            audio: load_or_generate(ctx, "audio", generate_audio),
            scene: load_or_generate(ctx, "scene", generate_scene),
            prefab: load_or_generate(ctx, "prefab", generate_prefab),
        }
    }
}

fn ensure_icons_dir() {
    let dir = Path::new(ICONS_DIR);
    if !dir.exists() {
        let _ = std::fs::create_dir_all(dir);
    }
}

fn load_or_generate(
    ctx: &egui::Context,
    name: &str,
    generator: fn() -> ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> egui::TextureHandle {
    let path = Path::new(ICONS_DIR).join(format!("{}.png", name));

    let color_image = if path.exists() {
        load_png(&path)
    } else {
        let img = generator();
        let _ = img.save(&path);
        image_buffer_to_color_image(&img)
    };

    ctx.load_texture(
        format!("icon_{}", name),
        color_image,
        egui::TextureOptions::LINEAR,
    )
}

fn load_png(path: &Path) -> ColorImage {
    let img = image::open(path)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(ICON_SIZE, ICON_SIZE))
        .into_rgba8();
    image_buffer_to_color_image(&img)
}

fn image_buffer_to_color_image(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ColorImage {
    let size = [img.width() as usize, img.height() as usize];
    let pixels: Vec<egui::Color32> = img
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    ColorImage {
        size,
        source_size: Vec2::new(size[0] as f32, size[1] as f32),
        pixels,
    }
}

// ---- Icon generators (48x48 pixel art) ----

fn fill_rect(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x0: u32, y0: u32, w: u32, h: u32, c: [u8; 4]) {
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            if x < ICON_SIZE && y < ICON_SIZE {
                img.put_pixel(x, y, Rgba(c));
            }
        }
    }
}

fn generate_folder() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let gold = [230, 190, 50, 255];
    let dark = [190, 150, 30, 255];
    let light = [250, 215, 90, 255];

    // Folder tab
    fill_rect(&mut img, 4, 8, 18, 6, gold);
    // Tab top highlight
    fill_rect(&mut img, 4, 8, 18, 2, light);
    // Folder body
    fill_rect(&mut img, 4, 14, 40, 26, gold);
    // Body top edge highlight
    fill_rect(&mut img, 4, 14, 40, 2, light);
    // Bottom shadow
    fill_rect(&mut img, 4, 38, 40, 2, dark);
    // Right shadow
    fill_rect(&mut img, 42, 14, 2, 26, dark);

    img
}

fn generate_script() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let blue = [50, 120, 210, 255];
    let dark = [35, 90, 170, 255];
    let white = [240, 240, 240, 255];

    // Page body
    fill_rect(&mut img, 6, 4, 36, 40, blue);
    // Bottom shadow
    fill_rect(&mut img, 6, 42, 36, 2, dark);
    // Right shadow
    fill_rect(&mut img, 40, 4, 2, 40, dark);

    // Corner fold (top-right)
    fill_rect(&mut img, 32, 4, 10, 10, dark);
    fill_rect(&mut img, 32, 4, 8, 8, [70, 140, 230, 255]);

    // "{" symbol
    fill_rect(&mut img, 14, 16, 2, 2, white);
    fill_rect(&mut img, 12, 18, 2, 4, white);
    fill_rect(&mut img, 10, 22, 2, 2, white);
    fill_rect(&mut img, 12, 24, 2, 4, white);
    fill_rect(&mut img, 14, 28, 2, 2, white);

    // "}" symbol
    fill_rect(&mut img, 30, 16, 2, 2, white);
    fill_rect(&mut img, 32, 18, 2, 4, white);
    fill_rect(&mut img, 34, 22, 2, 2, white);
    fill_rect(&mut img, 32, 24, 2, 4, white);
    fill_rect(&mut img, 30, 28, 2, 2, white);

    // "." dots in between
    fill_rect(&mut img, 20, 24, 2, 2, white);
    fill_rect(&mut img, 24, 24, 2, 2, white);

    img
}

fn generate_mesh() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let green = [60, 160, 70, 255];
    let dark = [40, 120, 50, 255];
    let white = [240, 240, 240, 255];
    let light = [100, 200, 110, 255];

    // Page body
    fill_rect(&mut img, 6, 4, 36, 40, green);
    fill_rect(&mut img, 6, 42, 36, 2, dark);
    fill_rect(&mut img, 40, 4, 2, 40, dark);

    // Simple cube wireframe (center of icon)
    // Front face
    fill_rect(&mut img, 14, 20, 14, 14, light);
    fill_rect(&mut img, 14, 20, 14, 2, white);
    fill_rect(&mut img, 14, 20, 2, 14, white);
    fill_rect(&mut img, 26, 20, 2, 14, [80, 180, 90, 255]);
    fill_rect(&mut img, 14, 32, 14, 2, dark);

    // Top face (parallelogram)
    fill_rect(&mut img, 18, 14, 14, 2, white);
    fill_rect(&mut img, 16, 16, 14, 2, white);
    fill_rect(&mut img, 14, 18, 14, 2, white);

    // Right face (parallelogram)
    fill_rect(&mut img, 28, 16, 2, 4, [80, 180, 90, 255]);
    fill_rect(&mut img, 28, 20, 2, 14, [80, 180, 90, 255]);

    img
}

fn generate_file() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let gray = [140, 140, 140, 255];
    let dark = [100, 100, 100, 255];
    let light = [180, 180, 180, 255];
    let line = [200, 200, 200, 255];

    // Page body
    fill_rect(&mut img, 6, 4, 36, 40, gray);
    fill_rect(&mut img, 6, 42, 36, 2, dark);
    fill_rect(&mut img, 40, 4, 2, 40, dark);

    // Corner fold
    fill_rect(&mut img, 32, 4, 10, 10, dark);
    fill_rect(&mut img, 32, 4, 8, 8, light);

    // Text lines
    fill_rect(&mut img, 12, 20, 20, 2, line);
    fill_rect(&mut img, 12, 26, 16, 2, line);
    fill_rect(&mut img, 12, 32, 22, 2, line);

    img
}

// ---- Material icon: shaded sphere (like Unity) ----

fn fill_circle(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, cx: f32, cy: f32, r: f32, color: [u8; 4]) {
    let r2 = r * r;
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                img.put_pixel(x, y, Rgba(color));
            }
        }
    }
}

/// Generate a shaded material sphere with the given base color (RGB 0..255).
/// Used both for the default icon and for per-material thumbnails.
pub fn generate_material_sphere(base_r: f32, base_g: f32, base_b: f32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));

    let cx = 24.0_f32;
    let cy = 24.0_f32;
    let r = 18.0_f32;

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist2 = dx * dx + dy * dy;
            if dist2 <= r * r {
                let nz = ((r * r - dist2) / (r * r)).sqrt();
                let nx = dx / r;
                let ny = dy / r;
                let ldot = (-0.5 * nx + -0.6 * ny + 0.6 * nz).max(0.0);
                let spec = ((-0.35 * nx + -0.42 * ny + 0.84 * nz).max(0.0)).powf(16.0);

                let ambient = 0.15;
                let pr = ((base_r * (ambient + 0.85 * ldot)) + 255.0 * spec * 0.6).min(255.0);
                let pg = ((base_g * (ambient + 0.85 * ldot)) + 255.0 * spec * 0.6).min(255.0);
                let pb = ((base_b * (ambient + 0.85 * ldot)) + 255.0 * spec * 0.6).min(255.0);

                let edge = (1.0 - nz).powf(2.0) * 0.4;
                let pr = (pr * (1.0 - edge)) as u8;
                let pg = (pg * (1.0 - edge)) as u8;
                let pb = (pb * (1.0 - edge)) as u8;

                img.put_pixel(x, y, Rgba([pr, pg, pb, 255]));
            }
        }
    }

    // Subtle ground shadow
    for x in 10..38 {
        let dx = x as f32 - cx;
        if dx * dx < 14.0 * 14.0 {
            let alpha = (1.0 - (dx * dx) / (14.0 * 14.0)) * 60.0;
            img.put_pixel(x, 43, Rgba([20, 10, 30, alpha as u8]));
            img.put_pixel(x, 44, Rgba([20, 10, 30, (alpha * 0.5) as u8]));
        }
    }

    img
}

fn generate_material() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // Default material sphere: purple/magenta
    generate_material_sphere(160.0, 80.0, 200.0)
}

// ---- Texture icon: checkerboard with image frame ----

fn generate_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));

    // Outer frame
    fill_rect(&mut img, 4, 4, 40, 40, [60, 60, 60, 255]);
    // Inner area
    fill_rect(&mut img, 6, 6, 36, 36, [200, 200, 200, 255]);

    // Checkerboard pattern (4x4 grid of 8px cells)
    let check_a = [220, 220, 220, 255];
    let check_b = [140, 140, 140, 255];
    for row in 0..4 {
        for col in 0..4 {
            let c = if (row + col) % 2 == 0 { check_a } else { check_b };
            fill_rect(&mut img, 7 + col * 8, 7 + row * 8, 8, 8, c);
        }
    }

    // Colorful diagonal stripe to indicate "image" (mountain/sun motif)
    // Sun (yellow circle, top-right)
    fill_circle(&mut img, 34.0, 14.0, 4.0, [255, 200, 50, 255]);

    // Green mountain (bottom-left triangle)
    for y in 28..40 {
        let half_w = (y - 28) as u32;
        let x_start = 14u32.saturating_sub(half_w);
        let x_end = (14 + half_w).min(39);
        fill_rect(&mut img, x_start + 7, y, x_end - x_start, 1, [70, 160, 80, 255]);
    }
    // Blue mountain (bottom-right triangle, overlapping)
    for y in 32..40 {
        let half_w = (y - 32) as u32;
        let x_start = 22u32.saturating_sub(half_w);
        let x_end = (22 + half_w).min(39);
        fill_rect(&mut img, x_start + 7, y, x_end - x_start, 1, [80, 130, 200, 255]);
    }

    // Frame border highlight
    fill_rect(&mut img, 4, 4, 40, 1, [90, 90, 90, 255]);
    fill_rect(&mut img, 4, 4, 1, 40, [90, 90, 90, 255]);
    fill_rect(&mut img, 4, 43, 40, 1, [40, 40, 40, 255]);
    fill_rect(&mut img, 43, 4, 1, 40, [40, 40, 40, 255]);

    img
}

// ---- Skeleton icon: bone shape ----

fn generate_skeleton() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let bone = [230, 230, 220, 255];
    let dark = [180, 175, 165, 255];
    let outline = [100, 95, 85, 255];

    // Top joint (ball)
    fill_circle(&mut img, 24.0, 10.0, 6.0, bone);
    fill_circle(&mut img, 24.0, 10.0, 4.0, [245, 245, 235, 255]);

    // Bone shaft (tapered)
    fill_rect(&mut img, 21, 14, 6, 20, bone);
    fill_rect(&mut img, 22, 14, 4, 20, [245, 245, 235, 255]);
    // Shaft shadow (right side)
    fill_rect(&mut img, 26, 14, 1, 20, dark);

    // Bottom joint (two smaller balls - fork)
    fill_circle(&mut img, 19.0, 38.0, 5.0, bone);
    fill_circle(&mut img, 29.0, 38.0, 5.0, bone);
    fill_circle(&mut img, 19.0, 38.0, 3.0, [245, 245, 235, 255]);
    fill_circle(&mut img, 29.0, 38.0, 3.0, [245, 245, 235, 255]);

    // Connection from shaft to fork
    fill_rect(&mut img, 19, 33, 10, 4, bone);
    fill_rect(&mut img, 20, 34, 8, 2, [245, 245, 235, 255]);

    // Outline dots for clarity
    let _ = outline; // used conceptually for the pixel art style

    img
}

// ---- Clip icon: timeline with animation curve and keyframe diamonds ----

fn fill_diamond(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, cx: u32, cy: u32, r: u32, c: [u8; 4]) {
    for dy in 0..=r {
        let w = r - dy;
        for dx in 0..=w {
            let px = [cx + dx, cx.wrapping_sub(dx)];
            let py = [cy + dy, cy.wrapping_sub(dy)];
            for &x in &px {
                for &y in &py {
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, Rgba(c));
                    }
                }
            }
        }
    }
}

fn generate_clip() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let orange = [230, 150, 40, 255];
    let dark = [180, 115, 25, 255];
    let white = [250, 250, 250, 255];
    let curve_color = [255, 220, 100, 255];
    let diamond_color = [255, 255, 255, 255];
    let timeline_bg = [200, 120, 20, 255];

    // Rounded-ish page body
    fill_rect(&mut img, 6, 4, 36, 40, orange);
    fill_rect(&mut img, 6, 42, 36, 2, dark);
    fill_rect(&mut img, 40, 4, 2, 40, dark);

    // Timeline ruler at top
    fill_rect(&mut img, 10, 8, 28, 3, timeline_bg);
    // Ruler tick marks
    for i in 0..7 {
        fill_rect(&mut img, 12 + i * 4, 8, 1, 3, [140, 85, 10, 255]);
    }

    // Animation curve (smooth S-curve from left to right)
    // y positions for the curve at each x step across the icon
    let curve_points: [(u32, u32); 9] = [
        (10, 32), (14, 30), (18, 24), (22, 18), (26, 16),
        (30, 20), (34, 28), (37, 30), (40, 26),
    ];

    // Draw curve line (2px thick)
    for window in curve_points.windows(2) {
        let (x0, y0) = window[0];
        let (x1, y1) = window[1];
        let steps = ((x1 as i32 - x0 as i32).abs()).max((y1 as i32 - y0 as i32).abs()) as u32;
        if steps == 0 { continue; }
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = (x0 as f32 + (x1 as f32 - x0 as f32) * t) as u32;
            let y = (y0 as f32 + (y1 as f32 - y0 as f32) * t) as u32;
            if x < ICON_SIZE && y < ICON_SIZE {
                img.put_pixel(x, y, Rgba(curve_color));
                if y + 1 < ICON_SIZE {
                    img.put_pixel(x, y + 1, Rgba(curve_color));
                }
            }
        }
    }

    // Keyframe diamonds at key positions
    fill_diamond(&mut img, 10, 32, 2, diamond_color);
    fill_diamond(&mut img, 22, 18, 2, diamond_color);
    fill_diamond(&mut img, 34, 28, 2, diamond_color);

    // Small "play" indicator (bottom-left corner)
    fill_rect(&mut img, 10, 38, 2, 1, white);
    fill_rect(&mut img, 10, 39, 3, 1, white);
    fill_rect(&mut img, 10, 40, 2, 1, white);

    img
}

// ---- Audio icon: speaker with sound waves ----

fn generate_audio() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let teal = [60, 190, 190, 255];
    let dark = [40, 150, 150, 255];
    let white = [240, 240, 240, 255];

    // Background circle
    fill_circle(&mut img, 24.0, 24.0, 20.0, teal);
    fill_circle(&mut img, 24.0, 24.0, 18.0, dark);

    // Speaker body (left side)
    fill_rect(&mut img, 10, 19, 6, 10, white);
    // Speaker cone
    fill_rect(&mut img, 16, 16, 2, 16, white);
    fill_rect(&mut img, 18, 14, 2, 20, white);

    // Sound waves (arcs as pixel art)
    // Wave 1 (small)
    fill_rect(&mut img, 24, 18, 2, 2, white);
    fill_rect(&mut img, 24, 28, 2, 2, white);
    fill_rect(&mut img, 26, 20, 2, 8, white);

    // Wave 2 (medium)
    fill_rect(&mut img, 30, 16, 2, 2, white);
    fill_rect(&mut img, 30, 30, 2, 2, white);
    fill_rect(&mut img, 32, 18, 2, 12, white);

    // Wave 3 (large)
    fill_rect(&mut img, 36, 14, 2, 2, [200, 240, 240, 200]);
    fill_rect(&mut img, 36, 32, 2, 2, [200, 240, 240, 200]);
    fill_rect(&mut img, 38, 16, 2, 16, [200, 240, 240, 150]);

    img
}

// ---- Scene icon: clapperboard ----

fn generate_scene() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let slate = [55, 55, 65, 255];
    let stripe_light = [220, 220, 220, 255];
    let stripe_dark = [55, 55, 65, 255];
    let body = [240, 240, 235, 255];
    let dark = [180, 180, 175, 255];

    // Clapper top (angled stripes)
    fill_rect(&mut img, 4, 4, 40, 14, slate);
    // Diagonal stripes on clapper
    for i in 0..5 {
        let x = 6 + i * 8;
        // Angled stripe (2px wide, leaning right)
        for row in 0..12 {
            let offset = row / 3;
            fill_rect(&mut img, x + offset, 5 + row, 3, 1, stripe_light);
        }
    }

    // Board body (white area)
    fill_rect(&mut img, 4, 18, 40, 24, body);
    fill_rect(&mut img, 4, 40, 40, 2, dark);
    fill_rect(&mut img, 42, 18, 2, 24, dark);

    // Text lines on board
    fill_rect(&mut img, 10, 24, 24, 2, [160, 160, 155, 255]);
    fill_rect(&mut img, 10, 30, 18, 2, [160, 160, 155, 255]);
    fill_rect(&mut img, 10, 36, 20, 2, [160, 160, 155, 255]);

    // Hinge circle
    fill_circle(&mut img, 8.0, 18.0, 3.0, slate);
    fill_circle(&mut img, 8.0, 18.0, 1.5, [100, 100, 110, 255]);

    let _ = stripe_dark; // part of the slate bg

    img
}

/// Prefab icon: stacked cubes in mauve (Catppuccin) suggesting a reusable template.
fn generate_prefab() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let mauve = [203, 166, 247, 255];       // Catppuccin mauve
    let mauve_dark = [150, 120, 190, 255];
    let mauve_light = [220, 195, 255, 255];
    let outline = [100, 70, 140, 255];

    // Back cube (offset upper-right)
    fill_rect(&mut img, 18, 4, 22, 18, mauve_dark);
    fill_rect(&mut img, 18, 4, 22, 2, outline);
    fill_rect(&mut img, 18, 4, 2, 18, outline);
    fill_rect(&mut img, 38, 4, 2, 18, outline);
    fill_rect(&mut img, 18, 20, 22, 2, outline);
    // Top face highlight
    fill_rect(&mut img, 20, 6, 18, 4, mauve_light);

    // Front cube (offset lower-left)
    fill_rect(&mut img, 8, 20, 24, 20, mauve);
    fill_rect(&mut img, 8, 20, 24, 2, outline);
    fill_rect(&mut img, 8, 20, 2, 20, outline);
    fill_rect(&mut img, 30, 20, 2, 20, outline);
    fill_rect(&mut img, 8, 38, 24, 2, outline);
    // Top face highlight
    fill_rect(&mut img, 10, 22, 20, 4, mauve_light);

    // Small "P" letter on front cube
    fill_rect(&mut img, 14, 28, 2, 8, [255, 255, 255, 200]);
    fill_rect(&mut img, 16, 28, 4, 2, [255, 255, 255, 200]);
    fill_rect(&mut img, 20, 28, 2, 4, [255, 255, 255, 200]);
    fill_rect(&mut img, 16, 32, 4, 2, [255, 255, 255, 200]);

    img
}

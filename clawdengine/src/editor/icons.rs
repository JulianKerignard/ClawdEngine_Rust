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
}

impl EditorIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        ensure_icons_dir();

        Self {
            folder: load_or_generate(ctx, "folder", generate_folder),
            script: load_or_generate(ctx, "script", generate_script),
            mesh: load_or_generate(ctx, "mesh", generate_mesh),
            file: load_or_generate(ctx, "file", generate_file),
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

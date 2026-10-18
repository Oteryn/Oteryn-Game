//! Small image-space animation, anchored to the authored citadel background.
//! No video decoder, extra image assets, or per-frame texture uploads.
use egui::{
    Color32, Pos2, Rect, Vec2,
    epaint::{Mesh, Vertex},
};

const COLS: usize = 112;
const ROWS: usize = 64;

fn mask(p: Pos2, center: Pos2, radius: Vec2) -> f32 {
    let d = (p - center) / radius;
    (1.0 - d.length_sq()).max(0.0).powi(2)
}

fn sample(p: Pos2, t: f32) -> Pos2 {
    // The sky mask stops before the castle; river motion stops below the bridge.
    let sky = mask(p, egui::pos2(0.46, 0.12), egui::vec2(0.35, 0.25));
    let river = mask(p, egui::pos2(0.78, 0.91), egui::vec2(0.21, 0.16));
    let mut uv = p;
    uv.x += sky * 0.0025 * (t * 0.13 + p.y * 14.0).sin();
    uv.x += river * 0.0015 * (p.y * 190.0 - t * 1.8).sin();
    uv.y += river * 0.0008 * (p.x * 135.0 + t * 1.1).sin();
    for (x, y, rx, ry) in [
        (0.045, 0.655, 0.032, 0.13),
        (0.931, 0.645, 0.022, 0.20),
        (0.899, 0.727, 0.024, 0.16),
        (0.731, 0.588, 0.017, 0.09),
    ] {
        let water = mask(p, egui::pos2(x, y), egui::vec2(rx, ry));
        uv.y += water * 0.006 * (p.y * 105.0 - t * 3.2).sin();
        uv.x += water * 0.0007 * (p.y * 140.0 - t * 2.4).sin();
    }
    uv
}

fn soft_light(painter: &egui::Painter, center: Pos2, radius: Vec2, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..=32 {
        let a = i as f32 * std::f32::consts::TAU / 32.0;
        mesh.colored_vertex(
            center + egui::vec2(a.cos(), a.sin()) * radius,
            Color32::TRANSPARENT,
        );
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    painter.add(mesh);
}

pub fn paint(painter: &egui::Painter, texture: &egui::TextureHandle, rect: Rect, time: f64) {
    let size = texture.size_vec2();
    let scale = (rect.width() / size.x).max(rect.height() / size.y);
    let image_size = size * scale;
    let image_origin = rect.center() - image_size * 0.5;
    let uv_rect = Rect::from_center_size(egui::pos2(0.5, 0.5), rect.size() / image_size);
    let painter = painter.with_clip_rect(rect);
    // Keep phase precision even after the client has been running for days.
    let t = (time % 3600.0) as f32;
    let mut mesh = Mesh::with_texture(texture.id());
    for y in 0..=ROWS {
        for x in 0..=COLS {
            let f = egui::vec2(x as f32 / COLS as f32, y as f32 / ROWS as f32);
            let uv = uv_rect.min + f * uv_rect.size();
            mesh.vertices.push(Vertex {
                pos: rect.min + f * rect.size(),
                uv: sample(uv, t),
                color: Color32::WHITE,
            });
        }
    }
    for y in 0..ROWS {
        for x in 0..COLS {
            let a = (y * (COLS + 1) + x) as u32;
            let b = a + (COLS + 1) as u32;
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(a + 1, b + 1, b);
        }
    }
    painter.add(mesh);
    // Translucent mist follows the valley, below the skyline and behind the UI.
    for i in 0..9 {
        let phase = i as f32 * 1.79;
        let p = egui::vec2(
            0.16 + i as f32 * 0.085 + 0.025 * (t * 0.10 + phase).sin(),
            0.79 + 0.075 * phase.sin() + 0.012 * (t * 0.17 + phase).sin(),
        );
        soft_light(
            &painter,
            image_origin + p * image_size,
            image_size * egui::vec2(0.13, 0.027),
            Color32::from_rgba_unmultiplied(163, 186, 196, 13),
        );
    }
    // Glows stay on the painted torches/windows, never float across the scene.
    for (i, (x, y)) in [
        (0.158, 0.290),
        (0.166, 0.355),
        (0.852, 0.346),
        (0.846, 0.505),
        (0.911, 0.150),
        (0.941, 0.354),
        (0.985, 0.340),
        (0.784, 0.564),
        (0.658, 0.586),
    ]
    .into_iter()
    .enumerate()
    {
        let phase = i as f32 * 2.31;
        let alpha = 22.0 + 8.0 * (t * 2.3 + phase).sin() + 4.0 * (t * 5.7 + phase).sin();
        soft_light(
            &painter,
            image_origin + egui::vec2(x, y) * image_size,
            image_size * egui::vec2(0.009, 0.021),
            Color32::from_rgba_unmultiplied(255, 169, 65, alpha as u8),
        );
    }
}

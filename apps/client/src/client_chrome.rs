//! Compact client chrome, independently drawn from the reference's proportions.
//! No original client textures, fonts or icons are embedded here.
use egui::{Color32, Context, FontId, Painter, Rect, Stroke};

pub fn install(ctx: &Context, high_contrast: bool) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    for text in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style.text_styles.insert(text, FontId::proportional(11.0));
    }
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(9.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(12.0));
    style.spacing.item_spacing = egui::vec2(3.0, 2.0);
    style.spacing.button_padding = egui::vec2(4.0, 2.0);
    style.spacing.interact_size = egui::vec2(18.0, 18.0);
    style.spacing.icon_width = 12.0;
    style.spacing.icon_width_inner = 8.0;
    style.spacing.icon_spacing = 3.0;
    style.spacing.scroll.foreground_color = false;
    style.spacing.scroll.bar_width = 8.0;
    style.visuals.window_fill = if high_contrast {
        Color32::BLACK
    } else {
        Color32::from_gray(57)
    };
    style.visuals.panel_fill = style.visuals.window_fill;
    style.visuals.override_text_color = Some(if high_contrast {
        Color32::WHITE
    } else {
        Color32::from_gray(215)
    });
    style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.window_stroke = Stroke::new(1.0, Color32::from_gray(125));
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.selection.bg_fill = Color32::from_gray(91);
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::from_gray(235));
    for (widget, fill) in [
        (&mut style.visuals.widgets.noninteractive, 51),
        (&mut style.visuals.widgets.inactive, 62),
        (&mut style.visuals.widgets.hovered, 78),
        (&mut style.visuals.widgets.active, 41),
        (&mut style.visuals.widgets.open, 70),
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
        widget.bg_fill = Color32::from_gray(if high_contrast { 0 } else { fill });
        widget.weak_bg_fill = widget.bg_fill;
        widget.bg_stroke = Stroke::new(
            1.0,
            Color32::from_gray(if high_contrast { 255 } else { 113 }),
        );
        widget.fg_stroke = Stroke::new(1.0, Color32::from_gray(220));
        widget.expansion = 0.0;
    }
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn texture(ctx: &Context) -> egui::TextureHandle {
    let id = egui::Id::new("oteryn-owned-chrome-grain");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let pixels = (0..64 * 64)
        .map(|index| {
            let mut hash = (index as u32).wrapping_add(0x9e37_79b9);
            hash = (hash ^ (hash >> 16)).wrapping_mul(0x7feb_352d);
            hash = (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b);
            hash ^= hash >> 16;
            let tone = 51 + (hash % 11) as u8;
            Color32::from_gray(tone)
        })
        .collect();
    let texture = ctx.load_texture(
        "oteryn-owned-chrome-grain",
        egui::ColorImage::new([64, 64], pixels),
        egui::TextureOptions {
            wrap_mode: egui::TextureWrapMode::Repeat,
            ..egui::TextureOptions::NEAREST
        },
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

fn paint(ctx: &Context, painter: &Painter, rect: Rect, inset: bool) {
    if !rect.is_positive() || !rect.is_finite() {
        return;
    }
    if ctx.style_of(egui::Theme::Dark).visuals.window_fill == Color32::BLACK {
        painter.rect_filled(rect, 0.0, Color32::BLACK);
    } else {
        painter.image(
            texture(ctx).id(),
            rect,
            Rect::from_min_max(
                egui::Pos2::ZERO,
                egui::pos2(rect.width() / 64.0, rect.height() / 64.0),
            ),
            Color32::WHITE,
        );
    }
    let light = Stroke::new(1.0, Color32::from_gray(117));
    let dark = Stroke::new(1.0, Color32::from_gray(27));
    let (top, bottom) = if inset { (dark, light) } else { (light, dark) };
    painter.line_segment([rect.left_bottom(), rect.left_top()], top);
    painter.line_segment([rect.left_top(), rect.right_top()], top);
    painter.line_segment([rect.right_top(), rect.right_bottom()], bottom);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], bottom);
}

pub fn surface(ui: &egui::Ui, rect: Rect, inset: bool) {
    paint(ui.ctx(), ui.painter(), rect, inset);
}

/// Paint only the chrome around the map; the scene must remain unobscured.
pub fn backdrop(ctx: &Context, scene: Option<Rect>) {
    let bounds = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    for rect in surrounding(bounds, scene) {
        paint(ctx, &painter, rect, false);
    }
}

fn surrounding(bounds: Rect, scene: Option<Rect>) -> Vec<Rect> {
    let Some(scene) = scene
        .filter(|r| r.is_finite() && r.is_positive())
        .map(|r| r.intersect(bounds))
    else {
        return vec![bounds];
    };
    if !scene.is_positive() {
        return vec![bounds];
    }
    vec![
        Rect::from_min_max(bounds.min, egui::pos2(bounds.right(), scene.top())),
        Rect::from_min_max(egui::pos2(bounds.left(), scene.bottom()), bounds.max),
        Rect::from_min_max(egui::pos2(bounds.left(), scene.top()), scene.left_bottom()),
        Rect::from_min_max(
            scene.right_top(),
            egui::pos2(bounds.right(), scene.bottom()),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chrome_never_covers_the_rendered_scene() {
        let bounds = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 620.0));
        let scene = Rect::from_min_max(egui::pos2(4.0, 30.0), egui::pos2(740.0, 470.0));
        let strips = surrounding(bounds, Some(scene));
        for strip in &strips {
            assert!(strip.intersect(scene).area() == 0.0);
        }
        let chrome_area: f32 = strips.iter().map(Rect::area).sum();
        assert_eq!(chrome_area + scene.area(), bounds.area());
        assert_eq!(surrounding(bounds, None), vec![bounds]);
    }
}

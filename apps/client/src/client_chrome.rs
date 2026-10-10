//! Modern Oteryn chrome; reference behavior does not require legacy artwork.
//! No original client textures, fonts or icons are embedded here.
use egui::{Color32, Context, FontId, Painter, Rect, Stroke};

pub fn install(ctx: &Context, high_contrast: bool) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    for text in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style.text_styles.insert(text, FontId::proportional(12.0));
    }
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(10.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(14.0));
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
        Color32::from_rgb(18, 19, 19)
    };
    style.visuals.panel_fill = style.visuals.window_fill;
    style.visuals.override_text_color = Some(if high_contrast {
        Color32::WHITE
    } else {
        Color32::from_rgb(232, 227, 215)
    });
    style.visuals.extreme_bg_color = if high_contrast {
        Color32::BLACK
    } else {
        Color32::from_rgb(12, 13, 13)
    };
    style.visuals.faint_bg_color = Color32::from_rgb(28, 28, 26);
    style.visuals.window_corner_radius = egui::CornerRadius::same(8);
    style.visuals.window_stroke = Stroke::new(
        1.0,
        if high_contrast {
            Color32::WHITE
        } else {
            Color32::from_rgb(80, 66, 46)
        },
    );
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.selection.bg_fill = Color32::from_rgb(98, 68, 36);
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(222, 180, 112));
    for (widget, fill) in [
        (&mut style.visuals.widgets.noninteractive, [18, 19, 19]),
        (&mut style.visuals.widgets.inactive, [30, 30, 27]),
        (&mut style.visuals.widgets.hovered, [52, 45, 33]),
        (&mut style.visuals.widgets.active, [98, 68, 36]),
        (&mut style.visuals.widgets.open, [41, 37, 29]),
    ] {
        widget.corner_radius = egui::CornerRadius::same(4);
        widget.bg_fill = if high_contrast {
            Color32::BLACK
        } else {
            Color32::from_rgb(fill[0], fill[1], fill[2])
        };
        widget.weak_bg_fill = widget.bg_fill;
        widget.bg_stroke = Stroke::new(
            1.0,
            if high_contrast {
                Color32::WHITE
            } else {
                Color32::from_rgb(85, 70, 48)
            },
        );
        widget.fg_stroke = Stroke::new(
            1.0,
            if high_contrast {
                Color32::WHITE
            } else {
                Color32::from_rgb(232, 227, 215)
            },
        );
        widget.expansion = 0.0;
    }
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// Settings-only design tokens. Text and interactive controls remain opaque.
pub fn install_preferences(ctx: &Context, high_contrast: bool, transparency: u8) {
    install(ctx, high_contrast);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    for text in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style.text_styles.insert(text, FontId::proportional(14.0));
    }
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(22.0));
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(28.0, 28.0);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 21, 24);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(18, 21, 24);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 84, 78));
    style.spacing.scroll.foreground_color = true;
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 7.0;
    let alpha = ((100 - transparency.min(70)) as f32 * 2.55).round() as u8;
    style.visuals.window_fill = if high_contrast {
        Color32::from_rgba_unmultiplied(0, 0, 0, alpha)
    } else {
        Color32::from_rgba_unmultiplied(25, 28, 31, alpha)
    };
    // One translucent backing avoids stacking opacity under nested sections.
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(18, 21, 24, 24);
    style.visuals.faint_bg_color = Color32::from_rgba_unmultiplied(35, 39, 42, 24);
    if !high_contrast {
        style.visuals.override_text_color = Some(Color32::from_rgb(236, 232, 223));
        style.visuals.selection.bg_fill = Color32::from_rgb(52, 45, 34);
        style.visuals.selection.stroke.color = Color32::from_rgb(217, 185, 120);
        style.visuals.window_stroke.color = Color32::from_rgb(121, 100, 68);
        style.visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(58, 61, 61);
    }
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn paint(ctx: &Context, painter: &Painter, rect: Rect, inset: bool) {
    if !rect.is_positive() || !rect.is_finite() {
        return;
    }
    let style = ctx.style_of(egui::Theme::Dark);
    let fill = if inset {
        style.visuals.extreme_bg_color
    } else {
        style.visuals.panel_fill
    };
    painter.rect_filled(rect, if inset { 4.0 } else { 0.0 }, fill);
    if inset {
        painter.rect_stroke(
            rect,
            4.0,
            style.visuals.window_stroke,
            egui::StrokeKind::Inside,
        );
    }
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
    fn preferences_transparency_only_fades_surfaces() {
        let ctx = Context::default();
        install_preferences(&ctx, false, 10);
        let style = ctx.style_of(egui::Theme::Dark);
        assert_eq!(style.visuals.window_fill.a(), 230);
        assert_eq!(style.visuals.text_color().a(), 255);
        assert_eq!(style.visuals.widgets.inactive.bg_fill.a(), 255);
        assert_eq!(style.visuals.panel_fill, Color32::TRANSPARENT);
        install_preferences(&ctx, false, 70);
        assert_eq!(ctx.style_of(egui::Theme::Dark).visuals.window_fill.a(), 77);
    }

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

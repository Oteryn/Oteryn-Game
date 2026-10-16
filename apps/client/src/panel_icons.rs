//! Licensed Lucide navigation icons and Oteryn-owned empty equipment silhouettes.
use egui::{Color32, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2};

pub fn shortcut(ui: &mut Ui, id: &str, label: &str) -> Response {
    shortcut_selected(ui, id, label, false)
}

// Cell order is pinned in assets/icons/manifest.json.
const PANELS: &[&str] = &[
    "skills",
    "battle",
    "spells",
    "vip",
    "help",
    "quest-log",
    "compendium",
    "cyclopedia",
    "highscores",
    "player-guide",
    "shortcuts",
    "party",
    "wheel",
    "quest-tracker",
    "unjustified-points",
    "prey",
    "kill-tracker",
    "reward-wall",
    "analytics",
    "bosstiary",
    "boss-slots",
    "bosstiary-tracker",
    "bestiary-tracker",
    "imbuement-tracker",
    "weapon-proficiency",
    "forge",
    "social",
    "task-board",
];

fn atlas(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let key = egui::Id::new("oteryn-lucide-panels-v1");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return Some(texture);
    }
    let image = image::load_from_memory_with_format(
        include_bytes!("../assets/icons/panels.webp"),
        image::ImageFormat::WebP,
    )
    .ok()?
    .into_rgba8();
    let pixels = egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    );
    let texture = ctx.load_texture("oteryn-lucide-panels", pixels, egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(key, texture.clone()));
    Some(texture)
}

pub fn shortcut_selected(ui: &mut Ui, id: &str, label: &str, selected: bool) -> Response {
    let index = PANELS.iter().position(|candidate| *candidate == id);
    let button = if let Some((index, texture)) = index.zip(atlas(ui.ctx())) {
        let uv = Rect::from_min_size(
            egui::pos2((index % 8) as f32 / 8.0, (index / 8) as f32 / 4.0),
            egui::vec2(1.0 / 8.0, 1.0 / 4.0),
        );
        let tint = if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        egui::Button::image(
            egui::Image::new((texture.id(), egui::vec2(18.0, 18.0)))
                .uv(uv)
                .tint(tint),
        )
        .selected(selected)
    } else {
        // Unknown module IDs remain accessible and explicitly labelled.
        egui::Button::new("?").selected(selected)
    };
    let response = ui.add_sized([24.0, 24.0], button);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}

/// Slot silhouettes describe the paper-doll position, never an invented item appearance.
pub fn equipment(ui: &mut Ui, id: &str, label: &str) -> Response {
    let response = ui.add_sized([30.0, 30.0], egui::Button::new(""));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let ink = Ink {
        painter: ui.painter(),
        origin: response.rect.center() - Vec2::splat(12.0),
        scale: 1.0,
        gold: Stroke::new(1.2, Color32::from_rgb(128, 126, 110)),
        blue: Stroke::new(1.2, Color32::from_rgb(106, 130, 151)),
    };
    equipment_icon(&ink, id);
    response.on_hover_text(label)
}

struct Ink<'a> {
    painter: &'a Painter,
    origin: Pos2,
    scale: f32,
    gold: Stroke,
    blue: Stroke,
}
impl Ink<'_> {
    fn point(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.scale
    }
    fn line(&self, a: [f32; 2], b: [f32; 2], accent: bool) {
        self.painter.line_segment(
            [self.point(a[0], a[1]), self.point(b[0], b[1])],
            if accent { self.blue } else { self.gold },
        );
    }
    fn path(&self, points: &[[f32; 2]], accent: bool) {
        self.painter.add(egui::Shape::line(
            points.iter().map(|p| self.point(p[0], p[1])).collect(),
            if accent { self.blue } else { self.gold },
        ));
    }
    fn circle(&self, x: f32, y: f32, radius: f32, accent: bool) {
        self.painter.circle_stroke(
            self.point(x, y),
            radius * self.scale,
            if accent { self.blue } else { self.gold },
        );
    }
    fn box_at(&self, x: f32, y: f32, width: f32, height: f32, accent: bool) {
        self.painter.rect_stroke(
            Rect::from_min_size(self.point(x, y), Vec2::new(width, height) * self.scale),
            1.0,
            if accent { self.blue } else { self.gold },
            egui::StrokeKind::Inside,
        );
    }
}

#[rustfmt::skip]
fn equipment_icon(i: &Ink<'_>, id: &str) {
    match id {
        "head" => { i.path(&[[4.0,18.0],[4.0,9.0],[8.0,4.0],[16.0,4.0],[20.0,9.0],[20.0,18.0],[16.0,18.0],[16.0,12.0],[8.0,12.0],[8.0,18.0],[4.0,18.0]],false); i.line([12.0,4.0],[12.0,12.0],true); }
        "necklace" => { i.path(&[[5.0,4.0],[5.0,9.0],[8.0,14.0],[12.0,16.0],[16.0,14.0],[19.0,9.0],[19.0,4.0]],false); i.circle(12.0,19.0,3.0,true); }
        "armor" => i.path(&[[7.0,3.0],[9.0,6.0],[15.0,6.0],[17.0,3.0],[21.0,8.0],[18.0,11.0],[17.0,21.0],[7.0,21.0],[6.0,11.0],[3.0,8.0],[7.0,3.0]],false),
        "right-hand" => { i.path(&[[12.0,2.0],[15.0,7.0],[13.0,16.0],[11.0,16.0],[9.0,7.0],[12.0,2.0]],false); i.line([7.0,16.0],[17.0,16.0],true); i.line([12.0,16.0],[12.0,22.0],false); }
        "left-hand" => { i.path(&[[4.0,4.0],[20.0,4.0],[18.0,16.0],[12.0,22.0],[6.0,16.0],[4.0,4.0]],false); i.line([12.0,6.0],[12.0,18.0],true); }
        "legs" => i.path(&[[6.0,3.0],[18.0,3.0],[18.0,21.0],[14.0,21.0],[12.0,10.0],[10.0,21.0],[6.0,21.0],[6.0,3.0]],false),
        "feet" => { i.path(&[[6.0,3.0],[14.0,3.0],[14.0,14.0],[20.0,17.0],[20.0,21.0],[5.0,21.0],[5.0,16.0],[6.0,3.0]],false); i.line([6.0,8.0],[13.0,8.0],true); }
        "ring" => { i.circle(12.0,14.0,7.0,false); i.path(&[[8.0,5.0],[12.0,2.0],[16.0,5.0],[12.0,8.0],[8.0,5.0]],true); }
        "ammo" => { i.line([4.0,20.0],[20.0,4.0],false); i.path(&[[14.0,4.0],[20.0,4.0],[20.0,10.0]],true); i.line([4.0,14.0],[10.0,20.0],false); }
        "backpack" => { i.box_at(5.0,6.0,14.0,16.0,false); i.path(&[[8.0,6.0],[8.0,2.0],[16.0,2.0],[16.0,6.0]],true); i.box_at(8.0,14.0,8.0,5.0,true); }
        _ => i.box_at(6.0,6.0,12.0,12.0,false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_covers_every_catalogued_panel_and_matches_source_manifest() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../assets/icons/manifest.json"))
                .unwrap_or_else(|error| unreachable!("Invalid icon manifest: {error}"));
        for panel in oteryn_client::panel_catalog::PANELS {
            assert!(PANELS.contains(&panel.id), "Missing icon for {}", panel.id);
        }
        for (index, id) in PANELS.iter().enumerate() {
            assert_eq!(manifest["icons"][index]["panel"].as_str(), Some(*id));
        }
        let image = image::load_from_memory_with_format(
            include_bytes!("../assets/icons/panels.webp"),
            image::ImageFormat::WebP,
        )
        .unwrap_or_else(|error| unreachable!("Invalid icon atlas: {error}"));
        assert_eq!([image.width(), image.height()], [384, 192]);
        let rgba = image.into_rgba8();
        for index in 0..PANELS.len() {
            let x = (index % 8) as u32 * 48;
            let y = (index / 8) as u32 * 48;
            assert!((y..y + 48).any(|row| (x..x + 48).any(|col| rgba.get_pixel(col, row)[3] > 0)));
        }
    }
}

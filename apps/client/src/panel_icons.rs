//! Oteryn-owned vector shortcuts. No font glyphs or reference-client artwork.
use egui::{Color32, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2};

pub fn shortcut(ui: &mut Ui, id: &str, label: &str) -> Response {
    shortcut_selected(ui, id, label, false)
}

pub fn shortcut_selected(ui: &mut Ui, id: &str, label: &str, selected: bool) -> Response {
    let response = ui.add_sized([22.0, 20.0], egui::Button::new("").selected(selected));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(response.rect) {
        let ink = Ink {
            painter: ui.painter(),
            origin: response.rect.center() - Vec2::splat(7.8),
            scale: 0.65,
            gold: Stroke::new(
                1.5,
                if ui.is_enabled() {
                    Color32::from_gray(213)
                } else {
                    Color32::from_gray(110)
                },
            ),
            blue: Stroke::new(
                1.5,
                if ui.is_enabled() {
                    Color32::from_rgb(139, 163, 181)
                } else {
                    Color32::from_gray(95)
                },
            ),
        };
        icon(&ink, id);
    }
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
    fn person(&self, x: f32, y: f32, accent: bool) {
        self.circle(x, y, 2.4, accent);
        self.path(
            &[
                [x - 4.0, y + 8.0],
                [x - 3.0, y + 4.5],
                [x, y + 3.5],
                [x + 3.0, y + 4.5],
                [x + 4.0, y + 8.0],
            ],
            accent,
        );
    }
    fn book(&self) {
        self.path(
            &[
                [3.0, 5.0],
                [8.0, 4.0],
                [12.0, 6.0],
                [16.0, 4.0],
                [21.0, 5.0],
                [21.0, 19.0],
                [16.0, 18.0],
                [12.0, 20.0],
                [8.0, 18.0],
                [3.0, 19.0],
                [3.0, 5.0],
            ],
            false,
        );
        self.line([12.0, 6.0], [12.0, 20.0], false);
    }
    fn check(&self, x: f32, y: f32) {
        self.path(&[[x, y + 2.0], [x + 2.0, y + 4.0], [x + 6.0, y]], true);
    }
    fn target(&self, x: f32, y: f32, radius: f32) {
        self.circle(x, y, radius, false);
        self.circle(x, y, radius * 0.38, true);
        self.line([x - radius - 2.0, y], [x - radius + 2.0, y], true);
        self.line([x + radius - 2.0, y], [x + radius + 2.0, y], true);
        self.line([x, y - radius - 2.0], [x, y - radius + 2.0], true);
        self.line([x, y + radius - 2.0], [x, y + radius + 2.0], true);
    }
    fn swords(&self) {
        self.path(
            &[
                [4.0, 20.0],
                [18.0, 6.0],
                [21.0, 3.0],
                [18.0, 3.0],
                [4.0, 17.0],
            ],
            false,
        );
        self.line([4.0, 14.0], [10.0, 20.0], false);
        self.path(
            &[
                [20.0, 20.0],
                [6.0, 6.0],
                [3.0, 3.0],
                [3.0, 6.0],
                [17.0, 20.0],
            ],
            true,
        );
        self.line([14.0, 20.0], [20.0, 14.0], true);
    }
    fn boss(&self) {
        self.path(
            &[
                [5.0, 9.0],
                [3.0, 4.0],
                [8.0, 7.0],
                [12.0, 4.0],
                [16.0, 7.0],
                [21.0, 4.0],
                [19.0, 9.0],
                [18.0, 18.0],
                [12.0, 21.0],
                [6.0, 18.0],
                [5.0, 9.0],
            ],
            false,
        );
        self.line([7.0, 12.0], [10.0, 13.0], true);
        self.line([14.0, 13.0], [17.0, 12.0], true);
        self.line([9.0, 17.0], [15.0, 17.0], false);
    }
}

// Static coordinate catalogue stays compact, like panel metadata.
#[rustfmt::skip]
fn icon(i: &Ink<'_>, id: &str) {
    match id {
        "skills" => { for (x, h) in [(4.0, 7.0), (10.0, 12.0), (16.0, 17.0)] { i.box_at(x, 21.0 - h, 4.0, h, false); } }
        "battle" => i.swords(),
        "spells" => { i.book(); i.line([6.0, 9.0], [9.0, 12.0], true); i.line([9.0, 9.0], [6.0, 12.0], true); i.line([15.0, 9.0], [18.0, 9.0], true); }
        "vip" => { i.person(11.0, 6.0, false); i.circle(19.0, 18.0, 2.5, true); }
        "help" => { i.circle(12.0, 12.0, 10.0, false); i.path( &[ [8.0, 8.0], [9.0, 5.0], [14.0, 5.0], [16.0, 8.0], [15.0, 11.0], [12.0, 13.0], [12.0, 15.0], ], true, ); i.circle(12.0, 18.0, 0.6, true); }
        "quest-log" => { i.book(); i.check(5.0, 9.0); i.line([15.0, 10.0], [19.0, 10.0], true); i.line([15.0, 14.0], [19.0, 14.0], true); }
        "compendium" => { i.box_at(4.0, 3.0, 14.0, 17.0, false); i.line([7.0, 3.0], [7.0, 20.0], false); i.path(&[[18.0, 6.0], [21.0, 6.0], [21.0, 22.0], [7.0, 22.0]], true); i.line([10.0, 8.0], [15.0, 8.0], true); }
        "cyclopedia" => { i.circle(12.0, 12.0, 10.0, false); i.path( &[ [12.0, 2.0], [7.0, 7.0], [7.0, 17.0], [12.0, 22.0], [17.0, 17.0], [17.0, 7.0], [12.0, 2.0], ], true, ); i.line([2.0, 12.0], [22.0, 12.0], false); }
        "highscores" => { i.path( &[ [6.0, 3.0], [18.0, 3.0], [17.0, 12.0], [12.0, 16.0], [7.0, 12.0], [6.0, 3.0], ], false, ); i.path(&[[6.0, 5.0], [2.0, 5.0], [3.0, 10.0], [7.0, 11.0]], true); i.path( &[[18.0, 5.0], [22.0, 5.0], [21.0, 10.0], [17.0, 11.0]], true, ); i.line([12.0, 16.0], [12.0, 21.0], false); i.line([7.0, 21.0], [17.0, 21.0], false); }
        "player-guide" => { i.circle(12.0, 12.0, 9.0, false); i.path( &[ [15.0, 6.0], [14.0, 14.0], [6.0, 18.0], [10.0, 10.0], [15.0, 6.0], ], true, ); }
        "shortcuts" => { for (x, y) in [(3.0, 3.0), (14.0, 3.0), (3.0, 14.0)] { i.box_at(x, y, 7.0, 7.0, false); } i.line([18.0, 14.0], [18.0, 22.0], true); i.line([14.0, 18.0], [22.0, 18.0], true); }
        "party" => { i.person(5.0, 8.0, true); i.person(19.0, 8.0, true); i.person(12.0, 5.0, false); }
        "wheel" => { i.circle(12.0, 12.0, 9.5, false); i.circle(12.0, 12.0, 2.2, true); for (x, y) in [ (12.0, 2.5), (21.5, 12.0), (12.0, 21.5), (2.5, 12.0), (5.3, 5.3), (18.7, 5.3), (18.7, 18.7), (5.3, 18.7), ] { i.line([12.0, 12.0], [x, y], false); } }
        "quest-tracker" => { i.box_at(3.0, 3.0, 13.0, 18.0, false); i.check(5.0, 6.0); i.line([6.0, 14.0], [11.0, 14.0], false); i.circle(18.0, 18.0, 4.0, true); }
        "unjustified-points" => { i.path( &[ [12.0, 2.0], [21.0, 6.0], [19.0, 16.0], [12.0, 22.0], [5.0, 16.0], [3.0, 6.0], [12.0, 2.0], ], false, ); i.line([12.0, 7.0], [12.0, 13.0], true); i.circle(12.0, 17.0, 0.7, true); }
        "prey" => i.target(12.0, 12.0, 8.0),
        "kill-tracker" => { i.swords(); i.box_at(15.0, 15.0, 8.0, 8.0, true); i.line([17.0, 17.0], [17.0, 21.0], false); i.line([20.0, 17.0], [20.0, 21.0], false); }
        "reward-wall" => { i.box_at(3.0, 10.0, 18.0, 12.0, false); i.box_at(2.0, 7.0, 20.0, 4.0, false); i.line([12.0, 7.0], [12.0, 22.0], true); i.path( &[ [12.0, 7.0], [6.0, 6.0], [5.0, 3.0], [8.0, 2.0], [12.0, 7.0], [16.0, 2.0], [19.0, 3.0], [18.0, 6.0], [12.0, 7.0], ], true, ); }
        "analytics" => { i.line([3.0, 3.0], [3.0, 21.0], false); i.line([3.0, 21.0], [22.0, 21.0], false); i.path( &[[5.0, 16.0], [10.0, 10.0], [15.0, 13.0], [21.0, 5.0]], true, ); i.line([7.0, 20.0], [7.0, 17.0], false); i.line([13.0, 20.0], [13.0, 14.0], false); i.line([19.0, 20.0], [19.0, 11.0], false); }
        "bosstiary" => i.boss(),
        "boss-slots" => { i.box_at(2.0, 4.0, 8.0, 17.0, false); i.box_at(14.0, 4.0, 8.0, 17.0, false); i.path( &[ [4.0, 11.0], [4.0, 8.0], [6.0, 10.0], [8.0, 8.0], [8.0, 11.0], [4.0, 11.0], ], true, ); i.line([16.0, 12.0], [20.0, 12.0], true); }
        "bosstiary-tracker" => { i.boss(); i.circle(19.0, 19.0, 4.0, true); i.line([19.0, 16.0], [19.0, 22.0], true); }
        "bestiary-tracker" => { for (x, y) in [(5.0, 8.0), (10.0, 5.0), (15.0, 5.0), (20.0, 8.0)] { i.circle(x, y, 2.2, false); } i.path( &[ [5.0, 19.0], [7.0, 14.0], [12.0, 11.0], [17.0, 14.0], [19.0, 19.0], [12.0, 21.0], [5.0, 19.0], ], true, ); }
        "imbuement-tracker" => { i.path( &[ [11.0, 2.0], [18.0, 10.0], [11.0, 20.0], [4.0, 10.0], [11.0, 2.0], ], false, ); i.line([4.0, 10.0], [18.0, 10.0], true); i.line([11.0, 2.0], [11.0, 20.0], false); i.circle(19.0, 19.0, 4.0, true); i.path(&[[19.0, 16.0], [19.0, 19.0], [21.0, 20.0]], true); }
        "weapon-proficiency" => { i.path( &[ [12.0, 2.0], [15.0, 7.0], [13.0, 16.0], [11.0, 16.0], [9.0, 7.0], [12.0, 2.0], ], false, ); i.line([7.0, 16.0], [17.0, 16.0], true); i.line([12.0, 16.0], [12.0, 22.0], false); i.path(&[[5.0, 8.0], [3.0, 13.0], [5.0, 18.0], [8.0, 21.0]], true); i.path( &[[19.0, 8.0], [21.0, 13.0], [19.0, 18.0], [16.0, 21.0]], true, ); }
        "forge" => { i.path( &[ [2.0, 13.0], [22.0, 13.0], [18.0, 16.0], [13.0, 16.0], [15.0, 21.0], [7.0, 21.0], [9.0, 16.0], [4.0, 16.0], [2.0, 13.0], ], false, ); i.path( &[ [9.0, 3.0], [15.0, 6.0], [13.0, 10.0], [7.0, 7.0], [9.0, 3.0], ], true, ); i.line([13.0, 8.0], [18.0, 11.0], true); }
        "social" => { i.person(7.0, 7.0, false); i.person(17.0, 9.0, true); i.path( &[ [13.0, 2.0], [22.0, 2.0], [22.0, 7.0], [18.0, 7.0], [16.0, 9.0], [16.0, 7.0], [13.0, 7.0], [13.0, 2.0], ], true, ); }
        "task-board" => { i.box_at(4.0, 4.0, 16.0, 18.0, false); i.box_at(8.0, 2.0, 8.0, 4.0, true); i.check(7.0, 9.0); i.line([7.0, 17.0], [17.0, 17.0], false); }
        _ => { i.box_at(4.0, 4.0, 16.0, 16.0, false); i.line([8.0, 12.0], [16.0, 12.0], true); }
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

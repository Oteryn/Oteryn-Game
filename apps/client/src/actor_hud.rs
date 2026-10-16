//! Own-character resource overlays from session vitals. Missing data stays missing.
//! Original UI textures are not used; the geometry is independently authored.
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};
use oteryn_client::{settings::ClientSettings, settings_catalog::FutureValue};
use oteryn_renderer::{TileCoord, TileView};
use oteryn_session::{ActorVitals, EntityDetail, WorldSpatialEntity};

fn enabled(settings: &ClientSettings, key: &str) -> bool {
    matches!(
        settings.future_preferences.get(key),
        Some(FutureValue::Bool(true))
    )
}

fn other_health_shapes(
    settings: &ClientSettings,
    view: &TileView,
    entities: &[WorldSpatialEntity],
    own: TileCoord,
    floor: i16,
    scene: Rect,
    scale: f32,
) -> Vec<Shape> {
    if !enabled(settings, "hud.hud.creatures_enabled") || !enabled(settings, "hud.hud.other_health")
    {
        return Vec::new();
    }
    let mut shapes = Vec::new();
    for entity in entities {
        let EntityDetail::Actor { health_percent, .. } = entity.detail else {
            continue;
        };
        let tile = TileCoord::new(entity.position.x, entity.position.y);
        if entity.position.floor != floor || tile == own || !view.contains(tile) {
            continue;
        }
        let [x, y] = view.tile_to_screen(tile);
        let center = scene.min
            + egui::vec2(
                x + oteryn_client::scene::SCENE_TILE_PX as f32 / 2.0,
                y + oteryn_client::scene::SCENE_TILE_PX as f32 / 2.0,
            ) * scale;
        let rect = Rect::from_min_size(
            center + egui::vec2(-16.0, -25.0) * scale,
            egui::vec2(32.0, 4.0) * scale,
        );
        shapes.push(Shape::rect_filled(rect, 0.0, Color32::BLACK));
        let track = rect.shrink(0.75 * scale);
        let fraction = f32::from(health_percent.min(100)) / 100.0;
        if fraction > 0.0 {
            let color = if health_percent > 60 {
                Color32::from_rgb(55, 183, 61)
            } else if health_percent > 30 {
                Color32::from_rgb(222, 170, 48)
            } else {
                Color32::from_rgb(205, 57, 47)
            };
            shapes.push(Shape::rect_filled(
                Rect::from_min_size(
                    track.min,
                    egui::vec2(track.width() * fraction, track.height()),
                ),
                0.0,
                color,
            ));
        }
    }
    shapes
}

fn percent(settings: &ClientSettings, key: &str, fallback: f32) -> f32 {
    match settings.future_preferences.get(key) {
        Some(FutureValue::Int(value)) => (*value).clamp(0, 100) as f32,
        _ => fallback,
    }
}

fn arc(center: Pos2, radius: f32, left: bool, fraction: f32) -> Vec<Pos2> {
    // The fill grows from the lower end. Zero resources do not draw a coloured line.
    let segments = 32;
    (0..=segments)
        .map(|index| {
            let progress = index as f32 / segments as f32 * fraction;
            let angle = if left {
                0.75 + 0.5 * progress
            } else {
                0.25 - 0.5 * progress
            } * std::f32::consts::PI;
            center + Vec2::angled(angle) * radius
        })
        .collect()
}

fn shapes(
    settings: &ClientSettings,
    vitals: Option<ActorVitals>,
    center: Pos2,
    scale: f32,
) -> Vec<Shape> {
    let mut out = Vec::new();
    if !enabled(settings, "hud.hud.player_enabled")
        || !center.is_finite()
        || !scale.is_finite()
        || scale <= 0.0
    {
        return out;
    }
    let Some(vitals) = vitals else { return out };
    let bars = enabled(settings, "hud.hud.resource_bars");
    let preset = match settings.future_preferences.get("hud.arc_size_preset") {
        Some(FutureValue::Choice(value)) => match value.as_str() {
            "small" => Some(0.75),
            "default" => Some(1.0),
            "large" => Some(1.25),
            _ => None,
        },
        _ => None,
    };
    // These are Oteryn renderer fallbacks, never persisted reference defaults.
    let distance = percent(settings, "hud.hud.arc_distance", 25.0);
    let alpha = (percent(settings, "hud.hud.arc_opacity", 100.0) * 2.55).round() as u8;
    for (index, key, current, maximum, color) in [
        (
            0,
            "hud.hud.owner_health",
            vitals.health,
            vitals.max_health,
            Color32::from_rgb(55, 183, 61),
        ),
        (
            1,
            "hud.hud.owner_mana",
            vitals.mana,
            vitals.max_mana,
            Color32::from_rgb(58, 118, 220),
        ),
    ] {
        if !enabled(settings, key) || maximum == 0 {
            continue;
        }
        let fraction = (current as f64 / maximum as f64).clamp(0.0, 1.0) as f32;
        if bars {
            let rect = Rect::from_min_size(
                center + egui::vec2(-17.0, -27.0 + index as f32 * 5.0) * scale,
                egui::vec2(34.0, 4.0) * scale,
            );
            out.push(Shape::rect_filled(rect, 0.0, Color32::BLACK));
            let track = rect.shrink(0.75 * scale);
            if fraction > 0.0 {
                out.push(Shape::rect_filled(
                    Rect::from_min_size(
                        track.min,
                        egui::vec2(track.width() * fraction, track.height()),
                    ),
                    0.0,
                    color,
                ));
            }
        }
        if enabled(settings, "hud.hud.arcs")
            && let Some(preset) = preset
        {
            let radius = (30.0 * preset + distance * 0.22) * scale;
            let left = index == 0;
            let background = Color32::from_rgba_unmultiplied(16, 16, 16, alpha);
            out.push(Shape::line(
                arc(center, radius, left, 1.0),
                Stroke::new(5.0 * scale, background),
            ));
            if fraction > 0.0 && alpha > 0 {
                let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
                out.push(Shape::line(
                    arc(center, radius, left, fraction),
                    Stroke::new(3.0 * scale, fill),
                ));
            }
        }
    }
    out
}

/// Settings-only illustrative sample, painted in the dialog's own clipped layer.
pub fn preview(ui: &egui::Ui, rect: Rect, settings: &ClientSettings) {
    let vitals = ActorVitals {
        health: 75,
        max_health: 100,
        mana: 60,
        max_mana: 100,
        ..Default::default()
    };
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .extend(shapes(
            settings,
            Some(vitals),
            rect.center() + egui::vec2(0.0, 12.0),
            1.0,
        ));
}

pub fn show(
    ctx: &egui::Context,
    scene: Rect,
    own: [f32; 2],
    settings: &ClientSettings,
    vitals: Option<ActorVitals>,
    character_name: Option<&str>,
) {
    if !scene.is_finite() || !scene.is_positive() {
        return;
    }
    let scale = scene.width()
        / (oteryn_client::scene::SCENE_COLUMNS * oteryn_client::scene::SCENE_TILE_PX) as f32;
    let half_tile = oteryn_client::scene::SCENE_TILE_PX as f32 / 2.0;
    let center = scene.min + egui::vec2(own[0] + half_tile, own[1] + half_tile) * scale;
    if !scene.contains(center) {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        "own-resource-hud".into(),
    ));
    let painter = painter.with_clip_rect(scene);
    painter.extend(shapes(settings, vitals, center, scale));
    if enabled(settings, "hud.hud.player_enabled")
        && enabled(settings, "hud.hud.owner_name")
        && let Some(name) = character_name
    {
        painter.text(
            center + egui::vec2(0.0, -30.0) * scale,
            egui::Align2::CENTER_BOTTOM,
            name,
            egui::FontId::proportional(11.0 * scale.clamp(0.75, 1.5)),
            Color32::WHITE,
        );
    }
    if enabled(settings, "hud.hud.player_enabled")
        && enabled(settings, "hud.show_harmony")
        && let Some(vitals) = vitals
    {
        let side = if settings.stored_choice("hud.harmony_position") == Some("mana") {
            1.0
        } else {
            -1.0
        };
        painter.text(
            center + egui::vec2(side * 38.0, 0.0) * scale,
            egui::Align2::CENTER_CENTER,
            vitals.harmony,
            egui::FontId::proportional(10.0 * scale.clamp(0.75, 1.5)),
            if vitals.serene {
                Color32::from_rgb(135, 211, 246)
            } else {
                Color32::from_rgb(236, 205, 126)
            },
        );
    }
}

pub fn show_other_health(
    ctx: &egui::Context,
    scene: Rect,
    view: &TileView,
    own: TileCoord,
    floor: i16,
    settings: &ClientSettings,
    entities: &[WorldSpatialEntity],
) {
    if !scene.is_finite() || !scene.is_positive() {
        return;
    }
    let scale = scene.width()
        / (oteryn_client::scene::SCENE_COLUMNS * oteryn_client::scene::SCENE_TILE_PX) as f32;
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        "other-health-hud".into(),
    ))
    .with_clip_rect(scene)
    .extend(other_health_shapes(
        settings, view, entities, own, floor, scene, scale,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{ActorPosition, EntityKind, EntityRef, StepDirection};
    fn configured() -> ClientSettings {
        let mut settings = ClientSettings::default();
        for key in [
            "hud.hud.player_enabled",
            "hud.hud.resource_bars",
            "hud.hud.owner_health",
            "hud.hud.owner_mana",
            "hud.hud.arcs",
        ] {
            settings
                .future_preferences
                .insert(key.into(), FutureValue::Bool(true));
        }
        settings.future_preferences.insert(
            "hud.arc_size_preset".into(),
            FutureValue::Choice("default".into()),
        );
        settings
    }
    #[test]
    fn unknown_vitals_and_disabled_master_never_draw_fabricated_resources() {
        let settings = configured();
        let center = egui::pos2(100.0, 100.0);
        assert!(shapes(&settings, None, center, 1.0).is_empty());
        let mut settings = settings;
        settings
            .future_preferences
            .insert("hud.hud.player_enabled".into(), FutureValue::Bool(false));
        assert!(
            shapes(
                &settings,
                Some(ActorVitals {
                    health: 50,
                    max_health: 100,
                    ..Default::default()
                }),
                center,
                1.0
            )
            .is_empty()
        );
        assert!(shapes(&configured(), Some(ActorVitals::default()), center, 1.0).is_empty());
    }
    #[test]
    fn independent_resources_use_actual_fraction_and_do_not_mutate_saved_preferences()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = configured();
        settings.validate()?;
        settings
            .future_preferences
            .insert("hud.hud.owner_mana".into(), FutureValue::Bool(false));
        let before = settings.clone();
        let result = shapes(
            &settings,
            Some(ActorVitals {
                health: 25,
                max_health: 100,
                mana: 50,
                max_mana: 50,
                ..Default::default()
            }),
            egui::pos2(100.0, 100.0),
            1.0,
        );
        assert_eq!(result.len(), 4); // HP track/fill and HP arc track/fill only.
        let (Shape::Rect(track), Shape::Rect(fill)) = (&result[0], &result[1]) else {
            return Err("bar shapes".into());
        };
        assert!((fill.rect.width() / track.rect.shrink(0.75).width() - 0.25).abs() < 0.001);
        assert_eq!(settings, before);
        Ok(())
    }
    #[test]
    fn sizes_distance_opacity_and_resource_exhaustion_change_actual_geometry()
    -> Result<(), &'static str> {
        let mut settings = configured();
        settings
            .future_preferences
            .insert("hud.hud.resource_bars".into(), FutureValue::Bool(false));
        settings
            .future_preferences
            .insert("hud.hud.owner_mana".into(), FutureValue::Bool(false));
        let vitals = Some(ActorVitals {
            health: 100,
            max_health: 100,
            ..Default::default()
        });
        let center = egui::pos2(100.0, 100.0);
        let mut radii = Vec::new();
        for size in ["small", "default", "large"] {
            settings.future_preferences.insert(
                "hud.arc_size_preset".into(),
                FutureValue::Choice(size.into()),
            );
            let result = shapes(&settings, vitals, center, 1.0);
            let Shape::Path(path) = &result[0] else {
                return Err("arc path");
            };
            radii.push(path.points[0].distance(center));
        }
        assert!(radii[0] < radii[1] && radii[1] < radii[2]);
        settings
            .future_preferences
            .insert("hud.hud.arc_opacity".into(), FutureValue::Int(0));
        assert_eq!(shapes(&settings, vitals, center, 1.0).len(), 1);
        assert_eq!(
            shapes(
                &settings,
                Some(ActorVitals {
                    health: 0,
                    max_health: 100,
                    ..Default::default()
                }),
                center,
                1.0
            )
            .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn other_health_uses_authoritative_percent_and_excludes_own_and_other_floors()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        for key in ["hud.hud.creatures_enabled", "hud.hud.other_health"] {
            settings
                .future_preferences
                .insert(key.into(), FutureValue::Bool(true));
        }
        let view = TileView::new(TileCoord::new(0, 0), 48, 15, 11)?;
        let actor = |x, floor, health_percent| WorldSpatialEntity {
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [x as u8; 16],
                generation: 1,
            },
            position: ActorPosition { x, y: 2, floor },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent,
            },
        };
        let scene = Rect::from_min_size(Pos2::ZERO, egui::vec2(720.0, 528.0));
        let shapes = other_health_shapes(
            &settings,
            &view,
            &[actor(1, 0, 25), actor(2, 1, 100), actor(3, 0, 0)],
            TileCoord::new(3, 2),
            0,
            scene,
            1.0,
        );
        assert_eq!(shapes.len(), 2); // Track + 25% fill; the zero-health own actor is excluded.
        let (Shape::Rect(track), Shape::Rect(fill)) = (&shapes[0], &shapes[1]) else {
            return Err("health shapes".into());
        };
        assert!((fill.rect.width() / track.rect.shrink(0.75).width() - 0.25).abs() < 0.001);
        settings
            .future_preferences
            .insert("hud.hud.creatures_enabled".into(), FutureValue::Bool(false));
        assert!(
            other_health_shapes(
                &settings,
                &view,
                &[actor(1, 0, 100)],
                TileCoord::new(3, 2),
                0,
                scene,
                1.0,
            )
            .is_empty()
        );
        Ok(())
    }
}

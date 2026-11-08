//! Confirmed-resource bars, monk indicators and configurable HP/mana arcs.
use super::*;
use crate::play::ReferencePreferences;
use oteryn_client_runtime::spell::ActorVitals;

pub(super) struct Options {
    combo: bool,
    bars: bool,
    harmony: bool,
    serene: bool,
    health_circle: bool,
    distance: u16,
    size: u16,
}
impl From<&ReferencePreferences> for Options {
    fn from(preferences: &ReferencePreferences) -> Self {
        Self {
            combo: preferences.resource_hud.combo,
            bars: preferences.hud_bars,
            harmony: preferences.show_harmony,
            serene: preferences.show_serene,
            health_circle: preferences.health_circle,
            distance: preferences.arc_distance,
            size: preferences.arc_size,
        }
    }
}
pub(super) fn draw(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    vitals: Option<&ActorVitals>,
    own: TileCoord,
    options: Options,
) -> Result<(), BatchError> {
    let Options {
        combo,
        bars,
        harmony,
        serene,
        health_circle,
        distance,
        size,
    } = options;
    if bars && let Some(vitals) = vitals {
        for (row, value, max, color) in [
            (
                0.0,
                vitals.health,
                vitals.max_health,
                PresentationColor::Red,
            ),
            (1.0, vitals.mana, vitals.max_mana, PresentationColor::Blue),
        ] {
            let at = [8.0, 56.0 + row * 14.0];
            pixel(
                batch,
                view,
                atlas,
                at,
                [120.0, 10.0],
                PresentationColor::LightGrey,
            )?;
            pixel(
                batch,
                view,
                atlas,
                at,
                [
                    oteryn_client_ui::spell_action_bar::fill_px(value, max),
                    10.0,
                ],
                color,
            )?;
        }
    }
    if let Some(vitals) = vitals {
        if harmony {
            for segment in 0..5_u32 {
                pixel(
                    batch,
                    view,
                    atlas,
                    [8.0 + segment as f32 * 24.0, 86.0],
                    [20.0, 8.0],
                    if segment < vitals.harmony.min(5) {
                        PresentationColor::Yellow
                    } else {
                        PresentationColor::LightGrey
                    },
                )?;
            }
        }
        if serene {
            pixel(
                batch,
                view,
                atlas,
                [8.0, 100.0],
                [120.0, 8.0],
                if vitals.serene {
                    PresentationColor::LightGreen
                } else {
                    PresentationColor::LightGrey
                },
            )?;
        }
    }
    if health_circle && let Some(vitals) = vitals {
        let [x, y] = view.tile_to_screen(own);
        let half = view.tile_px() as f32 / 2.0;
        let radius = half + f32::from(distance);
        let size = f32::from(size);
        for (side, value, max, color) in [
            (
                1.0,
                vitals.health,
                vitals.max_health,
                PresentationColor::Red,
            ),
            (-1.0, vitals.mana, vitals.max_mana, PresentationColor::Blue),
        ] {
            let filled = if max == 0 {
                0
            } else {
                (u64::from(value.min(max)) * 16 / u64::from(max)) as u32
            };
            for segment in 0..16_u32 {
                let angle = (segment as f32 + 0.5) * std::f32::consts::PI / 16.0;
                pixel(
                    batch,
                    view,
                    atlas,
                    [
                        x + half + side * radius * angle.sin() - size / 2.0,
                        y + half - radius * angle.cos() - size / 2.0,
                    ],
                    [size, size],
                    if segment < filled {
                        color
                    } else {
                        PresentationColor::LightGrey
                    },
                )?;
            }
        }
    }
    // The small combo gauge repeats confirmed Harmony charges beside the actor.
    if combo && let Some(vitals) = vitals {
        let [x, y] = view.tile_to_screen(own);
        let segment = (view.tile_px() as f32 - 4.0) / 5.0;
        for charge in 0..5_u32 {
            pixel(
                batch,
                view,
                atlas,
                [
                    x + 2.0 + charge as f32 * segment,
                    y + view.tile_px() as f32 + 2.0,
                ],
                [(segment - 1.0).max(1.0), 4.0],
                if charge < vitals.harmony.min(5) {
                    PresentationColor::Purple
                } else {
                    PresentationColor::LightGrey
                },
            )?;
        }
    }
    Ok(())
}

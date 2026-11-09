//! Server-owned health, party/PvP marks and NPC identity badges.
use super::*;
use oteryn_client_runtime::party_skull::PartyIcon;
use oteryn_session::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};

pub(super) struct Options {
    health: bool,
    own_marks: bool,
    other_marks: bool,
    npc: bool,
    floor: i16,
    own: Option<EntityRef>,
    scale: u16,
    age: std::time::Duration,
}
impl Options {
    pub(super) fn new(
        preferences: &crate::play::ReferencePreferences,
        floor: i16,
        own: Option<EntityRef>,
        age: std::time::Duration,
    ) -> Self {
        Self {
            age,
            health: preferences.creature_bars,
            scale: preferences.interface_scale,
            own_marks: preferences.hud_marks_own,
            other_marks: preferences.hud_marks_other,
            npc: preferences.hud_npc_icons,
            floor,
            own,
        }
    }
}

pub(super) fn draw(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    own: TileCoord,
    entities: &[WorldSpatialEntity],
    icons: &[PartyIcon],
    options: Options,
) -> Result<(), BatchError> {
    let scale = f32::from(options.scale) / 100.0;
    for entity in entities {
        let tile = TileCoord::new(entity.position.x, entity.position.y);
        if entity.position.floor != options.floor || !view.contains(tile) {
            continue;
        }
        let [x, y] = view.tile_to_screen(tile);
        if options.health
            && let EntityDetail::Actor { health_percent, .. } = entity.detail
        {
            let at = [x + 2.0 * scale, y + view.tile_px() as f32 - 5.0 * scale];
            let width = (view.tile_px() as f32 - 4.0 * scale).max(1.0);
            pixel(
                batch,
                view,
                atlas,
                at,
                [width, 3.0 * scale],
                PresentationColor::DarkRed,
            )?;
            pixel(
                batch,
                view,
                atlas,
                at,
                [
                    width * f32::from(health_percent.min(100)) / 100.0,
                    3.0 * scale,
                ],
                PresentationColor::LightGreen,
            )?;
        }
        if options.other_marks
            && Some(entity.entity) != options.own
            && let Some(icon) = icons.iter().find(|icon| icon.entity == entity.entity)
        {
            icons::draw(batch, view, atlas, [x, y], icon.shield, false, options.age)?;
            icons::draw(
                batch,
                view,
                atlas,
                [x + 16.0 * scale, y],
                icon.skull,
                true,
                options.age,
            )?;
        }
        if options.npc && entity.kind == EntityKind::Npc {
            for (row, bits) in [7_u8, 5, 7, 2].into_iter().enumerate() {
                for col in 0..3 {
                    if bits & (4 >> col) != 0 {
                        pixel(
                            batch,
                            view,
                            atlas,
                            [
                                x + view.tile_px() as f32 - 8.0 * scale + col as f32 * 2.0 * scale,
                                y + row as f32 * 2.0 * scale,
                            ],
                            [2.0 * scale; 2],
                            PresentationColor::SkyBlue,
                        )?;
                    }
                }
            }
        }
    }
    if options.own_marks
        && let Some(own_ref) = options.own
        && let Some(icon) = icons.iter().find(|icon| icon.entity == own_ref)
    {
        let [x, y] = view.tile_to_screen(own);
        icons::draw(batch, view, atlas, [x, y], icon.shield, false, options.age)?;
        icons::draw(
            batch,
            view,
            atlas,
            [x + 16.0 * scale, y],
            icon.skull,
            true,
            options.age,
        )?;
    }
    Ok(())
}

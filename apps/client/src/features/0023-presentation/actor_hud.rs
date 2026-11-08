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
}
impl Options {
    pub(super) fn new(
        preferences: &crate::play::ReferencePreferences,
        floor: i16,
        own: Option<EntityRef>,
    ) -> Self {
        Self {
            health: preferences.creature_bars,
            own_marks: preferences.hud_marks_own,
            other_marks: preferences.hud_marks_other,
            npc: preferences.hud_npc_icons,
            floor,
            own,
        }
    }
}

fn marks(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    at: [f32; 2],
    icon: &PartyIcon,
) -> Result<(), BatchError> {
    for (offset, value, pattern) in [
        (0.0, icon.shield, [5_u8, 7, 2]),
        (8.0, icon.skull, [7_u8, 5, 2]),
    ] {
        if value == 0 {
            continue;
        }
        let color = match value {
            1 | 3 | 5 | 7 | 9 => PresentationColor::Blue,
            2 | 4 | 6 | 8 | 10 => PresentationColor::Yellow,
            _ => PresentationColor::Red,
        };
        for (row, bits) in pattern.into_iter().enumerate() {
            for col in 0..3 {
                if bits & (4 >> col) != 0 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [at[0] + offset + col as f32 * 2.0, at[1] + row as f32 * 2.0],
                        [2.0; 2],
                        color,
                    )?;
                }
            }
        }
    }
    Ok(())
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
    for entity in entities {
        let tile = TileCoord::new(entity.position.x, entity.position.y);
        if entity.position.floor != options.floor || !view.contains(tile) {
            continue;
        }
        let [x, y] = view.tile_to_screen(tile);
        if options.health
            && let EntityDetail::Actor { health_percent, .. } = entity.detail
        {
            let at = [x + 2.0, y + view.tile_px() as f32 - 5.0];
            let width = (view.tile_px() as f32 - 4.0).max(1.0);
            pixel(
                batch,
                view,
                atlas,
                at,
                [width, 3.0],
                PresentationColor::DarkRed,
            )?;
            pixel(
                batch,
                view,
                atlas,
                at,
                [width * f32::from(health_percent.min(100)) / 100.0, 3.0],
                PresentationColor::LightGreen,
            )?;
        }
        if options.other_marks
            && Some(entity.entity) != options.own
            && let Some(icon) = icons.iter().find(|icon| icon.entity == entity.entity)
        {
            marks(batch, view, atlas, [x, y], icon)?;
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
                                x + view.tile_px() as f32 - 8.0 + col as f32 * 2.0,
                                y + row as f32 * 2.0,
                            ],
                            [2.0; 2],
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
        marks(batch, view, atlas, view.tile_to_screen(own), icon)?;
    }
    Ok(())
}

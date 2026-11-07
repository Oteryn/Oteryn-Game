//! Live item identity stays attached to the visible target, including its session binding.
use crate::input::{TargetKind, Targetable};
use oteryn_renderer::TileCoord;
use oteryn_session::world_map::{MapDefinition, MapItem, MapViewHeader, WorldMapViewSnapshot};
use oteryn_session::{ActorPosition, EntityDetail, EntityRef, WorldSpatialEntity};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundItem {
    Map {
        header: MapViewHeader,
        position: ActorPosition,
        item: MapItem,
    },
    Entity {
        reference: EntityRef,
        position: ActorPosition,
        definition: u32,
        quantity: u32,
        handle: Option<NonZeroU64>,
    },
}

impl GroundItem {
    #[must_use]
    pub const fn definition(self) -> u32 {
        match self {
            Self::Map { item, .. } => match item.definition {
                MapDefinition::Item(reference) | MapDefinition::Terrain(reference) => {
                    reference.get()
                }
            },
            Self::Entity { definition, .. } => definition,
        }
    }

    #[must_use]
    pub const fn quantity(self) -> u32 {
        match self {
            Self::Map { item, .. } => item.count,
            Self::Entity { quantity, .. } => quantity,
        }
    }

    pub(crate) const fn floor(self) -> i16 {
        match self {
            Self::Map { position, .. } | Self::Entity { position, .. } => position.floor,
        }
    }
}

pub(crate) fn map_targets(map: &WorldMapViewSnapshot) -> Vec<Targetable> {
    map.tiles
        .iter()
        .filter(|tile| tile.position.floor == map.header.origin.floor)
        .flat_map(|tile| {
            tile.items.iter().filter_map(move |item| {
                // Echo portal appearance: native `echo_warden::portal_entry` projects
                // item i54133. Its server caller activates on step-in, never USE.
                (matches!(item.definition, MapDefinition::Item(_)) && item.appearance_id != 54133)
                    .then_some(Targetable {
                        tile: TileCoord::new(tile.position.x, tile.position.y),
                        kind: TargetKind::Object,
                        glyph: false,
                        entity: None,
                        item: Some(GroundItem::Map {
                            header: map.header,
                            position: tile.position,
                            item: *item,
                        }),
                    })
            })
        })
        .collect()
}

pub(crate) fn entity_item(entity: &WorldSpatialEntity) -> Option<GroundItem> {
    match entity.detail {
        EntityDetail::Object {
            item_definition_ref,
            quantity,
            item_handle,
        }
        | EntityDetail::ObjectWithAppearance {
            item_definition_ref,
            quantity,
            item_handle,
            ..
        } => Some(GroundItem::Entity {
            reference: entity.entity,
            position: entity.position,
            definition: item_definition_ref,
            quantity,
            handle: item_handle,
        }),
        EntityDetail::Actor { .. } | EntityDetail::ActorWithIcons { .. } => None,
    }
}

//! W05d seasonal actors are selected by the committed reset calendar.
use super::*;
use crate::map::overlay::TilePos;

const SANTA: &str = "oteryn:npc.santa_claus";

// Crystal201189f data-global/npc/santa_claus.lua:1-25 identifies the native NPC;
// its donor spawn registry supplies no seasonal placement. Exact positions are
// the retained structured source Santa Claus revision443236:38-52 (captured by
// docs/reference/content-prep/season-20261107/W05d.md), not inferred offsets.
const SANTA_POSITIONS: [(u16, u16, i8); 20] = [
    (32655, 31665, 8),
    (33070, 32878, 6),
    (32307, 31837, 8),
    (33233, 32488, 7),
    (33192, 31810, 7),
    (32999, 31538, 10),
    (33447, 31317, 8),
    (32642, 31892, 9),
    (32322, 32834, 7),
    (32623, 32751, 7),
    (33613, 31899, 6),
    (32239, 31138, 7),
    (32357, 32210, 7),
    (32915, 32070, 9),
    (32809, 31245, 7),
    // Crystal data-global/scripts/lib/a_piece_of_cake_config.lua:232-239,
    // :489-495 binds these exact baker positions to their native identities.
    (32345, 31819, 7),
    (32348, 32272, 7),
    (33248, 31799, 7),
    (33192, 32401, 7),
    (32375, 32798, 7),
];

const BAKERS: [&str; 5] = [
    "oteryn:npc.benny_the_baker",
    "oteryn:npc.biff_the_baker",
    "oteryn:npc.brian_the_baker",
    "oteryn:npc.balduin_the_baker",
    "oteryn:npc.billy_the_baker",
];
fn key(index: usize) -> &'static str {
    if index < 15 {
        SANTA
    } else {
        BAKERS[index - 15]
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SeasonalNpcSpawns {
    positions: [bool; SANTA_POSITIONS.len()],
    actors: [Option<crate::foundation::ExactActorRef>; SANTA_POSITIONS.len()],
}

impl SeasonalNpcSpawns {
    pub(super) fn admit(
        boot: Option<&NpcBoot>,
        seasonal: &[String],
        enterable: impl Fn(TilePos) -> bool,
    ) -> Self {
        let positions = std::array::from_fn(|index| {
            let event = if index < 15 {
                "christmas"
            } else {
                "piece_of_cake"
            };
            let (x, y, z) = SANTA_POSITIONS[index];
            seasonal.iter().any(|active| active == event)
                && boot.is_some_and(|boot| boot.admits(key(index)))
                && enterable(TilePos { x, y, floor: -z })
        });
        Self {
            positions,
            actors: [None; SANTA_POSITIONS.len()],
        }
    }

    pub(super) fn is_empty(self) -> bool {
        !self.positions.iter().any(|admitted| *admitted)
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        let present: Vec<_> = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .map(|npc| (npc.actor, npc.position))
            .collect();
        let mut outcome = Ok(());
        for (index, (x, y, z)) in SANTA_POSITIONS.into_iter().enumerate() {
            let floor = -z;
            let key = key(index);
            if !self.positions[index]
                || self.actors[index].is_some_and(|actor| runtime.npc_key(actor) == Some(key))
            {
                continue;
            }
            if let Some((actor, _)) = present.iter().find(|(actor, position)| {
                position.x == i32::from(x)
                    && position.y == i32::from(y)
                    && position.floor == i16::from(z)
                    && runtime.npc_key(*actor) == Some(key)
            }) {
                self.actors[index] = Some(*actor);
                continue;
            }
            // Crystal src/creatures/creature.hpp:806 defaults an unconfigured NPC south.
            let placed = runtime.place_dynamic_npc(
                key,
                NpcPlacement {
                    x,
                    y,
                    floor,
                    direction: MovementFacing::South,
                },
            );
            outcome = outcome.and(placed.map(|actor| {
                self.actors[index] = Some(actor);
            }));
        }
        outcome
    }
}

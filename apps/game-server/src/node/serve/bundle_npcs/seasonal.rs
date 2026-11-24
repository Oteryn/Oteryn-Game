//! W05d seasonal actors are selected by the committed reset calendar.
use super::*;
use crate::map::overlay::TilePos;

const BRUTUS: &str = "oteryn:npc.brutus_the_baker";
const BRUTUS_POSITION: (u16, u16, i8) = (33303, 32064, 7);

const SANTA: &str = "oteryn:npc.santa_claus";
const MESSENGER: &str = "oteryn:npc.messenger_of_santa";
const SANTA_COUNT: usize = 15;

// Crystal201189f data-global/npc/santa_claus.lua:1-25 identifies the native NPC;
// its donor spawn registry supplies no seasonal placement. Exact positions are
// retained structured BR source, not donor spawn coordinates:
// docs/reference/content-prep/season-20261107/seasonal-calendar-source.json:87
// binds all15 placements to Santa Claus revision443236, original lines38-52.
const SOURCE_POSITIONS: [(u16, u16, i8); SANTA_COUNT + BAKERS.len()] = [
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
const SEASONAL_COUNT: usize = SOURCE_POSITIONS.len() + SANTA_COUNT;

fn key(index: usize) -> &'static str {
    if index < SANTA_COUNT {
        SANTA
    } else if index < SOURCE_POSITIONS.len() {
        BAKERS[index - SANTA_COUNT]
    } else {
        MESSENGER
    }
}

fn adjacent(position: TilePos) -> impl Iterator<Item = TilePos> {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            Some(TilePos {
                x: position.x.checked_add_signed(dx)?,
                y: position.y.checked_add_signed(dy)?,
                floor: position.floor,
            })
        })
}

fn santa_cell(anchor: TilePos, enterable: &impl Fn(TilePos) -> bool) -> Option<TilePos> {
    // Some retained anchors lie on an obstacle in the captured map.
    // Keep the source cell when possible and remain within one step otherwise.
    std::iter::once(anchor)
        .chain(adjacent(anchor))
        .find(|&cell| enterable(cell))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SeasonalNpcSpawns {
    brutus_admitted: bool,
    brutus_present: bool,
    brutus_actor: Option<crate::foundation::ExactActorRef>,
    positions: [Option<TilePos>; SEASONAL_COUNT],
    actors: [Option<crate::foundation::ExactActorRef>; SEASONAL_COUNT],
}

impl Default for SeasonalNpcSpawns {
    fn default() -> Self {
        Self {
            brutus_admitted: false,
            brutus_present: false,
            brutus_actor: None,
            positions: [None; SEASONAL_COUNT],
            actors: [None; SEASONAL_COUNT],
        }
    }
}

impl SeasonalNpcSpawns {
    pub(super) fn admit(
        boot: Option<&NpcBoot>,
        seasonal: &[String],
        enterable: impl Fn(TilePos) -> bool,
    ) -> Self {
        let positions = std::array::from_fn(|index| {
            let npc = key(index);
            let event = if BAKERS.contains(&npc) {
                "piece_of_cake"
            } else {
                "christmas"
            };
            if !seasonal.iter().any(|active| active == event)
                || !boot.is_some_and(|boot| boot.admits(npc))
            {
                return None;
            }
            let source = if npc == MESSENGER {
                index - SOURCE_POSITIONS.len()
            } else {
                index
            };
            let (x, y, z) = SOURCE_POSITIONS[source];
            let anchor = TilePos { x, y, floor: -z };
            match npc {
                SANTA => santa_cell(anchor, &enterable),
                // The retained TibiaWiki BR Messenger Of Santa rev416417
                // describes the merchant beside Santa in the same towns.
                MESSENGER => {
                    let santa = santa_cell(anchor, &enterable)?;
                    adjacent(santa).find(|&cell| enterable(cell))
                }
                _ => enterable(anchor).then_some(anchor),
            }
        });
        let (x, y, z) = BRUTUS_POSITION;
        Self {
            brutus_admitted: boot.is_some_and(|boot| boot.admits(BRUTUS))
                && enterable(TilePos { x, y, floor: -z }),
            positions,
            ..Self::default()
        }
    }

    pub(super) fn is_empty(self) -> bool {
        !self.brutus_admitted && self.positions.iter().all(Option::is_none)
    }

    /// Crystal data-global/scripts/lib/a_piece_of_cake_config.lua:456-483 and cake_server_save.lua:8-10: the persisted
    /// next-save projection owns Brutus, independently of the stage-one calendar.
    pub(super) fn set_brutus_present(&mut self, present: bool) {
        self.brutus_present = present;
    }

    pub(super) fn reconcile_brutus(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), NpcPlacementError> {
        if !self.brutus_admitted {
            return Ok(());
        }
        if !self.brutus_present {
            if let Some(actor) = self.brutus_actor {
                runtime.remove_dynamic_npc_actor(actor)?;
                self.brutus_actor = None;
            }
            return Ok(());
        }
        if self
            .brutus_actor
            .is_some_and(|actor| runtime.npc_key(actor) == Some(BRUTUS))
        {
            return Ok(());
        }
        // Crystal data-global/scripts/lib/a_piece_of_cake_config.lua:242-243.
        let (x, y, z) = BRUTUS_POSITION;
        if let Some(npc) = runtime.visible_entities().npcs.into_iter().find(|npc| {
            runtime.npc_key(npc.actor) == Some(BRUTUS)
                && npc.position.x == i32::from(x)
                && npc.position.y == i32::from(y)
                && npc.position.floor == i16::from(z)
        }) {
            self.brutus_actor = Some(npc.actor);
            return Ok(());
        }
        self.brutus_actor = Some(runtime.place_cake_brutus_npc()?);
        Ok(())
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        let present: Vec<_> = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .map(|npc| (npc.actor, npc.position))
            .collect();
        let mut outcome = self.reconcile_brutus(runtime);
        for (index, position) in self.positions.into_iter().enumerate() {
            let Some(TilePos { x, y, floor }) = position else {
                continue;
            };
            let key = key(index);
            if self.actors[index].is_some_and(|actor| runtime.npc_key(actor) == Some(key)) {
                continue;
            }
            if let Some((actor, _)) = present.iter().find(|(actor, position)| {
                position.x == i32::from(x)
                    && position.y == i32::from(y)
                    && position.floor == -i16::from(floor)
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

//! W05d calendar-bound NPCs on the existing Channel clock cadence.
//!
//! Cake: Crystal 201189f8 data-global/scripts/lib/a_piece_of_cake_config.lua:230-237,
//! 406-429,489-495. Santa: retained TibiaWiki BR page10297/revision443236, captured
//! 2026-09-28 in imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json.
use super::{ChannelRuntimeV1, MovementFacing, NpcBoot, NpcPlacement, NpcPlacementError};
use oteryn_protocol_oteryn::world_event::SeasonalEventKind;

const CAKE: SeasonalEventKind = SeasonalEventKind::BirthdayCake;
const CHRISTMAS: SeasonalEventKind = SeasonalEventKind::Christmas;
const NPCS: [(&str, (u16, u16, i8), SeasonalEventKind); 35] = [
    ("oteryn:npc.balduin_the_baker", (33192, 32401, 7), CAKE),
    ("oteryn:npc.benny_the_baker", (32345, 31819, 7), CAKE),
    ("oteryn:npc.biff_the_baker", (32348, 32272, 7), CAKE),
    ("oteryn:npc.billy_the_baker", (32375, 32798, 7), CAKE),
    ("oteryn:npc.brian_the_baker", (33248, 31799, 7), CAKE),
    ("oteryn:npc.santa_claus", (32239, 31138, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32307, 31837, 8), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32322, 32834, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32357, 32210, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32623, 32751, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32642, 31892, 9), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32655, 31665, 8), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32809, 31245, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32915, 32070, 9), CHRISTMAS),
    ("oteryn:npc.santa_claus", (32999, 31538, 10), CHRISTMAS),
    ("oteryn:npc.santa_claus", (33070, 32878, 6), CHRISTMAS),
    ("oteryn:npc.santa_claus", (33192, 31810, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (33233, 32488, 7), CHRISTMAS),
    ("oteryn:npc.santa_claus", (33447, 31317, 8), CHRISTMAS),
    ("oteryn:npc.santa_claus", (33613, 31899, 6), CHRISTMAS),
    (
        "oteryn:npc.messenger_of_santa",
        (32240, 31138, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32308, 31837, 8),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32323, 32834, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32358, 32210, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32624, 32751, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32642, 31893, 9),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32655, 31666, 8),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32810, 31245, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (32916, 32070, 9),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33000, 31538, 10),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33071, 32878, 6),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33193, 31810, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33234, 32488, 7),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33448, 31317, 8),
        CHRISTMAS,
    ),
    (
        "oteryn:npc.messenger_of_santa",
        (33614, 31899, 6),
        CHRISTMAS,
    ),
];

#[derive(Debug, Clone, Copy)]
pub(super) struct SeasonalNpcSpawns {
    admitted: [bool; NPCS.len()],
}

impl SeasonalNpcSpawns {
    pub(super) fn from_boot(boot: Option<&NpcBoot>) -> Self {
        Self {
            admitted: std::array::from_fn(|index| {
                boot.is_some_and(|boot| boot.admits(NPCS[index].0))
            }),
        }
    }

    pub(super) fn is_empty(self) -> bool {
        !self.admitted.iter().any(|admitted| *admitted)
    }

    pub(super) fn tick(
        self,
        runtime: &mut ChannelRuntimeV1,
        now_utc_ms: u64,
    ) -> Result<(), NpcPlacementError> {
        let Ok(unix_seconds) = i64::try_from(now_utc_ms / 1000) else {
            return Ok(());
        };
        let events = oteryn_gameplay_foundation::scheduled_events::seasonal_events_at(unix_seconds);
        let wanted = |kind| {
            if kind == CHRISTMAS {
                return oteryn_gameplay_foundation::scheduled_events::santa_occurrence_at(
                    unix_seconds,
                )
                .is_some();
            }
            events
                .iter()
                .any(|event| event.event == kind && event.active)
        };
        let mut outcome = Ok(());
        // One remove per placement also drains all fifteen Santa placements, whose NPC key
        // is intentionally shared. No ordinary NPC is named by this bounded source table.
        for (index, (key, _, kind)) in NPCS.iter().enumerate() {
            if self.admitted[index] && !wanted(*kind) {
                outcome = outcome.and(runtime.remove_dynamic_npc(key));
            }
        }
        for (index, (key, (x, y, z), kind)) in NPCS.iter().enumerate() {
            if !self.admitted[index] || !wanted(*kind) {
                continue;
            }
            let present = runtime.visible_entities().npcs.iter().any(|npc| {
                runtime.npc_key(npc.actor) == Some(*key)
                    && npc.position.x == i32::from(*x)
                    && npc.position.y == i32::from(*y)
                    && npc.position.floor == i16::from(*z)
            });
            if !present {
                outcome = outcome.and(
                    runtime
                        .place_dynamic_npc(
                            key,
                            NpcPlacement {
                                x: *x,
                                y: *y,
                                floor: -*z,
                                direction: MovementFacing::South,
                            },
                        )
                        .map(|_| ()),
                );
            }
        }
        outcome
    }
}

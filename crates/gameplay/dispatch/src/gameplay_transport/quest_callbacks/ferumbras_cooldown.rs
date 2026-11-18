//! Crystal 201189f8 movements_seal.lua and startup/tables/item.lua, AIDs 24839–24846.
use super::*;
use crate::map::overlay::TilePos;

type Location = (u16, u16, i8);
pub(super) struct Seal {
    source: Location,
    appearance: u16,
    retreat: Location,
    entrance: Option<Location>,
    track: &'static str,
    hours: i64,
}

macro_rules! seal {
    ($source:expr, $item:expr, $retreat:expr, $entrance:expr, $boss:literal, $hours:expr) => {
        Seal {
            source: $source,
            appearance: $item,
            retreat: $retreat,
            entrance: $entrance,
            track: concat!(
                "oteryn:quest-progress/ferumbras-ascension/defeated-at/",
                $boss
            ),
            hours: $hours,
        }
    };
}

const SEALS: [Seal; 8] = [
    seal!(
        (33482, 32775, -12),
        1949,
        (33484, 32775, -12),
        Some((33419, 32841, -11)),
        "tarbaz",
        44
    ),
    seal!(
        (33432, 32332, -14),
        1949,
        (33432, 32330, -14),
        Some((33452, 32356, -13)),
        "ragiaz",
        44
    ),
    seal!(
        (33199, 31439, -13),
        1949,
        (33197, 31438, -13),
        Some((33230, 31493, -13)),
        "plagirath",
        44
    ),
    seal!(
        (33402, 32406, -15),
        1949,
        (33399, 32402, -15),
        Some((33380, 32454, -14)),
        "razzagorn",
        44
    ),
    seal!(
        (33662, 32682, -10),
        1949,
        (33664, 32682, -10),
        Some((33680, 32736, -11)),
        "zamulosh",
        44
    ),
    seal!(
        (33677, 32689, -13),
        1949,
        (33675, 32690, -13),
        None,
        "mazoran",
        44
    ),
    seal!(
        (33477, 32698, -14),
        8280,
        (33477, 32701, -14),
        None,
        "shulgrax",
        44
    ),
    seal!(
        (33324, 31372, -14),
        1949,
        (33324, 31374, -14),
        Some((33270, 31474, -14)),
        "ferumbras_mortal_shell",
        332
    ),
];

fn position((x, y, floor): Location) -> TilePos {
    TilePos { x, y, floor }
}

pub(super) fn seal_at(map: &BundleMap, at: TilePos) -> Option<&'static Seal> {
    let seal = SEALS.iter().find(|seal| position(seal.source) == at)?;
    let base = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let matches = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter(|(ordinal, (id, _))| {
            let Ok(ordinal) = u8::try_from(*ordinal) else {
                return false;
            };
            placement_key(at, ordinal).is_some_and(|key| !map.facts.base_hidden(key))
                && map.facts.quest_callback_base_appearance(at, ordinal, **id)
                    == Some(seal.appearance)
        })
        .count();
    (matches == 1 && !map.facts.house_tile(at)).then_some(seal)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_ferumbras_seal(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let (Some(map), Some(at)) = (self.bundle_map(), native(entered.position())) else {
            return entered;
        };
        let Some(seal) = seal_at(map, at) else {
            return entered;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return entered;
        };
        let Ok(cooling) = self
            .root
            .quest_cooldown_active(
                self.character,
                fence.character_id,
                seal.track,
                seal.hours * 3600,
            )
            .await
        else {
            return entered;
        };
        let destination = if cooling {
            position(seal.retreat)
        } else {
            let Some(destination) = seal
                .entrance
                .map(position)
                .or_else(|| map.floors.teleport(at))
            else {
                return entered;
            };
            destination
        };
        let result = self
            .step_qualified_quest_portal(permit, actor, session, entered, destination, |map| {
                seal_at(map, at).is_some()
            })
            .await;
        if cooling && result != entered {
            map.facts.emit_movement_effect(destination, 11);
            self.states.chat.push_quest_feedback(
                session,
                "You need to wait for time to pass for this boss again.",
            );
        }
        result
    }
}

//! NPC-RT-1: the runtime NPC actors of one Channel (NPC-BEHAVIOUR-0 §3.1, NPC-PLACE-1 §3.2, §7).
//!
//! A Channel creates one runtime-only actor per NPC placement when it starts, in the canonical
//! actor order of the placement table: NPC key, then native floor, `y` and `x`. Each actor lives
//! in a fixed carrier slot ([`Slot::NpcOccupied`]); it is shown as `EntityKind::Npc`, blocks
//! movement like a creature, is never a combat target and is never removed within the
//! generation. Nothing is durable.
//!
//! The table here is the typed §3.2 shape. Until NPC-PLACE-1b ships the format v4 reader, only
//! tests supply it; the boot wiring and the join snapshot are the follow-up packet.

use super::{
    ActorLocalGeneration, ActorLocalId, ActorRef, CarrierError, ChannelRuntimeV1, ExactActorRef,
    LocalPosition, MovementFacing, Slot, VersionedPosition,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// `NPCPLACE1-RL-01` (= `NPCBEH0-RL-01`): NPC placements, and so NPC actors, per Channel.
pub(crate) const MAX_NPC_PLACEMENTS: usize = 2_048;
/// `NPCPLACE1-RL-02`: placements per NPC.
pub(crate) const MAX_PLACEMENTS_PER_NPC: usize = 16;
/// Longest NPC key, in bytes.
pub(crate) const MAX_NPC_KEY_BYTES: usize = 128;
const NPC_KEY_PREFIX: &str = "oteryn:npc.";

/// One placement of §3.2: native floor (`-z`), `y`, `x` and the initial facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NpcPlacement {
    pub(crate) floor: i8,
    pub(crate) y: u16,
    pub(crate) x: u16,
    pub(crate) direction: MovementFacing,
}

/// One NPC of §3.2: its key and its placements, strictly ascending by (`floor`, `y`, `x`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NpcPlacements {
    pub(crate) key: String,
    pub(crate) placements: Vec<NpcPlacement>,
}

/// The NPC table of §3.2: NPCs strictly ascending by key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NpcPlacementTable {
    pub(crate) npcs: Vec<NpcPlacements>,
}

/// Why a placement table or one of its NPCs gets no actors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NpcPlacementError {
    /// A key is empty, too long, outside ASCII `0x21..=0x7E` or not under `oteryn:npc.`.
    InvalidKey,
    /// The NPCs are not strictly ascending by key.
    KeysNotAscending,
    /// An NPC has no placement.
    NoPlacements,
    /// `NPCPLACE1-RL-02`.
    PlacementsPerNpcOverLimit,
    /// `NPCPLACE1-RL-01`.
    PlacementsOverLimit,
    /// The placements of an NPC are not strictly ascending by (`floor`, `y`, `x`).
    PlacementsNotAscending,
    /// Two placements share a cell.
    SharedCell,
    /// A native floor outside `-15..=0`, the frame's legacy `z` `0..=15`.
    FloorOutsideFrame,
    /// The Channel already has NPC actors: placement runs once, at Channel start.
    AlreadyPlaced,
    /// A cell another actor occupies.
    CellOccupied,
    /// §7: in a production World, an NPC the loaded catalogue holds or does not have.
    NpcNotAdmitted(String),
    Carrier(CarrierError),
}

/// What [`ChannelRuntimeV1::place_npcs`] did: the actors in frame order and, outside a
/// production World, the keys of the NPCs that got none (§7), for the caller to log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NpcPlacementOutcome {
    pub(crate) actors: Vec<ExactActorRef>,
    pub(crate) not_admitted: Vec<String>,
}

fn key_ok(key: &str) -> bool {
    key.len() <= MAX_NPC_KEY_BYTES
        && key.starts_with(NPC_KEY_PREFIX)
        && key.bytes().all(|byte| (0x21..=0x7E).contains(&byte))
}

/// The movement cell of a placement: the runtime floor is legacy `z`, the native floor `-z`.
fn cell(placement: NpcPlacement) -> Result<LocalPosition, NpcPlacementError> {
    if !(-15..=0).contains(&placement.floor) {
        return Err(NpcPlacementError::FloorOutsideFrame);
    }
    Ok(LocalPosition {
        x: i32::from(placement.x),
        y: i32::from(placement.y),
        floor: -i16::from(placement.floor),
    })
}

impl NpcPlacementTable {
    /// The §3.2 and §3.3 checks the format v4 reader also applies; extent and floors stay the
    /// reader's.
    pub(crate) fn validate(&self) -> Result<(), NpcPlacementError> {
        let mut total = 0usize;
        let mut cells = BTreeSet::new();
        for (index, npc) in self.npcs.iter().enumerate() {
            if !key_ok(&npc.key) {
                return Err(NpcPlacementError::InvalidKey);
            }
            if index > 0 && self.npcs[index - 1].key >= npc.key {
                return Err(NpcPlacementError::KeysNotAscending);
            }
            if npc.placements.is_empty() {
                return Err(NpcPlacementError::NoPlacements);
            }
            if npc.placements.len() > MAX_PLACEMENTS_PER_NPC {
                return Err(NpcPlacementError::PlacementsPerNpcOverLimit);
            }
            total += npc.placements.len();
            if total > MAX_NPC_PLACEMENTS {
                return Err(NpcPlacementError::PlacementsOverLimit);
            }
            if npc.placements.windows(2).any(|pair| {
                (pair[0].floor, pair[0].y, pair[0].x) >= (pair[1].floor, pair[1].y, pair[1].x)
            }) {
                return Err(NpcPlacementError::PlacementsNotAscending);
            }
            for placement in &npc.placements {
                let at = cell(*placement)?;
                if !cells.insert((at.floor, at.y, at.x)) {
                    return Err(NpcPlacementError::SharedCell);
                }
            }
        }
        Ok(())
    }
}

#[allow(
    dead_code,
    reason = "NPC-RT-1: no runtime caller until the boot wiring follow-up after NPC-PLACE-1b"
)]
impl ChannelRuntimeV1 {
    /// NPC-BEHAVIOUR-0 §3.1: one actor per placement, allocated in frame order under the pinned
    /// Movement context, once, at Channel start. `admitted` answers whether the loaded NPC
    /// catalogue has the key and does not hold it (§7). In a `production` World an NPC it
    /// refuses refuses the whole table; otherwise that NPC gets no actor and its key is
    /// returned. All or nothing: any refusal leaves the carrier unchanged.
    pub(crate) fn place_npcs(
        &mut self,
        table: &NpcPlacementTable,
        admitted: impl Fn(&str) -> bool,
        production: bool,
    ) -> Result<NpcPlacementOutcome, NpcPlacementError> {
        table.validate()?;
        self.carrier
            .validate_current_continuity(&self.continuity)
            .map_err(NpcPlacementError::Carrier)?;
        self.owner_fence().map_err(NpcPlacementError::Carrier)?;
        if self
            .carrier
            .slots
            .iter()
            .any(|slot| matches!(slot, Slot::NpcOccupied { .. }))
        {
            return Err(NpcPlacementError::AlreadyPlaced);
        }
        let mut outcome = NpcPlacementOutcome::default();
        let mut placed = Vec::new();
        for npc in &table.npcs {
            if admitted(&npc.key) {
                placed.push(npc);
            } else if production {
                return Err(NpcPlacementError::NpcNotAdmitted(npc.key.clone()));
            } else {
                outcome.not_admitted.push(npc.key.clone());
            }
        }
        let context = self.pinned_position_context();
        let count = placed.iter().map(|npc| npc.placements.len()).sum::<usize>();
        let free = self
            .carrier
            .slots
            .iter()
            .filter(|slot| matches!(slot, Slot::VacantReusable { .. }))
            .count();
        if free < count {
            return Err(NpcPlacementError::Carrier(CarrierError::CapacityExceeded));
        }
        for npc in &placed {
            for placement in &npc.placements {
                if self.carrier.cell_occupied(context, cell(*placement)?) {
                    return Err(NpcPlacementError::CellOccupied);
                }
            }
        }
        outcome.actors.reserve_exact(count);
        let slots_before = self.carrier.slots.clone();
        let free_head_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        for npc in placed {
            let key: Arc<str> = Arc::from(npc.key.as_str());
            for placement in &npc.placements {
                match self.admit_npc(Arc::clone(&key), *placement, context) {
                    Ok(actor) => outcome.actors.push(actor),
                    Err(error) => {
                        self.carrier.slots = slots_before;
                        self.carrier.free_head = free_head_before;
                        self.carrier.occupied = occupied_before;
                        return Err(error);
                    }
                }
            }
        }
        Ok(outcome)
    }

    /// One NPC actor in the free-list head slot, the same allocation `admit_inner` makes.
    fn admit_npc(
        &mut self,
        key: Arc<str>,
        placement: NpcPlacement,
        context: super::PreProductionPositionContext,
    ) -> Result<ExactActorRef, NpcPlacementError> {
        let position = cell(placement)?;
        let carrier = &mut self.carrier;
        let free_head = carrier
            .free_head
            .ok_or(NpcPlacementError::Carrier(CarrierError::CapacityExceeded))?;
        let index = usize::try_from(free_head)
            .map_err(|_| NpcPlacementError::Carrier(CarrierError::CapacityArithmeticOverflow))?;
        let Slot::VacantReusable {
            generation,
            next_free,
        } = carrier.slots[index]
        else {
            unreachable!("free-list head must name a reusable slot");
        };
        let Some(generation) = generation.checked_add(1) else {
            return Err(NpcPlacementError::Carrier(
                CarrierError::ActorGenerationExhausted,
            ));
        };
        let actor_local_id = index
            .checked_add(1)
            .and_then(|value| u32::try_from(value).ok())
            .map(ActorLocalId)
            .ok_or(NpcPlacementError::Carrier(
                CarrierError::CapacityArithmeticOverflow,
            ))?;
        let actor_ref = ActorRef {
            world_id: carrier.world_id,
            channel_id: carrier.channel_id,
            scope_generation: carrier.scope_generation,
            actor_local_id,
            actor_local_generation: ActorLocalGeneration(generation),
        };
        carrier.slots[index] = Slot::NpcOccupied {
            generation,
            key,
            position: VersionedPosition {
                actor_local_id,
                actor_local_generation: ActorLocalGeneration(generation),
                context,
                position,
                revision: 1,
                facing: Some(placement.direction),
            },
        };
        carrier.free_head = next_free;
        carrier.occupied.push(free_head);
        Ok(ExactActorRef(actor_ref))
    }

    /// The NPC key of a current NPC actor; `None` for any other or stale actor.
    pub(crate) fn npc_key(&self, actor: ExactActorRef) -> Option<&str> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::NpcOccupied {
                generation, key, ..
            } if *generation == actor.0.actor_local_generation.0 => Some(key),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::super::tests::runtime;
    use super::*;
    use crate::foundation::MovementLocalPosition;

    fn at(floor: i8, y: u16, x: u16) -> NpcPlacement {
        NpcPlacement {
            floor,
            y,
            x,
            direction: MovementFacing::South,
        }
    }

    fn npc(key: &str, placements: Vec<NpcPlacement>) -> NpcPlacements {
        NpcPlacements {
            key: key.into(),
            placements,
        }
    }

    fn table(npcs: Vec<NpcPlacements>) -> NpcPlacementTable {
        NpcPlacementTable { npcs }
    }

    fn all(_: &str) -> bool {
        true
    }

    #[test]
    fn actors_follow_frame_order_and_show_as_npcs() {
        let mut runtime = runtime(8);
        let placements = table(vec![
            npc("oteryn:npc.alice", vec![at(-7, 10, 20), at(-7, 11, 3)]),
            npc("oteryn:npc.bob", vec![at(-8, 1, 1)]),
        ]);
        let outcome = runtime.place_npcs(&placements, all, true).expect("placed");
        assert_eq!(outcome.actors.len(), 3);
        assert!(outcome.not_admitted.is_empty());
        let ids = outcome
            .actors
            .iter()
            .map(|actor| actor.0.actor_local_id.0)
            .collect::<Vec<_>>();
        assert_eq!(ids, [1, 2, 3]);
        assert_eq!(runtime.npc_key(outcome.actors[0]), Some("oteryn:npc.alice"));
        assert_eq!(runtime.npc_key(outcome.actors[2]), Some("oteryn:npc.bob"));

        let visible = runtime.visible_entities();
        assert!(visible.players.is_empty());
        assert!(visible.creatures.is_empty());
        let shown = visible
            .npcs
            .iter()
            .map(|entry| (entry.actor, entry.position, entry.revision))
            .collect::<Vec<_>>();
        // Native floor -7 is legacy z 7.
        assert_eq!(
            shown,
            [
                (
                    outcome.actors[0],
                    MovementLocalPosition {
                        x: 20,
                        y: 10,
                        floor: 7
                    },
                    1
                ),
                (
                    outcome.actors[1],
                    MovementLocalPosition {
                        x: 3,
                        y: 11,
                        floor: 7
                    },
                    1
                ),
                (
                    outcome.actors[2],
                    MovementLocalPosition {
                        x: 1,
                        y: 1,
                        floor: 8
                    },
                    1
                ),
            ]
        );
        assert!(matches!(
            &runtime.carrier.slots[0],
            Slot::NpcOccupied { position, .. } if position.facing == Some(MovementFacing::South)
        ));
    }

    #[test]
    fn npcs_block_movement_and_are_never_combat_targets() {
        let mut runtime = runtime(4);
        let outcome = runtime
            .place_npcs(
                &table(vec![npc("oteryn:npc.a", vec![at(-7, 5, 2)])]),
                all,
                true,
            )
            .expect("placed");
        let npc_actor = outcome.actors[0];
        let creature = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 1,
                y: 5,
                floor: 7,
            })
            .expect("creature");
        assert_eq!(
            runtime.position_occupied_by_other(
                creature,
                MovementLocalPosition {
                    x: 2,
                    y: 5,
                    floor: 7
                }
            ),
            Ok(true)
        );
        assert_eq!(
            runtime.position_occupied_by_other(
                creature,
                MovementLocalPosition {
                    x: 3,
                    y: 5,
                    floor: 7
                }
            ),
            Ok(false)
        );
        assert!(!runtime.contains_live_creature(npc_actor));
        assert!(
            !runtime
                .carrier
                .current_owner_exact_lookup(&runtime.continuity)
                .contains(npc_actor)
        );
        assert_eq!(
            runtime.carrier.lookup(&runtime.continuity, npc_actor.0),
            Err(CarrierError::NotCreature)
        );
        assert_eq!(
            runtime
                .carrier
                .check_remove(&runtime.continuity, npc_actor.0, &[]),
            Err(CarrierError::PlanConflict)
        );
        assert!(runtime.read_actor_position(npc_actor).is_err());
        // Census for combat and AI never lists an NPC.
        let census = runtime.positioned_actor_census().expect("census");
        assert_eq!(census.len(), 1);
        assert_eq!(census[0].0, creature);
    }

    #[test]
    fn a_stale_generation_fails_closed() {
        let mut runtime = runtime(4);
        let outcome = runtime
            .place_npcs(
                &table(vec![npc("oteryn:npc.a", vec![at(-7, 5, 2)])]),
                all,
                true,
            )
            .expect("placed");
        let mut stale = outcome.actors[0];
        stale.0.actor_local_generation = ActorLocalGeneration(stale.0.actor_local_generation.0 + 1);
        assert_eq!(runtime.npc_key(stale), None);
        assert_eq!(
            runtime.carrier.lookup(&runtime.continuity, stale.0),
            Err(CarrierError::StaleActorGeneration)
        );
    }

    #[test]
    fn production_refuses_an_npc_the_catalogue_refuses_and_changes_nothing() {
        let mut runtime = runtime(4);
        let placements = table(vec![
            npc("oteryn:npc.a", vec![at(-7, 5, 2)]),
            npc("oteryn:npc.held", vec![at(-7, 5, 3)]),
        ]);
        let before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.place_npcs(&placements, |key| key != "oteryn:npc.held", true),
            Err(NpcPlacementError::NpcNotAdmitted("oteryn:npc.held".into()))
        );
        assert_eq!(runtime.carrier.slots, before);

        let outcome = runtime
            .place_npcs(&placements, |key| key != "oteryn:npc.held", false)
            .expect("non-production");
        assert_eq!(outcome.actors.len(), 1);
        assert_eq!(outcome.not_admitted, ["oteryn:npc.held"]);
        assert_eq!(runtime.npc_key(outcome.actors[0]), Some("oteryn:npc.a"));
    }

    #[test]
    fn placement_runs_once_and_refuses_without_mutation() {
        let mut runtime = runtime(2);
        let placements = table(vec![npc("oteryn:npc.a", vec![at(-7, 5, 2)])]);
        runtime.place_npcs(&placements, all, true).expect("placed");
        let before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.place_npcs(&placements, all, true),
            Err(NpcPlacementError::AlreadyPlaced)
        );
        assert_eq!(runtime.carrier.slots, before);

        // Capacity: two placements, one free slot.
        let mut runtime = super::super::tests::runtime(1);
        let before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.place_npcs(
                &table(vec![npc("oteryn:npc.a", vec![at(-7, 5, 2), at(-7, 5, 3)])]),
                all,
                true
            ),
            Err(NpcPlacementError::Carrier(CarrierError::CapacityExceeded))
        );
        assert_eq!(runtime.carrier.slots, before);

        // A cell another actor occupies.
        let mut runtime = super::super::tests::runtime(4);
        runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 2,
                y: 5,
                floor: 7,
            })
            .expect("creature");
        let before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.place_npcs(&placements, all, true),
            Err(NpcPlacementError::CellOccupied)
        );
        assert_eq!(runtime.carrier.slots, before);
    }

    #[test]
    fn the_table_is_validated_in_the_section_3_2_shape() {
        let check = |npcs| table(npcs).validate();
        assert_eq!(check(vec![]), Ok(()));
        assert_eq!(
            check(vec![npc("oteryn:creature.rat", vec![at(-7, 1, 1)])]),
            Err(NpcPlacementError::InvalidKey)
        );
        assert_eq!(
            check(vec![npc("oteryn:npc.a b", vec![at(-7, 1, 1)])]),
            Err(NpcPlacementError::InvalidKey)
        );
        let long = format!("oteryn:npc.{}", "a".repeat(MAX_NPC_KEY_BYTES - 10));
        assert_eq!(
            check(vec![npc(&long, vec![at(-7, 1, 1)])]),
            Err(NpcPlacementError::InvalidKey)
        );
        let longest = format!("oteryn:npc.{}", "a".repeat(MAX_NPC_KEY_BYTES - 11));
        assert_eq!(check(vec![npc(&longest, vec![at(-7, 1, 1)])]), Ok(()));
        assert_eq!(
            check(vec![
                npc("oteryn:npc.b", vec![at(-7, 1, 1)]),
                npc("oteryn:npc.a", vec![at(-7, 1, 2)]),
            ]),
            Err(NpcPlacementError::KeysNotAscending)
        );
        assert_eq!(
            check(vec![
                npc("oteryn:npc.a", vec![at(-7, 1, 1)]),
                npc("oteryn:npc.a", vec![at(-7, 1, 2)]),
            ]),
            Err(NpcPlacementError::KeysNotAscending)
        );
        assert_eq!(
            check(vec![npc("oteryn:npc.a", vec![])]),
            Err(NpcPlacementError::NoPlacements)
        );
        assert_eq!(
            check(vec![npc("oteryn:npc.a", vec![at(-7, 1, 2), at(-7, 1, 1)])]),
            Err(NpcPlacementError::PlacementsNotAscending)
        );
        // Floor sorts first: native -8 before -7.
        assert_eq!(
            check(vec![npc("oteryn:npc.a", vec![at(-8, 9, 9), at(-7, 1, 1)])]),
            Ok(())
        );
        assert_eq!(
            check(vec![
                npc("oteryn:npc.a", vec![at(-7, 1, 1)]),
                npc("oteryn:npc.b", vec![at(-7, 1, 1)]),
            ]),
            Err(NpcPlacementError::SharedCell)
        );
        assert_eq!(
            check(vec![npc("oteryn:npc.a", vec![at(1, 1, 1)])]),
            Err(NpcPlacementError::FloorOutsideFrame)
        );
        assert_eq!(
            check(vec![npc("oteryn:npc.a", vec![at(-16, 1, 1)])]),
            Err(NpcPlacementError::FloorOutsideFrame)
        );
    }

    #[test]
    fn the_limits_hold_at_the_maximum_and_refuse_one_more() {
        let row = |n: usize, y: u16| -> Vec<NpcPlacement> {
            (0..n).map(|x| at(-7, y, x as u16)).collect()
        };
        assert_eq!(
            table(vec![npc("oteryn:npc.a", row(MAX_PLACEMENTS_PER_NPC, 0))]).validate(),
            Ok(())
        );
        assert_eq!(
            table(vec![npc(
                "oteryn:npc.a",
                row(MAX_PLACEMENTS_PER_NPC + 1, 0)
            )])
            .validate(),
            Err(NpcPlacementError::PlacementsPerNpcOverLimit)
        );
        let npcs = |total: usize| {
            (0..total.div_ceil(MAX_PLACEMENTS_PER_NPC))
                .map(|i| {
                    let n = (total - i * MAX_PLACEMENTS_PER_NPC).min(MAX_PLACEMENTS_PER_NPC);
                    npc(&format!("oteryn:npc.{i:04}"), row(n, i as u16))
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(table(npcs(MAX_NPC_PLACEMENTS)).validate(), Ok(()));
        assert_eq!(
            table(npcs(MAX_NPC_PLACEMENTS + 1)).validate(),
            Err(NpcPlacementError::PlacementsOverLimit)
        );
        // A full Channel of NPC actors.
        let mut runtime = runtime(MAX_NPC_PLACEMENTS);
        let outcome = runtime
            .place_npcs(&table(npcs(MAX_NPC_PLACEMENTS)), all, true)
            .expect("placed");
        assert_eq!(outcome.actors.len(), MAX_NPC_PLACEMENTS);
        assert_eq!(runtime.visible_entities().npcs.len(), MAX_NPC_PLACEMENTS);
    }
}

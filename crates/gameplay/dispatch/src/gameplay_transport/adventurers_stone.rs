//! Carried Adventurer's Stone admission and its paired Guild return.
use super::*;
use crate::durability::DurabilityError;
use crate::durability::character_last_position::LastVitals;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::quest_state::adventurers_stone::GuildStoneTravel;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, MovementPositionSnapshot};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use connection::UseOwnerError;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::HashSet;
use std::sync::Mutex;

// Crystal 201189f8 data-global/scripts/actions/adventurers_guild/adventurers_stone.lua:5-22,96.
// Tibia Fandom, Adventurer's Stone: use from a city temple. Depot use is disabled.
const ARRIVAL: MovementLocalPosition = MovementLocalPosition {
    x: 32210,
    y: 32300,
    floor: 6,
};
pub(super) const TEMPLES: [(u32, i16, [i32; 4]); 17] = [
    (5, 7, [32727, 31632, 32736, 31639]),
    (6, 7, [32358, 31777, 32364, 31787]),
    (7, 11, [32642, 31921, 32656, 31929]),
    (8, 7, [32365, 32231, 32374, 32243]),
    (9, 7, [32953, 32072, 32963, 32081]),
    (10, 8, [33188, 32844, 33201, 32857]),
    (11, 8, [33208, 31803, 33225, 31819]),
    (12, 11, [33018, 31514, 33032, 31531]),
    (13, 1, [33210, 32450, 33217, 32457]),
    (14, 7, [32313, 32818, 32322, 32830]),
    (15, 7, [32590, 32740, 32600, 32750]),
    (16, 7, [32209, 31130, 32215, 31136]),
    (17, 7, [32785, 31275, 32789, 31279]),
    (18, 9, [33444, 31313, 33452, 31324]),
    (20, 6, [33586, 31895, 33602, 31902]),
    (21, 6, [33510, 32360, 33516, 32365]),
    (22, 5, [33916, 31474, 33926, 31480]),
];
// Pinned donor doNotTeleport, lines 48-58, with only temple admission enabled.
const REFUSAL: &str =
    "Try to move more to the center of a temple to use the spiritual energy for a teleport.";

fn tile(at: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: u16::try_from(at.x).ok()?,
        y: u16::try_from(at.y).ok()?,
        floor: i8::try_from(at.floor.checked_neg()?).ok()?,
    })
}

// Prevent periodic captures of the old position between the durable write and its live install.
#[derive(Default)]
pub(super) struct GuildStoneMoves(Mutex<HashSet<GameSessionId>>);

impl GuildStoneMoves {
    pub(super) fn held(&self, session: GameSessionId) -> bool {
        self.0
            .lock()
            .map_or(true, |pending| pending.contains(&session))
    }

    fn start(&self, session: GameSessionId) -> Option<MoveHold<'_>> {
        self.0.lock().ok()?.insert(session).then(|| MoveHold {
            moves: self,
            session,
        })
    }
}

struct MoveHold<'a> {
    moves: &'a GuildStoneMoves,
    session: GameSessionId,
}

impl Drop for MoveHold<'_> {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.moves.0.lock() {
            pending.remove(&self.session);
        }
    }
}

pub(super) struct StoneMove {
    pub actor: ExactActorRef,
    pub fence: CurrentCharacterItemFence,
    pub command: CommandRef,
    pub expected: MovementPositionSnapshot,
    pub destination: MovementLocalPosition,
    pub travel: GuildStoneTravel,
}

pub(super) enum StoneMoveError {
    Refused,
    HolderBusy,
    OutcomeUnknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn refuse_stone(&self, session: GameSessionId) -> UseDisposition {
        self.states.chat.push_quest_feedback(session, REFUSAL);
        UseDisposition::NotPossible
    }

    // The carried-item caller already holds the spell lane and proved the exact item.
    pub(super) async fn use_adventurers_stone(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        fence: CurrentCharacterItemFence,
        item: [u8; 16],
        budget: &super::AdmissionRetryBudget<'_>,
    ) -> Result<Result<UseDisposition, UseDisposition>, UseOwnerError> {
        let session = command.game_session_id;
        match self
            .refresh_pvp_locks_within(actor, session, None, budget)
            .await
        {
            Ok(()) => {}
            Err(DurabilityError::RootUnavailable) => return Err(UseOwnerError::HolderBusy),
            Err(_) => return Ok(Err(self.refuse_stone(session))),
        }
        let prepared = self.prepare_stone_move(actor, command, fence, item).await;
        let (map, movement) = match prepared {
            Ok(prepared) => prepared,
            Err(disposition) => return Ok(Err(disposition)),
        };
        match self.commit_stone_move(map, movement, budget).await {
            Ok(_) => Ok(Ok(UseDisposition::Committed)),
            Err(StoneMoveError::Refused) => Ok(Err(self.refuse_stone(session))),
            Err(StoneMoveError::HolderBusy) => Err(UseOwnerError::HolderBusy),
            Err(StoneMoveError::OutcomeUnknown) => Err(UseOwnerError::OutcomeUnknown),
        }
    }

    async fn prepare_stone_move(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        fence: CurrentCharacterItemFence,
        item: [u8; 16],
    ) -> Result<(&crate::map::boot::BundleMap, StoneMove), UseDisposition> {
        let session = command.game_session_id;
        let map = self
            .bundle_map()
            .ok_or_else(|| self.refuse_stone(session))?;
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Err(self.refuse_stone(session));
        }
        let fight = self.combat_lock_fact(actor, session).await;
        let (expected, town) = {
            let runtime = self.runtime.lock().await;
            let expected = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?;
            let at = expected.position();
            let source = tile(at).ok_or_else(|| self.refuse_stone(session))?;
            let zones = self.movement_cells().spell_tiles().lookup(
                self.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            );
            let town = TEMPLES
                .iter()
                .find_map(|&(town, floor, [left, top, right, bottom])| {
                    (floor == at.floor
                        && (left..=right).contains(&at.x)
                        && (top..=bottom).contains(&at.y))
                    .then_some(town)
                });
            let states = self.states.spell_states.lock().await;
            if fight.pve_in_fight_until.is_some()
                || super::pvp_live::logout_blocked(&states, actor, session, self.owner_now())
                || super::pvp_live::pz_blocked(&states, actor, session, self.owner_now())
                || map.facts.house_tile(source)
                || !zones.is_ok_and(|zone| zone.flags().protection_zone)
            {
                return Err(self.refuse_stone(session));
            }
            let town = town
                .filter(|town| {
                    map.overlay
                        .base()
                        .subareas
                        .town(*town)
                        .is_some_and(|area| area.temple.is_some())
                })
                .ok_or_else(|| self.refuse_stone(session))?;
            (expected, town)
        };
        let id = CommandId::new(command.command_id).map_err(|_| UseDisposition::Rejected)?;
        Ok((
            map,
            StoneMove {
                actor,
                fence,
                command: CommandRef::new(session, id),
                expected,
                destination: ARRIVAL,
                travel: GuildStoneTravel::Enter { item, town },
            },
        ))
    }

    pub(super) async fn commit_stone_move(
        &self,
        map: &crate::map::boot::BundleMap,
        movement: StoneMove,
        budget: &super::AdmissionRetryBudget<'_>,
    ) -> Result<Option<MovementPositionSnapshot>, StoneMoveError> {
        let StoneMove {
            actor,
            fence,
            command,
            expected,
            destination,
            travel,
        } = movement;
        let session = command.game_session_id();
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let _hold = self
            .states
            .guild_stone_moves
            .start(session)
            .ok_or(StoneMoveError::Refused)?;
        // Travel advances the quest and Character root together. It follows the
        // same spell lane then Character revision order as other quest callbacks.
        let mut revision = self.revision_sequencer.acquire(fence.character_id).await;
        let (proof, placed) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let state = states
                .get(&runtime, actor, session)
                .ok_or(StoneMoveError::Refused)?;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(|_| StoneMoveError::Refused)?;
            if runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || states.is_dead(actor)
            {
                return Err(StoneMoveError::Refused);
            }
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                expected,
                destination,
                super::pvp_live::pz_blocked(&states, actor, session, self.owner_now()),
                &closed,
            )
            .map_err(|_| StoneMoveError::Refused)?;
            let mut placed = self.placed_record(&runtime, destination);
            placed.record.vitals = state
                .saved_vitals()
                .and_then(|(health, mana)| LastVitals::new(health, mana));
            placed.record.rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
            (proof, placed)
        };
        let attempt = async || {
            self.root
                .commit_guild_stone_travel(
                    self.character,
                    self.holder,
                    fence,
                    command,
                    travel,
                    placed,
                )
                .await
        };
        let result = budget
            .retry_use_holder("stone_travel", attempt, super::root_holder_busy)
            .await;
        revision.observe_endgame_progression();
        match result {
            Ok(()) => {}
            Err(DurabilityError::RootUnavailable) => return Err(StoneMoveError::HolderBusy),
            Err(DurabilityError::CommitRejected) => return Err(StoneMoveError::Refused),
            Err(_) => {
                // Other storage failures can follow a successful COMMIT, including a
                // failure to prove the holder's return. Resolve the exact original.
                budget
                    .retry_use_holder("stone_receipt", attempt, super::root_holder_busy)
                    .await
                    .map_err(|_| StoneMoveError::OutcomeUnknown)?;
            }
        }
        let mut runtime = self.runtime.lock().await;
        let arrival = runtime.commit_source_step(proof).ok();
        if arrival.is_some() {
            for at in [expected.position(), destination]
                .into_iter()
                .filter_map(tile)
            {
                let _ = map
                    .facts
                    .emit_movement_effect(at, super::character_home_town::TELEPORT_EFFECT);
            }
        }
        // A live installation failure cannot negate the durable receipt.
        Ok(arrival)
    }
}

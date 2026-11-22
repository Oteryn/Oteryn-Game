//! Native Annihilator stations and the Global forced exit of the admitted run.
use super::*;
use crate::durability::{DurabilityError, reward_claim_mint::RewardClaimMintError};
use crate::foundation::{CharacterLease, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::gameplay_transport::AdmissionRetryBudget;

const ENTRY: &str = "oteryn:quest-callback/annihilator-entry";
const CLAIM: &str = "oteryn:reward-claim.quest.u7_24.the_annihilator.reward";

// Tibia Fandom's The Annihilator Quest/Spoiler specifies ten minutes and the
// reward room. The native map's clear tile two squares east of its arena door
// provides a shared landing. Crystal's daily lever stays spent.
pub(super) const TRANSFER_AFTER_US: u64 = 600_000_000;
const REWARD_ROOM: Position = Position {
    x: 33227,
    y: 31659,
    floor: -13,
};

pub(super) fn owns(callback: &Callback) -> bool {
    callback.key == ENTRY
}

pub(super) fn lever_spent(
    map: &BundleMap,
    callback: &Callback,
    source: &crate::gameplay_transport::world_map::MapObjectTarget<()>,
) -> bool {
    use crate::map::view::{MapEntryKey, compose_tile};
    let Some([_, spent]) = callback.lever else {
        return false;
    };
    compose_tile(&map.overlay, &map.facts, source.pos)
        .ok()
        .flatten()
        .is_some_and(|tile| {
            tile.entries.iter().any(|entry| {
                entry.source.key() == MapEntryKey::Base(source.placement_key)
                    && entry.facts.appearance_id == spent
            })
        })
}

#[derive(PartialEq, Eq)]
struct Station {
    actor: ExactActorRef,
    session: GameSessionId,
    position: MovementPositionSnapshot,
    lease: CharacterLease,
}

pub(super) struct StationAdmission {
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    roster: [Station; 4],
}

impl StationAdmission {
    fn capture(runtime: &ChannelRuntimeV1, callback: &Callback) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let encounter = callback.effects.encounter.as_ref()?;
        let census = runtime.positioned_melee_census().ok()?;
        let mut stations = Vec::new();
        for member in &encounter.players {
            let mut candidates = census
                .iter()
                .filter(|(_, at, _)| at.position() == local(member.from));
            let &(actor, position, Some(session)) = candidates.next()? else {
                return None;
            };
            if candidates.next().is_some()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return None;
            }
            stations.push(Station {
                actor,
                session,
                position,
                lease: runtime.bound_attacker_lease(actor, session).ok()?,
            });
        }
        Some(Self {
            scope: runtime.encounter_owner_scope(),
            roster: stations.try_into().ok()?,
        })
    }

    pub(super) fn still_current(&self, runtime: &ChannelRuntimeV1, callback: &Callback) -> bool {
        Self::capture(runtime, callback)
            .is_some_and(|current| self.scope == current.scope && self.roster == current.roster)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn observe_annihilator_stations(
        &self,
        callback: &Callback,
        initiator: (ExactActorRef, GameSessionId),
    ) -> Result<StationAdmission, UseDisposition> {
        let admission = {
            let runtime = self.runtime.lock().await;
            StationAdmission::capture(&runtime, callback).ok_or(UseDisposition::Rejected)?
        };
        if !admission
            .roster
            .iter()
            .any(|station| (station.actor, station.session) == initiator)
        {
            return Err(UseDisposition::Rejected);
        }
        // The caller holds the spell lane, which also serializes native chest USE.
        // Durable reads run outside the runtime and spell-state guards.
        let budget = AdmissionRetryBudget::for_foreground_command(self.root)
            .map_err(|_| UseDisposition::Rejected)?;
        for station in &admission.roster {
            let fence = budget
                .retry_command_guard(async || {
                    self.current_quest_fence_durable(station.session).await
                })
                .await
                .map_err(|_| UseDisposition::Rejected)?
                .ok_or(UseDisposition::Rejected)?;
            if fence.character_id.as_bytes() != station.lease.character_id().as_bytes()
                || fence.character_lease_generation != station.lease.generation()
                || (fence.runtime_scope, fence.scope_ownership_generation) != admission.scope
            {
                return Err(UseDisposition::Rejected);
            }
            let claimed = budget
                .retry_command_guard(async || {
                    self.root
                        .reward_claim_is_claimed(
                            self.character,
                            self.holder,
                            fence.item_fence(),
                            CLAIM.to_owned(),
                        )
                        .await
                        .map_err(|error| match error {
                            RewardClaimMintError::Unavailable(error) => error,
                            _ => DurabilityError::CommitRejected,
                        })
                })
                .await
                .map_err(|_| UseDisposition::Rejected)?;
            if claimed {
                return Err(UseDisposition::NotPossible);
            }
        }
        Ok(admission)
    }

    pub(super) fn transfer_annihilator_group(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        run: &super::timed_encounter::TimedEncounter,
        now: u64,
    ) -> bool {
        let Some(encounter) = run.encounter() else {
            return true;
        };
        let Ok(census) = runtime.positioned_melee_census() else {
            return false;
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut proofs = Vec::new();
        let mut participants = Vec::new();
        for (actor, expected, session) in census {
            let Some(session) = session else { continue };
            if !run.group_contains(actor, session)
                || !super::encounter::room_contains(encounter, expected.position())
                || states.is_dead(actor)
            {
                continue;
            }
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                runtime,
                &self.movement_cells,
                actor,
                session,
                expected,
                local(REWARD_ROOM),
                &closed,
            ) else {
                return false;
            };
            proofs.push(proof);
            participants.push((actor, session, expected.position()));
        }
        if proofs.is_empty() {
            return true;
        }
        let Ok(snapshots) = runtime.commit_quest_relocation(proofs, || true) else {
            return false;
        };
        for ((actor, session, before), after) in participants.into_iter().zip(snapshots) {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(before),
                after.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                underwater(map, before),
                underwater(map, after.position()),
            );
            map.facts.emit_movement_effect(tile(REWARD_ROOM), 11);
        }
        true
    }
}

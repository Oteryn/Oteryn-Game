//! Actual selected Smelly callback, sharing its existing two-pulse timer owner.
use super::*;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_encounter_dispatch::{
    CREATURE, PARENT, SmellyEffect, SmellyEncounterOwner, SourceSmelly,
};
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
#[derive(Default)]
pub(super) struct NativeSmelly {
    owner: SmellyEncounterOwner,
}
impl std::fmt::Debug for NativeSmelly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeSmelly")
            .field("casters", &self.owner.entries.len())
            .finish()
    }
}
impl NativeSmelly {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.owner.next_due()
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            runtime, states, attack, room, native, map, proposals, now, now,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive_due_through(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            runtime,
            states,
            attack,
            room,
            native,
            map,
            &[],
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn drive_inner(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.owner.prune_retired_native(runtime);
        #[cfg(test)]
        if proposals.iter().any(|p| p.ability.key == PARENT)
            || self.owner.next_due().is_some_and(|due| due <= now)
        {
            eprintln!(
                "SRX3_SMELLY_DRIVER now={now} due={:?} proposals={}",
                self.owner.next_due(),
                proposals.iter().filter(|p| p.ability.key == PARENT).count()
            );
        }
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        for (actor, source, centre, proposal) in
            self.owner.native_due_through(now, eligible_through_us)
        {
            let Some(mut reader) = super::super::physical_spell::prepare_smelly_reader(
                runtime, states, attack, room, native, map, &source, &proposal, centre, now, stamp,
            )
            .await
            else {
                #[cfg(test)]
                eprintln!("SRX3_SMELLY_READER_UNKNOWN actor={actor:?} centre={centre:?}");
                return Err(());
            };
            let players = reader.event_players().to_vec();
            let visual = reader.source_visual_positions();
            #[cfg(test)]
            eprintln!("SRX3_SMELLY_CURRENT_TARGETS actor={actor:?} players={players:?}");
            let effects = self
                .owner
                .drain_native_actor_through(
                    actor,
                    runtime,
                    stamp,
                    states,
                    &mut reader,
                    &Now(now),
                    eligible_through_us,
                )
                .map_err(|_| ())?;
            for (_, effect) in effects {
                match effect.map_err(|_| ())? {
                    SmellyEffect::Damage(hits) => {
                        for (target, hit) in hits {
                            let hit = hit.map_err(|_| ())?;
                            let session = players
                                .iter()
                                .find(|(actor, _)| *actor == target)
                                .map(|(_, session)| *session)
                                .ok_or(())?;
                            super::super::physical_spell::record_native_player_hit(
                                runtime,
                                attack,
                                Some(actor),
                                target,
                                session,
                                &hit,
                                now,
                            );
                        }
                        emit(room, visual.iter().copied(), 17);
                    }
                    SmellyEffect::Presentation(event) => emit(
                        room,
                        event
                            .tiles
                            .into_iter()
                            .map(|(x, y, floor)| MovementLocalPosition { x, y, floor }),
                        21,
                    ),
                }
            }
        }
        for proposal in proposals {
            if proposal.ability.key != PARENT {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(proposal.issuer)
                .map_err(|_| ())?;
            if snapshot.health <= 0 || snapshot.state.policy.definition_key != CREATURE {
                continue;
            }
            let creature = Ref {
                family: crate::content::ProjectV2Family::Creature,
                key: snapshot.state.policy.definition_key.clone(),
                revision: snapshot.state.policy.definition_revision.clone(),
            };
            let source = SourceSmelly::qualify_native(native, &creature)
                .map_err(|_| ())?
                .ok_or(())?;
            self.owner
                .schedule(
                    runtime,
                    runtime.owner_fence().map_err(|_| ())?,
                    stamp,
                    &source,
                    proposal,
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|error| {
                    #[cfg(test)]
                    eprintln!(
                        "SRX3_SMELLY_SCHEDULE_REFUSED actor={:?} range={} error={error:?}",
                        proposal.issuer, proposal.range_tiles
                    );
                    #[cfg(not(test))]
                    let _ = error;
                })?;
        }
        Ok(())
    }
}
fn emit(
    room: &QualifiedNativeEntryRoom,
    positions: impl IntoIterator<Item = MovementLocalPosition>,
    id: u16,
) {
    use crate::map::boot::BundleCollisionIndexExt;
    if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
    {
        for at in positions.into_iter().filter_map(super::visual_position) {
            index.map().facts.emit_movement_effect(at, id);
        }
    }
}

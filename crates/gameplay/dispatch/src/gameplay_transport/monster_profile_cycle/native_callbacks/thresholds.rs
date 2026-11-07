//! The three pinned defense callbacks use the existing threshold/marker owner.
use super::*;
use crate::foundation::ExactActorRef;
use crate::source_threshold_heal::{ThresholdHealOwner, ThresholdHealSource};
#[derive(Default)]
pub(super) struct NativeThresholds {
    owner: ThresholdHealOwner,
    sources: Vec<(ExactActorRef, ThresholdHealSource)>,
    emitted: Vec<(ExactActorRef, String)>,
}
impl std::fmt::Debug for NativeThresholds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeThresholds")
            .field("sources", &self.sources.len())
            .field("markers", &self.owner.markers.len())
            .finish()
    }
}
fn is_callback(p: &ProfileAbilityProposal) -> bool {
    matches!(
        p.ability.key.as_str(),
        "oteryn:ability.creature.lisa.field-fill-lisa_heal"
            | "oteryn:ability.creature.tyrn.field-fill-tyrn_heal"
            | "oteryn:ability.spell.glooth_fairy_healing"
    )
}
fn present(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    key: &str,
    complete: bool,
) {
    if let Ok(Some(line))=oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::CallbackSpeech::threshold_heal(runtime,actor,key,complete){spawns::emit_speech(room,vec![line]);}
    if complete && key != "oteryn:creature.tyrn" {
        use crate::map::boot::BundleCollisionIndexExt;
        if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
            && let Some(at) = runtime
                .read_actor_position(actor)
                .ok()
                .and_then(|p| super::visual_position(p.position()))
        {
            index.map().facts.emit_movement_effect(at, 13);
        }
    }
}
impl NativeThresholds {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.owner
            .markers
            .iter()
            .filter_map(|m| m.pending.map(|p| p.0))
            .min()
    }
    pub(super) fn drive(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(runtime, room, native, proposals, now, now)
    }
    pub(super) fn drive_due_through(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(runtime, room, native, &[], eligible_through_us, now)
    }
    fn drive_inner(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        proposals: &[ProfileAbilityProposal],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.sources
            .retain(|(actor, _)| runtime.contains_live_creature(*actor));
        self.emitted
            .retain(|(actor, _)| runtime.contains_live_creature(*actor));
        self.owner
            .markers
            .retain(|m| runtime.contains_live_creature(m.actor));
        for (actor, source) in &self.sources {
            if eligible_through_us < now
                && !self.owner.markers.iter().any(|marker| {
                    marker.actor == *actor
                        && marker
                            .pending
                            .is_some_and(|pending| pending.0 <= eligible_through_us)
                })
            {
                continue;
            }
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            if let Some(outcome) = self
                .owner
                .tick_current(runtime, stamp, source, *actor, now)
                .map_err(|_| ())?
                && outcome.healed.is_some()
            {
                let snapshot = runtime.companion_snapshot(*actor).map_err(|_| ())?;
                present(
                    runtime,
                    room,
                    *actor,
                    &snapshot.state.policy.definition_key,
                    true,
                );
            }
        }
        for proposal in proposals.iter().filter(|p| is_callback(p)) {
            let snapshot = runtime
                .companion_snapshot(proposal.issuer)
                .map_err(|_| ())?;
            let Some(record) = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            if !self.sources.iter().any(|(actor, source)| {
                *actor == proposal.issuer && source.ability == proposal.ability
            }) {
                if self.sources.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                    return Err(());
                }
                let source = ThresholdHealSource::qualify_native(
                    native,
                    &record.profile.target,
                    proposal.entry_index,
                    *runtime.binding().world_id().as_bytes(),
                )
                .map_err(|_| ())?;
                self.sources.try_reserve(1).map_err(|_| ())?;
                self.sources.push((proposal.issuer, source));
            }
            let source = &self
                .sources
                .iter()
                .find(|(actor, source)| {
                    *actor == proposal.issuer && source.ability == proposal.ability
                })
                .ok_or(())?
                .1;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            self.emitted.try_reserve(1).map_err(|_| ())?;
            let emission = proposal.occurrence.id().as_str().to_owned();
            let outcome = self
                .owner
                .cast_current(runtime, stamp, source, proposal, now)
                .map_err(|_| ())?;
            if self.emitted.iter().any(|(actor, occurrence)| {
                *actor == proposal.issuer && occurrence == proposal.occurrence.id().as_str()
            }) {
                continue;
            }
            self.emitted.retain(|(actor, _)| *actor != proposal.issuer);
            self.emitted.push((proposal.issuer, emission));
            if outcome.started {
                present(
                    runtime,
                    room,
                    proposal.issuer,
                    &snapshot.state.policy.definition_key,
                    false,
                );
            }
            if outcome.healed.is_some() {
                present(
                    runtime,
                    room,
                    proposal.issuer,
                    &snapshot.state.policy.definition_key,
                    true,
                );
            }
        }
        Ok(())
    }
}

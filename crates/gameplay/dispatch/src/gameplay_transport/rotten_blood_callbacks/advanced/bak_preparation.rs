//! Retain the actual FIRST think across the existing durable owner boundary.
//! Entry, movement and later think occurrences cannot replace its causal key.
use super::*;
use crate::durability::quest_state::rotten_flame::BakFirstThinkClaim;
use crate::durability::quest_state::rotten_flame::{
    PreparedBakEssences, phase::BakPhasePublication,
};

#[derive(Debug)]
pub(super) struct Pending {
    claim: Arc<BakFirstThinkClaim>,
    pub(super) occurrence: ThinkOccurrence,
    pub(super) started_us: u64,
    pub(super) resolved: bool,
    publication: Option<Arc<BakPhasePublication>>,
}
pub(in crate::gameplay_transport) fn before_drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<Option<Arc<BakFirstThinkClaim>>, ()> {
    if room.pinned_activity_source_room()
        != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore)
    {
        return Ok(None);
    }
    if let Some(pending) = &states.rotten_blood_callbacks.advanced.first_think {
        if pending.resolved {
            return Ok(None);
        }
        return pending
            .claim
            .current(runtime)
            .then(|| Some(Arc::clone(&pending.claim)))
            .ok_or(());
    }
    if states
        .rotten_blood_callbacks
        .prepared_bak
        .as_ref()
        .and_then(|proof| proof.current_mask(runtime))
        .is_some()
    {
        return Ok(None);
    }
    let Some(occurrence) = occurrences.iter().find(|occurrence| {
        runtime.matches_live_creature_identity(occurrence.actor, bak::BAKRAGORE_KEY.as_bytes())
    }) else {
        return Ok(None);
    };
    await_claim(runtime, states, native, *occurrence, now)?;
    Ok(states
        .rotten_blood_callbacks
        .advanced
        .first_think
        .as_ref()
        .map(|pending| Arc::clone(&pending.claim)))
}

pub(super) fn await_claim(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    occurrence: ThinkOccurrence,
    now: u64,
) -> Result<bool, ()> {
    let prepared = states
        .rotten_blood_callbacks
        .prepared_bak
        .as_ref()
        .and_then(|proof| proof.current_mask(runtime))
        .is_some();
    let advanced = &mut states.rotten_blood_callbacks.advanced;
    if let Some(pending) = &advanced.first_think {
        return Ok(!pending.resolved || !prepared);
    }
    if prepared {
        return Ok(false);
    }
    let claim = BakFirstThinkClaim::from_current_owner(runtime, native, occurrence).ok_or(())?;
    advanced.first_think = Some(Pending {
        claim: Arc::new(claim),
        occurrence,
        started_us: now,
        resolved: false,
        publication: None,
    });
    states.owner_wake.notify_one();
    Ok(true)
}

impl super::super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_bak_first_think_claims(&self) {
        let claim = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let Some(pending) = states
                .rotten_blood_callbacks
                .advanced
                .first_think
                .as_ref()
                .filter(|p| !p.resolved && p.claim.current(&runtime))
            else {
                return;
            };
            Arc::clone(&pending.claim)
        };
        let Ok(proof) = self.root.commit_bak_first_think(self.holder, &claim).await else {
            return;
        };
        if !proof.publication_pending() {
            let Some(_permit) = self.spell_lane_permit().await else {
                return;
            };
            self.acknowledge_bak_first_think(&claim, proof).await;
            return;
        }
        let Some(origin) = proof.origin() else {
            return;
        };
        let Some(registered) = self
            .activity_runtimes
            .get(origin.runtime_scope(), origin.generation())
        else {
            return;
        };
        let source = self.runtime_view(&registered);
        // Source Channel tag1 precedes destination Instance tag2, for lanes
        // and physical owners alike. No fabricated source Character authority.
        let Some(_source_permit) = source.spell_lane_permit().await else {
            return;
        };
        let Some(_instance_permit) = self.spell_lane_permit().await else {
            return;
        };
        let publication = {
            let source_runtime = source.runtime.lock().await;
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if !origin.current(&source_runtime) || !claim.current(&runtime) {
                return;
            }
            let Some(pending) = states
                .rotten_blood_callbacks
                .advanced
                .first_think
                .as_mut()
                .filter(|pending| !pending.resolved && Arc::ptr_eq(&pending.claim, &claim))
            else {
                return;
            };
            if let Some(publication) = &pending.publication {
                Arc::clone(publication)
            } else {
                let Some(map) = source.bundle_map() else {
                    return;
                };
                let Some(publication) =
                    crate::gameplay_transport::quest_callbacks::prepare_bak_public_reset(
                        &source_runtime,
                        map,
                        proof,
                    )
                else {
                    return;
                };
                let publication = Arc::new(publication);
                // The exact historical reset result survives Unknown; never
                // resnapshot an already-reset tile as a different nil result.
                pending.publication = Some(Arc::clone(&publication));
                publication
            }
        };
        let Ok(published) = self
            .root
            .commit_bak_phase_publication(self.holder, &publication)
            .await
        else {
            return;
        };
        self.acknowledge_bak_first_think(&claim, published).await;
    }
    async fn acknowledge_bak_first_think(
        &self,
        claim: &Arc<BakFirstThinkClaim>,
        proof: PreparedBakEssences,
    ) {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !claim.current(&runtime) || proof.current_mask(&runtime).is_none() {
            return;
        }
        let Some(pending) = states
            .rotten_blood_callbacks
            .advanced
            .first_think
            .as_mut()
            .filter(|p| Arc::ptr_eq(&p.claim, claim))
        else {
            return;
        };
        pending.resolved = true;
        states.rotten_blood_callbacks.prepared_bak = Some(proof);
        states.owner_wake.notify_one();
    }
}

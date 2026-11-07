//! Credited World-change kills share the native kill queue; no independent drain.
use super::{
    ComposedFreshAdmission,
    kill_reward::{KillSettlementQueue, PendingKillSettlement},
};
use crate::durability::{
    character_progression::CharacterProgressionError,
    quest_state::{QuestTransitionOutcome, world_change_kills::NativeWorldChangeKill},
};
use crate::foundation::{ChannelRuntimeV1, GameSessionId, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::world_change_kills as rules;

/// Installed at fenced session admission, then copied while the lethal runtime
/// and attack guards are held. A new scope owner/content pin cannot reuse it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CaptureBinding {
    scope: RuntimeScopeRefV1,
    generation: u64,
    map: [u8; 32],
    content: [u8; 32],
    frame: [u8; 32],
    epoch: u64,
}
impl CaptureBinding {
    fn new(runtime: &ChannelRuntimeV1, epoch: u64) -> Self {
        let pin = runtime.content_pin();
        Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            map: pin.map_revision_digest(),
            content: pin.server_artifact_digest(),
            frame: pin.frame_binding_digest(),
            epoch,
        }
    }
}

pub(super) fn tracks_creature(creature: &str) -> bool {
    rules::for_creature(creature).is_some()
}

pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    queue: &KillSettlementQueue,
    entry: &mut PendingKillSettlement,
) {
    if !tracks_creature(&entry.creature) || !runtime.retains_corpse_projection(entry.target) {
        return;
    }
    let Ok(installed) = queue.2.lock() else {
        return;
    };
    let Some(installed) = installed.as_ref() else {
        return;
    };
    if *installed != CaptureBinding::new(runtime, installed.epoch) {
        return;
    }
    let principal = entry.facts.principal;
    if !runtime
        .bound_attacker_lease(principal.actor, principal.session)
        .is_ok_and(|lease| {
            lease.character_id() == principal.character
                && lease.generation() == principal.lease_generation
        })
    {
        return;
    }
    let Ok(proof) = runtime.native_credited_kill_proof(entry.target) else {
        return;
    };
    let winner = proof.principal();
    if proof.death() != entry.facts.death
        || proof.creature() != entry.creature
        || proof.position() != entry.facts.corpse
        || proof.occurrence() != entry.facts.occurrence
        || winner.character != principal.character
        || winner.session != principal.session
        || winner.lease_generation != principal.lease_generation
        || winner.actor != principal.actor
    {
        return;
    }
    entry.world_change_kill = NativeWorldChangeKill::from_native_proof(proof, installed.epoch).ok();
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_world_change_kill_binding(&self, session: GameSessionId) {
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || !runtime.positioned_actor_census().is_ok_and(|actors| {
                actors
                    .iter()
                    .any(|(_, _, observed)| *observed == Some(session))
            })
        {
            return;
        }
        let binding = CaptureBinding::new(&runtime, self.reset_epoch);
        let attack = self.states.attack.lock().await;
        if let Ok(mut installed) = attack.kills().2.lock() {
            *installed = Some(binding);
        }
    }

    async fn publish_world_change_death_notice(&self, entry: &PendingKillSettlement) {
        let Some(text) = rules::for_creature(&entry.creature).and_then(|rule| rule.notice) else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let receipts = {
            let attack = self.states.attack.lock().await;
            attack.kills().analyser_receipts(&runtime, entry.target)
        };
        let Ok(mut receipt) = receipts.lock() else {
            return;
        };
        if receipt.world_change_notice {
            return;
        }
        if self.states.chat.publish_native_death_notice(
            &mut runtime,
            entry.target,
            &entry.creature,
            text,
        ) {
            receipt.world_change_notice = true;
        }
    }

    /// Exact principal only: native shared XP membership never fabricates a kill
    /// recipient. The current Character/session/generation is rechecked twice.
    pub(super) async fn credit_world_change_death(&self, entry: &PendingKillSettlement) -> bool {
        let Some(context) = &entry.world_change_kill else {
            return true;
        };
        let Some(catalogue) = &self.quest_catalogue else {
            return false;
        };
        let fence = match self.current_quest_fence(context.session()).await {
            Ok(Some(fence)) => fence,
            Ok(None) => return true,
            Err(()) => return false,
        };
        if fence.character_id != context.character()
            || fence.game_session_id != context.session()
            || fence.character_lease_generation != context.lease_generation()
        {
            return true;
        }
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != context.death().runtime_scope()
                || runtime.binding().scope_generation()
                    != context.death().scope_ownership_generation()
                || !runtime
                    .bound_attacker_lease(entry.facts.principal.actor, context.session())
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == context.character().as_bytes()
                            && lease.generation() == context.lease_generation()
                    })
            {
                return true;
            }
        }
        self.publish_world_change_death_notice(entry).await;
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(context.character()).await;
        let outcome = slot
            .commit_world_change_kill(
                self.root,
                self.character,
                self.holder,
                fence,
                std::sync::Arc::clone(catalogue),
                &permit,
                context.clone(),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                self.refresh_quest_session(context.session(), false).await;
                true
            }
            Ok(_) => true,
            Err(
                CharacterProgressionError::Unavailable(_)
                | CharacterProgressionError::CharacterRevisionMismatch,
            ) => {
                self.schedule_quest_refresh(context.session());
                false
            }
            Err(error) => {
                super::operator_event(&format!("world_change_kill_refused reason={error:?}"));
                true
            }
        }
    }
}

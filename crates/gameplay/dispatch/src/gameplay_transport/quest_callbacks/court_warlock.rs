//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::{
    DurabilityError,
    boss_encounter::{SourceBossAdmission, SourceBossLever, SourceBossReservation},
};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

pub(super) struct SourceBossUse<'a> {
    pub(super) map: &'a BundleMap,
    pub(super) callback: &'a Callback,
    pub(super) key: u64,
    pub(super) revision: u64,
    pub(super) after: crate::map::view::EntryFacts,
    pub(super) cause: QuestCause,
    pub(super) lever: SourceBossLever,
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static PRE_SOURCE_BOSS_CHARGE:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn execute_source_boss_lever(
        &self,
        mut runtime_guard: tokio::sync::MutexGuard<'_, ChannelRuntimeV1>,
        mut spell_guard: tokio::sync::MutexGuard<'_, ChannelSpellStates>,
        actor: ExactActorRef,
        session: GameSessionId,
        source_use: SourceBossUse<'_>,
    ) -> UseDisposition {
        let SourceBossUse {
            map,
            callback,
            key,
            revision,
            after,
            cause,
            lever,
        } = source_use;
        let QuestCause::Use(command) = cause else {
            return UseDisposition::Rejected;
        };
        let runtime = &mut *runtime_guard;
        let states = &mut *spell_guard;
        let ready = self.execute_quest_encounter(
            runtime, states, actor, session, map, callback, key, revision, after, true, cause,
            None, None,
        );
        if ready != UseDisposition::Committed {
            return ready;
        }
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        drop(spell_guard);
        drop(runtime_guard);
        let mut party = Vec::new();
        for &(member, at, member_session) in &census {
            let Some(member_session) = member_session else {
                continue;
            };
            if !encounter
                .players
                .iter()
                .any(|p| local(p.from) == at.position())
            {
                continue;
            }
            let Ok(Some(fence)) = self.current_quest_fence(member_session).await else {
                return UseDisposition::Rejected;
            };
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(member) != Ok(at)
                || !runtime
                    .bound_attacker_lease(member, member_session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return UseDisposition::Rejected;
            }
            party.push(fence.item_fence());
        }
        // Key and revision describe the wire target. Current party fences
        // authorize the write but are never part of the command identity.
        let mut identity = match lever {
            SourceBossLever::CourtWarlock => b"cwarlock:lever:v2".to_vec(),
            SourceBossLever::Arbaziloth => b"arbaziloth:lever:v2".to_vec(),
        };
        identity.extend_from_slice(&key.to_be_bytes());
        identity.extend_from_slice(&revision.to_be_bytes());
        let admission = SourceBossReservation {
            id: super::super::npc_trade_command::derived(
                &identity,
                command.game_session_id(),
                command.command_id().get(),
            ),
            lever,
            party,
        };
        let Some(lever_projection) = map.facts.reserve_source_boss_lever(key, revision, after)
        else {
            return UseDisposition::Rejected;
        };
        let (mut runtime, mut states) = self.quest_turn_with(&self.states.spell_states).await;
        if runtime.positioned_melee_census().ok().as_ref() != Some(&census) {
            return UseDisposition::StaleState;
        }
        let staged =
            match self.stage_source_boss(&mut runtime, &mut states, actor, session, &source_use) {
                Ok(staged) => staged.detach(),
                Err(disposition) => return disposition,
            };
        drop(states);
        drop(runtime);
        #[cfg(test)]
        if let Ok(barrier) = PRE_SOURCE_BOSS_CHARGE.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        let retry = super::super::AdmissionRetryBudget::for_command(self.root);
        let paid = retry
            .retry_durability("source_boss_admission", || {
                self.source_boss_admission(&admission, SourceBossAdmission::Commit)
            })
            .await;
        #[cfg(test)]
        let paid = if matches!(paid, Ok(true))
            && self
                .states
                .quest_callbacks
                .lose_source_boss_commit_ack
                .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            Err(DurabilityError::CommitOutcomeUnknown)
        } else {
            paid
        };
        let paid = if matches!(
            paid,
            Err(DurabilityError::CommitOutcomeUnknown | DurabilityError::RootPassDeadlineExceeded)
        ) {
            retry
                .retry_durability("source_boss_admission_reconcile", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Reconcile)
                })
                .await
        } else {
            paid
        };
        match paid {
            Ok(true) => {
                #[cfg(test)]
                if self
                    .states
                    .quest_callbacks
                    .interrupt_source_boss_before_projection
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    return UseDisposition::Rejected;
                }
                let (mut runtime, mut states) =
                    self.quest_turn_with(&self.states.spell_states).await;
                let Some(staged) = staged.resume(&mut runtime) else {
                    return UseDisposition::Rejected;
                };
                self.publish_source_boss(staged, &mut states, map, lever_projection);
                UseDisposition::Committed
            }
            Ok(false) => UseDisposition::Rejected,
            Err(error) => {
                super::super::operator_event(&format!(
                    "source_boss_admission_deferred reason={error:?}"
                ));
                UseDisposition::Rejected
            }
        }
    }

    async fn source_boss_admission(
        &self,
        admission: &SourceBossReservation,
        operation: SourceBossAdmission,
    ) -> Result<bool, DurabilityError> {
        self.root
            .source_boss_lever_admission(
                self.character,
                self.holder,
                admission.clone(),
                self.reset_epoch,
                operation,
            )
            .await
    }
}

pub(super) fn source_boss_lever(callback: &Callback) -> Option<SourceBossLever> {
    match callback.key.as_str() {
        source::KEY => Some(SourceBossLever::CourtWarlock),
        "oteryn:quest-callback/nrwicked/arbaziloth" => Some(SourceBossLever::Arbaziloth),
        _ => None,
    }
}

//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::{
    DurabilityError,
    boss_encounter::{SourceBossAdmission, SourceBossLever, SourceBossReservation},
};
use oteryn_gameplay_entities::carrier::encounter_access::PublishedQuestEncounterProof;
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

#[derive(Clone)]
pub(super) struct SourceBossSeal {
    admission: SourceBossReservation,
    proof: PublishedQuestEncounterProof,
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static PRE_SOURCE_BOSS_CONFIRM:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn execute_source_boss_lever(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
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
        let Ok(mut publications) = self.states.quest_callbacks.source_boss_turn.try_lock() else {
            return UseDisposition::Exhausted;
        };
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
            if !runtime
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
        let reserved = self
            .source_boss_admission(&admission, SourceBossAdmission::Reserve)
            .await;
        #[cfg(test)]
        let reserved = if matches!(reserved, Ok(true))
            && self
                .states
                .quest_callbacks
                .lose_source_boss_commit_ack
                .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            Err(DurabilityError::CommitOutcomeUnknown)
        } else {
            reserved
        };
        let uncertain = matches!(
            reserved,
            Err(DurabilityError::CommitOutcomeUnknown | DurabilityError::RootPassDeadlineExceeded)
        );
        let reserved = if uncertain {
            super::super::AdmissionRetryBudget::for_command(self.root)
                .retry_durability("source_boss_admission_reconcile", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Reconcile)
                })
                .await
        } else {
            reserved
        };
        match reserved {
            Ok(true) => {}
            Ok(false) => {
                if uncertain {
                    self.try_refund_source_boss_lever(&admission).await;
                }
                return UseDisposition::Rejected;
            }
            Err(error) => {
                super::super::operator_event(&format!(
                    "source_boss_admission_deferred reason={error:?}"
                ));
                return UseDisposition::Rejected;
            }
        }
        let Some(lever_projection) = map.facts.reserve_source_boss_lever(key, revision, after)
        else {
            self.try_refund_source_boss_lever(&admission).await;
            return UseDisposition::Rejected;
        };
        let staged = match self.stage_source_boss(
            runtime, states, actor, session, map, callback, key, revision, after, cause,
        ) {
            Ok(staged) => staged,
            Err(disposition) => {
                self.try_refund_source_boss_lever(&admission).await;
                return disposition;
            }
        };
        // Authorize the staged publication. Until installation is confirmed,
        // the same durable admission remains eligible for recovery.
        let retry = super::super::AdmissionRetryBudget::for_command(self.root);
        let publish = || self.source_boss_admission(&admission, SourceBossAdmission::Publish);
        let publication = retry.retry_durability("source_boss_publish", publish).await;
        #[cfg(test)]
        let publication = if matches!(publication, Ok(true))
            && self
                .states
                .quest_callbacks
                .lose_source_boss_publish_ack
                .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            Err(DurabilityError::CommitOutcomeUnknown)
        } else {
            publication
        };
        let publication = if matches!(
            publication,
            Err(DurabilityError::CommitOutcomeUnknown | DurabilityError::RootPassDeadlineExceeded)
        ) {
            retry
                .retry_durability("source_boss_publish_reconcile", publish)
                .await
        } else {
            publication
        };
        match publication {
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
                let installed = staged.install(admission.id);
                #[cfg(test)]
                if let Ok(barrier) = PRE_SOURCE_BOSS_CONFIRM.try_with(std::sync::Arc::clone) {
                    barrier.0.notify_one();
                    barrier.1.notified().await;
                }
                let proof = installed.proof();
                let confirm = || {
                    self.source_boss_admission(
                        &admission,
                        SourceBossAdmission::ConfirmProjection(proof),
                    )
                };
                let confirmed = retry
                    .retry_durability("source_boss_projected", confirm)
                    .await;
                let confirmed = if matches!(
                    confirmed,
                    Err(DurabilityError::CommitOutcomeUnknown
                        | DurabilityError::RootPassDeadlineExceeded)
                ) {
                    retry
                        .retry_durability("source_boss_projected_reconcile", confirm)
                        .await
                } else {
                    confirmed
                };
                match confirmed {
                    Ok(true) => {
                        #[cfg(test)]
                        if self
                            .states
                            .quest_callbacks
                            .interrupt_source_boss_after_confirmation
                            .swap(false, std::sync::atomic::Ordering::AcqRel)
                        {
                            return UseDisposition::Rejected;
                        }
                        let proof = self.publish_installed_source_boss(
                            installed,
                            states,
                            map,
                            lever_projection,
                        );
                        // Keep proof before any await, even if this command is cancelled
                        // or the seal commits without an acknowledgement.
                        let pending = SourceBossSeal { admission, proof };
                        publications.push_back(pending.clone());
                        #[cfg(test)]
                        if self
                            .states
                            .quest_callbacks
                            .defer_source_boss_seal
                            .swap(false, std::sync::atomic::Ordering::AcqRel)
                        {
                            return UseDisposition::Committed;
                        }
                        let seal = || self.seal_source_boss(&pending);
                        let sealed = retry.retry_durability("source_boss_seal", seal).await;
                        if matches!(sealed, Ok(true)) {
                            publications.pop_back();
                        }
                        UseDisposition::Committed
                    }
                    // A refused or unacknowledged installation remains pending
                    // for the recovery cycle. Dropping it rolls back the carrier.
                    Ok(false) => UseDisposition::Rejected,
                    Err(error) => {
                        super::super::operator_event(&format!(
                            "source_boss_projection_deferred reason={error:?}"
                        ));
                        UseDisposition::Rejected
                    }
                }
            }
            Ok(false) => {
                self.try_refund_source_boss_lever(&admission).await;
                UseDisposition::Rejected
            }
            Err(error) => {
                super::super::operator_event(&format!(
                    "source_boss_publish_deferred reason={error:?}"
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

    async fn try_refund_source_boss_lever(&self, admission: &SourceBossReservation) {
        if let Err(error) = self
            .source_boss_admission(admission, SourceBossAdmission::Refund)
            .await
        {
            super::super::operator_event(&format!("source_boss_refund_deferred reason={error:?}"));
        }
    }

    async fn seal_source_boss(&self, pending: &SourceBossSeal) -> Result<bool, DurabilityError> {
        self.source_boss_admission(
            &pending.admission,
            SourceBossAdmission::SealProjection(pending.proof),
        )
        .await
    }

    pub(in crate::gameplay_transport) async fn refund_unpublished_source_boss_lever(&self) {
        if let Err(error) = self.recover_source_boss_admission().await {
            super::super::operator_event(&format!("source_boss_refund_deferred reason={error:?}"));
        }
    }

    pub(in crate::gameplay_transport) async fn recover_source_boss_admission(
        &self,
    ) -> Result<bool, DurabilityError> {
        use crate::foundation::RuntimeScopeRefV1;

        let scope = RuntimeScopeRefV1::channel(self.world_id, self.channel_id);
        let published: Vec<_> = {
            let Ok(publications) = self.states.quest_callbacks.source_boss_turn.try_lock() else {
                return Ok(false);
            };
            publications
                .iter()
                .map(|pending| pending.admission.id)
                .collect()
        };
        let has_unsealed_publication = !published.is_empty();
        let candidate = self.root.source_boss_recovery(scope, published).await?;
        if candidate.is_none() && !has_unsealed_publication {
            return Ok(false);
        }
        // The source command excludes recovery for its whole turn. This gate
        // does not block the Channel runtime while SQL revalidates the refund.
        let Ok(mut publications) = self.states.quest_callbacks.source_boss_turn.try_lock() else {
            return Ok(false);
        };
        let generation = {
            let Ok(runtime) = self.runtime.try_lock() else {
                return Ok(false);
            };
            let Ok(fence) = runtime.owner_fence() else {
                return Ok(false);
            };
            if runtime.binding().runtime_scope() != scope {
                return Ok(false);
            }
            fence.generation()
        };
        if let Some(candidate) = candidate
            && !publications
                .iter()
                .any(|pending| pending.admission.id == candidate.admission())
        {
            return self
                .root
                .refund_source_boss_recovery(self.character, self.holder, generation, candidate)
                .await;
        }
        if let Some(pending) = publications.front().cloned() {
            publications.rotate_left(1);
            if self.seal_source_boss(&pending).await? {
                publications.pop_back();
            }
        }
        Ok(false)
    }
}

pub(super) fn source_boss_lever(callback: &Callback) -> Option<SourceBossLever> {
    match callback.key.as_str() {
        source::KEY => Some(SourceBossLever::CourtWarlock),
        "oteryn:quest-callback/nrwicked/arbaziloth" => Some(SourceBossLever::Arbaziloth),
        _ => None,
    }
}

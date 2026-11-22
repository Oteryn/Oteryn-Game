//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::{
    DurabilityError,
    boss_encounter::{SourceBossAdmission, SourceBossLever},
    item_transfer::CurrentCharacterItemFence,
};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

struct SourceBossReservation {
    id: [u8; 16],
    lever: SourceBossLever,
    party: Vec<CurrentCharacterItemFence>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the source hook shares the existing lever owner turn"
    )]
    pub(super) async fn execute_source_boss_lever(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: crate::map::view::EntryFacts,
        cause: QuestCause,
        lever: SourceBossLever,
    ) -> UseDisposition {
        let QuestCause::Use(command) = cause else {
            return UseDisposition::Rejected;
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
        // Nothing from the staged carrier or lever is observable before this
        // durable linearization point. Reissuing Publish reads the same receipt.
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
                self.publish_staged_source_boss(staged, states, map, lever_projection);
                UseDisposition::Committed
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
                admission.lever,
                admission.party.clone(),
                admission.id,
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

    pub(in crate::gameplay_transport) async fn refund_unpublished_source_boss_lever(&self) {
        // A current USE holds this same native guard from reserve through publish.
        // Keep it across recovery so an unpublished row never races a live turn.
        let Ok(runtime) = self.runtime.try_lock() else {
            return;
        };
        let Ok(fence) = runtime.owner_fence() else {
            return;
        };
        if let Err(error) = self
            .root
            .refund_abandoned_source_boss_lever(
                self.character,
                self.holder,
                runtime.binding().runtime_scope(),
                fence.generation(),
            )
            .await
        {
            super::super::operator_event(&format!("source_boss_refund_deferred reason={error:?}"));
        }
    }
}

pub(super) fn source_boss_lever(callback: &Callback) -> Option<SourceBossLever> {
    match callback.key.as_str() {
        source::KEY => Some(SourceBossLever::CourtWarlock),
        "oteryn:quest-callback/nrwicked/arbaziloth" => Some(SourceBossLever::Arbaziloth),
        _ => None,
    }
}

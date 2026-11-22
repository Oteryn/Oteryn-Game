//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::{
    DurabilityError,
    boss_encounter::{SourceBossAdmission, SourceBossLever},
    item_transfer::CurrentCharacterItemFence,
};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

#[derive(Clone)]
pub(super) struct UnpublishedAdmission {
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
        let admission = UnpublishedAdmission {
            id: super::super::npc_trade_command::derived(
                &identity,
                command.game_session_id(),
                command.command_id().get(),
            ),
            lever,
            party,
        };
        {
            let Ok(mut refunds) = self.states.quest_callbacks.source_boss_refunds.lock() else {
                return UseDisposition::Rejected;
            };
            // Register compensation before writing. Even cancellation of the
            // outer USE leaves the exact unpublished transaction recoverable.
            if refunds.len() >= 64 || refunds.iter().any(|pending| pending.id == admission.id) {
                return UseDisposition::Rejected;
            }
            refunds.push(admission.clone());
        }
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
                } else {
                    self.forget_unpublished_source_boss_lever(admission.id);
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
        let disposition = self.execute_quest_encounter(
            runtime, states, actor, session, map, callback, key, revision, after, false, cause,
            None, None,
        );
        if disposition == UseDisposition::Committed {
            self.forget_unpublished_source_boss_lever(admission.id);
        } else {
            self.try_refund_source_boss_lever(&admission).await;
        }
        disposition
    }

    async fn source_boss_admission(
        &self,
        admission: &UnpublishedAdmission,
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

    fn forget_unpublished_source_boss_lever(&self, id: [u8; 16]) {
        if let Ok(mut refunds) = self.states.quest_callbacks.source_boss_refunds.lock() {
            refunds.retain(|pending| pending.id != id);
        }
    }

    async fn try_refund_source_boss_lever(&self, admission: &UnpublishedAdmission) {
        match self
            .source_boss_admission(admission, SourceBossAdmission::Refund)
            .await
        {
            Ok(true) => self.forget_unpublished_source_boss_lever(admission.id),
            Ok(false) => {}
            Err(error) => super::super::operator_event(&format!(
                "source_boss_refund_deferred reason={error:?}"
            )),
        }
    }

    pub(in crate::gameplay_transport) async fn refund_unpublished_source_boss_lever(&self) {
        let pending = {
            // The USE owns runtime through reservation, COMMIT and publication.
            // Never refund while that turn can still publish the encounter.
            let Ok(runtime) = self.runtime.try_lock() else {
                return;
            };
            if runtime.owner_fence().is_err() {
                return;
            }
            let Ok(mut refunds) = self.states.quest_callbacks.source_boss_refunds.lock() else {
                return;
            };
            if !refunds.is_empty() {
                refunds.rotate_left(1);
            }
            refunds.last().cloned()
        };
        if let Some(pending) = pending {
            self.try_refund_source_boss_lever(&pending).await;
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

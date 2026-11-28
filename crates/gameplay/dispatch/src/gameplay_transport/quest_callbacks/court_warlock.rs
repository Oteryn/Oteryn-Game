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

/// The SQL receipt remains the authority for payment. Keep the command identity
/// for a fresh USE by the same party if publication was interrupted.
pub(super) type PendingBossAdmissions = std::collections::HashMap<
    (u64, Vec<[u8; 16]>),
    (
        oteryn_gameplay_entities::ScopeRuntimeAssignmentBinding,
        std::time::Instant,
        [u8; 16],
    ),
>;

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static PRE_SOURCE_BOSS_CHARGE:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn execute_source_boss_lever(
        &self,
        mut runtime_guard: tokio::sync::MutexGuard<'_, ChannelRuntimeV1>,
        mut states_guard: tokio::sync::MutexGuard<'_, ChannelSpellStates>,
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
        let (runtime, states) = (&mut *runtime_guard, &mut *states_guard);
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
        let staging = |runtime: &ChannelRuntimeV1| {
            let mut census = runtime.positioned_melee_census()?;
            census.retain(|(_, at, _)| {
                encounter
                    .players
                    .iter()
                    .any(|p| local(p.from) == at.position())
            });
            Ok::<_, crate::domain::carrier_values::CarrierError>(census)
        };
        let Ok(census) = staging(runtime) else {
            return UseDisposition::Rejected;
        };
        drop(states_guard);
        drop(runtime_guard);
        let mut observed = Vec::new();
        let mut party = Vec::new();
        for &(member, _, member_session) in &census {
            let Some(member_session) = member_session else {
                continue;
            };
            let Ok(Some(fence)) = self.current_quest_fence(member_session).await else {
                return UseDisposition::Rejected;
            };
            observed.push((member, member_session, fence));
        }
        let (mut runtime_guard, mut states_guard) = self.callback_guards().await;
        let (runtime, states) = (&mut *runtime_guard, &mut *states_guard);
        if staging(runtime).ok().as_ref() != Some(&census) {
            return UseDisposition::StaleState;
        }
        for &(member, member_session, fence) in &observed {
            if !states.current_cooldown_transport(
                member,
                member_session,
                fence.connection_generation.get(),
            ) || !runtime
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
        let mut admission = SourceBossReservation {
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
        let reserved = match self.stage_source_boss(runtime, states, actor, session, &source_use) {
            Ok(reserved) => reserved,
            Err(disposition) => return disposition,
        };
        let mut characters: Vec<_> = admission
            .party
            .iter()
            .map(|member| *member.character_id.as_bytes())
            .collect();
        characters.sort_unstable();
        let party_key = (key, characters);
        let pending = &self.states.quest_callbacks.boss_admissions;
        let recovering = {
            let mut pending = pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let binding = runtime.binding();
            pending.retain(|_, (owner, since, _)| {
                *owner == binding && since.elapsed() < std::time::Duration::from_secs(20 * 60 * 60)
            });
            let recovering = pending.contains_key(&party_key);
            admission.id = pending
                .entry(party_key.clone())
                .or_insert((binding, std::time::Instant::now(), admission.id))
                .2;
            recovering
        };
        drop(states_guard);
        drop(runtime_guard);
        #[cfg(test)]
        if let Ok(barrier) = PRE_SOURCE_BOSS_CHARGE.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        let retry = super::super::AdmissionRetryBudget::for_command(self.root);
        let receipt = if recovering {
            retry
                .retry_durability("source_boss_admission_reconcile", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Reconcile)
                })
                .await
        } else {
            Ok(false)
        };
        let paid = if matches!(receipt, Ok(false)) {
            retry
                .retry_durability("source_boss_admission", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Commit)
                })
                .await
        } else {
            receipt
        };
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
        let ambiguous = matches!(
            paid,
            Err(DurabilityError::CommitOutcomeUnknown | DurabilityError::RootPassDeadlineExceeded)
        );
        let paid = if ambiguous {
            retry
                .retry_durability("source_boss_admission_reconcile", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Reconcile)
                })
                .await
        } else {
            paid
        };
        let (mut runtime, mut states) = self.callback_guards().await;
        match paid {
            Ok(true) => {
                #[cfg(test)]
                if self
                    .states
                    .quest_callbacks
                    .interrupt_source_boss_before_projection
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    reserved.slots.cancel(&mut runtime);
                    return UseDisposition::Rejected;
                }
                if observed.iter().any(|(member, session, fence)| {
                    !states.current_cooldown_transport(
                        *member,
                        *session,
                        fence.connection_generation.get(),
                    ) || !runtime
                        .bound_attacker_lease(*member, *session)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                && lease.generation() == fence.character_lease_generation
                        })
                }) {
                    reserved.slots.cancel(&mut runtime);
                    return UseDisposition::StaleState;
                }
                let disposition = self.publish_reserved_encounter(
                    reserved,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    map,
                    callback,
                    key,
                    revision,
                    after,
                    cause,
                    lever_projection,
                    None,
                );
                if disposition == UseDisposition::Committed {
                    pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&party_key);
                }
                disposition
            }
            Ok(false) => {
                reserved.slots.cancel(&mut runtime);
                if !recovering && !ambiguous {
                    pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&party_key);
                }
                UseDisposition::Rejected
            }
            Err(error) => {
                reserved.slots.cancel(&mut runtime);
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

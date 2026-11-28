//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::{
    DurabilityError,
    boss_encounter::{SourceBossAdmission, SourceBossLever, SourceBossReservation},
};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

// Only unprojected payments on this live owner can be resumed. The durable
// reconcile checks the current party fences, owner generation and cooldown IDs.
pub(super) type PaidParties = std::collections::HashMap<(u64, Vec<[u8; 16]>), [u8; 16]>;

pub(super) struct SourceBossUse<'a> {
    pub(super) map: &'a BundleMap,
    pub(super) callback: &'a Callback,
    pub(super) key: u64,
    pub(super) revision: u64,
    pub(super) after: crate::map::view::EntryFacts,
    pub(super) cause: QuestCause,
    pub(super) lever: SourceBossLever,
    pub(super) publication: Option<&'a std::sync::atomic::AtomicBool>,
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
            publication,
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
        let staging_party = |runtime: &ChannelRuntimeV1| {
            let mut census = runtime.positioned_melee_census()?;
            census.retain(|(_, at, _)| {
                encounter
                    .players
                    .iter()
                    .any(|p| local(p.from) == at.position())
            });
            Ok::<_, crate::foundation::CarrierError>(census)
        };
        let Ok(census) = staging_party(runtime) else {
            return UseDisposition::Rejected;
        };
        drop(states_guard);
        drop(runtime_guard);
        let mut observed = Vec::new();
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
        if staging_party(runtime).ok().as_ref() != Some(&census) {
            return UseDisposition::StaleState;
        }
        let party_is_current = |runtime: &ChannelRuntimeV1, states: &ChannelSpellStates| {
            observed.iter().all(|&(member, member_session, fence)| {
                states.current_cooldown_transport(
                    member,
                    member_session,
                    fence.connection_generation.get(),
                ) && runtime
                    .bound_attacker_lease(member, member_session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            })
        };
        if !party_is_current(runtime, states) {
            return UseDisposition::Rejected;
        }
        let party: Vec<_> = observed
            .iter()
            .map(|(_, _, fence)| fence.item_fence())
            .collect();
        // Key and revision describe the wire target. Current party fences
        // authorize the write but are never part of the command identity.
        let mut identity = match lever {
            SourceBossLever::CourtWarlock => b"cwarlock:lever:v2".to_vec(),
            SourceBossLever::Arbaziloth => b"arbaziloth:lever:v2".to_vec(),
        };
        identity.extend_from_slice(&key.to_be_bytes());
        identity.extend_from_slice(&revision.to_be_bytes());
        let mut characters: Vec<_> = party.iter().map(|f| *f.character_id.as_bytes()).collect();
        characters.sort_unstable();
        let party_key = (key, characters);
        let recoveries = &self.states.quest_callbacks.source_boss_recoveries;
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
        if let Err(disposition) =
            self.stage_source_boss(runtime, states, actor, session, &source_use)
        {
            return disposition;
        }
        drop(states_guard);
        drop(runtime_guard);
        #[cfg(test)]
        if let Ok(barrier) = PRE_SOURCE_BOSS_CHARGE.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        if let Some(publication) = publication {
            publication.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let prior = recoveries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&party_key)
            .copied();
        let retry = super::super::AdmissionRetryBudget::for_command(self.root);
        let fresh_id = admission.id;
        let mut paid = Ok(false);
        if let Some(id) = prior {
            admission.id = id;
            paid = retry
                .retry_durability("source_boss_admission_resume", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Reconcile)
                })
                .await;
        }
        if matches!(paid, Ok(false)) {
            admission.id = fresh_id;
            paid = retry
                .retry_durability("source_boss_admission", || {
                    self.source_boss_admission(&admission, SourceBossAdmission::Commit)
                })
                .await;
        }
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
            recoveries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(party_key.clone(), admission.id);
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
                recoveries
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(party_key.clone(), admission.id);
                #[cfg(test)]
                if self
                    .states
                    .quest_callbacks
                    .interrupt_source_boss_before_projection
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    return UseDisposition::Rejected;
                }
                let (mut runtime, mut states) = self.callback_guards().await;
                if !party_is_current(&runtime, &states)
                    || staging_party(&runtime).ok().as_ref() != Some(&census)
                {
                    return UseDisposition::StaleState;
                }
                let current = match self.stage_source_boss(
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    &source_use,
                ) {
                    Ok(current) => current,
                    Err(disposition) => return disposition,
                };
                let disposition = self.publish_source_boss(
                    current,
                    &mut runtime,
                    &mut states,
                    map,
                    lever_projection,
                );
                if disposition == UseDisposition::Committed {
                    recoveries
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&party_key);
                }
                disposition
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

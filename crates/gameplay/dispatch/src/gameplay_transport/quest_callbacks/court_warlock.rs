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

pub(super) struct PendingSourceBoss {
    admission: SourceBossReservation,
    staged: super::encounter::ReservedSourceBoss,
    projection: Option<crate::map::facts::SourceBossLeverProjection>,
    actor: ExactActorRef,
    session: GameSessionId,
    callback: Callback,
    key: u64,
    revision: u64,
    after: crate::map::view::EntryFacts,
    cause: QuestCause,
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
        let staging_party = |runtime: &ChannelRuntimeV1| {
            runtime.positioned_melee_census().map(|census| {
                census
                    .into_iter()
                    .filter_map(|(member, at, session)| {
                        let session = session?;
                        encounter
                            .players
                            .iter()
                            .any(|p| local(p.from) == at.position())
                            .then_some((member, at, session))
                    })
                    .collect::<Vec<_>>()
            })
        };
        let Ok(members) = staging_party(runtime) else {
            return UseDisposition::Rejected;
        };
        drop(spell_guard);
        drop(runtime_guard);
        let mut party = Vec::new();
        for &(member, at, member_session) in &members {
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
        if staging_party(&runtime).ok().as_ref() != Some(&members)
            || members
                .iter()
                .any(|member| self.states.quest_callbacks.closing(member.2))
        {
            return UseDisposition::StaleState;
        }
        let held = self
            .stage_source_boss(&mut runtime, &mut states, actor, session, &source_use)
            .and_then(super::encounter::StagedSourceBoss::reserve);
        let staged = match held {
            Ok(held) => held,
            Err(disposition) => return disposition,
        };
        {
            let Ok(mut pending) = self.states.quest_callbacks.source_boss.lock() else {
                return UseDisposition::Rejected;
            };
            if pending.is_some() {
                return UseDisposition::Rejected;
            }
            *pending = Some(PendingSourceBoss {
                admission: admission.clone(),
                staged,
                projection: Some(lever_projection),
                actor,
                session,
                callback: callback.clone(),
                key,
                revision,
                after,
                cause,
            });
        }
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
                if self.publish_paid_source_boss(map, admission.id).await {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                }
            }
            Ok(false) => {
                self.clear_source_boss(admission.id);
                UseDisposition::Rejected
            }
            Err(error) => {
                super::super::operator_event(&format!(
                    "source_boss_admission_deferred reason={error:?}"
                ));
                UseDisposition::Rejected
            }
        }
    }

    pub(super) fn has_pending_source_boss(&self, session: GameSessionId) -> bool {
        self.states
            .quest_callbacks
            .source_boss
            .lock()
            .map_or(true, |pending| {
                pending
                    .as_ref()
                    .is_some_and(|p| p.staged.players.iter().any(|player| player.1 == session))
            })
    }

    fn clear_source_boss(&self, id: [u8; 16]) {
        if let Ok(mut pending) = self.states.quest_callbacks.source_boss.lock()
            && pending
                .as_ref()
                .is_some_and(|entry| entry.admission.id == id)
        {
            *pending = None;
        }
    }

    pub(in crate::gameplay_transport) async fn reconcile_source_boss(
        &self,
        session: Option<GameSessionId>,
    ) -> bool {
        let admission = || {
            self.states
                .quest_callbacks
                .source_boss
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .filter(|p| {
                    session.is_none_or(|session| {
                        p.staged.players.iter().any(|player| player.1 == session)
                    })
                })
                .map(|p| p.admission.clone())
        };
        if admission().is_none() {
            return true;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        let Some(admission) = admission() else {
            return true;
        };
        match self
            .source_boss_admission(&admission, SourceBossAdmission::Reconcile)
            .await
        {
            Ok(true) => match self.bundle_map() {
                Some(map) => self.publish_paid_source_boss(map, admission.id).await,
                None => false,
            },
            Ok(false) => {
                self.clear_source_boss(admission.id);
                true
            }
            Err(_) => false,
        }
    }

    async fn publish_paid_source_boss(&self, map: &BundleMap, id: [u8; 16]) -> bool {
        let (mut runtime, mut states) = self.quest_turn_with(&self.states.spell_states).await;
        let Ok(mut pending) = self.states.quest_callbacks.source_boss.lock() else {
            return false;
        };
        let Some(p) = pending.as_mut().filter(|p| p.admission.id == id) else {
            return false;
        };
        let source = SourceBossUse {
            map,
            callback: &p.callback,
            key: p.key,
            revision: p.revision,
            after: p.after,
            cause: p.cause,
            lever: p.admission.lever,
        };
        let published = p.staged.native.publish(&mut runtime, |runtime| {
            if p.admission.party.iter().any(|fence| {
                !p.staged.players.iter().any(|(actor, session, _)| {
                    *session == fence.game_session_id
                        && runtime
                            .bound_attacker_lease(*actor, *session)
                            .is_ok_and(|lease| {
                                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                    && lease.generation() == fence.character_lease_generation
                            })
                })
            }) {
                return false;
            }
            let Ok(current) =
                self.stage_source_boss(runtime, &mut states, p.actor, p.session, &source)
            else {
                return false;
            };
            if current.players != p.staged.players {
                return false;
            }
            let Some(projection) = p.projection.take() else {
                return false;
            };
            self.publish_source_boss(current, &mut states, map, projection);
            true
        });
        if published {
            *pending = None;
        }
        published
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

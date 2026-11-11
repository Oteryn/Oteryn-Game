//! The production adapter uses the same native reservation and durable group owner
//! as other activities. It cannot allocate an Instance or clone Channel authority.
use super::super::connection::AdmissionRefusal;
use super::super::connection::{ConnectionIdentifiers, UseOutcome};
use super::super::native_activity::NativeActivityOutcome;
use super::super::{AdmissionRetryBudget, ComposedFreshAdmission, SecureIdentifiers};
use super::{SoulpitEntryIntent, outcome};
use crate::durability::DurabilityError;
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterRequest, ActivityHandoffOwner, ActivityParticipant,
    ActivityReturnReason,
};
use crate::durability::character_last_position::LastPositionCell;
use crate::durability::soulpit::{
    SOULPIT_ACTIVITY_KEY, SoulpitEntryRequest, SoulpitParticipantFact,
};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_domain::domain::soulpit::{self, EXIT_POSITION};
use oteryn_protocol_oteryn::world_object::UseDisposition;

// RootUnavailable is the nonblocking root holder's busy result. All other failures,
// including lost current authority and unknown commits, retain their fail-closed path.
fn classify_read(error: DurabilityError) -> AdmissionRefusal {
    match error {
        DurabilityError::RootUnavailable => AdmissionRefusal::Unavailable,
        _ => AdmissionRefusal::Rejected,
    }
}

pub(super) async fn read_with_budget<T>(
    retry: &AdmissionRetryBudget<'_>,
    mut read: impl AsyncFnMut() -> Result<T, DurabilityError>,
) -> Result<T, ()> {
    retry
        .retry_unavailable(async || read().await.map_err(classify_read))
        .await
        .map_err(|_| ())
}

fn owner_read<T>(read: Result<T, DurabilityError>) -> Result<Option<T>, ()> {
    match read {
        Ok(value) => Ok(Some(value)),
        // These two callers only read current Soulpit bindings/members. A bounded
        // read failing to acquire its locks does not prove the Channel was lost.
        Err(
            DurabilityError::RootUnavailable
            | DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::Database(crate::durability::SqlxFailureClass::RetryableStatement),
        ) => Ok(None),
        Err(_) => Err(()),
    }
}

fn read_retry_deadline(now: u64, prior: Option<u64>) -> Option<u64> {
    let retry = now.saturating_add(10_000);
    Some(prior.map_or(retry, |prior| prior.min(retry)))
}

async fn lock_owner_cycle<'a>(
    transitions: &'a tokio::sync::Mutex<()>,
    lower_bound: u64,
    clock: impl FnOnce() -> u64,
) -> (tokio::sync::MutexGuard<'a, ()>, u64) {
    let cycle = transitions.lock().await;
    let now = lower_bound.max(clock());
    (cycle, now)
}

pub struct NativeSoulpitHandoff;

impl NativeSoulpitHandoff {
    pub(in crate::gameplay_transport) async fn drive_soulpit(
        &self,
        admission: &ComposedFreshAdmission<'_, '_, '_>,
        now_micros: u64,
        global_tick: bool,
    ) -> Result<Option<u64>, ()> {
        let (_cycle, now_micros) = lock_owner_cycle(
            &admission.activity_runtimes.soulpit_transitions,
            now_micros,
            || admission.owner_now().get(),
        )
        .await;
        let world = admission
            .runtime
            .lock()
            .await
            .binding()
            .runtime_scope()
            .world_id();
        let owner = ActivityHandoffOwner::new(admission.root.clone());
        let mut after = None;
        let mut next = None;
        loop {
            let Some(bindings) = owner_read(
                owner
                    .active_soulpits(admission.character, admission.holder, world, after)
                    .await,
            )?
            else {
                return Ok(read_retry_deadline(now_micros, next));
            };
            for binding in &bindings {
                let registered = admission
                    .activity_runtimes
                    .get(binding.scope, binding.generation.get())
                    .ok_or(())?;
                let Some(members) = owner_read(
                    owner
                        .soulpit_members(admission.character, admission.holder, *binding)
                        .await,
                )?
                else {
                    return Ok(read_retry_deadline(now_micros, next));
                };
                let driven = super::owner_runtime::drive_registered(
                    admission,
                    &registered,
                    super::runtime::SoulpitActivityBinding {
                        handoff_id: binding.handoff_id,
                        scope: binding.scope,
                        generation: binding.generation,
                    },
                    &members,
                    now_micros,
                    global_tick,
                )
                .await;
                let due = match driven {
                    Ok(due) => due,
                    // A busy native owner retains all receipts and retries on
                    // the next short owner deadline; contention is not lost authority.
                    Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable) => {
                        Some(now_micros.saturating_add(10_000))
                    }
                    Err(_) => return Err(()),
                };
                if let Some(due) = due {
                    next = Some(next.map_or(due, |old: u64| old.min(due)));
                }
            }
            if bindings.len() < 15 {
                break;
            }
            after = bindings.last().map(|binding| binding.handoff_id);
        }
        for registered in admission.activity_runtimes.activity_owners()? {
            if registered.template.as_ref().is_none_or(|template| {
                template.key != crate::content::soulpit_activity::SOULPIT_TEMPLATE_KEY
                    || template.revision
                        != crate::content::soulpit_activity::SOULPIT_TEMPLATE_REVISION
            }) {
                continue;
            }
            match super::owner_runtime::cleanup_terminal(admission, &registered).await {
                Ok(()) => {}
                Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable) => {
                    next = read_retry_deadline(now_micros, next);
                }
                Err(_) => return Err(()),
            }
        }
        Ok(next)
    }

    pub(super) async fn enter_soulpit(
        &self,
        admission: &ComposedFreshAdmission<'_, '_, '_>,
        request: SoulpitEntryIntent,
    ) -> Result<Option<UseOutcome>, ()> {
        admission.enter_shared_soulpit(request).await
    }

    pub(super) async fn return_from_soulpit(
        &self,
        admission: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
    ) -> Result<(), ()> {
        return_shared_soulpit(admission.clone(), actor, session, command_id).await
    }
}

async fn return_shared_soulpit<'a, 'f, 's>(
    admission: ComposedFreshAdmission<'a, 'f, 's>,
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
) -> Result<(), ()> {
    let _cycle = admission.activity_runtimes.soulpit_transitions.lock().await;
    let admitted = admission.activity_runtimes.connection(session).ok_or(())?;
    if admitted.runtime_actor != Some(actor) {
        return Err(());
    }
    let fence = admitted.item_fence.ok_or(())?;
    let owner = ActivityHandoffOwner::new(admission.root.clone());
    let retry = AdmissionRetryBudget::for_command(admission.root);
    let mut after = None;
    let binding = loop {
        let bindings = read_with_budget(&retry, async || {
            owner
                .active_soulpits(
                    admission.character,
                    admission.holder,
                    admitted.world_id,
                    after,
                )
                .await
        })
        .await?;
        if let Some(binding) = bindings.iter().find(|binding| {
            binding.scope == fence.runtime_scope
                && binding.generation == fence.scope_ownership_generation
        }) {
            break *binding;
        }
        if bindings.len() < 15 {
            return Err(());
        }
        after = bindings.last().map(|binding| binding.handoff_id);
    };
    let members = read_with_budget(&retry, async || {
        owner
            .soulpit_members(admission.character, admission.holder, binding)
            .await
    })
    .await?;
    let member = members
        .iter()
        .find(|member| member.session == session && member.character == fence.character_id)
        .ok_or(())?;
    let registered = admission
        .activity_runtimes
        .get(binding.scope, binding.generation.get())
        .ok_or(())?;
    super::owner_runtime::leave_registered(
        &admission,
        &registered,
        super::runtime::SoulpitActivityBinding {
            handoff_id: binding.handoff_id,
            scope: binding.scope,
            generation: binding.generation,
        },
        &members,
        member.slot,
        admission.owner_now().get(),
    )
    .await
    .map_err(|_| ())?;
    match admission
        .return_native_activity(admitted, command_id, ActivityReturnReason::Exit)
        .await?
    {
        NativeActivityOutcome::Committed(commit) => admission.publish_activity_commit(&commit),
        NativeActivityOutcome::Refused(_) => Err(()),
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn soulpit_result(
        &self,
        result: NativeActivityOutcome,
    ) -> Result<Option<UseOutcome>, ()> {
        match result {
            NativeActivityOutcome::Committed(commit) => {
                self.publish_activity_commit(&commit)?;
                Ok(Some(outcome(UseDisposition::Committed)))
            }
            NativeActivityOutcome::Refused(_) => Ok(Some(outcome(UseDisposition::Rejected))),
        }
    }

    pub(in crate::gameplay_transport) async fn enter_shared_soulpit(
        &self,
        request: SoulpitEntryIntent,
    ) -> Result<Option<UseOutcome>, ()> {
        let _cycle = self.activity_runtimes.soulpit_transitions.lock().await;
        let Some(admitted) = self
            .activity_runtimes
            .connection(request.command.game_session_id)
        else {
            return Ok(request.selected.map(|_| outcome(UseDisposition::Rejected)));
        };
        if admitted.runtime_actor != Some(request.actor)
            || admitted.item_fence != request.command.item_fence
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        if let Some(replay) = self
            .retry_native_soulpit(
                admitted,
                request.command.command_id,
                &request.encoded_intent,
            )
            .await?
        {
            return self.soulpit_result(replay);
        }
        let Some(selected) = request.selected else {
            return Ok(None);
        };
        let native = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(())?;
        let solo = matches!(
            &selected,
            super::handoff::SoulpitSelection::Soulseals { .. }
        );
        let funding = match selected {
            super::handoff::SoulpitSelection::Core(core) => {
                crate::durability::soulpit::SoulpitFunding::Core {
                    source: core.item_instance_id,
                    definition: core.definition,
                }
            }
            super::handoff::SoulpitSelection::Soulseals { species } => {
                crate::durability::soulpit::SoulpitFunding::Soulseals { species }
            }
        };
        let species = match &funding {
            crate::durability::soulpit::SoulpitFunding::Core { definition, .. } => {
                let Some(species) =
                    crate::durability::item_craft::soulpit::entry_species(native, definition)
                else {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                };
                species
            }
            crate::durability::soulpit::SoulpitFunding::Soulseals { species } => species.as_str(),
        };
        let Some(record) = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == species)
        else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        {
            let runtime = self.runtime.lock().await;
            if crate::ai_think::CreatureAiTable::qualify_soulpit_creature(
                &runtime,
                native,
                &record.profile.target,
            )
            .is_err()
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
        }
        let Some(fence) = admitted.item_fence else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let retry = AdmissionRetryBudget::for_command(self.root);
        if self.ensure_soulpit_template_runtime(&retry).await.is_err() {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let template = self.activity_runtimes.soulpit_template().ok_or(())?;
        let mut after = None;
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let destination = loop {
            let candidates = read_with_budget(&retry, async || {
                owner
                    .ready_allocations(self.character, self.holder, admitted.world_id, after)
                    .await
            })
            .await?;
            let mut found = None;
            for allocation in &candidates {
                if let Some(registered) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                    && registered.assignment_source_revision == allocation.source_revision
                    && registered.template.as_ref().is_some_and(|key| {
                        key.key == template.template_key()
                            && key.revision == template.template_revision()
                    })
                {
                    found = Some((registered, allocation.clone()));
                    break;
                }
            }
            if let Some(found) = found {
                break found;
            }
            if candidates.len() < 15 {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let Some(crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. }) =
                candidates.last().map(|a| a.scope)
            else {
                return Err(());
            };
            after = Some(instance_id);
        };
        let (registered, allocation) = destination;
        let mut participants = Vec::new();
        let mut facts = Vec::new();
        {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            let census = runtime.positioned_actor_census().map_err(|_| ())?;
            // Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77:
            // every occupied staging tile participates; reject before burning a core.
            for slot in 0..if solo { 1 } else { soulpit::MAX_PARTICIPANTS } {
                let y = 31158 + i32::try_from(slot).map_err(|_| ())?;
                let mut occupants = census.iter().filter(|(_, at, session)| {
                    let p = at.position();
                    session.is_some() && (p.x, p.y, p.floor) == (32375, y, 8)
                });
                let Some((actor, position, Some(session))) = occupants.next() else {
                    continue;
                };
                if occupants.next().is_some() {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                let member = self.activity_runtimes.connection(*session).ok_or(())?;
                let member_fence = member.item_fence.ok_or(())?;
                if member.runtime_actor != Some(*actor)
                    || member_fence.runtime_scope != fence.runtime_scope
                    || member_fence.scope_ownership_generation != fence.scope_ownership_generation
                    || !member.continuity.selected_capabilities.contains(
                        oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                    )
                {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                let source = self.placed_record(&runtime, position.position());
                let exit = self.placed_record(
                    &runtime,
                    MovementLocalPosition {
                        x: i32::from(EXIT_POSITION.x),
                        y: i32::from(EXIT_POSITION.y),
                        floor: 8,
                    },
                );
                let cell = soulpit::entry_destination(slot as u8).map_err(|_| ())?;
                participants.push(ActivityParticipant {
                    source: member_fence,
                    destination_game_session_id: SecureIdentifiers.game_session_id().ok_or(())?,
                    destination_transport: Some(SecureIdentifiers.transport_ref().ok_or(())?),
                    selected_capabilities: member
                        .continuity
                        .selected_capabilities
                        .as_slice()
                        .to_vec(),
                    source_position: source.record,
                    source_position_write_sequence: source.write_sequence,
                    destination_cell: LastPositionCell {
                        x: i32::from(cell.x),
                        y: i32::from(cell.y),
                        floor: 8,
                    },
                    exit_position: exit.record,
                });
                facts.push(SoulpitParticipantFact {
                    character: member_fence.character_id,
                    game_master: false,
                });
            }
        }
        if !participants
            .iter()
            .any(|p| p.source == fence && p.source_position.cell.y == 31158)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let destination_content = {
            let runtime = registered.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let handoff_id = *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes();
        let entry = SoulpitEntryRequest {
            handoff_id,
            command: CommandRef::new(
                request.command.game_session_id,
                CommandId::new(request.command.command_id).map_err(|_| ())?,
            ),
            funding,
            intent: request.encoded_intent,
            started_at_micros: self.owner_now().get(),
            participants: facts,
        };
        let group = ActivityEnterRequest {
            handoff_id,
            command_id: request.command.command_id,
            initiating_character_id: crate::foundation::CharacterId::decode(
                fence.character_id.as_bytes(),
            )
            .map_err(|_| ())?,
            activity_key: SOULPIT_ACTIVITY_KEY.into(),
            template_revision: template.template_revision().into(),
            content_revision: template.content_revision().into(),
            destination_scope: allocation.scope,
            destination_generation: allocation.generation,
            destination_assignment_source_revision: allocation.source_revision,
            destination_content,
            time_limit_ms: soulpit::TIME_LIMIT_MICROS / 1000,
            participants,
        };
        self.soulpit_result(self.enter_native_soulpit(group, entry).await?)
    }
}

#[cfg(test)]
mod read_retry_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn soulpit_owner_busy_reads_retry_next_deadline_without_masking_authority_loss() {
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::Database(
                crate::durability::SqlxFailureClass::RetryableStatement
            ))),
            Ok(None)
        );
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::Database(
                crate::durability::SqlxFailureClass::Database(None)
            ))),
            Err(())
        );
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::RootPassDeadlineExceeded)),
            Ok(None),
            "a read deadline is not proof that the Channel owner became stale"
        );
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::RootUnavailable)),
            Ok(None)
        );
        assert_eq!(owner_read::<()>(Err(DurabilityError::Unavailable)), Err(()));
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::InvalidStoredState)),
            Err(())
        );
        assert_eq!(
            owner_read::<()>(Err(DurabilityError::CommitOutcomeUnknown)),
            Err(())
        );
        assert_eq!(read_retry_deadline(1_000, None), Some(11_000));
        assert_eq!(read_retry_deadline(1_000, Some(5_000)), Some(5_000));
    }

    #[test]
    fn soulpit_command_reads_retry_only_busy_root_under_existing_budget() -> Result<(), String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|error| format!("{error:?}"))?;
        runtime.block_on(async {
            tokio::time::pause();
            let budget = AdmissionRetryBudget {
                deadline: tokio::time::Instant::now() + Duration::from_secs(1),
                backoff: Duration::from_millis(50),
                stale_rounds: 1,
                root: None,
            };
            let mut calls = 0;
            let result = read_with_budget(&budget, async || {
                calls += 1;
                if calls == 1 {
                    Err(DurabilityError::RootUnavailable)
                } else {
                    Ok(7)
                }
            })
            .await;
            assert_eq!(result, Ok(7));
            assert_eq!(calls, 2);
            let mut calls = 0;
            let result = read_with_budget::<()>(&budget, async || {
                calls += 1;
                Err(DurabilityError::Unavailable)
            })
            .await;
            assert_eq!(result, Err(()));
            assert_eq!(calls, 1);
        });
        Ok(())
    }
}

#[cfg(test)]
mod clock_contention_tests {
    use super::lock_owner_cycle;
    use crate::domain::soulpit::SoulpitEncounter;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    use tokio::sync::{Mutex, oneshot};

    #[test]
    fn soulpit_tick_clock_waits_for_entry_transition() -> Result<(), String> {
        contention(false)
    }

    #[test]
    fn soulpit_tick_clock_waits_for_exit_transition() -> Result<(), String> {
        contention(true)
    }

    fn contention(exit: bool) -> Result<(), String> {
        fn debug(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(debug)?;
        runtime.block_on(async {
            let gate = Arc::new(Mutex::new(()));
            let clock = Arc::new(AtomicU64::new(100));
            let encounter = Arc::new(Mutex::new(if exit {
                Some(SoulpitEncounter::new(100, &[0, 1]).map_err(debug)?)
            } else {
                None
            }));
            let held = gate.lock().await;
            let (queued, waiting) = oneshot::channel();
            let tick_gate = Arc::clone(&gate);
            let tick_clock = Arc::clone(&clock);
            let tick_encounter = Arc::clone(&encounter);
            let tick = tokio::spawn(async move {
                queued
                    .send(())
                    .map_err(|_| "clock waiter receiver closed".to_string())?;
                let (_guard, now) =
                    lock_owner_cycle(&tick_gate, 100, || tick_clock.load(Ordering::SeqCst)).await;
                tick_encounter
                    .lock()
                    .await
                    .as_mut()
                    .ok_or("entry encounter")?
                    .advance(now)
                    .map_err(debug)?;
                Ok::<_, String>(now)
            });
            // On this current-thread runtime, the waiter reaches the held gate before
            // this receiver resumes. No guessed sleep controls the contention.
            waiting.await.map_err(debug)?;
            if exit {
                encounter
                    .lock()
                    .await
                    .as_mut()
                    .ok_or("exit encounter")?
                    .leave(1, 200)
                    .map_err(debug)?;
            } else {
                *encounter.lock().await = Some(SoulpitEncounter::new(200, &[0, 1]).map_err(debug)?);
            }
            clock.store(200, Ordering::SeqCst);
            drop(held);
            assert_eq!(tick.await.map_err(debug)??, 200);
            // Existing trusted PG scenarios can advance a future owner deadline without
            // sleeping. Preserve that lower bound while taking the fresh clock sample.
            let (_guard, now) = lock_owner_cycle(&gate, 300, || clock.load(Ordering::SeqCst)).await;
            assert_eq!(now, 300);
            Ok(())
        })
    }
}

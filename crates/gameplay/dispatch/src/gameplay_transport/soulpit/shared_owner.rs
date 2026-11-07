//! One installed composition root for native USE, Instance transfer and cadence.
use super::super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
};
use super::{BossActivityHandoff, SoulpitEntryIntent};
use crate::foundation::ExactActorRef;
use std::{future::Future, pin::Pin};

pub(in crate::gameplay_transport) struct SharedSoulpitOwner;
impl BossActivityHandoff for SharedSoulpitOwner {
    fn enter_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        intent: SoulpitEntryIntent,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UseOutcome>, ()>> + 'a>> {
        Box::pin(admission.enter_shared_soulpit(intent))
    }
    fn return_from_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        command: UseCommand,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + 'a>> {
        Box::pin(admission.return_shared_soulpit(actor, command))
    }
    fn drive_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        now: u64,
        global: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Option<u64>, ()>> + 'a>> {
        Box::pin(admission.drive_shared_soulpits(now, global))
    }
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(in crate::gameplay_transport) async fn drive_shared_soulpits(
        &self,
        now: u64,
        global: bool,
    ) -> Result<Option<u64>, ()> {
        let mut next = None;
        for registered in self.activity_runtimes.activity_owners()? {
            if registered.template.as_ref().is_none_or(|t| {
                t.key != "oteryn:room.soulpit-source" || t.revision != "crystal-201189f"
            }) {
                continue;
            }
            let due = match self
                .drive_one_soulpit(&registered, now.max(self.owner_now().get()), global)
                .await
            {
                Ok(due) => due,
                Err(
                    error @ (super::runtime::SoulpitRuntimeError::Durable(
                        crate::durability::soulpit::SoulpitWriteError::Unavailable(_),
                    )
                    | super::runtime::SoulpitRuntimeError::AdapterUnavailable),
                ) => {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT SHARED_RETRY {error:?}");
                    let _ = error;
                    // Retain uncertainty and the existing fifty-millisecond
                    // retry cadence, but yield so installation/lifecycle repair
                    // and other Instances can run before the next fenced read.
                    Some(
                        now.max(self.owner_now().get())
                            .checked_add(50_000)
                            .ok_or(())?,
                    )
                }
                Err(_) => return Err(()),
            };
            if let Some(due) = due {
                next = Some(next.map_or(due, |old: u64| old.min(due)));
            }
        }
        Ok(next)
    }

    pub(in crate::gameplay_transport) async fn drive_one_soulpit(
        &self,
        registered: &super::super::activity_runtime::RegisteredRuntime<'a>,
        now: u64,
        global: bool,
    ) -> Result<Option<u64>, super::runtime::SoulpitRuntimeError> {
        use super::runtime::{
            SoulpitActivityBinding, drive_soulpit_activity, drive_soulpit_deadlines,
        };
        let view = self.runtime_view(registered);
        let generation = crate::foundation::ScopeOwnershipGeneration::new(registered.generation)
            .map_err(|_| super::runtime::SoulpitRuntimeError::InvalidScope)?;
        let Some(permit) = view.spell_lane_permit().await else {
            #[cfg(test)]
            eprintln!("SRX3_SOULPIT READ_REFUSAL stage=lane");
            return Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable);
        };
        let Some(bound) = self
            .root
            .read_soulpit_scope_binding(self.character, self.holder, registered.scope, generation)
            .await?
        else {
            if let Some(cleanup) = self
                .root
                .read_soulpit_terminal_cleanup(
                    self.character,
                    self.holder,
                    registered.scope,
                    generation,
                )
                .await?
            {
                {
                    let mut runtime = registered.runtime.lock().await;
                    runtime
                        .owner_fence()
                        .map_err(|_| super::runtime::SoulpitRuntimeError::InvalidScope)?;
                    if !cleanup.matches(
                        runtime.binding().runtime_scope(),
                        runtime.binding().scope_generation(),
                    ) {
                        return Err(super::runtime::SoulpitRuntimeError::InvalidScope);
                    }
                    let (carrier, continuity, _) = runtime.soulpit_kernel_parts()?;
                    carrier.retire_soulpit(continuity, cleanup.handoff_id())?;
                }
                view.reset_soulpit_obelisk()
                    .await
                    .map_err(|_| super::runtime::SoulpitRuntimeError::AdapterUnavailable)?;
            }
            return Ok(None);
        };
        let scene = view.prepare_soulpit_scene(&permit).await.map_err(|_| {
            #[cfg(test)]
            eprintln!("SRX3_SOULPIT READ_REFUSAL stage=scene");
            super::runtime::SoulpitRuntimeError::AdapterUnavailable
        })?;
        let survivors = view.current_soulpit_survivors(&bound).await?;
        let binding = SoulpitActivityBinding {
            handoff_id: bound.handoff_id,
            scope: registered.scope,
            generation,
        };
        let mut owner = super::native_driver::NativeOwner {
            view: &view,
            bound,
            binding,
            scene,
            survivors,
            permit: Some(permit),
            runtime: Some(registered.runtime.lock().await),
            spells: Some(registered.states.spell_states.lock().await),
        };
        let due = if global {
            drive_soulpit_activity(&mut owner, now).await
        } else {
            drive_soulpit_deadlines(&mut owner, now).await
        }?;
        Ok(due)
    }
    pub(super) async fn current_soulpit_survivors(
        &self,
        bound: &crate::durability::soulpit::SoulpitScopeBinding,
    ) -> Result<
        Vec<crate::durability::character_progression::CurrentCharacterGameplayFence>,
        super::runtime::SoulpitRuntimeError,
    > {
        use crate::durability::admission_journal::party_target_binding::observe_current_visible_actor_for_source_read;
        use crate::foundation::GameSessionState;
        let (scope, generation) = {
            let runtime = self.runtime.lock().await;
            runtime
                .owner_fence()
                .map_err(|_| super::runtime::SoulpitRuntimeError::InvalidScope)?;
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
        };
        let mut survivors = Vec::with_capacity(bound.members.len());
        for member in &bound.members {
            // A current canonical session distinguishes a genuine departure
            // from a Type2 successor whose physical installation is unfinished.
            let (current, _) = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                self.root.clone(),
            )
            .current_session_at(member.destination_session)
            .await
            .map_err(|_| {
                #[cfg(test)]
                eprintln!("SRX3_SOULPIT READ_REFUSAL stage=current_session");
                super::runtime::SoulpitRuntimeError::AdapterUnavailable
            })?;
            if current.session_state() == GameSessionState::Terminal {
                continue;
            }
            let lease = current.current_character_lease();
            if current.current_runtime_scope() != scope
                || current.current_scope_generation() != generation
                || lease.character_id().as_bytes() != member.character.as_bytes()
            {
                return Err(super::runtime::SoulpitRuntimeError::InvalidScope);
            }
            let (actor, observed) = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                // Membership observes the retained living avatar during reconnect grace;
                // the canonical tuple and visible read below independently fence it.
                let census = runtime.positioned_melee_census()?;
                let Some((actor, _, _)) = census
                    .iter()
                    .find(|(_, _, session)| *session == Some(member.destination_session))
                else {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT READ_REFUSAL stage=physical_actor");
                    return Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable);
                };
                let control = runtime
                    .player_control_facts(*actor, member.destination_session)
                    .map_err(|_| super::runtime::SoulpitRuntimeError::AdapterUnavailable)?;
                let state = states
                    .get(&runtime, *actor, member.destination_session)
                    .ok_or(super::runtime::SoulpitRuntimeError::AdapterUnavailable)?;
                if state.vitals().health == 0 {
                    continue;
                }
                let agrees = match (current.session_state(), control.control_loss) {
                    (GameSessionState::Active, None) => current.current_transport().is_some(),
                    (GameSessionState::Reconnectable, Some(loss)) => {
                        current.current_transport().is_none()
                            && current
                                .current_control_loss_epoch()
                                .map(|epoch| epoch.get())
                                == Some(loss.epoch)
                            && current.current_original_grace_deadline()
                                == Some(loss.grace_deadline)
                    }
                    _ => false,
                };
                if !agrees
                    || runtime
                        .bound_attacker_lease(*actor, member.destination_session)
                        .ok()
                        != Some(lease)
                {
                    #[cfg(test)]
                    eprintln!(
                        "SRX3_SOULPIT READ_REFUSAL stage=canonical_physical_tuple state={:?} transport={} canonical_epoch={:?} canonical_deadline={:?} physical_loss={:?} lease_matches={}",
                        current.session_state(),
                        current.current_transport().is_some(),
                        current
                            .current_control_loss_epoch()
                            .map(|epoch| epoch.get()),
                        current.current_original_grace_deadline(),
                        control.control_loss,
                        runtime
                            .bound_attacker_lease(*actor, member.destination_session)
                            .ok()
                            == Some(lease)
                    );
                    return Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable);
                }
                let Some(observed) = observe_current_visible_actor_for_source_read(
                    &runtime,
                    *actor,
                    member.destination_session,
                ) else {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT READ_REFUSAL stage=visible_capture");
                    return Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable);
                };
                (*actor, observed)
            };
            let record = self
                .root
                .read_current_character(self.character, member.character)
                .await
                .map_err(|_| {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT READ_REFUSAL stage=current_character");
                    super::runtime::SoulpitRuntimeError::AdapterUnavailable
                })?;
            // Reconcile SQL against the same retained physical identity and
            // grace tuple. This read-only observation grants no player input.
            {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if !observed.matches(&runtime) {
                    #[cfg(test)]
                    eprintln!("SRX3_SOULPIT READ_REFUSAL stage=visible_recheck");
                    return Err(super::runtime::SoulpitRuntimeError::AdapterUnavailable);
                }
                let state = states
                    .get(&runtime, actor, member.destination_session)
                    .ok_or(super::runtime::SoulpitRuntimeError::AdapterUnavailable)?;
                if state.vitals().health == 0 {
                    continue;
                }
            }
            survivors.push(
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    character_id: member.character,
                    game_session_id: member.destination_session,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: lease.generation(),
                    runtime_scope: scope,
                    scope_ownership_generation: generation,
                    expected_character_revision: record.revision,
                },
            );
        }
        Ok(survivors)
    }
}

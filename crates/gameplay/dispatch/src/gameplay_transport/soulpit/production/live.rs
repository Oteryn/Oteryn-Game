use super::*;

struct Scene<'a> {
    cells: &'a NativeEntryMovementCells,
    targets: Vec<(ExactActorRef, ExactActorRef)>,
    deltas: Vec<(ExactActorRef, GameSessionId, i32)>,
    blocked: Vec<LogicalCell>,
}
impl ActivityHandoffScene for Scene<'_> {
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool {
        runtime::source_spawn_tile(self.cells, &self.blocked, at)
    }
    fn attack_target(&self, source: ExactActorRef) -> Option<ExactActorRef> {
        self.targets
            .iter()
            .find_map(|(actor, target)| (*actor == source).then_some(*target))
    }
    fn permits_condition(
        &self,
        _: ExactActorRef,
        target: ExactActorRef,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> bool {
        if from.floor != to.floor || !self.deltas.iter().any(|(actor, _, _)| *actor == target) {
            return false;
        }
        let steps = from.x.abs_diff(to.x).max(from.y.abs_diff(to.y));
        if steps > 32 {
            return false;
        }
        for step in 1..=steps {
            let cell = LogicalCell {
                x: from.x
                    + ((i64::from(to.x - from.x) * i64::from(step)) / i64::from(steps)) as i32,
                y: from.y
                    + ((i64::from(to.y - from.y) * i64::from(step)) / i64::from(steps)) as i32,
                z: i32::from(from.floor),
            };
            if self.blocked.contains(&cell)
                || !self
                    .cells
                    .spell_tiles()
                    .lookup(self.cells.scope(), cell)
                    .is_ok_and(|tile| {
                        !tile.flags().block_projectile && !tile.flags().protection_zone
                    })
            {
                return false;
            }
        }
        true
    }
    fn player_attribute_damage_delta(
        &self,
        target: ExactActorRef,
        session: GameSessionId,
        _: u64,
    ) -> Option<i32> {
        self.deltas.iter().find_map(|(actor, bound, delta)| {
            (*actor == target && *bound == session).then_some(*delta)
        })
    }
}

pub(super) struct LiveRuntime<'v, 'a, 'f, 's> {
    pub(super) admission: &'v ComposedFreshAdmission<'a, 'f, 's>,
    pub(super) run: SoulpitRun,
    pub(super) survivors: Vec<CurrentCharacterGameplayFence>,
    pub(super) now: u64,
}
impl LiveRuntime<'_, '_, '_, '_> {
    async fn refresh_survivors(&mut self) -> Result<(), SoulpitRuntimeError> {
        let present = {
            let runtime = self.admission.runtime.lock().await;
            let states = self
                .admission
                .states
                .spell_states
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            runtime
                .positioned_actor_census()?
                .into_iter()
                .filter_map(|(actor, position, session)| {
                    let session = session?;
                    if !in_arena(position.position())
                        || states.is_dead(actor)
                        || runtime.player_control_facts(actor, session).is_err()
                    {
                        return None;
                    }
                    let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                    self.run
                        .members
                        .iter()
                        .any(|(character, _)| character == &lease.character_id())
                        .then_some((session, lease))
                })
                .collect::<Vec<_>>()
        };
        let mut survivors = Vec::new();
        for (session, lease) in present {
            if let Some(fence) = crate::gameplay_transport::character_gameplay_fence::current(
                self.admission,
                session,
                true,
            )
            .await
                && fence.runtime_scope == self.run.binding.scope
                && fence.scope_ownership_generation == self.run.binding.generation
                && fence.character_id.as_bytes() == lease.character_id().as_bytes()
                && fence.character_lease_generation == lease.generation()
            {
                survivors.push(fence);
            }
        }
        survivors.sort_by_key(|fence| *fence.character_id.as_bytes());
        self.survivors = survivors;
        Ok(())
    }
}
impl<'view, 'assets, 'authority, 'services> ActivityHandoffOwnerRuntime
    for LiveRuntime<'view, 'assets, 'authority, 'services>
{
    fn binding(&self) -> SoulpitActivityBinding {
        self.run.binding
    }
    fn with_kernel<'a, T: Send + 'a>(
        &'a mut self,
        operation: impl for<'k> FnOnce(SoulpitKernel<'k>) -> Result<T, SoulpitRuntimeError> + Send + 'a,
    ) -> ActivityFuture<'a, T> {
        Box::pin(async move {
            let native = self
                .admission
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(SoulpitRuntimeError::MissingContent)?;
            let mut runtime = self.admission.runtime.lock().await;
            let states = self
                .admission
                .states
                .spell_states
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            let door = self
                .admission
                .door
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            let mut scene = Scene {
                cells: self.admission.movement_cells,
                targets: Vec::new(),
                deltas: Vec::new(),
                blocked: door.blocking_cells().iter().copied().collect(),
            };
            for (actor, _, session) in runtime.positioned_actor_census()? {
                if let Some(session) = session {
                    if !states.is_dead(actor)
                        && runtime.player_control_facts(actor, session).is_ok()
                        && let Some(state) = states.get(&runtime, actor, session)
                    {
                        scene.deltas.push((
                            actor,
                            session,
                            state.owned_conditions().attribute_damage_delta_at(self.now),
                        ));
                    }
                } else if let Some(target) = states
                    .monster_melee
                    .creature_ai
                    .state(actor)
                    .and_then(|state| state.target())
                {
                    scene.targets.push((actor, target));
                }
            }
            let kernel = runtime.borrow_soulpit_kernel()?;
            operation(SoulpitKernel {
                carrier: kernel.carrier,
                continuity: kernel.continuity,
                content: kernel.content,
                native,
                scene: &scene,
            })
        })
    }
    fn survivors(&self) -> Vec<CurrentCharacterGameplayFence> {
        self.survivors.clone()
    }
    fn read_encounter(&mut self) -> ActivityFuture<'_, SoulpitStoredEncounter> {
        Box::pin(async move {
            self.refresh_survivors().await?;
            let mut stored = self
                .admission
                .root
                .read_soulpit_encounter(
                    self.admission.character,
                    self.admission.holder,
                    self.run.binding.handoff_id,
                    self.run.binding.scope,
                    self.run.binding.generation,
                )
                .await?
                .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            if !stored.encounter.snapshot().phase.is_terminal() {
                for (character, slot) in self.run.members.clone() {
                    let present = self
                        .survivors
                        .iter()
                        .any(|fence| fence.character_id.as_bytes() == character.as_bytes());
                    let retained = stored.encounter.snapshot().surviving_slots.contains(&slot);
                    if present == retained {
                        continue;
                    }
                    let action = if present {
                        SoulpitAction::Rejoin { slot }
                    } else {
                        SoulpitAction::Leave { slot }
                    };
                    let step = super::runtime::request(
                        self.run.binding,
                        &stored,
                        self.now
                            .max(stored.encounter.snapshot().last_observed_at_micros),
                        action,
                        self.survivors(),
                    )?;
                    let outcome = self.commit_progress(step).await?;
                    stored = SoulpitStoredEncounter {
                        revision: outcome.revision,
                        species: stored.species,
                        encounter: outcome.encounter,
                    };
                    if stored.encounter.snapshot().phase.is_terminal() {
                        break;
                    }
                }
            }
            Ok(stored)
        })
    }
    fn commit_progress(
        &mut self,
        mut request: SoulpitStepRequest,
    ) -> ActivityFuture<'_, SoulpitStepOutcome> {
        Box::pin(async move {
            let _lane = self
                .admission
                .spell_lane_permit()
                .await
                .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            self.refresh_survivors().await?;
            let mut slots = Vec::new();
            for fence in &mut self.survivors {
                let mut slot = self
                    .admission
                    .revision_sequencer
                    .acquire(fence.character_id)
                    .await;
                fence.expected_character_revision = slot
                    .cursor(self.admission.root, self.admission.character)
                    .await
                    .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
                slots.push(slot);
            }
            request.survivors = self.survivors.clone();
            let result = self
                .admission
                .root
                .advance_soulpit(self.admission.character, self.admission.holder, request)
                .await?;
            Ok(result)
        })
    }
    fn register_creatures<'a>(
        &'a mut self,
        actors: &'a [oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveActor],
    ) -> ActivityFuture<'a, ()> {
        Box::pin(async move {
            let native = self
                .admission
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(SoulpitRuntimeError::MissingContent)?;
            let mut runtime = self.admission.runtime.lock().await;
            let mut states = self
                .admission
                .states
                .spell_states
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            for actor in actors {
                let snapshot = runtime.companion_snapshot_including_dead(actor.actor)?;
                // A physical spawn may die before its durable Spawned acknowledgement retries.
                if snapshot.health <= 0 {
                    continue;
                }
                let creature = crate::content::ProjectV2DefinitionRef {
                    family: crate::content::ProjectV2Family::Creature,
                    key: snapshot.state.policy.definition_key.clone(),
                    revision: snapshot.state.policy.definition_revision.clone(),
                };
                runtime.install_creature_policy(actor.actor, &creature.key)?;
                if states
                    .monster_melee
                    .creature_ai
                    .state(actor.actor)
                    .is_none()
                {
                    states
                        .monster_melee
                        .creature_ai
                        .admit_native_creature(&runtime, native, actor.actor, &creature, self.now)
                        .map_err(|_| SoulpitRuntimeError::MissingContent)?;
                }
            }
            Ok(())
        })
    }
    fn settle_boss_rewards(
        &mut self,
        reward: runtime::SoulpitBossReward,
    ) -> ActivityFuture<'_, ()> {
        Box::pin(async move { self.admission.settle_soulpit_boss_reward(&reward).await })
    }
    fn return_participants(
        &mut self,
        phase: crate::domain::soulpit::EncounterPhase,
    ) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            let sessions = {
                self.admission
                    .runtime
                    .lock()
                    .await
                    .activity_participant_sessions()?
            };
            for session in sessions {
                let admitted = self
                    .admission
                    .activity_runtimes
                    .connection(session)
                    .ok_or(SoulpitRuntimeError::InvalidState)?;
                let reason = if phase == crate::domain::soulpit::EncounterPhase::Completed {
                    ActivityReturnReason::Defeated
                } else {
                    ActivityReturnReason::Timeout
                };
                self.admission
                    .queue_soulpit_return(SoulpitReturnIntent {
                        scope: self.run.binding.scope,
                        generation: self.run.binding.generation.get(),
                        session,
                        admitted,
                        command: None,
                        exit_target: None,
                        reason,
                    })
                    .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            }
            Ok(())
        })
    }
}

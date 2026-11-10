//! Production adapter of the shared Instance owner to Crystal's process-local encounter.
//! Crystal 201189f data-global/lib/others/soulpit.lua:31-140 and
//! data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106.
use super::runtime::*;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::domain::soulpit::EncounterPhase;
use crate::durability::activity_handoff::{ActivityReturnReason, soulpit::SoulpitMembership};
use crate::durability::admission_journal::party_target_binding::{
    CurrentVisibleActorObservation, observe_current_visible_actor_for_source_read,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{
    SoulpitAction, SoulpitStepOutcome, SoulpitStepRequest, SoulpitStoredEncounter,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementLocalPosition,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission, activity_runtime::RegisteredRuntime, actor_spell::ChannelSpellStates,
    native_activity::NativeActivityOutcome,
};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveActor;

struct Blockers<'a> {
    local: &'a std::collections::BTreeSet<LogicalCell>,
    bundle: Option<crate::map::floor::BundleDoors<'a>>,
}
impl crate::map::floor::CellBlocker for Blockers<'_> {
    fn blocks(&self, cell: LogicalCell) -> bool {
        self.local.contains(&cell) || self.bundle.as_ref().is_some_and(|doors| doors.blocks(cell))
    }
}

struct Scene<'a> {
    room: &'a crate::content::QualifiedNativeEntryRoom,
    cells: &'a NativeEntryMovementCells,
    doors: &'a dyn crate::map::floor::CellBlocker,
    targets: Vec<(ExactActorRef, Option<ExactActorRef>)>,
    attributes: Vec<(ExactActorRef, GameSessionId, i32)>,
}
impl ActivityHandoffScene for Scene<'_> {
    fn emit_spawn_effect(&self, at: MovementLocalPosition, effect: u16) -> bool {
        super::super::soul_war_scene::emit_effect(self.room, at, effect)
    }
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool {
        crate::movement::relocate::creature_cell_admissible(self.cells, self.doors, at)
            && crate::durability::native_world_read::tile::<std::convert::Infallible>(self.room, at)
                .ok()
                .flatten()
                .is_some_and(|tile| !tile.flags().block_projectile)
    }
    fn attack_target(&self, source: ExactActorRef) -> Option<ExactActorRef> {
        self.targets
            .iter()
            .find(|(actor, _)| *actor == source)
            .and_then(|(_, target)| *target)
    }
    fn permits_condition(
        &self,
        _: ExactActorRef,
        _: ExactActorRef,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> bool {
        let cell = LogicalCell {
            x: to.x,
            y: to.y,
            z: i32::from(to.floor),
        };
        from.floor == to.floor
            && self
                .cells
                .spell_tiles()
                .lookup(self.cells.scope(), cell)
                .is_ok_and(|tile| !tile.flags().protection_zone)
            && crate::movement::relocate::creature_sight_clear(self.cells, self.doors, from, to)
    }
    fn player_attribute_damage_delta(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        _: u64,
    ) -> Option<i32> {
        self.attributes
            .iter()
            .find(|(a, s, _)| *a == actor && *s == session)
            .map(|(_, _, delta)| *delta)
    }
}

struct Owner<'a, 'o, 'f, 's> {
    admission: &'o ComposedFreshAdmission<'a, 'f, 's>,
    binding: SoulpitActivityBinding,
    members: &'o [SoulpitMembership],
    now: u64,
    visible: Vec<VisibleSurvivor>,
}

struct VisibleSurvivor {
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    observed: CurrentVisibleActorObservation,
}

fn durable(error: crate::durability::soulpit::SoulpitWriteError) -> SoulpitRuntimeError {
    match error {
        crate::durability::soulpit::SoulpitWriteError::Unavailable(_)
        | crate::durability::soulpit::SoulpitWriteError::StaleState => {
            SoulpitRuntimeError::AdapterUnavailable
        }
        error => SoulpitRuntimeError::Durable(error),
    }
}

fn alive_in_arena(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    let in_arena = runtime.read_actor_position(actor).is_ok_and(|snapshot| {
        let at = snapshot.position();
        let position = u16::try_from(at.x).ok().zip(u16::try_from(at.y).ok()).zip(
            at.floor
                .checked_neg()
                .and_then(|floor| i8::try_from(floor).ok()),
        );
        position.is_some_and(|((x, y), floor)| {
            crate::domain::soulpit::is_arena_position(crate::domain::map_position::TilePos {
                x,
                y,
                floor,
            })
        })
    });
    in_arena
        && !states.is_dead(actor)
        && states
            .get(runtime, actor, session)
            .is_some_and(|state| state.vitals().health > 0)
}

impl Owner<'_, '_, '_, '_> {
    async fn refresh_survivors(&mut self) -> Result<(), SoulpitRuntimeError> {
        let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            self.admission.root.clone(),
        );
        let mut visible = Vec::with_capacity(self.members.len());
        for member in self.members {
            let (current, now) = store
                .current_session_at(member.session)
                .await
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            let lease = current.current_character_lease();
            if current.current_game_session_id() != member.session
                || lease.character_id().as_bytes() != member.character.as_bytes()
            {
                return Err(SoulpitRuntimeError::InvalidScope);
            }
            if current.session_state() == GameSessionState::Terminal {
                continue;
            }
            if current.current_runtime_scope() != self.binding.scope
                || current.current_scope_generation() != self.binding.generation
            {
                return Err(SoulpitRuntimeError::InvalidScope);
            }
            let runtime = self
                .admission
                .runtime
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            let states = self
                .admission
                .states
                .spell_states
                .try_lock()
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
            runtime.owner_fence()?;
            if runtime.binding().runtime_scope() != self.binding.scope
                || runtime.binding().scope_generation() != self.binding.generation
            {
                return Err(SoulpitRuntimeError::InvalidScope);
            }
            // Durable Entered may precede installation of its successor. Absence
            // is uncertain until that canonical session actually becomes Terminal.
            let actor = runtime
                .positioned_melee_census()?
                .into_iter()
                .find_map(|(actor, _, session)| (session == Some(member.session)).then_some(actor))
                .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            let control = runtime.player_control_facts(actor, member.session)?;
            if runtime.bound_attacker_lease(actor, member.session)? != lease
                || states.get(&runtime, actor, member.session).is_none()
            {
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
            if !alive_in_arena(&runtime, &states, actor, member.session) {
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
                        && current.current_original_grace_deadline() == Some(loss.grace_deadline)
                        && loss.grace_deadline > now
                }
                _ => false,
            };
            if !agrees {
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
            let observed =
                observe_current_visible_actor_for_source_read(&runtime, actor, member.session)
                    .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            visible.push(VisibleSurvivor {
                actor,
                observed,
                fence: CurrentCharacterGameplayFence {
                    character_id: member.character,
                    game_session_id: member.session,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: lease.generation(),
                    runtime_scope: self.binding.scope,
                    scope_ownership_generation: self.binding.generation,
                    // Refreshed under its revision slot before every transaction.
                    expected_character_revision: crate::domain::CharacterRevision::new(1)
                        .map_err(|_| SoulpitRuntimeError::InvalidState)?,
                },
            });
        }
        self.visible = visible;
        Ok(())
    }
}

impl<'a, 'o, 'f, 's> ActivityHandoffOwnerRuntime for Owner<'a, 'o, 'f, 's> {
    fn binding(&self) -> SoulpitActivityBinding {
        self.binding
    }
    fn with_kernel<T>(
        &mut self,
        operation: impl FnOnce(SoulpitKernel<'_>) -> Result<T, SoulpitRuntimeError>,
    ) -> Result<T, SoulpitRuntimeError> {
        let mut runtime = self
            .admission
            .runtime
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let states = self
            .admission
            .states
            .spell_states
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let doors = self
            .admission
            .door
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let native = self
            .admission
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(SoulpitRuntimeError::MissingContent)?;
        let census = runtime.positioned_actor_census()?;
        let blockers = Blockers {
            local: doors.blocking_cells(),
            bundle: match self.admission.movement_cells().index() {
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                    Some(crate::map::floor::BundleDoors(index.map().facts.doors()))
                }
                _ => None,
            },
        };
        let scene = Scene {
            room: self
                .admission
                .rooms()
                .gate_native_combat_cast()
                .ok_or(SoulpitRuntimeError::MissingContent)?,
            cells: self.admission.movement_cells(),
            doors: &blockers,
            targets: census
                .iter()
                .filter(|(_, _, session)| session.is_none())
                .map(|(actor, _, _)| {
                    (
                        *actor,
                        states
                            .monster_melee
                            .creature_ai
                            .state(*actor)
                            .and_then(|ai| ai.target()),
                    )
                })
                .collect(),
            attributes: census
                .iter()
                .filter_map(|(actor, _, session)| {
                    let session = (*session)?;
                    let state = states.get(&runtime, *actor, session)?;
                    Some((
                        *actor,
                        session,
                        state.owned_conditions().attribute_damage_delta_at(self.now),
                    ))
                })
                .collect(),
        };
        let kernel = runtime.borrow_soulpit_kernel()?;
        if kernel.content.server_artifact_digest() != native.source_digest() {
            return Err(SoulpitRuntimeError::InvalidScope);
        }
        operation(SoulpitKernel {
            carrier: kernel.carrier,
            continuity: kernel.continuity,
            content: kernel.content,
            native,
            scene: &scene,
        })
    }
    fn survivors(&self) -> Result<Vec<CurrentCharacterGameplayFence>, SoulpitRuntimeError> {
        let runtime = self
            .admission
            .runtime
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let states = self
            .admission
            .states
            .spell_states
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let mut survivors = Vec::with_capacity(self.visible.len());
        for member in &self.visible {
            if !member.observed.matches(&runtime) {
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
            if alive_in_arena(
                &runtime,
                &states,
                member.actor,
                member.fence.game_session_id,
            ) {
                survivors.push(member.fence);
            }
        }
        Ok(survivors)
    }
    fn read_encounter(&mut self) -> ActivityFuture<'_, SoulpitStoredEncounter> {
        Box::pin(async move {
            self.admission
                .root
                .read_soulpit_encounter(
                    self.admission.character,
                    self.admission.holder,
                    self.binding.handoff_id,
                    self.binding.scope,
                    self.binding.generation,
                )
                .await
                .map_err(durable)?
                .ok_or(SoulpitRuntimeError::InvalidState)
        })
    }
    fn commit_progress(
        &mut self,
        mut request: SoulpitStepRequest,
    ) -> ActivityFuture<'_, SoulpitStepOutcome> {
        Box::pin(async move {
            let _permit = self
                .admission
                .spell_lane_permit()
                .await
                .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
            self.refresh_survivors().await?;
            let current = self.survivors()?;
            if request.survivors.iter().any(|expected| {
                !current.iter().any(|actual| {
                    actual.character_id == expected.character_id
                        && actual.item_fence() == expected.item_fence()
                })
            }) {
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
            request
                .survivors
                .sort_by_key(|member| *member.character_id.as_bytes());
            let mut slots = Vec::new();
            for member in &mut request.survivors {
                let mut slot = self
                    .admission
                    .revision_sequencer
                    .acquire(member.character_id)
                    .await;
                member.expected_character_revision = slot
                    .cursor(self.admission.root, self.admission.character)
                    .await
                    .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
                slots.push(slot);
            }
            let outcome = self
                .admission
                .root
                .advance_soulpit(self.admission.character, self.admission.holder, request)
                .await;
            // The lifecycle can advance a survivor revision when it credits Animus. Resolve
            // the sequencer cursor on its next acquire; no runtime lock crosses SQL.
            for slot in &mut slots {
                slot.observe_endgame_progression();
            }
            drop(slots);
            outcome.map_err(durable)
        })
    }
    fn register_creatures(
        &mut self,
        actors: &[SoulpitWaveActor],
    ) -> Result<(), SoulpitRuntimeError> {
        let runtime = self
            .admission
            .runtime
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let mut states = self
            .admission
            .states
            .spell_states
            .try_lock()
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        let native = self
            .admission
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(SoulpitRuntimeError::MissingContent)?;
        for actor in actors {
            if states
                .monster_melee
                .creature_ai
                .state(actor.actor)
                .is_some()
            {
                continue;
            }
            let policy = runtime.companion_snapshot(actor.actor)?.state.policy;
            let definition = crate::content::ProjectV2DefinitionRef {
                family: crate::content::ProjectV2Family::Creature,
                key: policy.definition_key.clone(),
                revision: policy.definition_revision.clone(),
            };
            states
                .monster_melee
                .creature_ai
                .admit_native_creature(&runtime, native, actor.actor, &definition, self.now)
                .map_err(|_| SoulpitRuntimeError::MissingContent)?;
        }
        Ok(())
    }
    fn settle_boss_rewards(&mut self, reward: SoulpitBossReward) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            self.admission
                .settle_soulpit_boss_rewards(&reward)
                .await
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)
        })
    }
    fn return_participants(
        &mut self,
        _: EncounterPhase,
    ) -> impl std::future::Future<Output = Result<(), SoulpitRuntimeError>> {
        return_members(self.admission.clone(), self.members.to_vec())
    }
}

async fn return_members<'a, 'f, 's>(
    admission: ComposedFreshAdmission<'a, 'f, 's>,
    members: Vec<SoulpitMembership>,
) -> Result<(), SoulpitRuntimeError> {
    for member in members {
        let Some(admitted) = admission.activity_runtimes.connection(member.session) else {
            continue;
        };
        let result = admission
            .return_due_activity(admitted, ActivityReturnReason::SoulpitTerminal)
            .await
            .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?;
        match result {
            NativeActivityOutcome::Committed(commit) => admission
                .publish_activity_commit(&commit)
                .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)?,
            NativeActivityOutcome::Refused(_) => {
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
        }
    }
    Ok(())
}

pub(super) async fn drive_registered<'a, 'f, 's>(
    admission: &ComposedFreshAdmission<'a, 'f, 's>,
    registered: &RegisteredRuntime<'a>,
    binding: SoulpitActivityBinding,
    members: &[SoulpitMembership],
    now: u64,
    global_tick: bool,
) -> Result<Option<u64>, SoulpitRuntimeError> {
    let view = admission.runtime_view(registered);
    let mut owner = Owner {
        admission: &view,
        binding,
        members,
        now,
        visible: Vec::new(),
    };
    let mut stored = owner.read_encounter().await?;
    if !stored.encounter.snapshot().phase.is_terminal() {
        owner.refresh_survivors().await?;
        let survivors = owner.survivors()?;
        let alive: Vec<_> = members
            .iter()
            .filter(|member| {
                survivors
                    .iter()
                    .any(|fence| fence.character_id == member.character)
            })
            .map(|member| member.slot)
            .collect();
        let prior = stored.encounter.snapshot().surviving_slots;
        let changes = prior
            .iter()
            .filter(|slot| !alive.contains(slot))
            .map(|slot| SoulpitAction::Leave { slot: *slot })
            .chain(
                alive
                    .iter()
                    .filter(|slot| !prior.contains(slot))
                    .map(|slot| SoulpitAction::Rejoin { slot: *slot }),
            )
            .collect::<Vec<_>>();
        for action in changes {
            if stored.encounter.snapshot().phase.is_terminal() {
                break;
            }
            let step = super::runtime::request(binding, &stored, now, action, owner.survivors()?)?;
            let outcome = owner.commit_progress(step).await?;
            stored.revision = outcome.revision;
            stored.encounter = outcome.encounter;
        }
    }
    if global_tick {
        drive_soulpit_activity(&mut owner, now).await
    } else {
        drive_soulpit_deadlines(&mut owner, now).await
    }
}

/// The last return removes the group from the active traversal. Its exact
/// terminal receipt remains the only authority to clean this private runtime.
pub(super) async fn cleanup_terminal<'a, 'f, 's>(
    admission: &ComposedFreshAdmission<'a, 'f, 's>,
    registered: &RegisteredRuntime<'a>,
) -> Result<(), SoulpitRuntimeError> {
    if !registered
        .runtime
        .lock()
        .await
        .activity_retirement_empty()?
    {
        return Ok(());
    }
    let generation = crate::foundation::ScopeOwnershipGeneration::new(registered.generation)
        .map_err(|_| SoulpitRuntimeError::InvalidScope)?;
    let Some(cleanup) = admission
        .root
        .read_soulpit_terminal_cleanup(
            admission.character,
            admission.holder,
            registered.scope,
            generation,
        )
        .await
        .map_err(durable)?
    else {
        return Ok(());
    };
    let view = admission.runtime_view(registered);
    let _permit = view
        .spell_lane_permit()
        .await
        .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
    {
        let mut runtime = registered.runtime.lock().await;
        runtime.owner_fence()?;
        if !cleanup.matches(
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation(),
        ) {
            return Err(SoulpitRuntimeError::InvalidScope);
        }
        if !runtime.activity_retirement_empty()? {
            return Err(SoulpitRuntimeError::AdapterUnavailable);
        }
        let (carrier, continuity, _) = runtime.soulpit_kernel_parts()?;
        carrier.retire_soulpit(continuity, cleanup.handoff_id())?;
    }
    view.reset_soulpit_obelisk()
        .await
        .map_err(|_| SoulpitRuntimeError::AdapterUnavailable)
}

pub(super) async fn leave_registered<'a, 'f, 's>(
    admission: &ComposedFreshAdmission<'a, 'f, 's>,
    registered: &RegisteredRuntime<'a>,
    binding: SoulpitActivityBinding,
    members: &[SoulpitMembership],
    slot: u8,
    now: u64,
) -> Result<(), SoulpitRuntimeError> {
    let view = admission.runtime_view(registered);
    let mut owner = Owner {
        admission: &view,
        binding,
        members,
        now,
        visible: Vec::new(),
    };
    let stored = owner.read_encounter().await?;
    if stored.encounter.snapshot().phase.is_terminal()
        || !stored.encounter.snapshot().surviving_slots.contains(&slot)
    {
        return Ok(());
    }
    owner.refresh_survivors().await?;
    let survivors = owner
        .survivors()?
        .into_iter()
        .filter(|fence| {
            !members
                .iter()
                .any(|member| member.slot == slot && member.character == fence.character_id)
        })
        .collect();
    let step = super::runtime::request(
        binding,
        &stored,
        now,
        SoulpitAction::Leave { slot },
        survivors,
    )?;
    owner.commit_progress(step).await?;
    Ok(())
}

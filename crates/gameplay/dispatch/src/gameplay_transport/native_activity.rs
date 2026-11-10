//! The native half of the shared activity handoff.
//!
//! Durable receipts decide whether identities changed. This owner freezes the
//! actual source actors, reserves actual destination slots, and retains both the
//! physical token and the moved live spell state while COMMIT is uncertain. No
//! runtime lock is held across SQL. Cancellation cannot drop an extracted player.

use super::ComposedFreshAdmission;
use super::activity_runtime::{RegisteredRuntime, ScopeKey};
use super::actor_spell::PreparedActivitySpellStates;
use super::attack::PreparedActivityAttackStates;
use super::connection::{AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity};
use super::kill_reward::{DrainOutcome, KillReleaseMark, ReleaseEnd, Seal};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterReceipt, ActivityEnterRequest,
    ActivityHandoffOwner, ActivityRefusal,
};
use crate::durability::character_last_position::LastVitals;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ActivityTransferMember, ActivityTransferReservation, ActivityTransferredActor, CharacterLease,
    GameSessionId, GameSessionState, MovementLocalPosition, RuntimeScopeRefV1,
};
use std::sync::Arc;

pub(super) async fn refresh_activity_accounts(
    source: &super::fresh_evidence::FreshEvidenceSource,
    root: &crate::durability::DurabilityRoot,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    accounts: &[[u8; 16]],
) -> Result<(), ()> {
    for account in accounts {
        source
            .refresh_account(root, node, &super::canonical_uuid(account))
            .await
            .map_err(|error| {
                super::operator_event(&format!("activity_account_evidence_unavailable {error}"))
            })?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ActivityAttemptKey {
    pub(super) session: GameSessionId,
    pub(super) connection_generation: u64,
    pub(super) command: u64,
}

/// The caller emits the accepted generic scope barrier for each pair, then
/// selects the complete destination owner before its first state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeActivityCommit {
    pub(super) handoff_id: [u8; 16],
    pub(super) deadline_unix_ms: Option<i64>,
    pub(super) connections: Vec<(AdmittedSession, AdmittedSession)>,
    pub(super) entry_effect_starts: Vec<(GameSessionId, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativeActivityOutcome {
    Committed(NativeActivityCommit),
    Refused(ActivityRefusal),
}

pub(super) struct NativeActivityAttempt<'a> {
    source: RegisteredRuntime<'a>,
    destination: RegisteredRuntime<'a>,
    request: ActivityEnterRequest,
    boss_room: bool,
    soulpit: Option<crate::durability::soulpit::SoulpitEntryRequest>,
    boss_pad_wire: Option<[u8; 32]>,
    pub(super) boss_pad_prefix:
        Option<crate::durability::activity_handoff::boss_room::QualifiedBossPadPrefix>,
    sources: Vec<AdmittedSession>,
    physical: ActivityTransferReservation,
    spells: PreparedActivitySpellStates,
    attacks: PreparedActivityAttackStates,
    kill_marks: Vec<KillReleaseMark>,
    receipt: Option<ActivityEnterReceipt>,
    moved: Option<Vec<ActivityTransferredActor>>,
    entry_effect_starts: Option<Vec<(GameSessionId, u64)>>,
    states_installed: bool,
    outcome: Option<NativeActivityOutcome>,
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn prune_refused_activity_attempts(
        &self,
        policy: super::activity_runtime::ActivityRefusalPrune,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .iter()
            .filter(|(key, _)| {
                policy.includes(key.session, key.connection_generation, Some(key.command))
            })
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        for (key, pending) in entries {
            let held = pending.lock().await;
            if held.receipt.is_some()
                || !matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)))
            {
                continue;
            }
            let mut entries = self.activity_runtimes.attempts.lock().map_err(|_| ())?;
            if entries
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &pending))
            {
                entries.remove(&key);
            }
        }
        self.prune_refused_activity_returns(policy).await
    }

    pub(super) async fn prune_completed_activity_entries(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, attempt)| (*key, Arc::clone(attempt)))
            .collect();
        for (key, pending) in entries {
            let attempt = pending.lock().await;
            if attempt.destination.scope != scope || attempt.destination.generation != generation {
                continue;
            }
            if attempt.outcome.is_none() {
                return Err(());
            }
            self.activity_runtimes
                .attempts
                .lock()
                .map_err(|_| ())?
                .remove(&key);
        }
        Ok(())
    }

    /// An ended socket can be cancelled after SQL commits and before the wire
    /// barrier is published. Resolve its retained physical attempt before the
    /// ordinary control-loss owner decides which current session actually ended.
    pub(super) async fn settle_native_activity_connection(
        &self,
        admitted: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let admitted = self.activity_runtimes.current_connection(admitted);
        let attempts: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            let belongs = pending.lock().await.sources.iter().any(|source| {
                source.game_session_id == admitted.game_session_id
                    && source.controller == admitted.controller
                    && source.item_fence == admitted.item_fence
            });
            if !belongs {
                continue;
            }
            if let NativeActivityOutcome::Committed(commit) =
                self.finish_native_activity(pending).await?
            {
                self.publish_activity_commit(&commit)?;
                return Ok(self.activity_runtimes.current_connection(admitted));
            }
            // A prior definite refusal may belong to this same connection.
            // It cannot hide a later retained attempt whose commit is unknown;
            // HashMap iteration order is not a semantic ordering of commands.
        }
        Ok(admitted)
    }

    /// The existing autonomous owner calls this even when the initiating socket
    /// has gone. Only retained attempts are retried; no fresh identities or new
    /// semantic decisions are issued by the retry pass.
    pub(super) async fn drain_native_activity_attempts(&self) -> Result<(), ()> {
        let attempts: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        let mut available = true;
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            match self.finish_native_activity(pending).await {
                Ok(NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit)?
                }
                Ok(NativeActivityOutcome::Refused(_)) => {}
                Err(()) => available = false,
            }
        }
        if available { Ok(()) } else { Err(()) }
    }
    /// Publish only a fully installed durable successor. The connection retains
    /// its source sequence until the generic snapshot barrier has been written.
    pub(super) fn publish_activity_commit(&self, commit: &NativeActivityCommit) -> Result<(), ()> {
        use oteryn_protocol_oteryn::activity_scope::{
            ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
        };
        let mut switches = Vec::new();
        switches
            .try_reserve_exact(commit.connections.len())
            .map_err(|_| ())?;
        for &(source, destination) in &commit.connections {
            if destination.controller.is_none() {
                self.activity_runtimes
                    .remember_offline_return(source, destination)?;
                continue;
            }
            let fence = destination.item_fence.ok_or(())?;
            let scope = match fence.runtime_scope {
                RuntimeScopeRefV1::Channel {
                    world_id,
                    channel_id,
                } => ActivityScope::Channel {
                    world: *world_id.as_bytes(),
                    channel: *channel_id.as_bytes(),
                },
                RuntimeScopeRefV1::Instance {
                    world_id,
                    instance_id,
                } => ActivityScope::Instance {
                    world: *world_id.as_bytes(),
                    instance: instance_id,
                },
            };
            let payload = encode_activity_scope(&ActivityScopeSnapshot {
                requires_ack: [source, destination]
                    .iter()
                    .filter_map(|session| session.item_fence)
                    .filter_map(|fence| {
                        self.activity_runtimes
                            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
                    })
                    .any(|owner| {
                        owner.template.as_ref().is_some_and(|template| {
                            template.key == crate::content::soulpit_activity::SOULPIT_TEMPLATE_KEY
                        })
                    }),
                handoff: commit.handoff_id,
                source_session: *source.game_session_id.as_bytes(),
                session: *destination.game_session_id.as_bytes(),
                scope,
                scope_generation: fence.scope_ownership_generation.get(),
                connection_generation: destination.continuity.connection_generation,
                lease_generation: fence.character_lease_generation,
                selection: destination
                    .continuity
                    .selected_capabilities
                    .as_slice()
                    .to_vec(),
            })
            .map_err(|_| ())?;
            switches.push(super::activity_runtime::ActivityConnectionSwitch {
                source,
                destination,
                payload,
                movement_effect_start: commit
                    .entry_effect_starts
                    .iter()
                    .find(|(session, _)| *session == destination.game_session_id)
                    .map(|(_, cursor)| *cursor),
            });
        }
        for switch in switches {
            self.activity_runtimes.stage_switch(switch)?;
        }
        Ok(())
    }
    /// Retry an already retained command before issuing any new server identity.
    /// The command's source connection and content selector remain bound.
    pub(super) async fn retry_native_activity(
        &self,
        admitted: AdmittedSession,
        command: u64,
        activity_key: &str,
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        let key = ActivityAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
        };
        let pending = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        let Some(pending) = pending else {
            return Ok(None);
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        let attempt = pending.lock().await;
        if attempt.request.activity_key != activity_key
            || !attempt.sources.iter().any(|source| {
                source.game_session_id == admitted.game_session_id
                    && source.controller == admitted.controller
                    && source.item_fence == admitted.item_fence
            })
        {
            return Ok(Some(NativeActivityOutcome::Refused(
                ActivityRefusal::ReplayConflict,
            )));
        }
        drop(attempt);
        self.finish_native_activity(pending).await.map(Some)
    }

    pub(super) async fn retry_native_soulpit_activity(
        &self,
        admitted: AdmittedSession,
        command: u64,
        intent: &[u8],
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        let key = ActivityAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
        };
        let pending = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = pending {
            let attempt = pending.lock().await;
            if attempt
                .soulpit
                .as_ref()
                .is_none_or(|core| core.intent != intent)
            {
                return Ok(Some(NativeActivityOutcome::Refused(
                    ActivityRefusal::ReplayConflict,
                )));
            }
        }
        self.retry_native_activity(
            admitted,
            command,
            crate::durability::soulpit::SOULPIT_ACTIVITY_KEY,
        )
        .await
    }

    /// Boss wire payload remains bound to its original source command when
    /// the two literal onUseExtra callbacks are evaluated under native owners.
    pub(super) async fn enter_native_boss_activity(
        &self,
        request: ActivityEnterRequest,
        wire: [u8; 32],
    ) -> Result<NativeActivityOutcome, ()> {
        self.enter_native_activity_inner(request, true, None, Some(wire))
            .await
    }

    pub(super) async fn enter_native_soulpit(
        &self,
        request: ActivityEnterRequest,
        entry: crate::durability::soulpit::SoulpitEntryRequest,
    ) -> Result<NativeActivityOutcome, ()> {
        self.enter_native_activity_inner(request, false, Some(entry), None)
            .await
    }
    pub(super) async fn retry_native_soulpit(
        &self,
        admitted: AdmittedSession,
        command: u64,
        intent: &[u8],
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        self.retry_native_soulpit_activity(admitted, command, intent)
            .await
    }

    async fn enter_native_activity_inner(
        &self,
        mut request: ActivityEnterRequest,
        boss_room: bool,
        soulpit: Option<crate::durability::soulpit::SoulpitEntryRequest>,
        boss_pad_wire: Option<[u8; 32]>,
    ) -> Result<NativeActivityOutcome, ()> {
        let refuse = |reason| Ok(NativeActivityOutcome::Refused(reason));
        if request.command_id == 0
            || request.participants.is_empty()
            || request.participants.len() > 15
        {
            return refuse(ActivityRefusal::InvalidInput);
        }
        request
            .participants
            .sort_by_key(|participant| *participant.source.character_id.as_bytes());
        if request
            .participants
            .windows(2)
            .any(|pair| pair[0].source.character_id == pair[1].source.character_id)
            || request
                .participants
                .iter()
                .enumerate()
                .any(|(index, participant)| {
                    request.participants[..index].iter().any(|earlier| {
                        earlier.source.game_session_id == participant.source.game_session_id
                            || earlier.destination_game_session_id
                                == participant.destination_game_session_id
                    })
                })
        {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(initiator) = request.participants.iter().find(|participant| {
            participant.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
        }) else {
            return refuse(ActivityRefusal::InvalidInput);
        };
        let key = ActivityAttemptKey {
            session: initiator.source.game_session_id,
            connection_generation: initiator.source.connection_generation.get(),
            command: request.command_id,
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        let existing = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = existing {
            let attempt = pending.lock().await;
            if attempt.request.activity_key != request.activity_key
                || attempt.boss_room != boss_room
                || attempt.soulpit != soulpit
                || attempt.boss_pad_wire != boss_pad_wire
                || attempt
                    .soulpit
                    .as_ref()
                    .map(|core| (&core.funding, &core.intent))
                    != soulpit.as_ref().map(|core| (&core.funding, &core.intent))
            {
                return refuse(ActivityRefusal::ReplayConflict);
            }
            drop(attempt);
            return self.finish_native_activity(pending).await;
        }
        let first = request.participants[0].source;
        if !matches!(
            (first.runtime_scope, request.destination_scope),
            (
                RuntimeScopeRefV1::Channel { .. },
                RuntimeScopeRefV1::Instance { .. }
            )
        ) || request.participants.iter().any(|participant| {
            participant.source.runtime_scope != first.runtime_scope
                || participant.source.scope_ownership_generation != first.scope_ownership_generation
        }) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(source) = self
            .activity_runtimes
            .get(first.runtime_scope, first.scope_ownership_generation.get())
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let Some(destination) = self.activity_runtimes.get(
            request.destination_scope,
            request.destination_generation.get(),
        ) else {
            return refuse(ActivityRefusal::NoCapacity);
        };
        if destination.assignment_source_revision != request.destination_assignment_source_revision
            || destination
                .template
                .as_ref()
                .is_none_or(|template| template.revision != request.template_revision)
        {
            return refuse(ActivityRefusal::ContentChanged);
        }
        let origin_pin = {
            let runtime = source.runtime.lock().await;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let boss_definition = if boss_room {
            let owner = ActivityHandoffOwner::new(self.root.clone());
            let Some(definition) = owner
                .read_boss_room_with_pin(
                    self.character,
                    self.holder,
                    initiator.source,
                    &request.activity_key,
                    origin_pin,
                )
                .await
                .map_err(|_| ())?
            else {
                return refuse(ActivityRefusal::ContentUnavailable);
            };
            if destination.template.as_ref().is_none_or(|template| {
                template.key != definition.room_template_key
                    || template.revision != definition.template_revision
            }) {
                return refuse(ActivityRefusal::ContentChanged);
            }
            Some(definition)
        } else {
            None
        };
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        let mut members = Vec::new();
        members
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        // Reserve the registry entry before taking any owned state out of a runtime.
        self.activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .try_reserve(1)
            .map_err(|_| ())?;
        #[cfg(test)]
        if soulpit.is_some() {
            eprintln!("SRX3_SOULPIT HANDOFF_STAGE prepared_destination");
        }
        let source_view = self.runtime_view(&source);
        let destination_view = self.runtime_view(&destination);
        // A shared lethal may be queued under another contributor's principal.
        // Drain every dependency before extracting live state or holding revision
        // slots; the frozen check below closes a capture between this and prepare.
        let mut principals = std::collections::BTreeSet::new();
        for participant in &request.participants {
            principals.insert(participant.source.game_session_id);
            principals.extend(
                source
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .proficiency_principals(participant.source.game_session_id),
            );
        }
        for principal in principals {
            if source_view.drain_kill_rewards(principal).await == DrainOutcome::Deferred {
                return Err(());
            }
        }
        // Shared-XP settlement acquires its lane before recipient revisions.
        // Both scope lanes must precede every participant slot here as well.
        let source_first = ScopeKey::new(source.scope, source.generation)
            < ScopeKey::new(destination.scope, destination.generation);
        let (_first_lane, _second_lane) = if source_first {
            (
                source.lane.acquire().await.map_err(|_| ())?,
                destination.lane.acquire().await.map_err(|_| ())?,
            )
        } else {
            (
                destination.lane.acquire().await.map_err(|_| ())?,
                source.lane.acquire().await.map_err(|_| ())?,
            )
        };
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        for participant in &request.participants {
            slots.push(
                self.revision_sequencer
                    .acquire(participant.source.character_id)
                    .await,
            );
        }
        let (mut from, mut to) = if source_first {
            let from = source.runtime.lock().await;
            let to = destination.runtime.lock().await;
            (from, to)
        } else {
            let to = destination.runtime.lock().await;
            let from = source.runtime.lock().await;
            (from, to)
        };
        from.owner_fence().map_err(|_| ())?;
        to.owner_fence().map_err(|_| ())?;
        let actual_pin = ActivityContentPin {
            coordinate_frame_digest: to.content_pin().frame_binding_digest(),
            map_revision_digest: to.map_revision_digest(),
            server_artifact_digest: to.content_pin().server_artifact_digest(),
        };
        if request.destination_content != actual_pin {
            return refuse(ActivityRefusal::ContentChanged);
        }
        if boss_definition
            .as_ref()
            .is_some_and(|definition| !boss_room_native_ready(&to, definition))
        {
            return refuse(ActivityRefusal::ContentUnavailable);
        }
        let doors = destination.door.lock().await;
        let mut spells = source.states.spell_states.lock().await;
        let mut destination_spells = destination.states.spell_states.lock().await;
        for (ordinal, participant) in request.participants.iter_mut().enumerate() {
            let Some(admitted) = self
                .activity_runtimes
                .connection(participant.source.game_session_id)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            if admitted.item_fence != Some(participant.source)
                || participant.selected_capabilities.as_slice()
                    != admitted.continuity.selected_capabilities.as_slice()
                || admitted.continuity.connection_generation
                    != participant.source.connection_generation.get()
                || participant
                    .destination_transport
                    .is_none_or(|next| next == controller.transport)
                || sources.iter().any(|old: &AdmittedSession| {
                    old.item_fence
                        .is_some_and(|fence| fence.character_id == participant.source.character_id)
                })
                || source
                    .states
                    .player_trades
                    .transitioning(admitted.game_session_id)
                || source.states.newhaven_travel.held(admitted.game_session_id)
                || source
                    .states
                    .dawnport_departures
                    .held(admitted.game_session_id)
                || source
                    .states
                    .dawnport_trials
                    .lock()
                    .map_err(|_| ())?
                    .contains(&admitted.game_session_id)
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            let lease = from
                .bound_attacker_lease(actor, admitted.game_session_id)
                .map_err(|_| ())?;
            if lease.character_id().as_bytes() != participant.source.character_id.as_bytes()
                || lease.generation() != participant.source.character_lease_generation
                || from
                    .player_control_facts(actor, admitted.game_session_id)
                    .map_err(|_| ())?
                    .control_loss
                    .is_some()
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            let current = from.read_actor_position(actor).map_err(|_| ())?.position();
            let mut saved = source_view.placed_record(&from, current);
            if saved.record.cell != participant.source_position.cell
                || saved.record.coordinate_frame_digest
                    != participant.source_position.coordinate_frame_digest
                || saved.record.map_revision_digest
                    != participant.source_position.map_revision_digest
                || saved.record.server_artifact_digest
                    != participant.source_position.server_artifact_digest
                || participant.exit_position.world_id != source.scope.world_id()
                || participant.exit_position.coordinate_frame_digest
                    != saved.record.coordinate_frame_digest
                || participant.exit_position.map_revision_digest != saved.record.map_revision_digest
                || participant.exit_position.server_artifact_digest
                    != saved.record.server_artifact_digest
            {
                return refuse(ActivityRefusal::ContentChanged);
            }
            let Some((_, _, state)) = spells
                .actors
                .iter()
                .find(|(held, session, _)| *held == actor && *session == admitted.game_session_id)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            // Handoffs require an explicit current snapshot even at full pools.
            // saved_vitals() uses CHAR-VITALS-1's compact NULL-at-maxima encoding,
            // which is valid for a future login but not an activity receipt.
            let vitals = state.vitals();
            saved.record.vitals = LastVitals::new(vitals.health, vitals.mana);
            saved.record.rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
            participant.source_position = saved.record;
            participant.source_position_write_sequence = saved.write_sequence;
            participant.exit_position.vitals = saved.record.vitals;
            participant.exit_position.rest = saved.record.rest;
            let cell = MovementLocalPosition {
                x: participant.destination_cell.x,
                y: participant.destination_cell.y,
                floor: participant.destination_cell.floor,
            };
            if !super::respawn_cell_admissible(
                destination_view.movement_cells().index(),
                destination_view.movement_cells().scope(),
                destination.scope.world_id(),
                actual_pin.server_artifact_digest,
                doors.blocking_cells(),
                cell,
            ) {
                return refuse(ActivityRefusal::Position);
            }
            let next_lease = CharacterLease::new(
                lease.character_id(),
                lease.generation().checked_add(1).ok_or(())?,
            )
            .map_err(|_| ())?;
            members.push(ActivityTransferMember {
                source_actor: actor,
                source_session: admitted.game_session_id,
                source_lease: lease,
                destination_session: participant.destination_game_session_id,
                destination_lease: next_lease,
                destination_cell: cell,
            });
            sources.push(admitted);
            debug_assert_eq!(sources.len(), ordinal + 1);
        }
        // Native membership and both owner lanes are frozen before source
        // onUseExtra. Reconstruct authored pad order inside the checked reader;
        // Character sorting above remains solely the durable lock order.
        let boss_pad_prefix =
            if let (Some(wire), Some(definition)) = (boss_pad_wire, boss_definition.as_ref()) {
                match source_view
                    .qualify_source_pad_prefix(&from, &to, definition, &request, wire)
                    .await?
                {
                    Ok(Some(prefix)) => {
                        if let Some(reason) = source_view
                            .apply_source_pad_prefix(
                                &mut from,
                                &mut spells,
                                &to,
                                &mut destination_spells,
                                &prefix,
                            )
                            .await?
                        {
                            return refuse(reason);
                        }
                        Some(prefix)
                    }
                    Ok(None) => None,
                    Err(reason) => return refuse(reason),
                }
            } else {
                None
            };
        // Acquire the leaf registry guard before extracting owned state; no
        // fallible registry acquisition or await remains before retention.
        let (mut attack, mut destination_attack) = if source_first {
            let attack = source.states.attack.lock().await;
            let destination_attack = destination.states.attack.lock().await;
            (attack, destination_attack)
        } else {
            let destination_attack = destination.states.attack.lock().await;
            let attack = source.states.attack.lock().await;
            (attack, destination_attack)
        };
        let pending = {
            let mut attempts = self.activity_runtimes.attempts.lock().map_err(|_| ())?;
            attempts.try_reserve(1).map_err(|_| ())?;
            let mut kill_marks = seal_activity_kills(attack.kills(), &members)?;
            let mut held = match spells.prepare_activity_states(&mut destination_spells, &members) {
                Ok(value) => value,
                Err(()) => {
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
            };
            let mut held_attacks =
                match attack.prepare_activity_attacks(&from, &mut destination_attack, &members) {
                    Ok(value) => value,
                    Err(()) => {
                        spells.abort_activity_states(&mut held);
                        finish_activity_kills(
                            attack.kills(),
                            &mut kill_marks,
                            ReleaseEnd::Retryable,
                        );
                        return refuse(ActivityRefusal::AuthorityRejected);
                    }
                };
            let prepared = if boss_room {
                from.prepare_activity_group_entry(&mut to, &members)
            } else {
                from.prepare_activity_transfer(&mut to, &members)
            };
            let physical = match prepared {
                Ok(value) => value,
                Err(_) => {
                    spells.abort_activity_states(&mut held);
                    attack.abort_activity_attacks(&mut held_attacks);
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::NoCapacity);
                }
            };
            let pending = Arc::new(tokio::sync::Mutex::new(NativeActivityAttempt {
                source: source.clone(),
                destination: destination.clone(),
                request,
                boss_room,
                soulpit,
                boss_pad_wire,
                boss_pad_prefix,
                sources,
                physical,
                spells: held,
                attacks: held_attacks,
                kill_marks,
                receipt: None,
                moved: None,
                entry_effect_starts: None,
                states_installed: false,
                outcome: None,
            }));
            // No await occurs between extraction and retention. A cancelled durable
            // future only releases the attempt mutex; its unique tokens remain owned.
            attempts.insert(key, Arc::clone(&pending));
            pending
        };
        drop(destination_attack);
        drop(attack);
        drop(destination_spells);
        drop(spells);
        drop(doors);
        drop(to);
        drop(from);
        drop(slots);
        drop(_second_lane);
        drop(_first_lane);
        #[cfg(test)]
        if pending.lock().await.soulpit.is_some() {
            eprintln!("SRX3_SOULPIT HANDOFF_STAGE retained_attempt");
        }
        self.finish_native_activity(pending).await
    }

    async fn finish_native_activity(
        &self,
        pending: Arc<tokio::sync::Mutex<NativeActivityAttempt<'a>>>,
    ) -> Result<NativeActivityOutcome, ()> {
        let mut attempt = pending.lock().await;
        if let Some(outcome) = &attempt.outcome {
            return Ok(outcome.clone());
        }
        let source = attempt.source.clone();
        let destination = attempt.destination.clone();
        let source_view = self.runtime_view(&source);
        let destination_view = self.runtime_view(&destination);
        // Shared-XP settlement acquires its lane before recipient revisions.
        // Both scope lanes must precede every participant slot here as well.
        let source_first = ScopeKey::new(source.scope, source.generation)
            < ScopeKey::new(destination.scope, destination.generation);
        let (_first_lane, _second_lane) = if source_first {
            (
                source.lane.acquire().await.map_err(|_| ())?,
                destination.lane.acquire().await.map_err(|_| ())?,
            )
        } else {
            (
                destination.lane.acquire().await.map_err(|_| ())?,
                source.lane.acquire().await.map_err(|_| ())?,
            )
        };
        let mut slots = Vec::new();
        for participant in &attempt.request.participants {
            slots.push(
                self.revision_sequencer
                    .acquire(participant.source.character_id)
                    .await,
            );
        }
        if attempt.receipt.is_none() {
            // Refresh each independent account and finish its publication cleanup
            // before requesting another: the publication lane is exclusive.
            // Clearing checkpoints retains accepted S2 facts; the group writer
            // still locks and verifies every participant's current guards.
            let mut accounts: Vec<_> = attempt
                .sources
                .iter()
                .map(|source| {
                    source
                        .controller
                        .map(|controller| controller.account_id)
                        .ok_or(())
                })
                .collect::<Result<_, _>>()?;
            accounts.sort_unstable();
            accounts.dedup();
            refresh_activity_accounts(self.evidence, self.root, self.holder, &accounts).await?;
            #[cfg(test)]
            if attempt.soulpit.is_some() {
                eprintln!("SRX3_SOULPIT HANDOFF_STAGE evidence_held_durable_enter");
            }
            let owner = ActivityHandoffOwner::new(self.root.clone());
            let result = if let Some(entry) = &attempt.soulpit {
                let native = self
                    .active_generation
                    .and_then(crate::content::ActiveGeneration::native_gameplay)
                    .ok_or(())?;
                let source_permit = if source_first {
                    &_first_lane
                } else {
                    &_second_lane
                };
                owner
                    .enter_soulpit(
                        self.character,
                        self.holder,
                        source_permit,
                        attempt.request.clone(),
                        entry,
                        native,
                    )
                    .await
            } else if attempt.boss_room {
                owner
                    .enter_boss_room_with_premium(
                        self.character,
                        self.holder,
                        attempt.request.clone(),
                        &self.premium_coordinator,
                    )
                    .await
            } else {
                owner
                    .enter(self.character, self.holder, attempt.request.clone())
                    .await
            };
            #[cfg(test)]
            if attempt.soulpit.is_some() {
                eprintln!("SRX3_SOULPIT HANDOFF_DURABLE_RESULT {result:?}");
            }
            let result = result.map_err(|_| ())?;
            match result {
                ActivityEnterOutcome::Entered(receipt)
                | ActivityEnterOutcome::Replayed(receipt) => {
                    if !receipt_matches(&attempt.request, &receipt) {
                        return Err(());
                    }
                    attempt.receipt = Some(receipt);
                }
                ActivityEnterOutcome::Refused(reason) => {
                    let (mut from, mut to) = if source_first {
                        let from = source.runtime.lock().await;
                        let to = destination.runtime.lock().await;
                        (from, to)
                    } else {
                        let to = destination.runtime.lock().await;
                        let from = source.runtime.lock().await;
                        (from, to)
                    };
                    let mut spells = source.states.spell_states.lock().await;
                    let mut attack = source.states.attack.lock().await;
                    from.abort_activity_transfer(&mut to, &mut attempt.physical)
                        .map_err(|_| ())?;
                    spells.abort_activity_states(&mut attempt.spells);
                    attack.abort_activity_attacks(&mut attempt.attacks);
                    finish_activity_kills(
                        attack.kills(),
                        &mut attempt.kill_marks,
                        ReleaseEnd::Retryable,
                    );
                    let result = NativeActivityOutcome::Refused(reason);
                    attempt.outcome = Some(result.clone());
                    return Ok(result);
                }
            }
        }
        #[cfg(test)]
        if attempt.soulpit.is_some() && srx3_faults::take_after_soulpit_commit() {
            return Err(());
        }
        if attempt
            .boss_pad_prefix
            .as_ref()
            .is_some_and(|prefix| prefix.refusal().is_some())
        {
            return Err(());
        }
        if !attempt.states_installed {
            let (mut from, mut to) = if source_first {
                let from = source.runtime.lock().await;
                let to = destination.runtime.lock().await;
                (from, to)
            } else {
                let to = destination.runtime.lock().await;
                let from = source.runtime.lock().await;
                (from, to)
            };
            let mut spells = destination.states.spell_states.lock().await;
            let (attack, mut destination_attack) = if source_first {
                let attack = source.states.attack.lock().await;
                let destination_attack = destination.states.attack.lock().await;
                (attack, destination_attack)
            } else {
                let destination_attack = destination.states.attack.lock().await;
                let attack = source.states.attack.lock().await;
                (attack, destination_attack)
            };
            // BossLever creates its source roster only after every pad has
            // qualified and the durable handoff is known. Initialize before
            // publishing the first destination Player; the callback owner
            // retains the actual handoff and each completed birth on retries.
            if let Some(prefix) = attempt.boss_pad_prefix.as_ref()
                && prefix.pads().first().is_some_and(|pad| {
                    pad.kind() == crate::durability::activity_handoff::boss_room::SourceBossLeverExtra::Outburst
                })
            {
                let active = self.active_generation.ok_or(())?;
                let native = active.native_gameplay().ok_or(())?;
                let room = destination_view.effective_rooms().gate_monster_ai().ok_or(())?;
                let callbacks = spells.native_callbacks.clone();
                callbacks.initialize_outburst_accepted_entry(
                    &mut to, active, room, native,
                    &mut spells.monster_melee.creature_ai, prefix,
                    attempt.request.handoff_id, destination_view.owner_now().get(),
                )?;
            }
            if attempt.moved.is_none() {
                let moved = from
                    .commit_activity_transfer(&mut to, &mut attempt.physical)
                    .map_err(|_| ())?;
                attempt.moved = Some(moved);
            }
            let moved = attempt.moved.as_ref().ok_or(())?.clone();
            // No await exposes a destination actor before its existing vitals
            // and cooldowns are installed under the same native owner guard.
            let NativeActivityAttempt {
                spells: held_spells,
                attacks: held_attacks,
                ..
            } = &mut *attempt;
            let attack_commit =
                destination_attack.prepare_activity_attack_commit(held_attacks, &moved)?;
            spells.commit_activity_states(held_spells, &moved)?;
            attack_commit.commit();
            attempt.states_installed = true;
            finish_activity_kills(
                attack.kills(),
                &mut attempt.kill_marks,
                ReleaseEnd::Committed,
            );
        }
        let moved = attempt.moved.as_ref().ok_or(())?;
        let mut connections = Vec::new();
        connections.try_reserve_exact(moved.len()).map_err(|_| ())?;
        for transfer in moved {
            let old = attempt
                .sources
                .iter()
                .find(|source| source.game_session_id == transfer.source_session)
                .copied()
                .ok_or(())?;
            let participant = attempt
                .request
                .participants
                .iter()
                .find(|participant| participant.source.game_session_id == transfer.source_session)
                .ok_or(())?;
            let transport = participant.destination_transport.ok_or(())?;
            let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(transfer.destination_session)
                .await
                .map_err(|_| ())?;
            let fence = super::item_fence_of(current).ok_or(())?;
            if current.session_state() != GameSessionState::Active
                || current.current_transport() != Some(transport)
                || fence.runtime_scope != destination.scope
                || fence.scope_ownership_generation.get() != destination.generation
                || fence.character_id.as_bytes() != transfer.character_id.as_bytes()
                || fence.character_lease_generation != transfer.destination_lease.generation()
                || fence.connection_generation.get() != 1
            {
                return Err(());
            }
            let next = AdmittedSession {
                game_session_id: transfer.destination_session,
                runtime_actor: Some(transfer.destination_actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: Some(ControllerBinding {
                    transport,
                    account_id: old.controller.ok_or(())?.account_id,
                }),
                item_fence: Some(fence),
                continuity: activity_continuity(old.continuity),
                ..old
            };
            source_view
                .transfer_activity_session_state(&destination_view, old, next)
                .await?;
            // M18: the transferred quest copy must seed the new session's native
            // death consumer before the activity lanes expose its actor.
            destination_view
                .refresh_grizzly_kill_snapshot(next.game_session_id)
                .await;
            connections.push((old, next));
        }
        if attempt.soulpit.is_some() && destination_view.rooms().gate_wire_position().is_some_and(|room|
            room.pinned_activity_source_room() == Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Soulpit)) {
            destination_view.activate_soulpit_obelisk().await?;
            if attempt.entry_effect_starts.is_none() {
                attempt.entry_effect_starts = Some(
                    destination_view
                        .emit_soulpit_entry_effects(&connections)
                        .await?,
                );
            }
        }
        // onUseExtra belongs to the committed entry, not ready-room birth.
        // All participants are now physically installed. The existing lanes
        // stay held; the data reader must borrow that destination permit.
        let needs_source_entry = {
            let _runtime = destination_view.runtime.lock().await;
            destination_view
                .states
                .spell_states
                .lock()
                .await
                .soul_war_callbacks
                .requires_committed_entry()
        };
        if needs_source_entry {
            let (_, next) = connections.first().ok_or(())?;
            let permit = if source_first {
                &_second_lane
            } else {
                &_first_lane
            };
            let (clock, observed_owner_us) = destination_view
                .current_soul_war_entry_clock_with_permit(
                    permit,
                    next.runtime_actor.ok_or(())?,
                    next.game_session_id,
                    attempt.request.handoff_id,
                )
                .await?
                .ok_or(())?;
            let now_us = destination_view.owner_now().get();
            let elapsed_ms = clock
                .elapsed_ms
                .checked_add(now_us.checked_sub(observed_owner_us).ok_or(())? / 1_000)
                .ok_or(())?;
            let mut runtime = destination_view.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            let mut spells = destination_view.states.spell_states.lock().await;
            // A retained physical owner claims this once, including retries
            // after the durable handoff and later participants in the roster.
            spells
                .soul_war_callbacks
                .committed_entry(&mut runtime, now_us, elapsed_ms)?;
        }
        let result = NativeActivityOutcome::Committed(NativeActivityCommit {
            handoff_id: attempt.request.handoff_id,
            deadline_unix_ms: attempt
                .receipt
                .as_ref()
                .map(|receipt| receipt.deadline_unix_ms),
            connections,
            entry_effect_starts: attempt.entry_effect_starts.clone().unwrap_or_default(),
        });
        attempt.outcome = Some(result.clone());
        Ok(result)
    }
}

/// Called with runtime, live state and attack guards held, after pre-draining all
/// principals. A queued or in-flight shared reward makes preparation retryable.
pub(super) fn seal_activity_kills(
    queue: &super::kill_reward::KillSettlementQueue,
    members: &[ActivityTransferMember],
) -> Result<Vec<KillReleaseMark>, ()> {
    if members.iter().any(|member| {
        !queue
            .proficiency_principals(member.source_session)
            .is_empty()
    }) {
        return Err(());
    }
    let mut marks = Vec::new();
    marks.try_reserve_exact(members.len()).map_err(|_| ())?;
    for member in members {
        match queue.seal((member.source_session, member.source_lease.generation())) {
            Seal::Sealed(mark) if queue.commit_point(mark) => marks.push(mark),
            _ => {
                finish_activity_kills(queue, &mut marks, ReleaseEnd::Retryable);
                return Err(());
            }
        }
    }
    Ok(marks)
}

pub(super) fn finish_activity_kills(
    queue: &super::kill_reward::KillSettlementQueue,
    marks: &mut Vec<KillReleaseMark>,
    end: ReleaseEnd,
) {
    for mark in marks.drain(..) {
        mark.settle(queue, end);
        if end == ReleaseEnd::Committed {
            mark.forget(queue);
        }
    }
}

pub(super) fn boss_room_native_ready(
    runtime: &crate::foundation::ChannelRuntimeV1,
    definition: &crate::durability::activity_handoff::boss_room::BossRoomDefinition,
) -> bool {
    let Ok(census) = runtime.positioned_actor_census() else {
        return false;
    };
    if census.iter().any(|(_, _, session)| session.is_some()) {
        return false;
    }
    census.iter().any(|(actor, position, session)| {
        let cell = position.position();
        session.is_none()
            && (cell.x, cell.y, cell.floor)
                == (
                    definition.boss_position.x,
                    definition.boss_position.y,
                    definition.boss_position.floor,
                )
            && runtime.companion_snapshot(*actor).is_ok_and(|snapshot| {
                snapshot.state.policy.definition_key == definition.boss_creature_key
            })
    })
}

fn receipt_matches(request: &ActivityEnterRequest, receipt: &ActivityEnterReceipt) -> bool {
    receipt.handoff_id == request.handoff_id
        && receipt.destination_scope == request.destination_scope
        && receipt.destination_generation == request.destination_generation
        && receipt.participants.len() == request.participants.len()
        && request.participants.iter().all(|participant| {
            receipt.participants.iter().any(|row| {
                row.character_id.as_bytes() == participant.source.character_id.as_bytes()
                    && row.source_game_session_id == participant.source.game_session_id
                    && row.destination_game_session_id == participant.destination_game_session_id
                    && row.destination_transport == participant.destination_transport
                    && Some(row.destination_lease_generation)
                        == participant.source.character_lease_generation.checked_add(1)
                    && row.destination_cell == participant.destination_cell
                    && row.exit_cell == participant.exit_position.cell
            })
        })
}

pub(super) fn activity_continuity(source: SessionContinuity) -> SessionContinuity {
    SessionContinuity {
        selected_capabilities: source.selected_capabilities,
        achievement_notice_revision: source.achievement_notice_revision.map(|_| 0),
        gameplay_idle: source.gameplay_idle,
        server_message_revision: source.server_message_revision,
        server_motd_sent: source.server_motd_sent,
        server_save_revision: source.server_save_revision,
        server_save_sent: source.server_save_sent,
        ..SessionContinuity::FRESH
    }
}

#[cfg(test)]
pub(in crate::gameplay_transport) mod srx3_faults {
    use std::cell::Cell;
    thread_local! { static AFTER_SOULPIT_COMMIT: Cell<bool> = const { Cell::new(false) }; }
    pub(in crate::gameplay_transport) fn stop_after_soulpit_commit_once() {
        AFTER_SOULPIT_COMMIT.set(true);
    }
    pub(super) fn take_after_soulpit_commit() -> bool {
        AFTER_SOULPIT_COMMIT.replace(false)
    }
}

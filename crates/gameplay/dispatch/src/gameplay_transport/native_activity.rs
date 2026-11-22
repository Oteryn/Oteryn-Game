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

include!("native_activity/replay.rs");

include!("native_activity/prepare.rs");

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
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

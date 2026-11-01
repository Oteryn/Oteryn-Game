//! Native runtime directory for accepted activity handoffs.
//!
//! The directory retains already allocated and activated owners. It never writes
//! grants, assignments or readiness, and never turns a source Channel pin into an
//! activity template. Connection dispatch changes its composed owner as one unit.

use super::{
    ComposedFreshAdmission, channel_states::ChannelSessionStates, connection::AdmittedSession,
};
use crate::content::NativeEntryMovementCells;
use crate::durability::runtime_scope_assignment::{AssignmentState, RuntimeScopeAssignment};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

mod lineage;
use lineage::{
    ActivityConnectionLineage, advance_lineage, current_connection, prune_delivered_lineage,
    same_connection,
};

/// Existing borrowed boot owners and independently owned activity runtimes use
/// the same lock API. Cloning the handle never clones a playable actor payload.
pub enum SharedOwner<'a, T> {
    Borrowed(&'a T),
    Owned(Arc<T>),
}
impl<T> Clone for SharedOwner<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(value) => Self::Borrowed(value),
            Self::Owned(value) => Self::Owned(Arc::clone(value)),
        }
    }
}
impl<T> Deref for SharedOwner<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}
impl<'a, T> From<&'a T> for SharedOwner<'a, T> {
    fn from(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}
impl<T> From<Arc<T>> for SharedOwner<'_, T> {
    fn from(value: Arc<T>) -> Self {
        Self::Owned(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ScopeKey {
    tag: u8,
    world: [u8; 16],
    identity: [u8; 16],
    generation: u64,
}
impl ScopeKey {
    pub(super) fn new(scope: RuntimeScopeRefV1, generation: u64) -> Self {
        let (tag, identity) = match scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => (1, *channel_id.as_bytes()),
            RuntimeScopeRefV1::Instance { instance_id, .. } => (2, instance_id),
        };
        Self {
            tag,
            world: *scope.world_id().as_bytes(),
            identity,
            generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTemplateKey {
    pub key: String,
    pub revision: String,
}

/// These are borrowed from the real activation owner for the exact template.
/// Their lifetime is the containing active generation, like the Channel assets.
#[derive(Clone, Copy)]
pub struct ActivityRuntimeAssets<'a> {
    pub movement_cells: &'a NativeEntryMovementCells,
    pub rooms: super::SeamRooms<'a>,
}

/// The proposed runtime could not be registered under the current allocation,
/// node, content and capability proofs. No runtime was published on this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityRuntimeRegistrationError;

impl std::fmt::Display for ActivityRuntimeRegistrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("activity runtime registration was refused")
    }
}

impl std::error::Error for ActivityRuntimeRegistrationError {}

#[derive(Clone)]
pub(super) struct RegisteredRuntime<'a> {
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: u64,
    pub(super) assignment_source_revision: u64,
    pub(super) template: Option<ActivityTemplateKey>,
    pub(super) runtime: SharedOwner<'a, tokio::sync::Mutex<ChannelRuntimeV1>>,
    pub(super) door: SharedOwner<'a, tokio::sync::Mutex<crate::world_runtime::LocalObjectRuntime>>,
    pub(super) states: Arc<ChannelSessionStates>,
    pub(super) required_capabilities:
        Arc<super::world_required_capabilities::BoundRequiredCapabilities>,
    pub(super) lane: crate::durability::spell_owner_commit::SpellLane,
    pub(super) assets: ActivityRuntimeAssets<'a>,
}

/// A committed connection successor. The pending record is retained until its
/// exact source connection crosses the wire barrier; taking it is not authority.
#[derive(Debug, Clone)]
pub struct ActivityConnectionSwitch {
    pub source: AdmittedSession,
    pub destination: AdmittedSession,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct ActivityRuntimeRegistry<'a> {
    templates: Mutex<
        HashMap<
            String,
            &'a crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
        >,
    >,
    runtimes: Mutex<HashMap<ScopeKey, RegisteredRuntime<'a>>>,
    connections: Mutex<HashMap<GameSessionId, AdmittedSession>>,
    pending: Mutex<HashMap<GameSessionId, ActivityConnectionSwitch>>,
    pub(super) attempts: Mutex<
        HashMap<
            super::native_activity::ActivityAttemptKey,
            Arc<tokio::sync::Mutex<super::native_activity::NativeActivityAttempt<'a>>>,
        >,
    >,
    successors: Mutex<HashMap<GameSessionId, ActivityConnectionLineage>>,
    // Only current offline successors retain account identity. Historical
    // transport aliases are not an account directory and may be pruned.
    offline_accounts: Mutex<
        HashMap<
            GameSessionId,
            (
                crate::durability::item_transfer::CurrentCharacterItemFence,
                [u8; 16],
            ),
        >,
    >,
    pub(super) returns: Mutex<
        HashMap<
            super::native_activity_return::ActivityReturnAttemptKey,
            Arc<tokio::sync::Mutex<super::native_activity_return::NativeActivityReturnAttempt<'a>>>,
        >,
    >,
    /// Server recovery proposals survive SQL cancellation/unknown commit.
    pub(super) departed_recoveries:
        Mutex<HashMap<GameSessionId, crate::durability::activity_handoff::ActivityRecoveryRequest>>,
    pub(super) recoveries:
        Mutex<HashMap<Vec<u8>, Arc<super::activity_recovery_admission::NativeActivityRecovery>>>,
    /// Serializes multi-owner handoff preparation, never ordinary gameplay.
    pub(super) transitions: tokio::sync::Mutex<()>,
    pub(super) changed: tokio::sync::Notify,
}

/// Only a later accepted command or an already terminal session can discard a
/// definite refusal. Its native abort has completed and it owns no remaining
/// physical, spell or kill-release token. Unknown and committed attempts are
/// never selected by this policy.
#[derive(Clone, Copy)]
pub(super) enum ActivityRefusalPrune {
    BeforeCommand {
        session: GameSessionId,
        connection_generation: u64,
        command: u64,
    },
    Terminal(GameSessionId),
}

impl ActivityRefusalPrune {
    pub(super) fn includes(
        self,
        session: GameSessionId,
        generation: u64,
        command: Option<u64>,
    ) -> bool {
        match self {
            Self::BeforeCommand {
                session: current,
                connection_generation,
                command: next,
            } => {
                session == current
                    && generation <= connection_generation
                    && command.is_some_and(|old| old < next)
            }
            Self::Terminal(current) => session == current,
        }
    }
}

impl<'a> ActivityRuntimeRegistry<'a> {
    pub(super) fn register_template(
        &self,
        template: &'a crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<(), ()> {
        let mut templates = self.templates.lock().map_err(|_| ())?;
        if let Some(existing) = templates.get(template.template_key()) {
            return if std::ptr::eq(*existing, template) {
                Ok(())
            } else {
                Err(())
            };
        }
        templates.insert(template.template_key().into(), template);
        Ok(())
    }
    pub(super) fn template(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<&'a crate::content::bosses::activity_template::QualifiedActivityRoomTemplate> {
        let template = *self.templates.lock().ok()?.get(key)?;
        (template.template_revision() == revision).then_some(template)
    }

    pub(super) fn get(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Option<RegisteredRuntime<'a>> {
        self.runtimes
            .lock()
            .ok()?
            .get(&ScopeKey::new(scope, generation))
            .cloned()
    }

    fn insert(&self, owner: RegisteredRuntime<'a>) -> Result<(), ()> {
        let key = ScopeKey::new(owner.scope, owner.generation);
        let mut owners = self.runtimes.lock().map_err(|_| ())?;
        if let Some(existing) = owners.get(&key) {
            return if std::ptr::eq(&*existing.runtime, &*owner.runtime)
                && Arc::ptr_eq(&existing.states, &owner.states)
                && existing.template == owner.template
            {
                Ok(())
            } else {
                Err(())
            };
        }
        // A new ownership generation needs a separately retired old runtime;
        // registration never silently swaps a live owner under its sessions.
        if owners
            .values()
            .any(|existing| existing.scope == owner.scope)
        {
            return Err(());
        }
        owners.try_reserve(1).map_err(|_| ())?;
        owners.insert(key, owner);
        self.changed.notify_waiters();
        Ok(())
    }

    fn retire_and_remove(
        &self,
        owner: &RegisteredRuntime<'a>,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
    ) -> Result<(), ()> {
        let key = ScopeKey::new(owner.scope, owner.generation);
        let mut owners = self.runtimes.lock().map_err(|_| ())?;
        let Some(current) = owners.get(&key) else {
            return Ok(());
        };
        if current.template.is_none()
            || !std::ptr::eq(&*current.runtime, &*owner.runtime)
            || !runtime.activity_retirement_empty().map_err(|_| ())?
        {
            return Err(());
        }
        // No cancellation point or fallible acquisition separates invalidating
        // the native owner from removing its exact registry entry.
        runtime.retire_owner_cycle();
        owners.remove(&key);
        self.changed.notify_waiters();
        Ok(())
    }

    pub(super) fn activity_owners(&self) -> Result<Vec<RegisteredRuntime<'a>>, ()> {
        let owners = self.runtimes.lock().map_err(|_| ())?;
        Ok(owners
            .values()
            .filter(|owner| owner.template.is_some())
            .cloned()
            .collect())
    }

    pub(super) fn remember(&self, admitted: AdmittedSession) -> Result<(), ()> {
        let (Some(actor), Some(fence), Some(controller)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            admitted.controller,
        ) else {
            return Ok(());
        };
        if actor.runtime_scope() != fence.runtime_scope
            || actor.scope_generation() != fence.scope_ownership_generation
        {
            // House custody retains its separate existing carrier/transport path.
            return Ok(());
        }
        if matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            // A Channel connection without the activity capability cannot join a
            // handoff. Its existing command owner retains its own typed refusals.
            return Ok(());
        }
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return Err(());
        }
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        if let Some(old) = sessions.get(&admitted.game_session_id)
            && (old.continuity.connection_generation > admitted.continuity.connection_generation
                || (old.continuity.connection_generation
                    == admitted.continuity.connection_generation
                    && old
                        .controller
                        .is_some_and(|old| old.transport != controller.transport)))
        {
            return Err(());
        }
        let mut accounts = self.offline_accounts.lock().map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        sessions.insert(admitted.game_session_id, admitted);
        accounts.remove(&admitted.game_session_id);
        Ok(())
    }

    /// This follows only this process's exact controlled connection chain. A
    /// later reconnect has another transport/generation and cannot be selected
    /// for control loss by an obsolete socket's cancellation.
    pub(super) fn current_connection(&self, source: AdmittedSession) -> AdmittedSession {
        let Ok(successors) = self.successors.lock() else {
            return source;
        };
        current_connection(&successors, source)
    }

    /// An already committed offline return has no transport and emits no wire
    /// barrier. Retain its exact predecessor so the old grace lifecycle follows
    /// the new owner without selecting a later reconnect generation.
    pub(super) fn remember_offline_return(
        &self,
        source: AdmittedSession,
        next: AdmittedSession,
    ) -> Result<(), ()> {
        let (Some(old_fence), Some(new_fence), Some(actor)) =
            (source.item_fence, next.item_fence, next.runtime_actor)
        else {
            return Err(());
        };
        if next.controller.is_some()
            || source.game_session_id == next.game_session_id
            || old_fence.character_id != new_fence.character_id
            || old_fence.game_session_id != source.game_session_id
            || new_fence.game_session_id != next.game_session_id
            || old_fence.runtime_scope == new_fence.runtime_scope
            || old_fence.runtime_scope.world_id() != new_fence.runtime_scope.world_id()
            || old_fence.character_lease_generation.checked_add(1)
                != Some(new_fence.character_lease_generation)
            || new_fence.connection_generation.get() != 1
            || next.continuity.connection_generation != 1
            || actor.runtime_scope() != new_fence.runtime_scope
            || actor.scope_generation() != new_fence.scope_ownership_generation
            || source.continuity.selected_capabilities != next.continuity.selected_capabilities
        {
            return Err(());
        }
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let Some(registered) = sessions.get(&source.game_session_id) else {
            // A completed attempt is published repeatedly until room retirement.
            // Only its first publication has a registered source; a later replay
            // must not recreate removed aliases, offline state or playable control.
            return Ok(());
        };
        if !same_connection(*registered, source) {
            return Err(());
        }
        if let Some(lineage) = successors.get(&source.game_session_id) {
            return if same_connection(lineage.source, source) && lineage.destination == next {
                Ok(())
            } else {
                Err(())
            };
        }
        if sessions.contains_key(&next.game_session_id) {
            return Err(());
        }
        let mut accounts = self.offline_accounts.lock().map_err(|_| ())?;
        let account = match source.controller {
            Some(controller) => controller.account_id,
            None => accounts
                .get(&source.game_session_id)
                .filter(|(fence, _)| Some(*fence) == source.item_fence)
                .map(|(_, account)| *account)
                .ok_or(())?,
        };
        successors.try_reserve(1).map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        accounts.try_reserve(1).map_err(|_| ())?;
        advance_lineage(&mut successors, source, next);
        // The existing loss/grace loop may retain this source snapshot after
        // its transport ended. Keep that lifecycle root until its proven end.
        successors.insert(
            source.game_session_id,
            ActivityConnectionLineage {
                source,
                destination: next,
                root: true,
            },
        );
        sessions.remove(&source.game_session_id);
        sessions.insert(next.game_session_id, next);
        accounts.remove(&source.game_session_id);
        accounts.insert(next.game_session_id, (new_fence, account));
        pending.remove(&source.game_session_id);
        prune_delivered_lineage(&mut successors, &pending);
        drop(accounts);
        drop(sessions);
        drop(successors);
        drop(pending);
        self.forget_source_commands(source);
        self.changed.notify_waiters();
        Ok(())
    }

    /// Account identity is retained for the exact current offline successor,
    /// independently of playable control and of historical wire aliases.
    pub(super) fn account_for(&self, admitted: AdmittedSession) -> Option<[u8; 16]> {
        if let Some(controller) = admitted.controller {
            return Some(controller.account_id);
        }
        let accounts = self.offline_accounts.lock().ok()?;
        let (fence, account) = accounts.get(&admitted.game_session_id)?;
        (Some(*fence) == admitted.item_fence).then_some(*account)
    }

    /// Called synchronously after the existing terminal owner removed this exact
    /// actor. A mere disconnect, handoff receipt or failed release never gets here.
    pub(super) fn forget_terminal_connection(&self, session: GameSessionId, actor: ExactActorRef) {
        if let Ok(mut sessions) = self.connections.lock()
            && sessions
                .get(&session)
                .is_some_and(|held| held.runtime_actor == Some(actor))
        {
            sessions.remove(&session);
            if let Ok(mut accounts) = self.offline_accounts.lock() {
                accounts.remove(&session);
            }
        }
        // The listener may still hold an earlier snapshot while reconciling
        // this terminal result. Its completion hook, not actor removal alone,
        // releases the last exact socket/lifecycle alias and pending barrier.
    }

    /// Forget ingress only after the old session's committed handoff. An open
    /// source stream owns its Arc independently and can finish its result; the
    /// directory cannot retain one obsolete stream for every past room visit.
    fn forget_source_commands(&self, source: AdmittedSession) {
        let Some(fence) = source.item_fence else {
            return;
        };
        if let Some(owner) = self.get(fence.runtime_scope, fence.scope_ownership_generation.get()) {
            owner.states.command_ingress.forget(source.game_session_id);
        }
    }

    /// The listener calls this only once no more socket/loss/grace callbacks can
    /// reference this exact chain. Unknown operations and cancelled lifecycles
    /// do not call it. Durable rows, current connections and owner tokens stay.
    pub(super) fn finish_connection_lineage(&self, ended: AdmittedSession) -> Result<(), ()> {
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let target = current_connection(&successors, ended);
        let mut old_sources = Vec::new();
        old_sources.try_reserve(pending.len()).map_err(|_| ())?;
        pending.retain(|_, switch| {
            if same_connection(current_connection(&successors, switch.source), target) {
                if sessions
                    .get(&switch.source.game_session_id)
                    .is_some_and(|held| same_connection(*held, switch.source))
                {
                    sessions.remove(&switch.source.game_session_id);
                }
                old_sources.push(switch.source);
                false
            } else {
                true
            }
        });
        successors.retain(|_, lineage| !same_connection(lineage.destination, target));
        prune_delivered_lineage(&mut successors, &pending);
        drop(sessions);
        drop(successors);
        drop(pending);
        for source in old_sources {
            self.forget_source_commands(source);
        }
        Ok(())
    }

    pub(super) fn connection(&self, session: GameSessionId) -> Option<AdmittedSession> {
        self.connections.lock().ok()?.get(&session).copied()
    }

    pub(super) fn stage_switch(&self, switch: ActivityConnectionSwitch) -> Result<(), ()> {
        let source = switch.source;
        let next = switch.destination;
        let (Some(old_controller), Some(new_controller), Some(old_fence), Some(new_fence)) = (
            source.controller,
            next.controller,
            source.item_fence,
            next.item_fence,
        ) else {
            return Err(());
        };
        if source.game_session_id == next.game_session_id
            || old_controller.account_id != new_controller.account_id
            || old_controller.transport == new_controller.transport
            || old_fence.character_id != new_fence.character_id
            || old_fence.runtime_scope == new_fence.runtime_scope
            || old_fence.character_lease_generation.checked_add(1)
                != Some(new_fence.character_lease_generation)
            || new_fence.game_session_id != next.game_session_id
            || new_fence.connection_generation.get() != next.continuity.connection_generation
            || next.continuity.next_command_id != 1
            || next.continuity.server_sequence != 0
            || next.continuity.selected_capabilities != source.continuity.selected_capabilities
            || next.runtime_actor.is_none_or(|actor| {
                actor.runtime_scope() != new_fence.runtime_scope
                    || actor.scope_generation() != new_fence.scope_ownership_generation
            })
        {
            return Err(());
        }
        let mut pending = self.pending.lock().map_err(|_| ())?;
        if let Some(old) = pending.get(&source.game_session_id) {
            return if old.source == source
                && old.destination == next
                && old.payload == switch.payload
            {
                Ok(())
            } else {
                Err(())
            };
        }
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let Some(registered) = sessions.get(&source.game_session_id) else {
            // A retained committed result can outlive its delivered/pruned
            // source. It is already installed, never a fresh publication grant.
            return Ok(());
        };
        if !same_connection(*registered, source) {
            return Err(());
        }
        if let Some(lineage) = successors.get(&source.game_session_id) {
            return if same_connection(lineage.source, source) && lineage.destination == next {
                Ok(())
            } else {
                Err(())
            };
        }
        if sessions
            .get(&next.game_session_id)
            .is_some_and(|held| !same_connection(*held, next))
        {
            return Err(());
        }
        pending.try_reserve(1).map_err(|_| ())?;
        successors.try_reserve(1).map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        // SQL and physical installation already committed this destination.
        // Keep the original physical socket root, but compress delivered hops
        // instead of retaining two AdmittedSession values for every past visit.
        let continued = advance_lineage(&mut successors, source, next);
        sessions.insert(next.game_session_id, next);
        successors.insert(
            source.game_session_id,
            ActivityConnectionLineage {
                source,
                destination: next,
                root: !continued,
            },
        );
        pending.insert(source.game_session_id, switch);
        prune_delivered_lineage(&mut successors, &pending);
        self.changed.notify_waiters();
        Ok(())
    }

    pub(super) fn pending_switch(
        &self,
        source: AdmittedSession,
    ) -> Result<Option<ActivityConnectionSwitch>, ()> {
        let pending = self.pending.lock().map_err(|_| ())?;
        let Some(switch) = pending.get(&source.game_session_id) else {
            return Ok(None);
        };
        if switch.source.controller != source.controller
            || switch.source.item_fence != source.item_fence
            || switch.source.runtime_actor != source.runtime_actor
            || switch.source.continuity.connection_generation
                != source.continuity.connection_generation
        {
            return Err(());
        }
        Ok(Some(switch.clone()))
    }

    pub(super) fn finish_switch(
        &self,
        source: AdmittedSession,
        destination: AdmittedSession,
    ) -> Result<(), ()> {
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let current = pending.get(&source.game_session_id).ok_or(())?;
        if !same_connection(current.source, source) || current.destination != destination {
            return Err(());
        }
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        // First publication made the actual destination visible. Repeating
        // remember here could downgrade a reconnect or resurrect an actor that
        // already returned/terminated while this earlier barrier was in flight.
        pending.remove(&source.game_session_id);
        if sessions
            .get(&source.game_session_id)
            .is_some_and(|held| same_connection(*held, source))
        {
            sessions.remove(&source.game_session_id);
        }
        prune_delivered_lineage(&mut successors, &pending);
        drop(sessions);
        drop(successors);
        drop(pending);
        self.forget_source_commands(source);
        Ok(())
    }
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    /// The lifecycle calls this only after its fresh durable retirement witness
    /// and Ground Complete. Minimal session lineage survives lost wire receipts;
    /// completed tokens must not retain the retired physical runtime forever.
    pub(super) async fn retire_and_unregister_activity(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let Some(owner) = self.activity_runtimes.get(scope, generation) else {
            return Ok(());
        };
        if owner.template.is_none() {
            return Err(());
        }
        self.prune_completed_activity_entries(scope, generation)
            .await?;
        self.prune_completed_activity_returns(scope, generation)
            .await?;
        let mut runtime = owner.runtime.lock().await;
        self.activity_runtimes
            .retire_and_remove(&owner, &mut runtime)
    }

    pub(super) fn transfer_activity_session_state(
        &self,
        destination: &Self,
        source: AdmittedSession,
        next: AdmittedSession,
    ) -> Result<(), ()> {
        let old = source.game_session_id;
        let new = next.game_session_id;
        let actor = next.runtime_actor.ok_or(())?;
        let character = source.item_fence.ok_or(())?.character_id;
        if next.item_fence.ok_or(())?.character_id != character {
            return Err(());
        }
        move_session_entry(
            &self.states.progression_sessions,
            &destination.states.progression_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.quest_sessions,
            &destination.states.quest_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.wheel_sessions,
            &destination.states.wheel_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.atelier_sessions,
            &destination.states.atelier_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.proficiencies,
            &destination.states.proficiencies,
            old,
            new,
        )?;
        {
            let mut from = self.states.premium_sessions.lock().map_err(|_| ())?;
            let mut to = destination.states.premium_sessions.lock().map_err(|_| ())?;
            from.transfer_activity(&mut to, old, new);
        }
        self.states.last_positions.transfer_activity(
            &destination.states.last_positions,
            old,
            new,
            actor,
        )?;
        if next.controller.is_none() {
            let mut from = self.states.lost.lock().map_err(|_| ())?;
            let mut to = destination.states.lost.lock().map_err(|_| ())?;
            to.try_reserve(1).map_err(|_| ())?;
            from.remove(&old);
            to.insert(new, next);
        }
        // Settlement publishes these effects before releasing its recipient
        // revisions/lane. These same guards remain held across this move, so a
        // committed kill cannot strand a late mark on the retired source owner.
        self.states
            .level_refresh
            .transfer_activity(&destination.states.level_refresh, character)?;
        self.states
            .title_refresh
            .transfer_activity(&destination.states.title_refresh, character)?;
        self.states
            .soul_kills
            .transfer_activity(&destination.states.soul_kills, character)?;
        self.states
            .cyclopedia_tracker_refresh
            .transfer_activity(&destination.states.cyclopedia_tracker_refresh, character)?;
        destination.states.command_ingress.admit(new);
        self.states.appearance_commands.forget(old);
        self.states.chat.forget(old);
        self.states.player_trades.forget(old);
        self.states.inspection_permissions.forget(old);
        self.states.npc_conversations.forget(old);
        self.states.tutorial_sessions.forget(old);
        self.states.newhaven_travel.forget(old);
        Ok(())
    }

    pub(super) async fn register_current_runtime(&self) -> Result<(), ()> {
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        runtime.owner_fence().map_err(|_| ())?;
        self.activity_runtimes.insert(RegisteredRuntime {
            scope: binding.runtime_scope(),
            generation: binding.scope_generation().get(),
            assignment_source_revision: binding.source_revision(),
            template: None,
            runtime: self.runtime.clone(),
            door: self.door.clone(),
            states: Arc::clone(&self.states),
            lane: self.spell_lane.clone(),
            required_capabilities: Arc::clone(&self.required_capabilities),
            assets: ActivityRuntimeAssets {
                movement_cells: self.movement_cells,
                rooms: self.rooms,
            },
        })
    }

    /// Called by the existing activation owner with its consumed native template
    /// parts. Current allocation and Node possession are independently re-read;
    /// registration does not publish readiness or make a historical receipt current.
    pub async fn register_activity_runtime(
        &self,
        template: ActivityTemplateKey,
        assignment: &RuntimeScopeAssignment,
        runtime: ChannelRuntimeV1,
        door: crate::world_runtime::LocalObjectRuntime,
        assets: ActivityRuntimeAssets<'a>,
    ) -> Result<(), ActivityRuntimeRegistrationError> {
        if template.key.is_empty()
            || template.key.len() > 512
            || template.revision.is_empty()
            || template.revision.len() > 128
            || !matches!(assignment.scope, RuntimeScopeRefV1::Instance { .. })
            || assignment.scope.world_id() != self.world_id
            || assignment.state != AssignmentState::Assigned
            || assignment.holder != Some(self.holder.fact())
        {
            return Err(ActivityRuntimeRegistrationError);
        }
        self.root
            .require_current_node_registration(self.holder)
            .await
            .map_err(|_| ActivityRuntimeRegistrationError)?;
        if self
            .root
            .read_runtime_scope_assignment(assignment.scope)
            .await
            .map_err(|_| ActivityRuntimeRegistrationError)?
            .as_ref()
            != Some(assignment)
        {
            return Err(ActivityRuntimeRegistrationError);
        }
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        let active = self
            .active_generation
            .ok_or(ActivityRuntimeRegistrationError)?;
        let room = assets
            .rooms
            .gate_native_combat_cast()
            .ok_or(ActivityRuntimeRegistrationError)?;
        if binding.runtime_scope() != assignment.scope
            || binding.scope_generation().get() != assignment.ownership_generation
            || binding.source_revision() != assignment.source_revision
            || binding.node_id() != self.holder.fact().node_id()
            || binding.node_registration_revision() != self.holder.fact().registration_revision()
            || pin.world_id() != self.world_id
            || pin.activation_sequence() != active.activation_sequence()
            || pin.server_artifact_digest() != active.identity().server_artifact_digest()
            || pin.client_artifact_digest() != active.identity().client_artifact_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || assets.movement_cells.scope().world_id != self.world_id
            || runtime.owner_fence().is_err()
        {
            return Err(ActivityRuntimeRegistrationError);
        }
        let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
            self.root.clone(),
        );
        let guard=guards.load(&[crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(assignment.scope)])
            .await.map_err(|_| ActivityRuntimeRegistrationError)?.into_iter().next().flatten().ok_or(ActivityRuntimeRegistrationError)?;
        let crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1::Runtime {world_policy_revision,..}=guard.state else {return Err(ActivityRuntimeRegistrationError);};
        let required_capabilities = self
            .required_capabilities
            .for_activity_runtime(
                assignment.scope,
                assignment.ownership_generation,
                &world_policy_revision,
            )
            .map_err(|_| ActivityRuntimeRegistrationError)?;
        let states = ChannelSessionStates {
            gameplay_idle_policy: self.states.gameplay_idle_policy,
            // One node shutdown gate covers commands in every scoped owner.
            input_gate: Arc::clone(&self.states.input_gate),
            ..Default::default()
        };
        self.activity_runtimes
            .insert(RegisteredRuntime {
                scope: assignment.scope,
                generation: assignment.ownership_generation,
                assignment_source_revision: assignment.source_revision,
                template: Some(template),
                runtime: Arc::new(tokio::sync::Mutex::new(runtime)).into(),
                door: Arc::new(tokio::sync::Mutex::new(door)).into(),
                states: Arc::new(states),
                lane: crate::durability::spell_owner_commit::SpellLane::for_scope(assignment.scope),
                assets,
                required_capabilities,
            })
            .map_err(|_| ActivityRuntimeRegistrationError)
    }

    pub(super) fn runtime_view(&self, owner: &RegisteredRuntime<'a>) -> Self {
        let mut view = self.clone();
        view.runtime = owner.runtime.clone();
        view.door = owner.door.clone();
        view.states = Arc::clone(&owner.states);
        view.required_capabilities = Arc::clone(&owner.required_capabilities);
        view.spell_lane = owner.lane.clone();
        view.movement_cells = owner.assets.movement_cells;
        view.rooms = owner.assets.rooms;
        if let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = owner.scope
        {
            view.world_id = world_id;
            view.channel_id = channel_id;
        }
        view
    }

    pub(super) async fn resolve_runtime_view(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<Self>, ()> {
        let Some(actor) = admitted.runtime_actor else {
            return Ok(None);
        };
        let Some(fence) = admitted.item_fence else {
            return Ok(None);
        };
        if actor.runtime_scope() != fence.runtime_scope {
            return Ok(None);
        } // Existing House owner.
        let runtime = self.runtime.lock().await;
        // Legacy Channel commands still reach their existing fence validator.
        // This never routes an Instance or a selected activity connection around
        // its exact generation check; the actual Channel and owner stay current.
        let ordinary_channel = matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1);
        if runtime.binding().runtime_scope() == fence.runtime_scope
            && (runtime.binding().scope_generation() == fence.scope_ownership_generation
                || ordinary_channel)
        {
            runtime.owner_fence().map_err(|_| ())?;
            return Ok(None);
        }
        drop(runtime);
        let target = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
            .ok_or(())?;
        let next = self.runtime_view(&target);
        {
            let runtime = next.runtime.lock().await;
            let guard = runtime.owner_fence().map_err(|_| ())?;
            if !guard.is_current_for_scope(fence.runtime_scope, fence.scope_ownership_generation)
                || runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_err()
            {
                return Err(());
            }
        }
        Ok(Some(next))
    }
}

fn move_session_entry<T>(
    source: &Mutex<HashMap<GameSessionId, T>>,
    destination: &Mutex<HashMap<GameSessionId, T>>,
    old: GameSessionId,
    new: GameSessionId,
) -> Result<(), ()> {
    let mut source = source.lock().map_err(|_| ())?;
    let mut destination = destination.lock().map_err(|_| ())?;
    if destination.contains_key(&new) {
        // Retry after another post-commit owner was unavailable; an owned entry
        // already moved is never cloned or overwritten from stale state.
        return if source.contains_key(&old) {
            Err(())
        } else {
            Ok(())
        };
    }
    destination.try_reserve(1).map_err(|_| ())?;
    if let Some(value) = source.remove(&old) {
        destination.insert(new, value);
    }
    Ok(())
}

#[cfg(test)]
mod refusal_retention_tests {
    use super::*;

    fn session(tag: u8) -> Result<GameSessionId, String> {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        GameSessionId::decode(&bytes).map_err(|error| format!("fixture session: {error:?}"))
    }

    #[test]
    fn later_command_preserves_replay_current_future_and_server_occurrences() -> Result<(), String>
    {
        let current = session(1)?;
        let policy = ActivityRefusalPrune::BeforeCommand {
            session: current,
            connection_generation: 2,
            command: 7,
        };
        assert!(policy.includes(current, 2, Some(6)));
        assert!(policy.includes(current, 1, Some(6)));
        assert!(
            !policy.includes(current, 2, Some(7)),
            "current command still needs its retained result"
        );
        assert!(!policy.includes(current, 2, Some(8)));
        assert!(
            !policy.includes(current, 3, Some(6)),
            "obsolete connections cannot prune a successor"
        );
        assert!(
            !policy.includes(current, 2, None),
            "autonomous occurrences have no command high-water"
        );
        assert!(
            !policy.includes(session(2)?, 2, Some(6)),
            "another participant keeps its own cache"
        );
        Ok(())
    }

    #[test]
    fn proven_terminal_session_can_forget_its_own_server_refusals() -> Result<(), String> {
        let current = session(1)?;
        let policy = ActivityRefusalPrune::Terminal(current);
        assert!(policy.includes(current, 1, Some(7)));
        assert!(policy.includes(current, 2, None));
        assert!(!policy.includes(session(2)?, 2, None));
        Ok(())
    }
}

#[cfg(test)]
#[path = "activity_runtime/switch_tests.rs"]
mod switch_tests;

#[cfg(test)]
#[path = "activity_runtime/lineage_tests.rs"]
mod lineage_tests;

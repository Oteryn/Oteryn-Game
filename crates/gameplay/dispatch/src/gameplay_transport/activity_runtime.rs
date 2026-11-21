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
pub(super) mod source_bundle;
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

/// Channel assets borrow the real activation owner. Instance assets own their
/// qualified room and fresh scoped bundle map through handoff and recovery.
#[derive(Clone)]
pub struct ActivityRuntimeAssets<'a> {
    pub movement_cells: SharedOwner<'a, NativeEntryMovementCells>,
    pub rooms: super::SeamRooms<'a>,
}

impl ActivityRuntimeAssets<'_> {
    pub(super) fn movement_cells(&self) -> &NativeEntryMovementCells {
        &self.movement_cells
    }
    pub(super) fn rooms(&self) -> &super::SeamRooms<'_> {
        &self.rooms
    }
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
    pub movement_effect_start: Option<u64>,
}

#[derive(Default)]
pub struct ActivityRuntimeRegistry<'a> {
    pub(super) boss_rooms_after: Mutex<Option<[u8; 16]>>,
    pub(super) departed_after:
        Mutex<Option<crate::durability::activity_handoff::DepartedActivityCursor>>,
    pub(super) departed_retirement_after: Mutex<Option<[u8; 16]>>,
    pub(super) soulpit_after: std::sync::Mutex<Option<[u8; 16]>>,
    pub(super) soulpit_transitions: tokio::sync::Mutex<()>,
    soulpit_template:
        Mutex<Option<&'a crate::content::soulpit_activity::QualifiedSoulpitActivityTemplate>>,
    /// One source-script cache across this generation's derived room copies.
    /// Values are detached helper DATA, never Player payloads or canonical KV/icons.
    rotten_area_caches: Mutex<
        HashMap<
            super::monster_profile_cycle::rotten_area::CacheKey,
            Arc<Mutex<super::monster_profile_cycle::rotten_area::SharedCache>>,
        >,
    >,
    source_rooms: Mutex<HashMap<&'static str, &'a crate::content::QualifiedNativeEntryRoom>>,
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

include!("activity_runtime/registry.rs");

include!("activity_runtime/transfer.rs");

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
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
                movement_cells: self.movement_cells.clone(),
                rooms: self.rooms.clone(),
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
        // A generic template runtime is no qualified source room: its
        // creatures keep main's classic death path.
        let mut runtime = runtime;
        runtime
            .use_classic_creature_deaths()
            .map_err(|_| ActivityRuntimeRegistrationError)?;
        self.register_activity_runtime_with_callbacks(
            template, assignment, runtime, door, assets, None,
        )
        .await
    }

    pub(super) async fn register_activity_runtime_with_callbacks(
        &self,
        template: ActivityTemplateKey,
        assignment: &RuntimeScopeAssignment,
        runtime: ChannelRuntimeV1,
        door: crate::world_runtime::LocalObjectRuntime,
        assets: ActivityRuntimeAssets<'a>,
        source_callbacks: Option<super::soul_war_callbacks::SoulWarCallbacks>,
    ) -> Result<(), ActivityRuntimeRegistrationError> {
        self.register_activity_runtime_with_room(
            template,
            assignment,
            runtime,
            door,
            assets,
            source_callbacks,
            None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the source-qualified room accompanies independently owned runtime registration parts"
    )]
    pub(super) async fn register_source_activity_runtime(
        &self,
        template: ActivityTemplateKey,
        assignment: &RuntimeScopeAssignment,
        runtime: ChannelRuntimeV1,
        door: crate::world_runtime::LocalObjectRuntime,
        assets: ActivityRuntimeAssets<'a>,
        source_callbacks: Option<super::soul_war_callbacks::SoulWarCallbacks>,
        room: Arc<crate::content::QualifiedNativeEntryRoom>,
    ) -> Result<(), ActivityRuntimeRegistrationError> {
        let role = room
            .pinned_activity_source_room()
            .ok_or(ActivityRuntimeRegistrationError)?;
        let pin = runtime.content_pin();
        if template.key != format!("oteryn:room.{}-source", role.name())
            || !matches!(
                room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            )
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.compiled().client_digest() != pin.client_artifact_digest()
        {
            return Err(ActivityRuntimeRegistrationError);
        }
        self.register_activity_runtime_with_room(
            template,
            assignment,
            runtime,
            door,
            assets,
            source_callbacks,
            Some(room),
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the existing independently qualified runtime owners remain one registration"
    )]
    async fn register_activity_runtime_with_room(
        &self,
        template: ActivityTemplateKey,
        assignment: &RuntimeScopeAssignment,
        runtime: ChannelRuntimeV1,
        door: crate::world_runtime::LocalObjectRuntime,
        assets: ActivityRuntimeAssets<'a>,
        source_callbacks: Option<super::soul_war_callbacks::SoulWarCallbacks>,
        source_room: Option<Arc<crate::content::QualifiedNativeEntryRoom>>,
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
            .rooms()
            .gate_native_combat_cast()
            .ok_or(ActivityRuntimeRegistrationError)?;
        if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            assets.movement_cells().index()
        {
            use crate::map::boot::BundleCollisionIndexExt;
            if index.map().overlay.runtime_scope() != assignment.scope {
                return Err(ActivityRuntimeRegistrationError);
            }
        }
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
            || assets.movement_cells().scope().world_id != self.world_id
            || assets.movement_cells() != room.movement_cells()
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
        // The source registration must retain the same immutable room already
        // owned by its seam assets, not a second room authority beside them.
        if source_room
            .as_deref()
            .is_some_and(|source| !std::ptr::eq(source, room))
        {
            return Err(ActivityRuntimeRegistrationError);
        }
        let mut states = self.states.for_activity();
        if let Some(callbacks) = source_callbacks {
            states.spell_states.get_mut().soul_war_callbacks = callbacks;
        }
        if let Some(source) = source_room.as_deref() {
            let spells = states.spell_states.get_mut();
            spells
                .soul_war_callbacks
                .prepare_ambient_entry(active, source)
                .map_err(|_| ActivityRuntimeRegistrationError)?;
            spells
                .spite_global
                .register_source_instance(
                    &runtime,
                    active
                        .native_gameplay()
                        .ok_or(ActivityRuntimeRegistrationError)?,
                    source,
                    self.owner_now().get(),
                )
                .map_err(|_| ActivityRuntimeRegistrationError)?;
        }
        states
            .spell_states
            .lock()
            .await
            .initialize_owner_bookkeeping(&runtime)
            .ok_or(ActivityRuntimeRegistrationError)?;
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
}

include!("activity_runtime/lookup.rs");

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

include!("activity_runtime/refusal_tests.rs");

#[cfg(test)]
#[path = "activity_runtime/switch_tests.rs"]
mod switch_tests;

#[cfg(test)]
#[path = "activity_runtime/lineage_tests.rs"]
mod lineage_tests;

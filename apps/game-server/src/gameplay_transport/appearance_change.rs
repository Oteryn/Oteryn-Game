//! APP-3 command, current appearance projection and ordered visibility publication leaf.
//! CP D1094 authoring; connection/registries/offer/owner call sites remain closed until A2.
//! Prepared commands are immutable expectations, not current authority. Every mutation uses
//! the real APP-1 writer; every observation separately reads its current gameplay fence.
use std::collections::BTreeMap;
use std::sync::Arc;

use super::actor_spell::ChannelSpellStates;
use crate::ability::condition::TemporaryDisplayedAppearance;
use crate::content::native_gameplay::NativeGameplayState;
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceSelection, Sex, displayed_look, validate_selection,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityRoot;
use crate::durability::character_appearance::{
    AppearanceChangeOccurrence, AppearanceChangeOutcome, AppearanceChangeRequest,
    AppearanceValidationContext, AppearanceWriteError, CommittedCharacterAppearance,
    DurableAppearanceSelection,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{
    ChannelRuntimeV1, CommandRef, ConnectionGeneration, ExactActorRef, GameSessionId,
    GameSessionState, RuntimeScopeRefV1, encode_state_delta,
};
use oteryn_protocol_oteryn::appearance as wire;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityFields, EntityKind, EntityLights, EntityRef, WorldSpatialEntitiesDelta,
    WorldSpatialEntitiesSnapshot, encode_world_spatial_entities_delta_with_fields,
    validate_snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Unsupported,
    Stale,
    Unavailable,
    Invalid,
    Exhausted,
    Wire,
}
impl From<wire::Error> for Error {
    fn from(_: wire::Error) -> Self {
        Self::Wire
    }
}
fn selected(capabilities: &[u32]) -> bool {
    capabilities.contains(&wire::CAPABILITY_APPEARANCE_V1) && capabilities.contains(&6)
}

fn selection(value: &wire::Selection) -> AppearanceSelection<String> {
    AppearanceSelection {
        outfit_key: value.outfit.clone(),
        colours: value.colours,
        addons: value.addons,
        mount_key: value.mount.clone(),
    }
}
fn wire_selection(value: &AppearanceSelection<String>) -> wire::Selection {
    wire::Selection {
        outfit: value.outfit_key.clone(),
        colours: value.colours,
        addons: value.addons,
        mount: value.mount_key.clone(),
    }
}
fn wire_selected(value: &DurableAppearanceSelection) -> wire::Selected {
    wire::Selected {
        selection: wire_selection(&value.selection),
        revision: value.committed_character_revision.get(),
        provenance: value.content_revision.clone(),
    }
}
fn stored(value: &wire::Selected) -> Result<DurableAppearanceSelection, Error> {
    Ok(DurableAppearanceSelection {
        selection: selection(&value.selection),
        content_revision: value.provenance.clone(),
        committed_character_revision: CharacterRevision::new(value.revision)
            .map_err(|_| Error::Invalid)?,
    })
}

/// Owned immutable active-artifact data for APP-1's existing 'static content argument.
/// This adapter grants no authority; the writer independently checks its native root context.
struct OwnedContent(Arc<NativeGameplayState>);
impl AppearanceContent<String> for OwnedContent {
    fn palette_size(&self) -> u16 {
        self.0
            .cosmetics()
            .map_or(0, AppearanceContent::palette_size)
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        self.0.cosmetics()?.outfit_addon_count(key)
    }
    fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
        self.0
            .cosmetics()
            .is_some_and(|c| c.outfit_is_starter(key, sex))
    }
    fn addon_is_starter(&self, key: &String, sex: Sex, addon: u8) -> bool {
        self.0
            .cosmetics()
            .is_some_and(|c| c.addon_is_starter(key, sex, addon))
    }
    fn mount_exists(&self, key: &String) -> bool {
        self.0.cosmetics().is_some_and(|c| c.mount_exists(key))
    }
}
fn context(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
) -> Result<wire::Context, Error> {
    runtime.owner_fence().map_err(|_| Error::Stale)?;
    if native.source_digest() != runtime.content_pin().server_artifact_digest() {
        return Err(Error::Stale);
    }
    let cosmetics = native.cosmetics().ok_or(Error::Unavailable)?;
    if cosmetics.source_digest() != native.source_digest() {
        return Err(Error::Stale);
    }
    Ok(wire::Context {
        content: runtime.content_pin().client_artifact_digest(),
        cosmetics: cosmetics.document_sha256(),
    })
}
/// Independently reads current DB authority; no receipt or request supplies these values.
async fn fresh_fence(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    session: GameSessionId,
    connection: ConnectionGeneration,
) -> Result<CurrentCharacterGameplayFence, Error> {
    let store = FreshAdmissionStore::from_root(root.clone());
    let (current, _) = store
        .current_session_at(session)
        .await
        .map_err(|_| Error::Unavailable)?;
    if current.session_state() != GameSessionState::Active
        || current.current_connection_generation() != connection
    {
        return Err(Error::Stale);
    }
    let character = CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        .map_err(|_| Error::Invalid)?;
    let record = root
        .read_current_character(authority, character)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok(CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: session,
        connection_generation: connection,
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: record.revision,
    })
}
fn current_runtime(
    runtime: &ChannelRuntimeV1,
    fence: &CurrentCharacterGameplayFence,
    actor: ExactActorRef,
) -> Result<(), Error> {
    runtime.owner_fence().map_err(|_| Error::Stale)?;
    let binding = runtime.binding();
    let control = runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| Error::Stale)?;
    let lease = runtime
        .bound_attacker_lease(actor, fence.game_session_id)
        .map_err(|_| Error::Stale)?;
    if control.control_loss.is_some()
        || lease.character_id().as_bytes() != fence.character_id.as_bytes()
        || lease.generation() != fence.character_lease_generation
        || fence.runtime_scope
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || fence.scope_ownership_generation != binding.scope_generation()
    {
        return Err(Error::Stale);
    }
    Ok(())
}

/// One original occurrence is retained across unknown outcomes. No constructor accepts an
/// actor/account/sex from the wire. Sex is supplied by the actual Character creation owner.
pub(crate) struct PreparedChange {
    command: CommandRef,
    fence: CurrentCharacterGameplayFence,
    request: AppearanceChangeRequest,
    sex: Sex,
    content: Arc<NativeGameplayState>,
}
#[allow(
    clippy::too_many_arguments,
    reason = "actual admitted controller, native owners and immutable intent are separate inputs"
)]
pub(crate) async fn prepare_change(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    runtime: &ChannelRuntimeV1,
    native: Arc<NativeGameplayState>,
    actor: ExactActorRef,
    command: CommandRef,
    connection: ConnectionGeneration,
    capabilities: &[u32],
    occurrence: AppearanceChangeOccurrence,
    sex: Sex,
    payload: &[u8],
) -> Result<PreparedChange, Error> {
    if !selected(capabilities) {
        return Err(Error::Unsupported);
    }
    let change = wire::decode_change(payload)?;
    let fence = fresh_fence(root, authority, command.game_session_id(), connection).await?;
    current_runtime(runtime, &fence, actor)?;
    let cosmetics = native.cosmetics().ok_or(Error::Unavailable)?;
    if change.context != context(runtime, &native)?
        || change.expected_character_revision != fence.expected_character_revision.get()
        || change.content_revision != cosmetics.content_revision()
    {
        return Err(Error::Stale);
    }
    let request = AppearanceChangeRequest {
        occurrence,
        before: change.before.as_ref().map(stored).transpose()?,
        after: selection(&change.selection),
        content_revision: change.content_revision,
    };
    Ok(PreparedChange {
        command,
        fence,
        request,
        sex,
        content: native,
    })
}

#[derive(Debug)]
pub(crate) enum Settlement {
    Committed(Box<CommittedCharacterAppearance>),
    Rejected,
    /// Includes storage/COMMIT ambiguity. Retain the original request and reconcile it;
    /// this variant has no terminal CommandResult payload and never permits publication.
    Unresolved,
}
fn settlement_error(error: AppearanceWriteError) -> Settlement {
    match error {
        AppearanceWriteError::InvalidInput
        | AppearanceWriteError::StateMismatch
        | AppearanceWriteError::Rejected(_) => Settlement::Rejected,
        AppearanceWriteError::Fence(
            CharacterProgressionError::InvalidInput
            | CharacterProgressionError::AuthorityRejected
            | CharacterProgressionError::MissingProgressionState
            | CharacterProgressionError::CharacterRevisionMismatch
            | CharacterProgressionError::ProgressionContextMismatch
            | CharacterProgressionError::ConflictingOccurrence
            | CharacterProgressionError::RespawnPending,
        ) => Settlement::Rejected,
        _ => Settlement::Unresolved,
    }
}
impl PreparedChange {
    pub(crate) fn command(&self) -> CommandRef {
        self.command
    }
    /// The caller holds the native per-Character RevisionSlot without the runtime lock.
    /// A2 must add its appearance cursor arm; this leaf never guesses or rewrites that cursor.
    pub(crate) async fn commit(
        &self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
    ) -> Settlement {
        let revision = self
            .content
            .cosmetics()
            .map(|c| c.content_revision().to_owned());
        let Some(content_revision) = revision else {
            return Settlement::Rejected;
        };
        let validation = AppearanceValidationContext {
            sex: self.sex,
            content: OwnedContent(Arc::clone(&self.content)),
            content_revision,
        };
        match root
            .commit_character_appearance(
                authority,
                node,
                self.fence,
                self.request.clone(),
                validation,
            )
            .await
        {
            Ok(
                AppearanceChangeOutcome::Committed(receipt)
                | AppearanceChangeOutcome::AlreadyCommitted(receipt),
            ) if receipt.matches_request(&self.fence, &self.request, self.sex) => {
                Settlement::Committed(Box::new(receipt))
            }
            Ok(_) => Settlement::Unresolved,
            Err(error) => settlement_error(error),
        }
    }
    pub(crate) async fn reconcile(
        &self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Settlement {
        match root
            .reconcile_character_appearance(authority, self.request.occurrence)
            .await
        {
            Ok(Some(receipt)) if receipt.matches_request(&self.fence, &self.request, self.sex) => {
                Settlement::Committed(Box::new(receipt))
            }
            _ => Settlement::Unresolved,
        }
    }
}
impl Settlement {
    pub(crate) fn result_payload(&self) -> Result<Option<Vec<u8>>, Error> {
        let result = match self {
            Self::Committed(receipt) => wire::ChangeResult::Committed {
                revision: receipt.committed_character_revision.get(),
            },
            Self::Rejected => wire::ChangeResult::Rejected,
            Self::Unresolved => return Ok(None),
        };
        Ok(Some(wire::encode_result(result)?))
    }
}

/// Concrete native read inputs, not a new authority interface. `unlocks` below is the existing
/// pure account-fact view; its compatible-provenance resolver remains its actual owner's job.
pub(crate) struct ActorSource<'a> {
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) node: &'a NodeIncarnationProof,
    pub(crate) runtime: &'a ChannelRuntimeV1,
    pub(crate) states: &'a ChannelSpellStates,
    pub(crate) native: &'a NativeGameplayState,
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) connection: ConnectionGeneration,
    pub(crate) sex: Sex,
    pub(crate) now_us: u64,
}
impl ActorSource<'_> {
    async fn current_selection(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<
        (
            CurrentCharacterGameplayFence,
            Option<DurableAppearanceSelection>,
        ),
        Error,
    > {
        let fence = fresh_fence(self.root, authority, self.session, self.connection).await?;
        let interpretation = self
            .root
            .read_character_root_interpretation(authority, fence.character_id)
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Stale)?;
        if self
            .native
            .cosmetics()
            .ok_or(Error::Unavailable)?
            .content_revision()
            != interpretation.revisions()[2]
        {
            return Err(Error::Stale);
        }
        // This transaction qualifies the stored read, not future actor publication. The
        // publisher repeats it after preparing the observer, using this original expectation.
        let stored = self
            .root
            .read_current_character_appearance(authority, self.node, fence)
            .await
            .map_err(|_| Error::Unavailable)?;
        current_runtime(self.runtime, &fence, self.actor)?;
        Ok((fence, stored))
    }
    fn look(
        &self,
        stored: Option<&DurableAppearanceSelection>,
        unlocks: &impl AccountUnlocks<String>,
    ) -> Result<(wire::Look, u64), Error> {
        let cosmetics = self.native.cosmetics().ok_or(Error::Unavailable)?;
        let state = self
            .states
            .get(self.runtime, self.actor, self.session)
            .ok_or(Error::Stale)?;
        let conditions = state.owned_conditions();
        if !conditions.accepts_time(self.now_us) {
            return Err(Error::Stale);
        }
        let fallback = cosmetics.fallback(self.sex);
        if validate_selection(&fallback, self.sex, cosmetics, unlocks).is_err() {
            return Err(Error::Unavailable);
        }
        let base = stored.map_or(&fallback, |s| &s.selection);
        let display = displayed_look(base, &fallback, self.sex, cosmetics, unlocks);
        let (key, colours, addons) = match conditions.displayed_temporary_appearance_at(self.now_us)
        {
            Some(TemporaryDisplayedAppearance::Item(item)) => {
                if item.artifact_digest() != self.native.source_digest() {
                    return Err(Error::Stale);
                }
                let reference = self
                    .native
                    .item_index()
                    .and_then(|i| {
                        i.definition_ref(
                            crate::content::item_ref::ITEM_FAMILY,
                            item.definition_key(),
                            item.revision_ref(),
                        )
                    })
                    .ok_or(Error::Unavailable)?;
                return Ok((
                    wire::Look::Item {
                        definition_ref: reference.get(),
                    },
                    state.revision(),
                ));
            }
            Some(TemporaryDisplayedAppearance::Outfit(value)) => {
                (&value.outfit_key, value.colours, value.addons)
            }
            None => (display.outfit_key, display.colours, display.addons),
        };
        let count = cosmetics
            .outfit_addon_count(key)
            .ok_or(Error::Unavailable)?;
        if count > 8
            || (count < 8 && u16::from(addons) >> count != 0)
            || colours
                .iter()
                .any(|c| usize::from(*c) >= cosmetics.palette().len())
        {
            return Err(Error::Unavailable);
        }
        Ok((
            wire::Look::Outfit {
                look_type: cosmetics
                    .look_type(key, self.sex)
                    .ok_or(Error::Unavailable)?,
                colours,
                addons,
            },
            state.revision(),
        ))
    }
}

/// Own-client catalogue from the concrete pinned native view. No premium gate, synthetic
/// unlock, guessed sex, or nearby-look fallback is introduced. Current earned compatibility
/// facts must come through the existing AccountUnlocks owner; unavailable facts authorize none.
pub(crate) async fn catalogue_for_actor(
    authority: &ReconciledCharacterAuthority<'_, '_>,
    source: &ActorSource<'_>,
    unlocks: &impl AccountUnlocks<String>,
) -> Result<wire::Catalogue, Error> {
    let (fence, stored) = source.current_selection(authority).await?;
    let cosmetics = source.native.cosmetics().ok_or(Error::Unavailable)?;
    let fallback = cosmetics.fallback(source.sex);
    validate_selection(&fallback, source.sex, cosmetics, unlocks)
        .map_err(|_| Error::Unavailable)?;
    let mut outfits = Vec::new();
    for key in cosmetics.outfit_keys() {
        let mut choice = AppearanceSelection {
            outfit_key: key.to_owned(),
            colours: fallback.colours,
            addons: 0,
            mount_key: None,
        };
        if validate_selection(&choice, source.sex, cosmetics, unlocks).is_err() {
            continue;
        }
        let mut allowed_addons = 0;
        for addon in 0..8 {
            choice.addons = 1 << addon;
            if validate_selection(&choice, source.sex, cosmetics, unlocks).is_ok() {
                allowed_addons |= choice.addons;
            }
        }
        outfits.push(wire::OutfitOption {
            key: key.to_owned(),
            look_type: cosmetics
                .look_type(key, source.sex)
                .ok_or(Error::Unavailable)?,
            allowed_addons,
        });
    }
    let mut mounts = Vec::new();
    for key in cosmetics.mount_keys() {
        let mut choice = fallback.clone();
        choice.mount_key = Some(key.to_owned());
        if validate_selection(&choice, source.sex, cosmetics, unlocks).is_ok() {
            mounts.push(key.to_owned());
        }
    }
    let catalogue = wire::Catalogue {
        context: context(source.runtime, source.native)?,
        character_revision: fence.expected_character_revision.get(),
        selected: stored.as_ref().map(wire_selected),
        palette: cosmetics.palette().to_vec(),
        outfits,
        mounts,
    };
    wire::encode_catalogue(&catalogue)?;
    Ok(catalogue)
}

/// Connection-owned mapping allocator. No reference is reused, including after leave.
/// It is intentionally absent from session continuity: resume constructs a fresh allocator.
pub(crate) struct SessionAppearance {
    generation: ConnectionGeneration,
    session: GameSessionId,
    catalogue: wire::Catalogue,
    capabilities: Vec<u32>,
    next: u64,
    revision: u64,
    bindings: BTreeMap<EntityRef, wire::Binding>,
}
pub(crate) struct Publication {
    frames: [Vec<u8>; 2],
    pub(crate) last_sequence: u64,
    pub(crate) visibility_revision: u64,
}
impl Publication {
    /// Consume in this exact order. Failure of either write ends the connection; allocated
    /// references/revisions cannot be rolled back or reused after a possibly successful send.
    pub(crate) fn into_frames(self) -> [Vec<u8>; 2] {
        self.frames
    }
}
impl SessionAppearance {
    pub(crate) fn new(
        generation: ConnectionGeneration,
        session: GameSessionId,
        capabilities: &[u32],
        catalogue: wire::Catalogue,
    ) -> Result<Self, Error> {
        if !selected(capabilities) {
            return Err(Error::Unsupported);
        }
        wire::encode_catalogue(&catalogue)?;
        Ok(Self {
            generation,
            session,
            catalogue,
            capabilities: capabilities.to_vec(),
            next: 1,
            revision: 0,
            bindings: BTreeMap::new(),
        })
    }
    pub(crate) fn snapshot(&mut self) -> Result<(u64, Vec<u8>), Error> {
        let mut bindings: Vec<_> = self.bindings.values().cloned().collect();
        bindings.sort_by_key(|b| b.reference);
        let revision = self.revision.checked_add(1).ok_or(Error::Exhausted)?;
        let high_water = u32::try_from(self.next - 1).map_err(|_| Error::Exhausted)?;
        let payload = wire::encode_snapshot(&wire::Snapshot {
            catalogue: self.catalogue.clone(),
            bindings,
            high_water,
        })?;
        self.revision = revision;
        Ok((revision, payload))
    }
    /// Must be called after the corresponding visibility leave was queued/sent. Exact generation
    /// equality means an old leave cannot erase a replacement actor's mapping.
    pub(crate) fn retire(&mut self, entity: EntityRef) {
        self.bindings.remove(&entity);
    }
    pub(crate) async fn stage_catalogue(
        &mut self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        source: &ActorSource<'_>,
        unlocks: &impl AccountUnlocks<String>,
        sequence: u64,
    ) -> Result<Option<(u64, Vec<u8>)>, Error> {
        if source.session != self.session || source.connection != self.generation {
            return Err(Error::Stale);
        }
        let catalogue = catalogue_for_actor(authority, source, unlocks).await?;
        if catalogue.context != self.catalogue.context
            || catalogue.palette != self.catalogue.palette
            || catalogue.character_revision < self.catalogue.character_revision
        {
            return Err(Error::Stale);
        }
        if catalogue == self.catalogue {
            return Ok(None);
        }
        let payload = wire::encode_catalogue(&catalogue)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Exhausted)?;
        let sequence = sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let frame = encode_state_delta(
            self.generation.get(),
            sequence,
            wire::STATE_DOMAIN_APPEARANCE,
            self.revision,
            revision,
            wire::DELTA_TYPE_APPEARANCE_CATALOGUE_V1,
            &payload,
        )
        .map_err(|_| Error::Wire)?;
        self.revision = revision;
        self.catalogue = catalogue;
        Ok(Some((sequence, frame)))
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "actual source, visibility and existing envelope cursors must stay distinct"
    )]
    pub(crate) async fn stage_current(
        &mut self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        source: &ActorSource<'_>,
        unlocks: &impl AccountUnlocks<String>,
        visible: &WorldSpatialEntitiesSnapshot,
        lights: &EntityLights,
        sequence: u64,
        visibility_revision: u64,
    ) -> Result<Option<Publication>, Error> {
        // source.connection belongs to the observed player, not this observer's connection.
        if self.catalogue.context != context(source.runtime, source.native)? {
            return Err(Error::Stale);
        }
        // The current DB reader refuses a replaced session/connection/lease/scope/node/root.
        // No immutable committed receipt supplies this fence or the current selected base.
        let (fence, stored) = source.current_selection(authority).await?;
        let observer_fence =
            fresh_fence(source.root, authority, self.session, self.generation).await?;
        let observer = source
            .runtime
            .positioned_player_for_session(self.session)
            .map_err(|_| Error::Stale)?
            .ok_or(Error::Stale)?
            .0;
        // The observer's independently current session is distinct from the observed player.
        source
            .root
            .read_current_character_appearance(authority, source.node, observer_fence)
            .await
            .map_err(|_| Error::Unavailable)?;
        current_runtime(source.runtime, &observer_fence, observer)?;
        // Final target source boundary after all observer preparation awaits. A replacement
        // or revision advance in between suppresses this publication; do not reconstruct a
        // newer fence from the receipt. Separate TXs are not a joint atomic authority grant.
        let current = source
            .root
            .read_current_character_appearance(authority, source.node, fence)
            .await
            .map_err(|_| Error::Unavailable)?;
        if current != stored {
            return Err(Error::Stale);
        }
        // Use the existing VIS producer's native player projection, including its qualified
        // coordinates. A retained spatial snapshot must not move an actor backwards merely
        // because a stationary appearance UPDATE is being emitted.
        let players = source.runtime.visible_entities().players;
        let target = players
            .iter()
            .find(|p| p.actor == source.actor)
            .ok_or(Error::Stale)?;
        let observer_position = players
            .iter()
            .find(|p| p.actor == observer)
            .ok_or(Error::Stale)?
            .position;
        let shown = visible
            .entities
            .iter()
            .find(|p| {
                p.entity.identity == source.actor.placement_identity()
                    && p.entity.generation == source.actor.actor_local_generation()
            })
            .ok_or(Error::Stale)?;
        if shown.position.x != target.position.x
            || shown.position.y != target.position.y
            || shown.position.floor != target.position.floor
            || visible.own_identity != observer.placement_identity()
            || visible.actor_position.x != observer_position.x
            || visible.actor_position.y != observer_position.y
            || visible.actor_position.floor != observer_position.floor
        {
            return Err(Error::Stale);
        }
        let (look, condition_revision) = source.look(stored.as_ref(), unlocks)?;
        current_runtime(source.runtime, &fence, source.actor)?;
        let binding = wire::Binding {
            reference: 0,
            entity: EntityRef {
                identity: source.actor.placement_identity(),
                generation: source.actor.actor_local_generation(),
            },
            context: context(source.runtime, source.native)?,
            character_revision: fence.expected_character_revision.get(),
            selection_revision: stored
                .as_ref()
                .map(|s| s.committed_character_revision.get()),
            condition_revision,
            owner_time_us: source.now_us,
            look,
        };
        self.stage_visible(binding, visible, lights, sequence, visibility_revision)
    }
    fn stage_visible(
        &mut self,
        mut binding: wire::Binding,
        visible: &WorldSpatialEntitiesSnapshot,
        lights: &EntityLights,
        sequence: u64,
        visibility_revision: u64,
    ) -> Result<Option<Publication>, Error> {
        validate_snapshot(visible).map_err(|_| Error::Invalid)?;
        if visible.content_generation != self.catalogue.context.content
            || binding.context != self.catalogue.context
        {
            return Err(Error::Stale);
        }
        if let wire::Look::Outfit { colours, .. } = &binding.look
            && colours
                .iter()
                .any(|c| usize::from(*c) >= self.catalogue.palette.len())
        {
            return Err(Error::Invalid);
        }
        let mut entity = *visible
            .entities
            .iter()
            .find(|e| e.entity == binding.entity && e.kind == EntityKind::Player)
            .ok_or(Error::Stale)?;
        if matches!(binding.look, wire::Look::Item { .. }) && !self.capabilities.contains(&4) {
            return Err(Error::Unsupported);
        }
        if let Some(old) = self.bindings.get(&binding.entity) {
            if binding.character_revision < old.character_revision
                || binding.selection_revision < old.selection_revision
                || binding.condition_revision < old.condition_revision
                || binding.owner_time_us < old.owner_time_us
            {
                return Err(Error::Stale);
            }
            if old.character_revision == binding.character_revision
                && old.selection_revision == binding.selection_revision
                && old.condition_revision == binding.condition_revision
                && old.look == binding.look
            {
                return Ok(None);
            }
        }
        if self.bindings.len() == wire::MAX_BINDINGS
            && !self
                .bindings
                .keys()
                .any(|e| e.identity == binding.entity.identity)
        {
            return Err(Error::Exhausted);
        }
        binding.reference = u32::try_from(self.next).map_err(|_| Error::Exhausted)?;
        let EntityDetail::Actor { appearance_ref, .. } = &mut entity.detail else {
            return Err(Error::Invalid);
        };
        *appearance_ref = binding.reference;
        let mapping_revision = self.revision.checked_add(1).ok_or(Error::Exhausted)?;
        let mapping_sequence = sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let entity_sequence = mapping_sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let next_visibility = visibility_revision.checked_add(1).ok_or(Error::Exhausted)?;
        let payload = wire::encode_mappings(&wire::Mappings {
            context: binding.context,
            bindings: vec![binding.clone()],
            forget: Vec::new(),
        })?;
        let mapping_frame = encode_state_delta(
            self.generation.get(),
            mapping_sequence,
            wire::STATE_DOMAIN_APPEARANCE,
            self.revision,
            mapping_revision,
            wire::DELTA_TYPE_APPEARANCE_MAPPINGS_V1,
            &payload,
        )
        .map_err(|_| Error::Wire)?;
        // An appearance-only change emits an UPDATE at the existing stationary position;
        // no position revision is forged and no entity outside the visibility view is added.
        let delta = WorldSpatialEntitiesDelta {
            content_generation: visible.content_generation,
            actor_position: visible.actor_position,
            enter: Vec::new(),
            update: vec![entity],
            leave: Vec::new(),
        };
        let fields = EntityFields::selected(&self.capabilities);
        let entity_lights = if fields.light {
            lights
                .iter()
                .filter(|(identity, _)| **identity == binding.entity.identity)
                .map(|(identity, light)| (*identity, *light))
                .collect()
        } else {
            EntityLights::new()
        };
        let payload =
            encode_world_spatial_entities_delta_with_fields(&delta, fields, &entity_lights)
                .map_err(|_| Error::Wire)?;
        let entity_frame = encode_state_delta(
            self.generation.get(),
            entity_sequence,
            1,
            visibility_revision,
            next_visibility,
            2,
            &payload,
        )
        .map_err(|_| Error::Wire)?;
        self.next += 1;
        self.revision = mapping_revision;
        self.bindings
            .retain(|e, _| e.identity != binding.entity.identity);
        self.bindings.insert(binding.entity, binding);
        Ok(Some(Publication {
            frames: [mapping_frame, entity_frame],
            last_sequence: entity_sequence,
            visibility_revision: next_visibility,
        }))
    }
}

#[cfg(test)]
#[path = "appearance_change_tests.rs"]
mod tests;

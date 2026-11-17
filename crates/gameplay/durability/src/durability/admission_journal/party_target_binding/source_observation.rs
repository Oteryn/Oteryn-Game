//! Detached physical DATA for a current scoped read, never a namespace grant.
use super::*;

#[derive(Debug)]
pub struct CurrentPartyActorObservation {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: u64,
    pub(super) character: [u8; 16],
    pub(super) lease: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl CurrentPartyActorObservation {
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn server_digest(&self) -> [u8; 32] {
        self.server
    }
    pub fn frame_digest(&self) -> [u8; 32] {
        self.frame
    }
    pub fn map_digest(&self) -> [u8; 32] {
        self.map
    }
    /// A consumer must repeat this under the actual current physical owner after
    /// SQL. Moving within this same scope does not invalidate identity/lease DATA.
    pub fn matches(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.matches_identity(runtime)
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
    }
    pub(super) fn matches_identity(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation().get() == self.generation
            && pin.server_artifact_digest() == self.server
            && pin.client_artifact_digest() == self.client
            && pin.frame_binding_digest() == self.frame
            && runtime.map_revision_digest() == self.map
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &self.character
                        && lease.generation() == self.lease
                })
    }
}

pub fn observe_current_party_actor_for_source_read(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<CurrentPartyActorObservation> {
    if runtime
        .player_control_facts(actor, session)
        .ok()?
        .control_loss
        .is_some()
    {
        return None;
    }
    observe_actor_identity(runtime, actor, session)
}

pub(super) fn observe_actor_identity(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<CurrentPartyActorObservation> {
    runtime.owner_fence().ok()?;
    let lease = runtime.bound_attacker_lease(actor, session).ok()?;
    let pin = runtime.content_pin();
    Some(CurrentPartyActorObservation {
        actor,
        session,
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation().get(),
        character: *lease.character_id().as_bytes(),
        lease: lease.generation(),
        server: pin.server_artifact_digest(),
        client: pin.client_artifact_digest(),
        frame: pin.frame_binding_digest(),
        map: runtime.map_revision_digest(),
    })
}

/// The same canonical journal/Character/account/scope validation as the borrowed
/// periodic reader. Physical observation is input DATA; `authority` remains the
/// independently current owner of this exact SQL transaction.
pub async fn current_colocated_party_character_for_observation_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemScopeAuthority,
    observed: &CurrentPartyActorObservation,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    if authority.runtime_scope() != observed.scope || authority.generation() != observed.generation
    {
        return Ok(None);
    }
    let Some(row) = load_session_for_update(tx, observed.session.as_bytes()).await? else {
        return Ok(None);
    };
    if row.try_get::<i16, _>("session_state")? != ACTIVE
        || row.try_get::<Vec<u8>, _>("world_id")? != observed.scope.world_id().as_bytes()
        || row.try_get::<Vec<u8>, _>("runtime_scope_world_id")?
            != observed.scope.world_id().as_bytes()
        || !row_matches_scope(&row, observed.scope)?
        || row.try_get::<String, _>("scope_ownership_generation")?
            != observed.generation.to_string()
        || row.try_get::<Vec<u8>, _>("character_id")? != observed.character
        || row.try_get::<String, _>("character_lease_generation")? != observed.lease.to_string()
        || !canonical_current_party_session(tx, root, observed.session, &row, observed.scope)
            .await?
    {
        return Ok(None);
    }
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(Some(observed.character))
}

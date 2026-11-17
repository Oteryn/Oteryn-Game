//! Physical presence DATA for source callbacks; grace never grants player input.
use super::*;

#[derive(Debug)]
pub struct CurrentVisibleActorObservation {
    identity: CurrentPartyActorObservation,
    control_loss: Option<(u64, i64)>,
}
impl CurrentVisibleActorObservation {
    pub fn matches(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.identity.matches_identity(runtime)
            && runtime
                .player_control_facts(self.identity.actor, self.identity.session)
                .is_ok_and(|facts| {
                    facts
                        .control_loss
                        .map(|loss| (loss.epoch, loss.grace_deadline))
                        == self.control_loss
                })
    }
}

pub fn observe_current_visible_actor_for_source_read(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<CurrentVisibleActorObservation> {
    let control_loss = runtime
        .player_control_facts(actor, session)
        .ok()?
        .control_loss;
    Some(CurrentVisibleActorObservation {
        identity: source_observation::observe_actor_identity(runtime, actor, session)?,
        control_loss: control_loss.map(|loss| (loss.epoch, loss.grace_deadline)),
    })
}

/// A genuine current scope transaction and the canonical admission reconciler
/// independently establish the Character. The observation supplies no authority.
pub async fn current_visible_character_for_source_read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemScopeAuthority,
    observed: &CurrentVisibleActorObservation,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let identity = &observed.identity;
    if authority.runtime_scope() != identity.scope || authority.generation() != identity.generation
    {
        return Ok(None);
    }
    let Some(row) = load_session_for_update(tx, identity.session.as_bytes()).await? else {
        return Ok(None);
    };
    let state: i16 = row.try_get("session_state")?;
    let loss_matches = match observed.control_loss {
        None => state == ACTIVE,
        Some((epoch, deadline)) => {
            state == super::super::RECONNECTABLE
                && row.try_get::<Option<String>, _>("control_loss_epoch")?
                    == Some(epoch.to_string())
                && row.try_get::<Option<i64>, _>("original_grace_deadline")? == Some(deadline)
                && row
                    .try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
                    .is_none()
        }
    };
    if !loss_matches
        || row.try_get::<Vec<u8>, _>("world_id")? != identity.scope.world_id().as_bytes()
        || !row_matches_scope(&row, identity.scope)?
        || row.try_get::<String, _>("scope_ownership_generation")?
            != identity.generation.to_string()
        || row.try_get::<Vec<u8>, _>("character_id")? != identity.character
        || row.try_get::<String, _>("character_lease_generation")? != identity.lease.to_string()
        || !canonical_current_source_session(tx, root, identity.session, &row, identity.scope, true)
            .await?
    {
        return Ok(None);
    }
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(Some(identity.character))
}

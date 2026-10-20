//! Visibility input for a party consent operation, issued from the actual current journal
//! and held physical owner. It grants no invitation or membership by itself.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::{ACTIVE, active_committed_binding_is_valid, load_session_for_update};
use crate::durability::{
    DurabilityError, DurabilityRoot,
    fresh_admission::FreshAdmissionStore,
    spell_item_transaction::{
        SpellItemAuthority, SpellItemScopeAuthority, check_scope_transaction, check_transaction,
    },
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementPositionSnapshot,
    RuntimeScopeRefV1,
};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug)]
pub(crate) struct QualifiedPartyVisibleTarget<'owner> {
    runtime: &'owner ChannelRuntimeV1,
    caster: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    target_position: MovementPositionSnapshot,
    character: [u8; 16],
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
}
impl QualifiedPartyVisibleTarget<'_> {
    pub(crate) fn character(&self) -> [u8; 16] {
        self.character
    }
    pub(crate) fn matches(&self, authority: &SpellItemAuthority) -> bool {
        let actual = self.runtime.binding();
        authority.runtime_scope()
            == RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
            && authority.scope_generation() == self.scope_generation
            && self.world == *actual.world_id().as_bytes()
            && self.channel == *actual.channel_id().as_bytes()
            && self
                .runtime
                .player_control_facts(self.caster, authority.game_session_id())
                .is_ok_and(|p| p.control_loss.is_none())
            && self
                .runtime
                .player_control_facts(self.target, self.session)
                .is_ok_and(|p| p.control_loss.is_none())
            && self.runtime.read_actor_position(self.target) == Ok(self.target_position)
    }
}
pub(crate) async fn prove_visible_party_target_in_transaction<'owner>(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &'owner ChannelRuntimeV1,
    caster: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
) -> Result<Option<QualifiedPartyVisibleTarget<'owner>>, DurabilityError> {
    check_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let binding = runtime.binding();
    if caster == target
        || runtime.owner_fence().is_err()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || authority.scope_generation() != binding.scope_generation().get()
        || !runtime
            .player_control_facts(caster, authority.game_session_id())
            .is_ok_and(|p| p.control_loss.is_none())
        || !runtime
            .player_control_facts(target, session)
            .is_ok_and(|p| p.control_loss.is_none())
    {
        return Ok(None);
    }
    let caster_position = runtime
        .read_actor_position(caster)
        .map_err(|_| DurabilityError::Unavailable)?;
    let target_position = runtime
        .read_actor_position(target)
        .map_err(|_| DurabilityError::Unavailable)?;
    let c = caster_position.position();
    let t = target_position.position();
    let dx = i64::from(t.x) - i64::from(c.x);
    let dy = i64::from(t.y) - i64::from(c.y);
    if c.floor != t.floor || !(-8..=9).contains(&dx) || !(-6..=7).contains(&dy) {
        return Ok(None);
    }
    let Some(row) = load_session_for_update(tx, session.as_bytes()).await? else {
        return Ok(None);
    };
    if row.try_get::<i16, _>("session_state")? != ACTIVE
        || row.try_get::<Vec<u8>, _>("world_id")? != binding.world_id().as_bytes()
        || row.try_get::<Vec<u8>, _>("runtime_scope_world_id")? != binding.world_id().as_bytes()
        || row
            .try_get::<Option<Vec<u8>>, _>("runtime_scope_channel_id")?
            .as_deref()
            != Some(binding.channel_id().as_bytes().as_slice())
        || row.try_get::<String, _>("scope_ownership_generation")?
            != binding.scope_generation().get().to_string()
        || !canonical_current_party_session(tx, root, session, &row, authority.runtime_scope())
            .await?
    {
        return Ok(None);
    }
    let character: [u8; 16] = row
        .try_get::<Vec<u8>, _>("character_id")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Some(QualifiedPartyVisibleTarget {
        runtime,
        caster,
        target,
        session,
        target_position,
        character,
        world: *binding.world_id().as_bytes(),
        channel: *binding.channel_id().as_bytes(),
        scope_generation: binding.scope_generation().get(),
    }))
}

/// Same current journal owner, without a viewport filter: source party/Serene consumers
/// decide visibility from the actual position afterward. No name/character guess is accepted.
pub(crate) async fn current_colocated_party_character_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    check_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let binding = runtime.binding();
    if runtime.owner_fence().is_err()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || authority.scope_generation() != binding.scope_generation().get()
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|p| p.control_loss.is_none())
    {
        return Ok(None);
    }
    let Some(row) = load_session_for_update(tx, session.as_bytes()).await? else {
        return Ok(None);
    };
    if row.try_get::<i16, _>("session_state")? != ACTIVE
        || row.try_get::<Vec<u8>, _>("world_id")? != binding.world_id().as_bytes()
        || row.try_get::<Vec<u8>, _>("runtime_scope_world_id")? != binding.world_id().as_bytes()
        || row
            .try_get::<Option<Vec<u8>>, _>("runtime_scope_channel_id")?
            .as_deref()
            != Some(binding.channel_id().as_bytes().as_slice())
        || row.try_get::<String, _>("scope_ownership_generation")?
            != binding.scope_generation().get().to_string()
        || !canonical_current_party_session(tx, root, session, &row, authority.runtime_scope())
            .await?
    {
        return Ok(None);
    }
    row.try_get::<Vec<u8>, _>("character_id")?
        .try_into()
        .map(Some)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

/// Commandless periodic source reader. The genuine independently fenced scope authority
/// owns this SQL transaction; canonical current actor/session evidence remains separately required.
pub(crate) async fn current_colocated_party_character_for_scope_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let binding = runtime.binding();
    if runtime.owner_fence().is_err()
        || authority.world_bytes() != *binding.world_id().as_bytes()
        || authority.channel_bytes() != *binding.channel_id().as_bytes()
        || authority.generation() != binding.scope_generation().get()
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|p| p.control_loss.is_none())
    {
        return Ok(None);
    }
    let Some(row) = load_session_for_update(tx, session.as_bytes()).await? else {
        return Ok(None);
    };
    if row.try_get::<i16, _>("session_state")? != ACTIVE
        || row.try_get::<Vec<u8>, _>("world_id")? != binding.world_id().as_bytes()
        || row.try_get::<Vec<u8>, _>("runtime_scope_world_id")? != binding.world_id().as_bytes()
        || row
            .try_get::<Option<Vec<u8>>, _>("runtime_scope_channel_id")?
            .as_deref()
            != Some(binding.channel_id().as_bytes().as_slice())
        || row.try_get::<String, _>("scope_ownership_generation")?
            != binding.scope_generation().get().to_string()
        || !canonical_current_party_session(
            tx,
            root,
            session,
            &row,
            RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        )
        .await?
    {
        return Ok(None);
    }
    row.try_get::<Vec<u8>, _>("character_id")?
        .try_into()
        .map(Some)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

/// Fresh entry has no committed reconnect attempt. Resolve its genuinely committed operation
/// through the existing bounded canonical reconciler, inside the same physical transaction.
/// An origin-present but absent/corrupt receipt must never fall back to a legacy record.
async fn canonical_current_party_session(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    session: GameSessionId,
    row: &sqlx::postgres::PgRow,
    scope: RuntimeScopeRefV1,
) -> Result<bool, DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        return Ok(false);
    };
    let key = crate::durability::runtime_scope_assignment::scope_key(world_id, channel_id);
    let fresh_origin: bool = sqlx::query_scalar("SELECT fresh_replay_key IS NOT NULL OR initial_fresh_replay_key IS NOT NULL FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    // The immutable receipt cannot supersede independently current account/Character claims.
    let claims_current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id) JOIN game_durability_admission_character_guards c USING(character_id) JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id JOIN game_durability_admission_runtime_guards g ON g.scope_key=$2 WHERE s.game_session_id=encode($1,'hex')::uuid AND c.account_id=r.account_id AND c.world_id=r.world_id AND c.eligible AND c.lease_generation=s.character_lease_generation AND c.holder_game_session_id=s.game_session_id AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id AND g.ready AND g.ownership_generation=s.scope_ownership_generation)")
        .bind(session.as_bytes().as_slice()).bind(key.as_slice()).fetch_one(&mut **tx).await?;
    if !claims_current {
        return Ok(false);
    }
    if !fresh_origin {
        return active_committed_binding_is_valid(tx, session.as_bytes(), row).await;
    }
    let store = FreshAdmissionStore::from_root(root.clone());
    let current = store.current_session_in_transaction(tx, session).await?;
    if current.current_control_loss_epoch().is_some() {
        // Same-session complete resume retains its full typed loss/budget/protection proof.
        // A genuine replaced current session uses the existing complete legacy validator.
        let resumed = current.current_game_session_id() == current.commit().game_session_id()
            && store.resumed_history_locked(tx, current).await?.is_some();
        if resumed {
            let predecessor = row
                .try_get::<Option<String>, _>("predecessor_generation")?
                .and_then(|value| value.parse::<u64>().ok());
            if predecessor.and_then(|value| value.checked_add(1))
                != Some(current.current_connection_generation().get())
            {
                return Ok(false);
            }
        } else if !active_committed_binding_is_valid(tx, session.as_bytes(), row).await? {
            return Ok(false);
        }
    } else if current.current_game_session_id() != current.commit().game_session_id()
        || current.current_connection_generation().get() != 1
        || current.current_transport() != Some(current.commit().initial_transport())
        || current.current_character_lease().generation()
            != current.commit().character_lease_generation()
    {
        return Ok(false);
    }
    let scope = current.current_runtime_scope();
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        return Ok(false);
    };
    Ok(current.session_state() == GameSessionState::Active
        && current.current_game_session_id() == session
        && row.try_get::<Vec<u8>, _>("character_id")? == current.commit().character_id().as_bytes()
        && row.try_get::<Vec<u8>, _>("world_id")? == current.commit().world_id().as_bytes()
        && row.try_get::<Vec<u8>, _>("runtime_scope_world_id")? == world_id.as_bytes()
        && row
            .try_get::<Option<Vec<u8>>, _>("runtime_scope_channel_id")?
            .as_deref()
            == Some(channel_id.as_bytes().as_slice())
        && row
            .try_get::<Option<Vec<u8>>, _>("runtime_scope_instance_id")?
            .is_none()
        && row.try_get::<String, _>("scope_ownership_generation")?
            == current.current_scope_generation().get().to_string()
        && row.try_get::<String, _>("character_lease_generation")?
            == current.current_character_lease().generation().to_string()
        && row.try_get::<String, _>("current_generation")?
            == current.current_connection_generation().get().to_string()
        && row.try_get::<Option<Vec<u8>>, _>("current_transport_ref")?
            == current.current_transport().map(|t| t.to_bytes().to_vec())
        && row.try_get::<Option<String>, _>("control_loss_epoch")?
            == current
                .current_control_loss_epoch()
                .map(|e| e.get().to_string())
        && row.try_get::<Option<i64>, _>("original_grace_deadline")?
            == current.current_original_grace_deadline())
}

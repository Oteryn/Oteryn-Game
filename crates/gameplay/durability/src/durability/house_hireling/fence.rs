//! Named-House inventory authority for Hireling services. An activity Instance is never a House.
use super::super::character_progression::{assert_item_only_fence, numeric_u64};
use super::super::house_scope_handoff::HouseId;
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::super::DurabilityError;
use crate::foundation::RuntimeScopeRefV1;
use sqlx::{Postgres, Row, Transaction};

/// History-only custody proof, without a service ACL grant. Must run after recovery
/// and admission relation locks. The caller's channel is checked
/// against the committed House handoff, never substituted for the House scope fence.
pub(crate) async fn assert_house_custody_fence(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    origin_channel: [u8; 16],
) -> Result<Option<HouseId>, DurabilityError> {
    let RuntimeScopeRefV1::Instance { world_id, instance_id } = fence.runtime_scope else {
        return Ok(None);
    };
    let Some(row) = sqlx::query(
        "SELECT s.runtime_scope_house_key AS house, s.account_id::text AS account, \
                s.character_lease_generation::text AS lease, \
                r.profile_revision, r.ruleset_revision, r.content_revision, r.starter_template_revision \
         FROM game_durability_reconnect_sessions s \
         JOIN game_character_roots r USING(character_id) \
         JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id \
         JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id \
         WHERE s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid \
           AND s.world_id=encode($3,'hex')::uuid AND s.runtime_scope_world_id=s.world_id \
           AND s.runtime_scope_kind=2 AND s.runtime_scope_channel_id IS NULL \
           AND s.runtime_scope_house_key IS NOT NULL \
           AND s.runtime_scope_instance_id=encode($4,'hex')::uuid \
           AND s.origin_channel_id=encode($5,'hex')::uuid \
           AND s.current_generation=$6::text::numeric \
           AND s.character_lease_generation=$7::text::numeric \
           AND s.scope_ownership_generation=$8::text::numeric AND s.session_state IN(1,2) \
           AND r.lifecycle=1 AND r.account_id=s.account_id AND r.world_id=s.world_id \
           AND c.eligible AND c.account_id=s.account_id AND c.world_id=s.world_id \
           AND c.lease_generation=s.character_lease_generation \
           AND c.holder_game_session_id=s.game_session_id \
           AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id \
         FOR SHARE OF s,a,c",
    ).bind(fence.game_session_id.as_bytes().as_slice())
     .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice())
     .bind(instance_id.as_slice()).bind(origin_channel.as_slice())
     .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
     .bind(fence.scope_ownership_generation.get().to_string()).fetch_optional(&mut **tx).await?
    else { return Ok(None); };
    let house: String = row.try_get("house")?;
    if HouseId::new(world_id, &house).is_none_or(|id| id.instance_id()!=instance_id)
        || numeric_u64(&row, "lease")? != fence.character_lease_generation
        || !prove_current_incarnation(tx, node).await? {
        return Ok(None);
    }
    let fact=node.fact();
    let held: bool=sqlx::query_scalar(
        "SELECT game_runtime_scope_assignment_history_valid() AND EXISTS( \
         SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key=game_house_scope_key(encode($1,'hex')::uuid,$2) \
           AND scope_kind=2 AND world_id=encode($1,'hex')::uuid AND house_key=$2 \
           AND state=1 AND ownership_generation=$3::text::numeric \
           AND holder_node_id=encode($4,'hex')::uuid AND holder_registration_revision=$5::text::numeric \
         FOR SHARE)",
    ).bind(world_id.as_bytes().as_slice()).bind(&house)
     .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
     .bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
    if !held { return Ok(None); }
    let root=sqlx::query(
        "SELECT profile_revision,ruleset_revision,content_revision,starter_template_revision \
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let current=sqlx::query(
        "SELECT profile_revision,ruleset_revision,content_revision,starter_template_revision \
         FROM game_character_interpretations ORDER BY interpretation_revision DESC LIMIT 1",
    ).fetch_optional(&mut **tx).await?;
    let (Some(root),Some(current))=(root,current) else {return Ok(None)};
    for column in ["profile_revision","ruleset_revision","content_revision","starter_template_revision"] {
        if root.try_get::<String,_>(column)? != current.try_get::<String,_>(column)? {return Ok(None)}
    }
    Ok(HouseId::new(world_id, &house))
}

/// Service/input authority adds the original locked current ACL to named custody.
pub(crate) async fn assert_house_inventory_fence(
    tx: &mut Transaction<'_, Postgres>, node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence, origin_channel: [u8;16],
) -> Result<bool, DurabilityError> {
    let Some(current) = assert_house_custody_fence(tx,node,fence,origin_channel).await? else { return Ok(false); };
    let world_id = current.world_id();
    let house = current.house_key();
    let fact = node.fact();
    // Preserve the native House writer's Character -> optional Guild -> property
    // order (0595:78-92). A current ACL read must stay protected until this same
    // physical transaction publishes its service result. The helper reuses
    // game_house_access and grants no table-write authority to the runtime.
    Ok(sqlx::query_scalar(
        "SELECT game_hireling_service_access_lock(encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
          $3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid, \
          $7::text::numeric,encode($8,'hex')::uuid,$9)",
    ).bind(fence.game_session_id.as_bytes().as_slice())
     .bind(fence.character_id.as_bytes().as_slice())
     .bind(fence.connection_generation.get().to_string())
     .bind(fence.character_lease_generation.to_string())
     .bind(fence.scope_ownership_generation.get().to_string())
     .bind(fact.node_id().as_bytes().as_slice())
     .bind(fact.registration_revision().to_string())
     .bind(world_id.as_bytes().as_slice()).bind(house)
     .fetch_one(&mut **tx).await?)
}

pub(crate) async fn assert_inventory_fence(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    origin_channel: [u8;16],
) -> Result<bool,DurabilityError> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } if *channel_id.as_bytes()==origin_channel =>
            Ok(assert_item_only_fence(tx,fence,node).await?.is_ok()),
        RuntimeScopeRefV1::Instance { .. }=>assert_house_inventory_fence(tx,node,fence,origin_channel).await,
        _=>Ok(false),
    }
}

//! NX-1 refresh must retain the already admitted named-House session fence.
use super::*;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::runtime_scope_assignment::prove_current_incarnation;

pub(super) async fn current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    node: &NodeIncarnationProof,
) -> Result<bool, DurabilityError> {
    if assert_item_only_fence(tx, fence, node).await?.is_ok() {
        return Ok(true);
    }
    if !matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) || !prove_current_incarnation(tx, node).await?
    {
        return Ok(false);
    }
    let house: Option<String> = sqlx::query_scalar("SELECT runtime_scope_house_key FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND runtime_scope_kind=2 AND runtime_scope_house_key IS NOT NULL FOR SHARE")
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(house) = house.and_then(|key| HouseId::new(fence.runtime_scope.world_id(), &key))
    else {
        return Ok(false);
    };
    if house.runtime_scope()? != fence.runtime_scope {
        return Ok(false);
    }
    // This existing owner proves the exact session/lease/connection, named House,
    // assignment generation, current node and live Character root under locks.
    sqlx::query_scalar("SELECT game_item_action_house_current(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(node.fact().node_id().as_bytes().as_slice())
        .bind(node.fact().registration_revision().to_string()).bind(house.world_id().as_bytes().as_slice())
        .bind(house.house_key()).fetch_one(&mut **tx).await.map_err(Into::into)
}

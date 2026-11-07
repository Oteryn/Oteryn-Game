//! Named House authority is not an activity Instance. Only the death leaves use this arm.
use super::*;
use crate::durability::house_scope_handoff::HouseId;

pub(in crate::durability) async fn assert_house_death_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    expected_revision: Option<u64>,
    node: &NodeIncarnationProof,
    house: &HouseId,
) -> std::result::Result<
    std::result::Result<FencedCharacterRoot, CharacterProgressionError>,
    DurabilityError,
> {
    if house.world_id() != fence.runtime_scope.world_id()
        || house.runtime_scope().ok() != Some(fence.runtime_scope)
    {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    if !crate::durability::runtime_scope_assignment::prove_current_incarnation(tx, node).await? {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let fact = node.fact();
    let current:bool=sqlx::query_scalar("SELECT game_item_action_house_current(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
        .bind(fact.registration_revision().to_string()).bind(house.world_id().as_bytes().as_slice()).bind(house.house_key())
        .fetch_one(&mut **tx).await?;
    if !current {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let account:Option<String>=sqlx::query_scalar("SELECT account_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND runtime_scope_house_key=$3 AND runtime_scope_instance_id=encode($4,'hex')::uuid FOR SHARE")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(house.house_key())
        .bind(house.instance_id().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(account) = account else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    assert_root_after_session_fence(tx, fence, expected_revision, false, account).await
}

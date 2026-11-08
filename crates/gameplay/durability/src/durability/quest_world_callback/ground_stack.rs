//! M02: physical Ground predicates participate in the existing fenced quest TX.
//! Nomads Land chosen recipe s4 (Wiki spoiler revision 1002600) needs actual
//! crates/amphorae. Static ItemPresence projections cannot establish this fact.
use super::*;
use crate::durability::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, assert_spell_item_scope_with_recovery,
    check_scope_transaction,
};
use crate::durability::spell_items_abi::SpellGroundTarget;
use oteryn_gameplay_quests::quest::world_callbacks::{
    GroundStackEntry, GroundStackFact, WorldRequirement, ground_stack_matches,
};

fn owner_error(error: SpellItemError) -> DurabilityError {
    match error {
        SpellItemError::Database(error) => error.into(),
        SpellItemError::Durability(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

#[derive(sqlx::FromRow)]
struct GroundRoot {
    item_instance_id: sqlx::types::Uuid,
    definition_family: String,
    definition_production_key: String,
    definition_revision_ref: String,
    quantity: i64,
    stack_ordinal: Option<i64>,
    ownership_generation: String,
    map_revision: String,
    content_revision: String,
    native_room_placement_context: Vec<u8>,
}

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    guard: (
        &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        &super::super::runtime_scope_assignment::NodeIncarnationProof,
    ),
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
    requirements: &[WorldRequirement],
) -> Result<bool, DurabilityError> {
    if !requirements.iter().any(|r| r.ground_stack.is_some()) {
        return Ok(true);
    }
    // The scope issuer also takes namespace 33 with a blocking lock. Acquire
    // it non-blockingly first, before asking that issuer for its same-TX token.
    // Otherwise the later NOWAIT rows cannot prevent a pickup lock cycle.
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex')||encode($2,'hex'),33))",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !available {
        return Ok(false);
    }
    let authority = assert_spell_item_scope_with_recovery(
        tx,
        permit,
        guard.0,
        guard.1,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await
    .map_err(owner_error)?;
    for requirement in requirements {
        let Some(stack) = &requirement.ground_stack else {
            continue;
        };
        let p = requirement.position;
        let target = SpellGroundTarget {
            spatial_position: [
                i32::from(p.x).to_be_bytes().as_slice(),
                i32::from(p.y).to_be_bytes().as_slice(),
                i16::from(p.floor).to_be_bytes().as_slice(),
            ]
            .concat(),
            map_revision: context.map_revision.clone(),
            content_revision: context.content_revision.clone(),
            placement_context: context.placement_context.clone(),
        };
        if !matches(tx, &authority, &target, stack).await? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The authority is issued for this same physical transaction. Keep both the
/// scope's established Ground-owner advisory lock and exact Item/custody row
/// locks through the quest COMMIT: a concurrent drop cannot create a phantom.
pub(crate) async fn matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
    expected: &[GroundStackEntry],
) -> Result<bool, DurabilityError> {
    check_scope_transaction(tx, authority)
        .await
        .map_err(owner_error)?;
    let Some(channel) = authority.channel_bytes() else {
        return Ok(false);
    };
    if expected.is_empty() || expected.len() > 8 || expected.iter().any(|i| i.count == 0) {
        return Ok(false);
    }
    let world = authority.world_bytes();
    // Existing Ground namespace (0034/0049). Try-lock plus NOWAIT Item locks
    // preserve Item-before-Ground ordering: a concurrent pickup must cause
    // refusal/rollback instead of a cycle with this progress transaction.
    let available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex')||encode($2,'hex'),33))",
    )
    .bind(world.as_slice())
    .bind(channel.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !available {
        return Ok(false);
    }
    let mut rows = sqlx::query_as::<_, GroundRoot>("SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,g.stack_ordinal,g.runtime_scope_ownership_generation::text AS ownership_generation,g.map_revision,g.content_revision,g.native_room_placement_context FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.scope_kind=1 AND g.instance_id IS NULL AND g.spatial_position=$3 AND i.lifecycle=1 ORDER BY i.item_instance_id LIMIT 65 FOR UPDATE OF i,g NOWAIT")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(&target.spatial_position)
        .fetch_all(&mut **tx).await?;
    if rows.len() != expected.len() {
        return Ok(false);
    }
    rows.sort_by_key(|r| r.stack_ordinal);
    let mut previous = 0;
    for row in &rows {
        let Some(ordinal) = row.stack_ordinal.filter(|n| *n > previous) else {
            return Ok(false);
        };
        previous = ordinal;
        if row.ownership_generation != authority.generation().to_string() {
            return Ok(false);
        }
    }
    Ok(ground_stack_matches(
        expected,
        rows.iter().map(|row| {
            if row.definition_family != "Item"
                || row.definition_revision_ref != "definition-r1"
                || row.map_revision != target.map_revision
                || row.content_revision != target.content_revision
                || row.native_room_placement_context != target.placement_context
            {
                return None;
            }
            Some(GroundStackFact {
                instance: *row.item_instance_id.as_bytes(),
                item: &row.definition_production_key,
                count: u32::try_from(row.quantity).ok()?,
            })
        }),
    ))
}

/// Isolated PG fixture entry: the supplied owner token is still checked against
/// this exact physical transaction; no mutable registry or authority bypass.
#[cfg(any(test, feature = "test-support"))]
pub async fn fixture_matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
    expected: &[GroundStackEntry],
) -> Result<bool, DurabilityError> {
    matches(tx, authority, target, expected).await
}

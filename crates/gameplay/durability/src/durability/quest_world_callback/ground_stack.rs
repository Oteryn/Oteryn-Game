//! Read exact Ground custody under the native physical-scope owner lock.
use super::super::spell_item_transaction::{self, SpellItemError};
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;

fn observed<T>(value: Result<T, SpellItemError>) -> Result<Option<T>, DurabilityError> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(SpellItemError::Rejected(_)) => Ok(None),
        Err(SpellItemError::Database(error)) => Err(error.into()),
        Err(SpellItemError::Durability(error)) => Err(error),
        Err(_) => Err(DurabilityError::InvalidStoredState),
    }
}

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
    callback: &Callback,
) -> Result<bool, DurabilityError> {
    let mut requirements: Vec<_> = callback
        .world_requirements
        .iter()
        .filter(|r| r.ground_stack.is_some())
        .collect();
    if requirements.is_empty() {
        return Ok(true);
    }
    // Malformed fixture/caller data cannot bypass the production parser.
    if !oteryn_gameplay_quests::quest::world_callbacks::valid_ground_requirements(callback) {
        return Ok(false);
    }
    let Some(authority) = observed(
        spell_item_transaction::assert_spell_item_scope_with_recovery(
            tx,
            permit,
            recovery,
            node,
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
        )
        .await,
    )?
    else {
        return Ok(false);
    };
    requirements.sort_by_key(|r| r.position);
    for required in requirements {
        let Some(expected) = &required.ground_stack else {
            return Ok(false);
        };
        if required.appearance != 0
            || expected.len() > 64
            || expected.iter().any(|e| e.quantity == 0)
        {
            return Ok(false);
        }
        let mut position = i32::from(required.position.x).to_be_bytes().to_vec();
        position.extend_from_slice(&i32::from(required.position.y).to_be_bytes());
        position.extend_from_slice(&i16::from(required.position.floor).to_be_bytes());
        let target = super::super::spell_items_abi::SpellGroundTarget {
            spatial_position: position,
            map_revision: context.map_revision.clone(),
            content_revision: context.content_revision.clone(),
            placement_context: context.placement_context.clone(),
        };
        let Some(rows) = observed(
            spell_item_transaction::locked_tile_rows_for_scope(tx, &authority, &target).await,
        )?
        else {
            return Ok(false);
        };
        if rows.len() != expected.len() {
            return Ok(false);
        }
        let mut previous = None;
        for (row, expected) in rows.iter().zip(expected) {
            let ordinal = row
                .try_get::<String, _>("stack_ordinal")?
                .parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if ordinal == 0
                || previous.is_some_and(|previous| ordinal >= previous)
                || row.try_get::<String, _>("definition_family")? != "Item"
                || row.try_get::<String, _>("definition_production_key")? != expected.item
                || row.try_get::<i64, _>("quantity")? != i64::from(expected.quantity)
            {
                return Ok(false);
            }
            previous = Some(ordinal);
        }
    }
    Ok(true)
}

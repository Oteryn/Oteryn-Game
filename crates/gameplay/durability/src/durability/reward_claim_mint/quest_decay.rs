//! Quest rewards compose the existing absolute ItemType deadline and expiry owner.
use super::*;
use crate::content::ReferenceItemField as F;
use crate::content::ReferenceTemporalMode;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::spell_items_abi::{QualifiedItemDecay, QualifiedItemDefinition};
use oteryn_gameplay_quests::quest::reward_callbacks::ItemReward;

pub(super) fn resolve(
    native: &NativeGameplayState,
    reward: &ItemReward,
) -> std::result::Result<Option<QualifiedItemDecay>, DurabilityError> {
    let id = reward
        .item
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let policy = native
        .item_policy_for_source_id(id)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let Some(expected) = &reward.decay else {
        if matches!(&policy.record().semantics.temporal,F::Known(t) if t.consumption_mode==F::Known(ReferenceTemporalMode::DurableAbsoluteDeadline))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        return Ok(None);
    };
    let item = QualifiedItemDefinition::from_native_policy(policy)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decay = item.decay.ok_or(DurabilityError::InvalidStoredState)?;
    if reward.count != 1
        || !reward.contents.is_empty()
        || item.stack_maximum != 1
        || item.container_capacity.is_some()
        || decay.duration_millis != expected.duration_millis
        || decay
            .target
            .as_ref()
            .map(|t| (&t.production_key, &t.revision_ref))
            != expected.target.as_ref().map(|t| (&t.key, &t.revision))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(decay))
}

pub(super) async fn schedule(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenClaim,
    digest: [u8; 32],
    decay: QualifiedItemDecay,
) -> std::result::Result<(), DurabilityError> {
    let item = sqlx::types::Uuid::from_bytes(frozen.item_instance_id).to_string();
    let transaction = sqlx::types::Uuid::from_bytes(frozen.transaction_id).to_string();
    let output = serde_json::json!({"item":item,"count":1,"key":frozen.request.item.definition.production_key,"revision":frozen.request.item.definition.revision_ref,
        "decay":{"duration_millis":decay.duration_millis,"target":decay.target.map(|t|serde_json::json!({"family":t.family,"key":t.production_key,"revision":t.revision_ref}))}});
    crate::durability::spell_item_temporal::schedule_action_item_in_transaction(
        tx,
        &transaction,
        1,
        &output,
        digest,
        frozen.occurred_at_unix_ms,
    )
    .await
}

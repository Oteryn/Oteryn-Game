//! M52: bounded, audited World-event retirement on the existing physical Item ledger.
use super::{DurabilityError, RaidStepIdentity};
use crate::durability::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::foundation::RuntimeScopeRefV1;
use sqlx::{Postgres, Transaction};

/// Caller commits this batch before separately reacquiring publication fences.
/// Completed phase receipts preserve later drops, including after owner recovery.
pub async fn clear_feroxa_ground_batch(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    identity: RaidStepIdentity,
    phase: u8,
    now_ms: i64,
    frame: &SpellGroundTarget,
    content: [u8; 32],
) -> Result<bool, DurabilityError> {
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let RuntimeScopeRefV1::Channel { channel_id, .. } = authority.runtime_scope() else {
        return Err(DurabilityError::Unavailable);
    };
    if channel_id != identity.channel || authority.generation() != identity.generation.get() {
        return Err(DurabilityError::Unavailable);
    }
    Ok(sqlx::query_scalar("SELECT game_feroxa_clear_ground_batch(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::smallint,$6,$7,$8,$9,$10)")
        .bind(identity.firing_id.as_slice()).bind(authority.world_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(identity.generation.get().to_string()).bind(i16::from(phase)).bind(now_ms)
        .bind(&frame.map_revision).bind(&frame.content_revision).bind(&frame.placement_context)
        .bind(content.as_slice()).fetch_one(&mut **tx).await?)
}

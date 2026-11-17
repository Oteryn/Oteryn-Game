//! Project the four already-consumed shared sources from their immutable receipts.
use crate::durability::DurabilityError;
use crate::foundation::WorldId;
use oteryn_gameplay_quests::quest::shared_world_progress::{BINDINGS, SharedAction};
use sqlx::{Postgres, Transaction};
/// Bit order: heart, soul net, coral path, stair path. At most four receipts are
/// read, with exact world/save/source identities; no global receipt history scan.
pub(super) async fn used_sources_mask(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<u8, DurabilityError> {
    let mut keys = Vec::with_capacity(4);
    for binding in BINDINGS
        .iter()
        .filter(|binding| !matches!(binding.action, SharedAction::HiveTask(_)))
    {
        let mut key = world.as_bytes().to_vec();
        key.extend_from_slice(&epoch.to_be_bytes());
        key.extend_from_slice(&binding.position.0.to_be_bytes());
        key.extend_from_slice(&binding.position.1.to_be_bytes());
        key.push(binding.position.2.unsigned_abs());
        key.extend_from_slice(&binding.appearance.to_be_bytes());
        keys.push(key);
    }
    let used:Vec<Vec<u8>>=sqlx::query_scalar("SELECT occurrence FROM game_world_change_credit_receipts WHERE world_id=encode($1,'hex')::uuid AND source_kind=2 AND occurrence=ANY($2::bytea[]) LIMIT 4")
        .bind(world.as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    let mut mask = 0_u8;
    for (bit, key) in keys.iter().enumerate() {
        if used.iter().any(|value| value == key) {
            mask |= 1_u8 << bit
        }
    }
    Ok(mask)
}

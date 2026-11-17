//! Read-only fresh corpse selection for quest tools. The enclosing owner must
//! retain its transaction and runtime guard through any subsequent mutation.
use super::*;
use std::num::NonZeroU32;

mod actor;
pub(crate) use actor::CorpseActorSnapshot;
pub use actor::{FreshCorpseTarget, read_for_actor};

/// Select the exact top Ground corpse, using its original race provenance and
/// database time. `maximum_age_ms` is a server/source policy, never client input.
/// This read grants no Item mutation or quest-progress authority.
pub async fn read_current_fresh(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
    item: [u8; 16],
    race: &str,
    race_revision: &str,
    maximum_age_ms: NonZeroU32,
) -> Result<Option<TypedDefinitionRef>> {
    // Reuse the physical owner's scope/generation, same-transaction check,
    // complete tile ordering, custody locks and bounded stack read.
    let rows = locked_tile_rows_for_scope(tx, authority, target).await?;
    let Some(top) = rows.first() else {
        return Ok(None);
    };
    if uuid(&top.try_get::<String, _>("item_instance_id")?)? != item
        || !top.try_get::<bool, _>("corpse")?
    {
        return Ok(None);
    }
    // A shared appearance is not Creature identity. Only the original atomic
    // corpse MINT supplies the race. Neither a generous age limit nor a delayed
    // decay sweep can make an already transformed/spent corpse fresh again.
    let current: bool = sqlx::query_scalar(
        "WITH instant AS MATERIALIZED (
           SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms
         ) SELECT EXISTS(
           SELECT 1 FROM game_item_instances i
           JOIN game_item_mint_receipts m ON m.transaction_id=i.minted_transaction_id
             AND m.item_instance_id=i.item_instance_id
           JOIN game_corpse_source_races p ON p.item_instance_id=i.item_instance_id
             AND p.mint_transaction_id=m.transaction_id
           CROSS JOIN instant n
           WHERE i.item_instance_id=encode($1,'hex')::uuid
             AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1
             AND i.last_transaction_id IS NULL
             AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                 =(m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)
             AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
             AND p.race_key=$2 AND p.race_definition_revision=$3
             AND m.materialized_at<=n.now_ms AND n.now_ms-m.materialized_at<$4
             AND game_corpse_retirement_deadline(i.item_instance_id)>n.now_ms
             AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules s
                 WHERE s.item_instance_id=i.item_instance_id
                   AND s.source_transaction_id=m.transaction_id
                   AND s.source_ordinal=0 AND s.source_stage=1
                   AND s.expires_at_unix_ms<=n.now_ms)
         )",
    )
    .bind(item.as_slice())
    .bind(race)
    .bind(race_revision)
    .bind(i64::from(maximum_age_ms.get()))
    .fetch_one(&mut **tx)
    .await?;
    if current {
        Ok(Some(decoded_definition(top)?))
    } else {
        Ok(None)
    }
}

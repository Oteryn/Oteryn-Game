//! An original physical Quest transform arms a repeatable exact-item deadline.
use super::DurabilityError;
use super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
};
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn start(
    tx: &mut Transaction<'_, Postgres>,
    cause: &str,
    item: &str,
    duration_ms: u32,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_quest_tool_windows(start_transaction_id,item_instance_id,duration_ms)
         VALUES($1::uuid,$2::uuid,$3)",
    )
    .bind(cause)
    .bind(item)
    .bind(i32::try_from(duration_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn current(
    tx: &mut Transaction<'_, Postgres>,
    item: &str,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(
         SELECT 1 FROM game_quest_tool_windows w
         JOIN game_quest_callback_consumptions c
           ON (c.transaction_id,c.item_instance_id)=(w.start_transaction_id,w.item_instance_id)
         JOIN game_item_instances i ON i.item_instance_id=w.item_instance_id
         WHERE w.item_instance_id=$1::uuid AND i.definition_production_key=c.replacement
           AND i.definition_revision_ref=c.before_item->>'definition_revision_ref'
           AND w.deadline_ms>floor(extract(epoch FROM statement_timestamp())*1000)
           AND NOT EXISTS(SELECT 1 FROM game_quest_tool_expiries x
              WHERE (x.start_transaction_id,x.item_instance_id)=(w.start_transaction_id,w.item_instance_id))
        )",
    ).bind(item).fetch_one(&mut **tx).await?)
}

pub(super) async fn expire_due(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit == 0 {
        return Ok(0);
    }
    check_scope_transaction(tx, authority).await?;
    let channel = authority
        .channel_bytes()
        .ok_or(SpellItemError::Rejected("quest tool deadline Channel"))?;
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS item,w.start_transaction_id::text AS source,
         c.before_item->>'definition_production_key' AS cold,to_jsonb(i) AS before_item,
         game_character_uuid_v7()::text AS transaction_id
         FROM game_quest_tool_windows w
         JOIN game_quest_callback_consumptions c
           ON (c.transaction_id,c.item_instance_id)=(w.start_transaction_id,w.item_instance_id)
         JOIN game_item_instances i ON i.item_instance_id=w.item_instance_id
         WHERE i.world_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1
           AND i.definition_production_key=c.replacement
           AND i.definition_revision_ref=c.before_item->>'definition_revision_ref'
           AND w.deadline_ms<=floor(extract(epoch FROM statement_timestamp())*1000)
           AND NOT EXISTS(SELECT 1 FROM game_quest_tool_expiries x
              WHERE (x.start_transaction_id,x.item_instance_id)=(w.start_transaction_id,w.item_instance_id))
           AND (EXISTS(
             SELECT 1 FROM game_item_ground_locations g
             WHERE g.item_instance_id=i.item_instance_id AND g.world_id=i.world_id
               AND g.channel_id=encode($2,'hex')::uuid AND g.runtime_scope_ownership_generation=$3::text::numeric
           ) OR EXISTS(
             SELECT 1 FROM game_durability_reconnect_sessions s
             JOIN game_durability_admission_character_guards g USING(character_id)
             JOIN LATERAL game_quest_callback_carried(s.character_id,i.world_id) carried ON carried.item=i.item_instance_id
             WHERE s.runtime_scope_kind=1 AND s.runtime_scope_world_id=i.world_id
               AND s.runtime_scope_channel_id=encode($2,'hex')::uuid AND s.scope_ownership_generation=$3::text::numeric
               AND s.session_state=2 AND g.eligible AND g.holder_game_session_id=s.game_session_id
               AND g.lease_generation=s.character_lease_generation
           ))
         ORDER BY w.deadline_ms,i.item_instance_id LIMIT $4 FOR UPDATE OF i SKIP LOCKED",
    )
    .bind(authority.world_bytes().as_slice()).bind(channel.as_slice())
    .bind(authority.generation().to_string())
    .bind(i64::try_from(limit).map_err(|_| SpellItemError::Rejected("quest tool deadline bound"))?)
    .fetch_all(&mut **tx).await?;
    for row in &rows {
        let transaction: String = row.try_get("transaction_id")?;
        let item: String = row.try_get("item")?;
        let source: String = row.try_get("source")?;
        let before: serde_json::Value = row.try_get("before_item")?;
        let cold: String = row.try_get("cold")?;
        sqlx::query(
            "INSERT INTO game_quest_tool_expiries(transaction_id,start_transaction_id,item_instance_id,before_item)
             VALUES($1::uuid,$2::uuid,$3::uuid,$4)",
        ).bind(&transaction).bind(source).bind(&item).bind(before)
        .execute(&mut **tx).await?;
        let changed = sqlx::query(
            "UPDATE game_item_instances SET definition_production_key=$2,last_transaction_id=$3::uuid
             WHERE item_instance_id=$1::uuid",
        ).bind(&item).bind(cold).bind(&transaction).execute(&mut **tx).await?;
        if changed.rows_affected() != 1 {
            return Err(SpellItemError::Rejected("quest tool deadline successor"));
        }
    }
    Ok(rows.len())
}

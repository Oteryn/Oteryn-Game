//! Each original floor deadline executes once through the existing fenced Item owner.
use crate::durability::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

pub(in crate::durability) async fn drain_due(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit > 500 {
        return Err(SpellItemError::Rejected("floor deadline batch bound"));
    }
    if limit == 0 {
        return Ok(0);
    }
    check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    let channel = authority
        .channel_bytes()
        .ok_or(SpellItemError::Rejected("floor deadline Channel scope"))?;
    let rows=sqlx::query(r#"SELECT d.source_transaction_id::text,to_jsonb(i) AS before_item,
      CASE WHEN g.item_instance_id IS NULL THEN NULL ELSE to_jsonb(g) END AS ground,
      e.location AS entry,e.nested,
      CASE WHEN eq.item_instance_id IS NULL THEN NULL ELSE to_jsonb(eq) END AS equipped,
      CASE WHEN eq.item_instance_id IS NULL THEN NULL ELSE to_jsonb(es) END AS equipment_before,
      COALESCE(e.character_id,eq.character_id)::text AS character,
      sess.game_session_id::text AS session,sess.character_lease_generation::text AS lease,sess.current_generation::text AS connection,
      es.revision::text AS equipment_revision,p.decay_key,
      game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id
     FROM game_floor_transform_deadlines d JOIN game_floor_transform_receipts source ON source.transaction_id=d.source_transaction_id
     JOIN game_floor_transform_recipes p ON p.recipe=source.recipe
     JOIN game_item_instances i ON i.item_instance_id=d.item_instance_id
     LEFT JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
     LEFT JOIN LATERAL(
      SELECT c.character_id,c.world_id,to_jsonb(c) AS location,false AS nested FROM game_item_container_entries c WHERE c.item_instance_id=i.item_instance_id
      UNION ALL
      SELECT owner.owner_character,n.world_id,to_jsonb(n),true FROM game_item_nested_entries n,
       LATERAL game_item_loop_owner(n.parent_item_instance_id) owner WHERE n.item_instance_id=i.item_instance_id
        AND owner.owner_character IS NOT NULL AND owner.depth BETWEEN 0 AND 7
     ) e ON TRUE
     LEFT JOIN game_item_container_slots bag ON bag.character_id=e.character_id
     LEFT JOIN game_character_equipment_slots eq ON eq.item_instance_id=i.item_instance_id
     LEFT JOIN game_character_equipment_state es ON es.character_id=eq.character_id
     LEFT JOIN game_durability_admission_character_guards cg ON cg.character_id=COALESCE(e.character_id,eq.character_id)
     LEFT JOIN game_durability_reconnect_sessions sess ON sess.game_session_id=cg.holder_game_session_id
     LEFT JOIN game_character_roots character_root ON character_root.character_id=cg.character_id
     WHERE i.lifecycle=1 AND i.quantity=1 AND i.world_id=encode($1,'hex')::uuid
      AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1' AND i.definition_production_key=p.output_key
      AND d.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
      AND ((g.world_id=i.world_id AND g.channel_id=encode($2,'hex')::uuid AND g.runtime_scope_ownership_generation=$3::text::numeric)
       OR (((e.world_id=i.world_id AND bag.character_id=e.character_id) OR eq.world_id=i.world_id)
        AND cg.eligible AND cg.lease_generation=sess.character_lease_generation AND sess.session_state=2
        AND sess.character_id=cg.character_id AND sess.runtime_scope_kind=1 AND sess.runtime_scope_world_id=i.world_id
        AND sess.runtime_scope_channel_id=encode($2,'hex')::uuid AND sess.scope_ownership_generation=$3::text::numeric
        AND character_root.lifecycle=1 AND character_root.world_id=i.world_id))
     ORDER BY d.expires_at_unix_ms,i.item_instance_id LIMIT $4 FOR UPDATE OF i SKIP LOCKED"#)
        .bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string())
        .bind(i64::try_from(limit).map_err(|_|SpellItemError::Rejected("floor deadline batch bound"))?).fetch_all(&mut **tx).await?;
    for row in &rows {
        let before: Value = row.try_get("before_item")?;
        let item = before["item_instance_id"]
            .as_str()
            .ok_or(SpellItemError::Rejected("floor deadline Item identity"))?;
        let ground: Option<Value> = row.try_get("ground")?;
        let equipped: Option<Value> = row.try_get("equipped")?;
        let entry: Option<Value> = row.try_get("entry")?;
        let kind = if ground.is_some() {
            1
        } else if equipped.is_some() {
            3
        } else {
            2
        };
        let location = ground
            .or(equipped)
            .or(entry)
            .ok_or(SpellItemError::Rejected("floor deadline custody"))?;
        let nested = row.try_get::<Option<bool>, _>("nested")?.unwrap_or(false);
        let custody = json!({"kind":kind,"location":location,"nested":kind==2 && nested});
        let equipment: Option<Value> = row.try_get("equipment_before")?;
        let character: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("character")?
        };
        let session: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("session")?
        };
        let lease: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("lease")?
        };
        let connection: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("connection")?
        };
        let transaction: String = row.try_get("transaction_id")?;
        let event: String = row.try_get("event_id")?;
        let source: String = row.try_get("source_transaction_id")?;
        sqlx::query("INSERT INTO game_floor_transform_expiry_receipts(transaction_id,event_id,source_transaction_id,world_id,channel_id,ownership_generation,before_item,custody,character_id,game_session_id,character_lease_generation,connection_generation,equipment_before,occurred_at_unix_ms) VALUES($1::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9::uuid,$10::uuid,$11::text::numeric,$12::text::numeric,$13,floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
            .bind(&transaction).bind(&event).bind(&source).bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string())
            .bind(&before).bind(&custody).bind(&character).bind(&session).bind(&lease).bind(&connection).bind(&equipment).execute(&mut **tx).await?;
        let changed=sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND state_revision=$4::text::numeric AND lifecycle=1")
            .bind(item).bind(row.try_get::<String,_>("decay_key")?).bind(&transaction).bind(before["state_revision"].to_string()).execute(&mut **tx).await?;
        if changed.rows_affected() != 1 {
            return Err(SpellItemError::Rejected(
                "floor deadline compare-and-change",
            ));
        }
        if kind == 3 {
            let changed=sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$1::uuid WHERE character_id=$2::uuid AND revision=$3::text::numeric")
                .bind(&transaction).bind(&character).bind(row.try_get::<Option<String>,_>("equipment_revision")?).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return Err(SpellItemError::Rejected("floor deadline equipment epoch"));
            }
        }
        let envelope:Value=sqlx::query_scalar("SELECT jsonb_build_object('receipt',to_jsonb(r),'source',to_jsonb(source),'recipe',to_jsonb(p)) FROM game_floor_transform_expiry_receipts r JOIN game_floor_transform_receipts source ON source.transaction_id=r.source_transaction_id JOIN game_floor_transform_recipes p ON p.recipe=source.recipe WHERE r.transaction_id=$1::uuid")
            .bind(&transaction).fetch_one(&mut **tx).await?;
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| SpellItemError::Rejected("floor deadline audit encoding"))?;
        sqlx::query("INSERT INTO game_floor_transform_expiry_audit(event_id,transaction_id,envelope,envelope_sha256) VALUES($1::uuid,$2::uuid,$3,sha256($3))")
            .bind(&event).bind(&transaction).bind(bytes).execute(&mut **tx).await?;
    }
    Ok(rows.len())
}

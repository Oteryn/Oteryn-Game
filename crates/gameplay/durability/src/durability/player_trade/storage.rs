use super::super::item_mint_audit::{self as mint, Type2Transaction};
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use snapshot::{number, uuid};
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn lock_occurrence(
    tx: &mut Transaction<'_, Postgres>,
    occurrence: [u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:player-trade:'||encode($1,'hex'),0))")
        .bind(occurrence.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn lookup(
    tx: &mut Transaction<'_, Postgres>,
    request: &PlayerTradeRequest,
    intent: &[u8],
) -> Pass<TradeReceipt> {
    let row = sqlx::query(
        "SELECT c.intent,c.transaction_id::text,c.event_id::text,c.envelope_sha256, \
                a.envelope,a.envelope_sha256 AS audit_sha256 \
           FROM game_player_trades c LEFT JOIN game_item_audit_outbox a ON a.event_id=c.event_id \
          WHERE c.occurrence_id=encode($1,'hex')::uuid",
    )
    .bind(request.occurrence_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(Ok(TradeReceipt::Absent));
    };
    if row.try_get::<Vec<u8>, _>("intent")? != intent {
        return Ok(Err(Refusal::Rejected));
    }
    if super::super::item_mint::uuid_text(row.try_get("transaction_id")?)? != request.transaction_id
        || super::super::item_mint::uuid_text(row.try_get("event_id")?)? != request.event_id
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let wire: Vec<u8> = row
        .try_get::<Option<Vec<u8>>, _>("envelope")?
        .ok_or(DurabilityError::InvalidStoredState)?;
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(&wire).to_vec();
    if digest != row.try_get::<Vec<u8>, _>("envelope_sha256")?
        || digest != row.try_get::<Vec<u8>, _>("audit_sha256")?
        || !audit::decode(&wire).is_ok_and(|(envelope, payload)| {
            envelope.event_id == request.event_id
                && payload.request_binding == intent
                && envelope
                    .transaction_event
                    .is_some_and(|t| t.transaction_id == request.transaction_id)
        })
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Ok(TradeReceipt::AlreadyCommitted))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply<C: PlayerTradeContent>(
    tx: &mut Type2Transaction<'_>,
    record: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    current: &[CurrentCharacterItemFence; 2],
    content: &C,
    request: &PlayerTradeRequest,
    intent: &[u8],
) -> Pass<TradeReceipt> {
    let fact = node.fact();
    if fact.node_id().as_bytes() != &request.node_id
        || fact.registration_revision() != request.node_registration_revision
        || content.loop_content_digest() != request.content_digest
        || content.generation_digest() != request.weight_digest
    {
        return Ok(Err(Refusal::Rejected));
    }
    for (index, now) in current.iter().enumerate() {
        let captured = request.participants[index].accepted;
        let mut expected = captured;
        if index == 0 && now.connection_generation >= captured.connection_generation {
            expected.connection_generation = now.connection_generation;
        }
        if *now != expected {
            return Ok(Err(Refusal::Stale));
        }
    }
    let commands = request.participants.each_ref().map(|p| p.command);
    if !snapshot::fence_pair(tx, record, node, current, &commands, request.content_digest).await? {
        return Ok(Err(Refusal::Rejected));
    }
    let conflicting: bool=sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_player_trades WHERE transaction_id=encode($1,'hex')::uuid \
           OR event_id=encode($2,'hex')::uuid \
           OR (pair_game_session_id=encode($3,'hex')::uuid AND pair_command_id=$4::text::numeric))",
    ).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice())
        .bind(request.pair.game_session_id().as_bytes().as_slice())
        .bind(request.pair.command_id().get().to_string()).fetch_one(&mut ***tx).await?;
    if conflicting {
        return Ok(Err(Refusal::Rejected));
    }
    // A different occurrence cannot reuse either accept's full-u64 CommandRef.
    // This recheck runs after both Character locks, so concurrent requests from
    // the same parties have a typed loser before any value write.
    for command in commands {
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_player_trade_participants \
              WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)",
        )
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .fetch_one(&mut ***tx)
        .await?;
        if used {
            return Ok(Err(Refusal::Rejected));
        }
    }
    let items = [
        uuid(&request.participants[0].leaf.item, "item_instance_id")?,
        uuid(&request.participants[1].leaf.item, "item_instance_id")?,
    ];
    let leaves = match snapshot::read_pair(tx, current, items).await? {
        Ok(v) => v,
        Err(e) => return Ok(Err(e)),
    };
    if leaves != request.participants.each_ref().map(|p| p.leaf.clone()) {
        return Ok(Err(Refusal::Stale));
    }
    let destinations = match snapshot::destinations(tx, current, content, &leaves).await? {
        Ok(v) => v,
        Err(e) => return Ok(Err(e)),
    };
    sqlx::query(
        "SELECT game_player_trade_lock_slots(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(current[0].character_id.as_bytes().as_slice())
    .bind(current[1].character_id.as_bytes().as_slice())
    .execute(&mut ***tx)
    .await?;
    // No write before all live fences, snapshots, trade flags, room and weight pass.
    let tuple = tx.tuple();
    let wire = match audit::encode(request, intent, destinations, tuple.tuple()) {
        Ok(w) => w,
        Err(_) => return Ok(Err(Refusal::Rejected)),
    };
    let (world, channel) =
        scope_of(&current[0]).map_err(|_| DurabilityError::InvalidStoredState)?;
    let time = request.occurred_at_unix_ms;
    let expiry = time
        .checked_add(mint::AUDIT_RETENTION_P90D_MS)
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_player_trades(occurrence_id,transaction_id,event_id,world_id,channel_id,intent,occurred_at,envelope_sha256,pair_game_session_id,pair_command_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,sha256($8),encode($9,'hex')::uuid,$10::text::numeric)")
        .bind(request.occurrence_id.as_slice()).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice())
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(intent).bind(time).bind(&wire).bind(request.pair.game_session_id().as_bytes().as_slice()).bind(request.pair.command_id().get().to_string())
        .execute(&mut ***tx).await?;
    for (index, p) in request.participants.iter().enumerate() {
        sqlx::query("INSERT INTO game_player_trade_participants(occurrence_id,side,character_id,game_session_id,command_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric)")
            .bind(request.occurrence_id.as_slice()).bind(i16::try_from(index).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(p.accepted.character_id.as_bytes().as_slice()).bind(p.command.game_session_id().as_bytes().as_slice())
            .bind(p.command.command_id().get().to_string()).execute(&mut ***tx).await?;
    }
    for (index, leaf) in leaves.iter().enumerate() {
        let other = 1 - index;
        sqlx::query("INSERT INTO game_player_trade_receipts(occurrence_id,side,item_instance_id,source_character_id,destination_character_id,source_parent,source_ordinal,destination_parent,destination_ordinal,destination_capacity,before_item,before_timed,before_text,before_text_revision,before_key) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9::text::numeric,$10,$11,$12,$13,$14,$15)")
            .bind(request.occurrence_id.as_slice()).bind(i16::try_from(index).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(items[index].as_slice()).bind(current[index].character_id.as_bytes().as_slice())
            .bind(current[other].character_id.as_bytes().as_slice()).bind(uuid(&leaf.entry,"parent_item_instance_id")?.as_slice())
            .bind(number(&leaf.entry,"placement_ordinal")?.to_string()).bind(uuid(&leaves[other].entry,"parent_item_instance_id")?.as_slice())
            .bind(destinations[index].0.to_string()).bind(i32::try_from(destinations[index].1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(&leaf.item).bind(&leaf.timed).bind(&leaf.text).bind(&leaf.text_revision).bind(&leaf.key).execute(&mut ***tx).await?;
    }
    // Both receipts were admitted against their real source entries before either DELETE.
    for item in items {
        let removed = sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut ***tx)
        .await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    for (index, item) in items.iter().enumerate() {
        sqlx::query("UPDATE game_item_instances SET last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(request.transaction_id.as_slice()).bind(item.as_slice()).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,parent_item_instance_id,character_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
            .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(uuid(&leaves[1-index].entry,"parent_item_instance_id")?.as_slice())
            .bind(current[1-index].character_id.as_bytes().as_slice()).bind(destinations[index].0.to_string())
            .bind(request.transaction_id.as_slice()).execute(&mut ***tx).await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$7,$8,sha256($8),1)")
        .bind(request.event_id.as_slice()).bind(request.transaction_id.as_slice()).bind(i64::from(tuple.schema_revision()))
        .bind(tuple.retention_profile_id()).bind(items[0].as_slice()).bind(time).bind(expiry).bind(&wire)
        .execute(&mut ***tx).await?;
    Ok(Ok(TradeReceipt::Committed))
}

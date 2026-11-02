//! Q16: named-House positions use the existing Character projection and order key.
//! Origin Channel and its cell are routing history, never substituted for House authority.
use super::*;
use super::super::house_scope_handoff::HouseId;
use crate::foundation::ChannelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseLastPosition {
    pub house: HouseId,
    pub origin_channel: ChannelId,
    pub origin: LastPositionCell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedLastPosition {
    pub position: StoredLastPosition,
    pub house: Option<HouseLastPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedPeriodicLastPosition {
    pub write: PeriodicLastPosition,
    pub house: Option<HouseLastPosition>,
}

impl DurabilityRoot {
    /// The same bounded batch/transaction as Channel writes; each entry keeps its actual scope.
    pub async fn write_scoped_character_last_positions(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof, writes: Vec<ScopedPeriodicLastPosition>,
    ) -> Result<Vec<Result<LastPositionWrite>>> {
        if writes.len() > LAST_POSITION_WRITE_BATCH { return Err(CharacterProgressionError::InvalidInput); }
        if writes.is_empty() { return Ok(Vec::new()); }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let mut outcomes = Vec::with_capacity(writes.len());
            for entry in writes {
                let Some(house) = entry.house else {
                    outcomes.push(write_periodic(&mut tx, &node, entry.write).await?);
                    continue;
                };
                let write = entry.write;
                if write.write_sequence == 0 || write.fence.character_lease_generation == 0
                    || house.house.world_id() != write.record.world_id
                    || house.house.runtime_scope()? != write.fence.runtime_scope
                { outcomes.push(Err(CharacterProgressionError::InvalidInput)); continue; }
                if super::super::house_hireling::fence::assert_house_custody_fence(
                    &mut tx, &node, &write.fence.item_fence(), *house.origin_channel.as_bytes(),
                ).await?.as_ref() != Some(&house.house) { outcomes.push(Err(CharacterProgressionError::AuthorityRejected)); continue; }
                outcomes.push(Ok(upsert_scoped_last_position(&mut tx,
                    write.fence.character_id.as_bytes(), LastPositionKey {
                        character_lease_generation: write.fence.character_lease_generation,
                        write_sequence: write.write_sequence,
                    }, &write.record, Some(&house)).await?));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(outcomes)
        })).await?)
    }

    /// A fenced admission read from the same row, retaining its named-House metadata.
    pub async fn read_scoped_character_last_position(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof, fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<ScopedLastPosition>> {
        if fence.character_lease_generation == 0 || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_session_fence(&mut tx, &fence.item_fence(), &node).await? {
                return Ok(Err(error));
            }
            // Existing respawn/arrival placement outranks every saved position.
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let stored = if pending { None } else { load_scoped_last_position(&mut tx, fence.character_id).await? };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(stored))
        })).await?
    }

    /// History only. A caller must independently prove current session, ACL, assignment and
    /// current content/collision before reconstructing a House actor from this projection.
    pub async fn read_saved_house_position(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, character: CharacterId,
    ) -> Result<Option<ScopedLastPosition>> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let stored = load_scoped_last_position(&mut tx, character).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(stored.filter(|s| s.house.is_some()))
        })).await?)
    }
}

pub(in crate::durability) async fn load_scoped_last_position(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, character_id: CharacterId,
) -> std::result::Result<Option<ScopedLastPosition>, DurabilityError> {
    let row = sqlx::query("SELECT world_id::text, tile_x, tile_y, tile_floor, coordinate_frame_digest, \
        map_revision_digest, server_artifact_digest, character_lease_generation::text, write_sequence::text, \
        health,mana,rest,house_key,origin_channel_id::text,origin_tile_x,origin_tile_y,origin_tile_floor \
        FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let position = decode_row(&row)?;
        let house = match row.try_get::<Option<String>,_>("house_key")? {
            None => None,
            Some(key) => Some(HouseLastPosition {
                house: HouseId::new(position.record.world_id, &key).ok_or(DurabilityError::InvalidStoredState)?,
                origin_channel: ChannelId::decode(&super::super::character_progression::uuid_text(
                    &row.try_get::<String,_>("origin_channel_id")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                origin: LastPositionCell { x: row.try_get("origin_tile_x")?, y: row.try_get("origin_tile_y")?, floor: row.try_get("origin_tile_floor")? },
            }),
        };
        Ok(ScopedLastPosition { position, house })
    }).transpose()
}

/// The original ordered upsert extended with scope metadata in the same row, never a second ledger.
pub(in crate::durability) async fn upsert_scoped_last_position(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, character_id: &[u8;16], key: LastPositionKey,
    record: &LastPositionRecord, house: Option<&HouseLastPosition>,
) -> std::result::Result<LastPositionWrite, DurabilityError> {
    if key.character_lease_generation == 0 || key.write_sequence == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
          WHERE character_id = encode($1,'hex')::uuid) OR EXISTS (SELECT 1 FROM game_character_pending_arrivals WHERE character_id = encode($1,'hex')::uuid)",
    )
    .bind(character_id.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(LastPositionWrite::PendingObligation);
    }
    // `LastVitals` bounds both values to `MAX_VITAL_POOL`, below `i32::MAX`.
    let (health, mana) = match record.vitals {
        Some(vitals) => (
            Some(i32::try_from(vitals.health).map_err(|_| DurabilityError::InvalidStoredState)?),
            Some(i32::try_from(vitals.mana).map_err(|_| DurabilityError::InvalidStoredState)?),
        ),
        None => (None, None),
    };
    let written = sqlx::query(
        "INSERT INTO game_character_last_positions AS p \
            (character_id, world_id, tile_x, tile_y, tile_floor, coordinate_frame_digest, \
             map_revision_digest, server_artifact_digest, character_lease_generation, \
             write_sequence, health, mana, rest, house_key, origin_channel_id, origin_tile_x, origin_tile_y, origin_tile_floor) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, $7, $8, \
                 $9::text::numeric(20,0), $10::text::numeric(20,0), $11, $12, $13, $14, encode($15,'hex')::uuid, $16, $17, $18) \
         ON CONFLICT (character_id) DO UPDATE SET \
             world_id = EXCLUDED.world_id, tile_x = EXCLUDED.tile_x, \
             tile_y = EXCLUDED.tile_y, tile_floor = EXCLUDED.tile_floor, \
             coordinate_frame_digest = EXCLUDED.coordinate_frame_digest, \
             map_revision_digest = EXCLUDED.map_revision_digest, \
             server_artifact_digest = EXCLUDED.server_artifact_digest, \
             character_lease_generation = EXCLUDED.character_lease_generation, \
             write_sequence = EXCLUDED.write_sequence, \
             health = EXCLUDED.health, mana = EXCLUDED.mana, rest = EXCLUDED.rest, \
             house_key = EXCLUDED.house_key, origin_channel_id = EXCLUDED.origin_channel_id, \
             origin_tile_x = EXCLUDED.origin_tile_x, origin_tile_y = EXCLUDED.origin_tile_y, \
             origin_tile_floor = EXCLUDED.origin_tile_floor \
         WHERE (p.character_lease_generation, p.write_sequence) \
             < (EXCLUDED.character_lease_generation, EXCLUDED.write_sequence)",
    )
    .bind(character_id.as_slice())
    .bind(record.world_id.as_bytes().as_slice())
    .bind(record.cell.x)
    .bind(record.cell.y)
    .bind(record.cell.floor)
    .bind(record.coordinate_frame_digest.as_slice())
    .bind(record.map_revision_digest.as_slice())
    .bind(record.server_artifact_digest.as_slice())
    .bind(key.character_lease_generation.to_string())
    .bind(key.write_sequence.to_string())
    .bind(health)
    .bind(mana)
    .bind(
        record
            .rest
            .map(|rest| {
                rest.into_iter()
                    .map(i32::try_from)
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )
    .bind(house.map(|h| h.house.house_key()))
    .bind(house.map(|h| h.origin_channel.as_bytes().as_slice()))
    .bind(house.map(|h| h.origin.x))
    .bind(house.map(|h| h.origin.y))
    .bind(house.map(|h| h.origin.floor))
    .execute(&mut **tx)
    .await?;
    Ok(if written.rows_affected() == 1 {
        LastPositionWrite::Written
    } else {
        LastPositionWrite::Superseded
    })
}

/// A terminal session can no longer occupy its original native entry reservation.
/// Keep the original handoff owner and same transaction; stale ownership leaves
/// cleanup to native stale-scope recovery, never holds Account/Character claims.
pub(in crate::durability) async fn release_terminal_house_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, node: &NodeIncarnationProof,
    session: crate::foundation::GameSessionId, character: crate::foundation::CharacterId, at:i64,
) -> std::result::Result<(), DurabilityError> {
    use super::super::house_scope_handoff::{HouseHandoffError, release_house_entry_tile_in_transaction};
    let id: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(handoff_id) FROM game_house_scope_handoffs \
        WHERE destination_game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND state=2")
        .bind(session.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(id) = id else { return Ok(()); };
    let id = id.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
    match release_house_entry_tile_in_transaction(tx,node,id,at).await {
        Ok(_) | Err(HouseHandoffError::AuthorityRejected) => Ok(()),
        Err(HouseHandoffError::Unavailable(error)) => Err(error),
        Err(_) => Err(DurabilityError::InvalidStoredState),
    }
}

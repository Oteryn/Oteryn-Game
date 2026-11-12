use super::*;
/// Called after the terminal writer revalidates the exact session and before it
/// commits TERMINAL. A missing accepted cut or unknown custody is unavailable.
pub(in crate::durability) async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    catalogue: Option<&LogoutCatalogue>,
    at: i64,
) -> Result<Capture> {
    let configured:Option<(bool,Option<String>)>=sqlx::query_as("SELECT game_journal_items_export_enabled,source_authority FROM game_journal_export_settings WHERE singleton")
  .fetch_optional(&mut **tx).await?;
    let Some((true, Some(source))) = configured else {
        return Ok(Capture::Disabled);
    };
    let Some(catalogue) = catalogue else {
        return Ok(Capture::Unavailable);
    };
    if catalogue.world != current.current_runtime_scope().world_id() {
        return Ok(Capture::Unavailable);
    }
    let session = current.commit().game_session_id();
    let root=sqlx::query("SELECT r.character_id::text,r.world_id::text,r.account_id::text,r.character_revision::text FROM game_character_roots r JOIN game_durability_reconnect_sessions s ON s.character_id=r.character_id AND s.account_id=r.account_id WHERE s.game_session_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid AND r.lifecycle=1 AND s.session_state IN(1,2) AND s.current_generation=$3::text::numeric AND s.character_lease_generation=$4::text::numeric AND s.scope_ownership_generation=$5::text::numeric FOR UPDATE OF r")
  .bind(session.as_bytes().as_slice()).bind(catalogue.world.as_bytes().as_slice())
  .bind(current.current_connection_generation().get().to_string()).bind(current.current_character_lease().generation().to_string())
  .bind(current.current_scope_generation().get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(root) = root else {
        return Ok(Capture::Unavailable);
    };
    let character: String = root.try_get("character_id")?;
    let account: String = root.try_get("account_id")?;
    let world: String = root.try_get("world_id")?;
    let revision: String = root.try_get("character_revision")?;
    // A partially relocated custody graph is unavailable, not an empty World.
    let foreign_world:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_slots WHERE character_id=$1::text::uuid AND world_id<>$2::text::uuid UNION ALL SELECT 1 FROM game_character_equipment_slots WHERE character_id=$1::text::uuid AND world_id<>$2::text::uuid UNION ALL SELECT 1 FROM game_item_character_depot_locations WHERE character_id=$1::text::uuid AND world_id<>$2::text::uuid UNION ALL SELECT 1 FROM game_item_character_inbox_locations WHERE character_id=$1::text::uuid AND world_id<>$2::text::uuid UNION ALL SELECT 1 FROM game_item_container_entries WHERE character_id=$1::text::uuid AND world_id<>$2::text::uuid)")
        .bind(&character).bind(&world).fetch_one(&mut **tx).await?;
    if foreign_world {
        return Ok(Capture::Unavailable);
    }
    // Access/ownership or reclaim provenance alone is not an accepted house export
    // right. Until that mapping is qualified, withhold the entire affected snapshot.
    let unknown_house: bool =
        sqlx::query_scalar("SELECT game_journal_unknown_house($1::text::uuid)")
            .bind(&character)
            .fetch_one(&mut **tx)
            .await?;
    if unknown_house {
        return Ok(Capture::Unavailable);
    }
    let generation:Option<String>=sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE")
  .fetch_optional(&mut **tx).await?;
    let Some(generation) = generation else {
        return Ok(Capture::Unavailable);
    };
    // Registration is a separate control operation requiring the accepted transfer
    // receipt or baseline. The logout never synthesizes receipt members.
    let streams:Vec<Vec<u8>>=sqlx::query_scalar("SELECT stream_key FROM game_journal_streams WHERE active AND source_authority=$1 AND account_id=$2::text::uuid AND recovery_generation=$3::text::numeric AND character_id=$4::text::uuid AND (convert_from(stream_key,'UTF8')::jsonb->>'character_revision')::numeric<=$5::text::numeric AND family='ITEMS' ORDER BY stream_key LIMIT 2")
  .bind(source).bind(&account).bind(&generation).bind(&character).bind(&revision).fetch_all(&mut **tx).await?;
    let [key] = streams.as_slice() else {
        return Ok(Capture::Unavailable);
    };
    let stream = ReadGameJournalCursorV1::decode(key).map_err(invalid)?;
    if stream.world_id != world || !super::ownership::approved_cut(tx, &stream).await? {
        return Ok(Capture::Unavailable);
    }
    let first: String = sqlx::query_scalar("SELECT game_journal_lock_stream($1)")
        .bind(key)
        .fetch_one(&mut **tx)
        .await?;
    let Some(first) = oteryn_gameplay_native_admission_source::character_commands::counter(&first)
    else {
        return Ok(Capture::Unavailable);
    };
    let rows = sqlx::query(include_str!("snapshot.sql"))
        .bind(&character)
        .bind(&world)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() > 524288 {
        return Ok(Capture::Unavailable);
    }
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let depth: i32 = row.try_get("depth")?;
        let cycle: bool = row.try_get("cycle")?;
        let lifecycle: i16 = row.try_get("lifecycle")?;
        let item_world: String = row.try_get("item_world_id")?;
        if depth > 32 || cycle || lifecycle != 1 || item_world != world {
            return Ok(Capture::Unavailable);
        }
        let quantity: i64 = row.try_get("quantity")?;
        let town: Option<i32> = row.try_get("town_id")?;
        let slot: Option<i16> = row.try_get("equipment_slot")?;
        let depot_box: Option<i16> = row.try_get("depot_box")?;
        let (Ok(quantity), Ok(town_id), Ok(equipment_slot), Ok(depot_box)) = (
            u32::try_from(quantity),
            town.map(u16::try_from).transpose(),
            slot.map(u8::try_from).transpose(),
            depot_box.map(u8::try_from).transpose(),
        ) else {
            return Ok(Capture::Unavailable);
        };
        if town_id.is_some_and(|town| !catalogue.depots.contains(&town)) {
            return Ok(Capture::Unavailable);
        }
        items.push(ItemRow {
            item_instance_id: row.try_get("item_instance_id")?,
            definition_family: row.try_get("definition_family")?,
            definition_production_key: row.try_get("definition_production_key")?,
            definition_revision_ref: row.try_get("definition_revision_ref")?,
            quantity,
            root: row.try_get("root")?,
            town_id,
            depot_box,
            house_id: None,
            spatial_position_hex: None,
            equipment_slot,
            parent_item_instance_id: row.try_get("parent_item_instance_id")?,
            placement_ordinal: row.try_get("placement_ordinal")?,
        });
    }
    items.sort_unstable_by(|a, b| a.item_instance_id.cmp(&b.item_instance_id));
    if !items
        .windows(2)
        .all(|v| v[0].item_instance_id < v[1].item_instance_id)
    {
        return Ok(Capture::Unavailable);
    }
    let snapshot: String = sqlx::query_scalar("SELECT uuidv7()::text")
        .fetch_one(&mut **tx)
        .await?;
    let Some(payloads) = payloads(&revision, &snapshot, items, &catalogue.depots)? else {
        return Ok(Capture::Unavailable);
    };
    let count = payloads.len();
    if first
        .checked_add(u64::try_from(count - 1).map_err(invalid)?)
        .is_none()
    {
        return Ok(Capture::Unavailable);
    }
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT uuidv7()::text FROM generate_series(1,$1::integer) n ORDER BY n",
    )
    .bind(i32::try_from(count).map_err(invalid)?)
    .fetch_all(&mut **tx)
    .await?;
    let mut records = Vec::with_capacity(count);
    for (offset, (payload, id)) in payloads.into_iter().zip(ids).enumerate() {
        let kind = match &payload {
            ItemsPayload::Begin(_) => "ITEMS_BEGIN",
            ItemsPayload::Page(_) => "ITEMS_PAGE",
            ItemsPayload::End(_) => "ITEMS_END",
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let record = PublishGameJournalRecordV1 {
            contract_version: 1,
            operation: "PublishGameJournalRecordV1".into(),
            source_authority: stream.source_authority.clone(),
            account_id: stream.account_id.clone(),
            world_id: stream.world_id.clone(),
            character_id: stream.character_id.clone(),
            character_revision: stream.character_revision.clone(),
            from_projection_revision: stream.from_projection_revision.clone(),
            to_projection_revision: stream.to_projection_revision.clone(),
            projection_epoch: stream.projection_epoch.clone(),
            family: stream.family,
            recovery_generation: stream.recovery_generation.clone(),
            source_revision: (first + u64::try_from(offset).map_err(invalid)?).to_string(),
            record_id: id,
            occurred_at: at.to_string(),
            coverage_start_at: at.to_string(),
            kind: kind.into(),
            payload,
        };
        let Ok(bytes) = record.encode() else {
            return Ok(Capture::Unavailable);
        };
        records.push(bytes);
    }
    for bytes in records {
        let _: bool = sqlx::query_scalar("SELECT game_journal_enqueue($1,$2)")
            .bind(key)
            .bind(bytes)
            .fetch_one(&mut **tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_journal_logout_cuts(game_session_id,stream_key,snapshot_id,occurred_at,first_revision,record_count,character_revision_at_logout) VALUES(encode($1,'hex')::uuid,$2,$3::text::uuid,$4,$5::text::numeric,$6,$7::text::numeric)")
  .bind(session.as_bytes().as_slice()).bind(key).bind(snapshot).bind(at).bind(first.to_string()).bind(i32::try_from(count).map_err(invalid)?).bind(revision).execute(&mut **tx).await?;
    Ok(Capture::Queued)
}
fn payloads(
    revision: &str,
    id: &str,
    items: Vec<ItemRow>,
    depots: &[u16],
) -> Result<Option<Vec<ItemsPayload>>> {
    let item_count = u32::try_from(items.len()).map_err(invalid)?;
    let mut pages = Vec::new();
    let mut rows = Vec::new();
    let mut size = 0usize;
    for row in items {
        let bytes = serde_json::to_vec(&row).map_err(invalid)?;
        // A cursor is <=1024 bytes; the additional publish members plus the page
        // wrapper fit within the reserved 2048 bytes, even with uint64 counters.
        if bytes.len() > 65536 - 2048 {
            return Ok(None);
        }
        if rows.len() == 32 || size + bytes.len() + 1 > 65536 - 2048 {
            pages.push(ItemsPage {
                snapshot_id: id.into(),
                page_index: u32::try_from(pages.len()).map_err(invalid)?,
                items: std::mem::take(&mut rows),
            });
            size = 0;
        }
        size += bytes.len() + 1;
        rows.push(row);
    }
    if !rows.is_empty() {
        pages.push(ItemsPage {
            snapshot_id: id.into(),
            page_index: u32::try_from(pages.len()).map_err(invalid)?,
            items: rows,
        });
    }
    if pages.len() > 16384 {
        return Ok(None);
    }
    let page_count = u32::try_from(pages.len()).map_err(invalid)?;
    let mut hash = Sha256::new();
    for page in &pages {
        hash.update(serde_json::to_vec(page).map_err(invalid)?);
    }
    let digest = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut result = Vec::with_capacity(pages.len() + 2);
    result.push(ItemsPayload::Begin(ItemsBegin {
        snapshot_id: id.into(),
        character_revision: revision.into(),
        page_count,
        item_count,
        depots: depots.to_vec(),
        houses: Vec::new(),
    }));
    result.extend(pages.into_iter().map(ItemsPayload::Page));
    result.push(ItemsPayload::End(ItemsEnd {
        snapshot_id: id.into(),
        page_count,
        item_count,
        pages_sha256: digest,
    }));
    Ok(Some(result))
}

#[cfg(test)]
mod tests;

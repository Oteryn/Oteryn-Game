//! HOUSE-1b disposition custody, never a player-to-player house sale.
//!
//! Migration 0121 installs the private definer and its custody proof through the native
//! MIGRATOR. It calls real INBOX-1a delivery with all native guards enabled. The Root caller
//! supplies current recovery/admission/node/assignment checks; PUBLIC/runtime receive no
//! direct definer or table mutation grant. The registered House event, production caller
//! permissions and content projection revision remain integration gates.

use super::move_out::{
    self as owner, HouseMoveOutError as E, HouseMoveOutOutcome, Result, ReturnedHouseItem,
};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

async fn returned(
    tx: &mut Transaction<'_, Postgres>,
    operation: [u8; 16],
    step: u64,
) -> Result<Vec<ReturnedHouseItem>> {
    let rows=sqlx::query("SELECT uuid_send(item_instance_id) AS item,uuid_send((source_provenance->>'reclaim_subject_character_id')::uuid) AS subject,ordinal::text FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step=$2::text::numeric(20,0) ORDER BY item_instance_id")
        .bind(operation.as_slice()).bind(step.to_string()).fetch_all(&mut **tx).await?;
    rows.iter()
        .map(|row| {
            Ok(ReturnedHouseItem {
                item: owner::bytes(row, "item")?,
                reclaim_character: owner::bytes(row, "subject")?,
                ordinal: owner::number(row, "ordinal")?,
            })
        })
        .collect()
}

pub(crate) async fn step(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    operation: [u8; 16],
    house: &HouseId,
    execution_scope: RuntimeScopeRefV1,
    generation: u64,
    step: u64,
) -> Result<HouseMoveOutOutcome> {
    let step_items = i64::try_from(owner::DISPOSITION_STEP_ITEMS).map_err(|_| E::InvalidCustody)?;
    // Native admission relation EXCLUSIVE locks serialize cursor writers. Resolve the actual
    // immutable census subjects and protect roots before the property, as HOUSE-OWN0 §9
    // requires; neither a caller-selected recipient nor a provenance cache is used.
    sqlx::query("SELECT r.character_id FROM game_character_roots r WHERE r.character_id IN(SELECT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT $2) ORDER BY r.character_id FOR KEY SHARE")
        .bind(operation.as_slice()).bind(step_items).fetch_all(&mut **tx).await?;
    // Same ordering as freeze and the native-location fence trigger.
    sqlx::query("SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE").bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).execute(&mut **tx).await?;
    let row = owner::load(tx, operation, house).await?;
    let next = owner::number(&row, "next_step")?;
    if step < next {
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_disposition_steps WHERE operation_id=encode($1,'hex')::uuid AND step=$2::text::numeric(20,0))").bind(operation.as_slice()).bind(step.to_string()).fetch_one(&mut **tx).await?;
        if !exists {
            return Err(E::InvalidCustody);
        }
        // Retained outcome only: no new grant or mutation, even after authority changed.
        return Ok(HouseMoveOutOutcome::Step {
            view: owner::view(&row)?,
            items: returned(tx, operation, step).await?,
            replayed: true,
        });
    }
    if row.try_get::<i16, _>("state")? == 4 {
        return Err(E::Cancelled);
    }
    if step != next || row.try_get::<i16, _>("state")? == 3 {
        return Err(E::UnexpectedStep);
    }
    if row.try_get::<i16, _>("state")? != 2 {
        return Err(E::NotFrozen);
    }
    owner::current_world_job(tx, node, house, execution_scope, generation).await?;
    owner::current_owner(tx, &row, house).await?;
    let occupied:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE runtime_scope_kind=2 AND runtime_scope_world_id=encode($1,'hex')::uuid AND runtime_scope_house_key=$2 AND session_state IN(1,2)) OR EXISTS(SELECT 1 FROM game_house_scope_handoffs WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND state IN(1,2))")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).fetch_one(&mut **tx).await?;
    if occupied {
        return Err(E::Occupied);
    }
    // Validate the complete frozen manifest, rather than trusting a caller's cursor or count.
    let all=sqlx::query("SELECT uuid_send(item_instance_id) AS item FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid ORDER BY item_instance_id").bind(operation.as_slice()).fetch_all(&mut **tx).await?;
    let mut digest = Sha256::new();
    for item in &all {
        digest.update(owner::bytes(item, "item")?);
    }
    let digest: [u8; 32] = digest.finalize().into();
    let snapshot = owner::view(&row)?;
    if snapshot.frozen_count != Some(all.len() as u64) || snapshot.frozen_digest != Some(digest) {
        return Err(E::InvalidCustody);
    }
    let items=sqlx::query("SELECT uuid_send(item_instance_id) AS item FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT $2 FOR UPDATE")
        .bind(operation.as_slice()).bind(step_items).fetch_all(&mut **tx).await?;
    // Item locks precede counters in the accepted order; avoid opposite item orders across
    // subjects when each SQL delivery later reacquires its already-held item/root lock.
    for item in &items {
        sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR NO KEY UPDATE").bind(owner::bytes(item,"item")?.as_slice()).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_character_inbox_counters(character_id) SELECT DISTINCT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND item_instance_id IN(SELECT item_instance_id FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT $2) ORDER BY 1 ON CONFLICT DO NOTHING")
        .bind(operation.as_slice()).bind(step_items).execute(&mut **tx).await?;
    sqlx::query("SELECT character_id FROM game_character_inbox_counters WHERE character_id IN(SELECT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT $2) ORDER BY character_id FOR UPDATE")
        .bind(operation.as_slice()).bind(step_items).fetch_all(&mut **tx).await?;
    for item in &items {
        sqlx::query("SELECT * FROM game_house_disposition_deliver(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid)")
            .bind(operation.as_slice()).bind(step.to_string()).bind(owner::bytes(item,"item")?.as_slice()).fetch_one(&mut **tx).await?;
    }
    let extra:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 AND NOT EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=encode($3,'hex')::uuid AND c.item_instance_id=h.item_instance_id AND c.moved_step IS NULL))")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).bind(operation.as_slice()).fetch_one(&mut **tx).await?;
    if extra {
        return Err(E::InvalidCustody);
    }
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL").bind(operation.as_slice()).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_house_disposition_steps(operation_id,step,moved_count) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),$3)")
        .bind(operation.as_slice()).bind(step.to_string()).bind(items.len() as i64).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_house_dispositions SET next_step=next_step+1,state=CASE WHEN $2=0 THEN 3 ELSE 2 END WHERE operation_id=encode($1,'hex')::uuid")
        .bind(operation.as_slice()).bind(remaining).execute(&mut **tx).await?;
    Ok(HouseMoveOutOutcome::Step {
        view: owner::view(&owner::load(tx, operation, house).await?)?,
        items: returned(tx, operation, step).await?,
        replayed: false,
    })
}

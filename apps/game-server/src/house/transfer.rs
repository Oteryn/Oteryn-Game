//! HOUSE-1b disposition custody, never a player-to-player house sale.
//!
//! The private definer calls the real INBOX-1a function with all native guards enabled. The
//! Root caller supplies current recovery/admission/node/assignment checks; PUBLIC/runtime
//! receive no direct EXECUTE or table grants in this uninstalled design. Registered house
//! event and content projection revision remain production integration gates.

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
    // Native admission relation EXCLUSIVE locks serialize cursor writers. Resolve the actual
    // immutable census subjects and protect roots before the property, as HOUSE-OWN0 §9
    // requires; neither a caller-selected recipient nor a provenance cache is used.
    sqlx::query("SELECT r.character_id FROM game_character_roots r WHERE r.character_id IN(SELECT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT 100) ORDER BY r.character_id FOR KEY SHARE")
        .bind(operation.as_slice()).fetch_all(&mut **tx).await?;
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
    let items=sqlx::query("SELECT uuid_send(item_instance_id) AS item FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT 100 FOR UPDATE")
        .bind(operation.as_slice()).fetch_all(&mut **tx).await?;
    // Item locks precede counters in the accepted order; avoid opposite item orders across
    // subjects when each SQL delivery later reacquires its already-held item/root lock.
    for item in &items {
        sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR NO KEY UPDATE").bind(owner::bytes(item,"item")?.as_slice()).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_character_inbox_counters(character_id) SELECT DISTINCT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND item_instance_id IN(SELECT item_instance_id FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT 100) ORDER BY 1 ON CONFLICT DO NOTHING")
        .bind(operation.as_slice()).execute(&mut **tx).await?;
    sqlx::query("SELECT character_id FROM game_character_inbox_counters WHERE character_id IN(SELECT (source_provenance->>'reclaim_subject_character_id')::uuid FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid AND moved_step IS NULL ORDER BY item_instance_id LIMIT 100) ORDER BY character_id FOR UPDATE")
        .bind(operation.as_slice()).fetch_all(&mut **tx).await?;
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

/// Private definer and commit-time native-source/Inbox/step proofs; not a production grant.
pub const PRIVATE_DISPOSITION_TRANSFER_SCHEMA: &str = r#"
CREATE FUNCTION game_house_disposition_census_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_dispositions o
  JOIN game_item_house_interior_locations h ON h.item_instance_id=NEW.item_instance_id AND h.world_id=o.world_id AND h.house_key=o.house_key
  JOIN game_item_house_reclaim_provenance p ON p.item_instance_id=h.item_instance_id
  JOIN game_item_instances i ON i.item_instance_id=h.item_instance_id
  WHERE o.operation_id=NEW.operation_id AND o.state=1 AND NEW.moved_step IS NULL AND NEW.ordinal IS NULL
   AND to_jsonb(h)=NEW.source_location AND to_jsonb(p)=NEW.source_provenance AND to_jsonb(i)=NEW.source_item
   AND p.world_id=h.world_id AND p.house_key=h.house_key AND p.placement_transaction_id=h.placed_transaction_id)
 THEN RAISE EXCEPTION 'census must capture actual native source/provenance' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER house_disposition_census_proven BEFORE INSERT ON game_house_disposition_items FOR EACH ROW EXECUTE FUNCTION game_house_disposition_census_proven();
CREATE FUNCTION game_house_disposition_operation_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE n BIGINT;d BYTEA;
BEGIN
 IF NEW.state=4 THEN RETURN NULL; END IF;
 SELECT count(*),sha256(COALESCE(string_agg(uuid_send(item_instance_id),''::bytea ORDER BY item_instance_id),''::bytea)) INTO n,d FROM game_house_disposition_items WHERE operation_id=NEW.operation_id;
 IF n<>NEW.frozen_count OR d<>NEW.frozen_digest THEN RAISE EXCEPTION 'frozen census count/digest changed' USING ERRCODE='23514'; END IF;
 IF OLD.state=1 THEN
  IF EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.world_id=NEW.world_id AND h.house_key=NEW.house_key AND NOT EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.item_instance_id=h.item_instance_id)) OR
     EXISTS(SELECT 1 FROM game_house_disposition_items c LEFT JOIN game_item_house_interior_locations h USING(item_instance_id) LEFT JOIN game_item_house_reclaim_provenance p USING(item_instance_id) LEFT JOIN game_item_instances i USING(item_instance_id) WHERE c.operation_id=NEW.operation_id AND (to_jsonb(h) IS DISTINCT FROM c.source_location OR to_jsonb(p) IS DISTINCT FROM c.source_provenance OR to_jsonb(i) IS DISTINCT FROM c.source_item))
   THEN RAISE EXCEPTION 'frozen set must equal native source set' USING ERRCODE='23514'; END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_house_disposition_steps s WHERE s.operation_id=NEW.operation_id AND s.step=OLD.next_step AND s.created_xact_id=pg_current_xact_id()
    AND s.moved_count=(SELECT count(*) FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.moved_step=s.step AND c.moving_xact_id=s.created_xact_id)) OR
    ((NEW.state=3) <> NOT EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=NEW.operation_id AND c.moved_step IS NULL))
   THEN RAISE EXCEPTION 'step lacks same-transaction census evidence' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER house_disposition_operation_proven AFTER UPDATE ON game_house_dispositions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_operation_proven();
CREATE FUNCTION game_house_disposition_deliver(op UUID,st NUMERIC,it UUID) RETURNS TABLE(character_id UUID,ordinal NUMERIC)
LANGUAGE plpgsql SECURITY DEFINER AS $$
#variable_conflict use_column
DECLARE c game_house_disposition_items%ROWTYPE;o game_house_dispositions%ROWTYPE;subject UUID;dest RECORD;
BEGIN
 SELECT * INTO o FROM game_house_dispositions WHERE operation_id=op FOR UPDATE;
 IF NOT FOUND OR o.state<>2 OR o.next_step<>st THEN RAISE EXCEPTION 'disposition step not current' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_house_ownership h WHERE h.world_id=o.world_id AND h.house_key=o.house_key AND h.owner_character_id=o.owner_character_id AND h.ownership_revision=o.ownership_revision FOR SHARE)
 THEN RAISE EXCEPTION 'disposition native ownership changed' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_kind=2 AND s.runtime_scope_world_id=o.world_id AND s.runtime_scope_house_key=o.house_key AND s.session_state IN(1,2)) OR
    EXISTS(SELECT 1 FROM game_house_scope_handoffs h WHERE h.world_id=o.world_id AND h.house_key=o.house_key AND h.state IN(1,2))
 THEN RAISE EXCEPTION 'disposition has canonical sessions or entrants' USING ERRCODE='23514'; END IF;
 SELECT * INTO c FROM game_house_disposition_items WHERE operation_id=op AND item_instance_id=it FOR UPDATE;
 IF NOT FOUND OR c.moved_step IS NOT NULL OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h
   JOIN game_item_house_reclaim_provenance p USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
   WHERE h.item_instance_id=it AND h.world_id=o.world_id AND h.house_key=o.house_key AND to_jsonb(h)=c.source_location AND to_jsonb(p)=c.source_provenance AND to_jsonb(i)=c.source_item)
 THEN RAISE EXCEPTION 'disposition native source/provenance changed' USING ERRCODE='23514'; END IF;
 subject:=(c.source_provenance->>'reclaim_subject_character_id')::uuid;
 UPDATE game_house_disposition_items SET moved_step=st,moving_xact_id=pg_current_xact_id() WHERE operation_id=op AND item_instance_id=it;
 DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=it;
 DELETE FROM game_item_house_interior_locations WHERE item_instance_id=it;
 SELECT * INTO dest FROM game_character_inbox_deliver(it,subject,o.world_id,'HOUSE_DISPOSITION',op::text);
 UPDATE game_house_disposition_items SET ordinal=dest.ordinal WHERE operation_id=op AND item_instance_id=it;
 RETURN QUERY SELECT dest.character_id,dest.ordinal;
END $$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_disposition_census_proven','game_house_disposition_operation_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_disposition_deliver(UUID,NUMERIC,UUID) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_disposition_deliver(UUID,NUMERIC,UUID) FROM PUBLIC;
"#;

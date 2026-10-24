//! Fixture-only occupancy, created and removed between two real wire exchanges.
use super::*;

pub(super) async fn set(harness: &Harness, index: &ItemDefinitionIndex, full: bool) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for seed in 130..149 {
        if full {
            let key = "oteryn:item.tibia.i3577";
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).expect("meat revision"))
                .bind(id(seed+50).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice())
                .bind(i64::from(seed-128)).bind(id(seed+50).as_slice()).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(seed).as_slice()).execute(&mut *tx).await?;
            sqlx::query(
                "DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(seed).as_slice())
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

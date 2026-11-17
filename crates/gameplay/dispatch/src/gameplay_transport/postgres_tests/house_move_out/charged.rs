//! The actual sale-completion job wraps a partly used cask into depot custody.
use super::*;
pub(super) async fn seed(h: &Harness, original: [u8; 16]) -> TestResult<[u8; 16]> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let item: Vec<u8>=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),world_id,'Item','oteryn:item.tibia.i25880','definition-r1',1,1,game_character_uuid_v7() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid RETURNING uuid_send(item_instance_id)").bind(original.as_slice()).fetch_one(&mut *tx).await?;
    let placement: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations SELECT encode($1,'hex')::uuid,world_id,house_key,spatial_position,stack_ordinal+100,encode($3,'hex')::uuid FROM game_item_house_interior_locations WHERE item_instance_id=encode($2,'hex')::uuid").bind(&item).bind(original.as_slice()).bind(&placement).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance SELECT encode($1,'hex')::uuid,world_id,house_key,reclaim_subject_character_id,encode($3,'hex')::uuid,provenance_revision FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($2,'hex')::uuid").bind(&item).bind(original.as_slice()).bind(&placement).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,3,1)").bind(&item).execute(&mut *tx).await?;
    tx.commit().await?;
    item.try_into()
        .map_err(|_| "charged furniture identity".into())
}
pub(super) async fn delivered(h: &Harness, item: [u8; 16]) -> TestResult {
    let actual:(String,i32,String)=sqlx::query_as("SELECT i.definition_production_key,t.charges,t.state_revision::text FROM game_item_instances i JOIN game_item_character_inbox_locations l USING(item_instance_id,world_id) JOIN game_item_timed_states t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND l.character_id=encode($2,'hex')::uuid").bind(item.as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actual, ("oteryn:item.tibia.i23398".into(), 3, "1".into()));
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let target = h
        .root
        .house_furniture_kit_target(&authority, item)
        .await?
        .ok_or("cask unwrap form")?;
    assert_eq!(target.production_key, "oteryn:item.tibia.i25880");
    Ok(())
}

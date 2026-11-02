//! Same-transaction fulfilment hooks for the existing Store owner.
use super::super::DurabilityError;
use super::model::{HIRELING_LAMP_KEY,MAX_HIRELINGS};
use sqlx::{Postgres,Transaction};

pub(crate) async fn grant_store_hireling(
    tx:&mut Transaction<'_,Postgres>,character:[u8;16],id:[u8;16],lamp:[u8;16],name:&str,sex:u8,
) -> Result<(),DurabilityError> {
    if !(1..=2).contains(&sex) || !(3..=18).contains(&name.len()) || !name.is_ascii() {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_hirelings WHERE owner_character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    if count>=MAX_HIRELINGS as i64{return Err(DurabilityError::InvalidStoredState)}
    let minted:bool=sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_instances i \
          JOIN game_item_character_inbox_locations l USING(item_instance_id,world_id) \
          WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.definition_family='Item' \
            AND i.definition_production_key=$2 \
            AND i.quantity=1 AND i.lifecycle=1 AND l.character_id=encode($3,'hex')::uuid)",
    ).bind(lamp.as_slice()).bind(HIRELING_LAMP_KEY).bind(character.as_slice()).fetch_one(&mut **tx).await?;
    if !minted{return Err(DurabilityError::InvalidStoredState)}
    sqlx::query("INSERT INTO game_house_hirelings(hireling_id,world_id,owner_character_id,name,sex,look_type,active,house_key,x,y,floor,lamp_item_instance_id,legacy_grant) \
        SELECT encode($1,'hex')::uuid,world_id,character_id,$3,$4,$5,FALSE,NULL,0,0,0,encode($6,'hex')::uuid,FALSE \
        FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
        .bind(id.as_slice()).bind(character.as_slice()).bind(name).bind(i16::from(sex))
        .bind(if sex==2 {136_i32}else{128_i32}).bind(lamp.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(crate) async fn grant_store_hireling_entitlement(
    tx:&mut Transaction<'_,Postgres>,character:[u8;16],entitlement:u16,
) -> Result<(),DurabilityError> {
    if !(1001..=1004).contains(&entitlement) && !(2001..=2009).contains(&entitlement) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let has_hireling:bool=sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_house_hirelings WHERE owner_character_id=encode($1,'hex')::uuid)",
    ).bind(character.as_slice()).fetch_one(&mut **tx).await?;
    if !has_hireling{return Err(DurabilityError::InvalidStoredState)}
    sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,$2)")
        .bind(character.as_slice()).bind(entitlement as i16).execute(&mut **tx).await?;
    Ok(())
}

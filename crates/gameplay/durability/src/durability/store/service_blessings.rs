use super::DeliveryIdentity;
use super::codec::uuid_string;
use sqlx::{Postgres, Transaction};

// Crystal blessing IDs, bound to the existing Game death/NPC catalogue keys.
const BLESSINGS: [&str; 8] = [
    "oteryn:blessing.twist_of_fate",
    "oteryn:blessing.the_wisdom_of_solitude",
    "oteryn:blessing.the_spark_of_the_phoenix",
    "oteryn:blessing.the_fire_of_the_suns",
    "oteryn:blessing.the_spiritual_shielding",
    "oteryn:blessing.the_embrace_of_tibia",
    "oteryn:blessing.heart_of_the_mountain",
    "oteryn:blessing.blood_of_the_mountain",
];
pub(super) fn keys(id: Option<u8>) -> Option<&'static [&'static str]> {
    match id {
        None => Some(&BLESSINGS),
        Some(id) => {
            let index = usize::from(id.checked_sub(1)?);
            BLESSINGS.get(index..=index)
        }
    }
}

pub(super) async fn can_reserve(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    keys: &[&str],
    count: i16,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT NOT EXISTS(
             SELECT 1 FROM game_character_blessings b
             LEFT JOIN game_character_blessing_extra_charges e USING(character_id,blessing_key)
             WHERE b.character_id=encode($1,'hex')::uuid AND b.blessing_key=ANY($2)
               AND 1+coalesce(e.remaining,0)+$3>5)",
    )
    .bind(character.as_slice())
    .bind(keys)
    .bind(count)
    .fetch_one(&mut **tx)
    .await
}

pub(super) async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    identity: &DeliveryIdentity,
    operation: [u8; 16],
    keys: &[&str],
    count: i16,
) -> Result<(), sqlx::Error> {
    // Capture which keys were already held before inserting any missing base charges.
    let added: Vec<String> = sqlx::query_scalar(
        "INSERT INTO game_character_blessings(character_id,blessing_key,provenance)
         SELECT encode($1,'hex')::uuid,requested,$3 FROM unnest($2::text[]) requested
         ON CONFLICT(character_id,blessing_key) DO NOTHING RETURNING blessing_key",
    )
    .bind(identity.character_id.as_slice())
    .bind(keys)
    .bind(uuid_string(&operation))
    .fetch_all(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_blessing_extra_charges(character_id,blessing_key,remaining,grant_operation_id)
         SELECT encode($1,'hex')::uuid,requested,$4-(requested=ANY($3))::integer,encode($5,'hex')::uuid
         FROM unnest($2::text[]) requested WHERE $4>(requested=ANY($3))::integer
         ON CONFLICT(character_id,blessing_key) DO UPDATE
           SET remaining=game_character_blessing_extra_charges.remaining+EXCLUDED.remaining,
               grant_operation_id=EXCLUDED.grant_operation_id",
    )
    .bind(identity.character_id.as_slice())
    .bind(keys)
    .bind(&added)
    .bind(count)
    .bind(operation.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(in crate::durability) async fn consume_extra_charges(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    keys: &[String],
) -> Result<(), sqlx::Error> {
    // One row carries only the charges above the base blessing.
    sqlx::query(
        "DELETE FROM game_character_blessing_extra_charges
         WHERE character_id=encode($1,'hex')::uuid AND blessing_key=ANY($2) AND remaining=1",
    )
    .bind(character.as_slice())
    .bind(keys)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE game_character_blessing_extra_charges SET remaining=remaining-1
         WHERE character_id=encode($1,'hex')::uuid AND blessing_key=ANY($2)",
    )
    .bind(character.as_slice())
    .bind(keys)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

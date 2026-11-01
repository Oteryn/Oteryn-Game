//! PREY-DIFF joins the permanent entitlement to the existing Store receipt/outbox.
//! The Prey owner supplies the current fenced grant and its immutable delivery proof.
use super::codec::{DeliveryReceipt, encode_receipt, uuid_string, uuid_v7};
use crate::durability::DurabilityError;
use sqlx::{Postgres, Row, Transaction};

#[allow(clippy::too_many_arguments)] // The immutable Store binding names every authority dimension.
pub(crate) async fn write_prey_expansion_receipt(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: [u8; 16],
    account: [u8; 16],
    character: [u8; 16],
    world: [u8; 16],
    fulfilment: &str,
    recovery_generation: u64,
) -> Result<DeliveryReceipt, DurabilityError> {
    if [operation_id, account, character, world].iter().any(|id| !uuid_v7(id))
        || fulfilment.is_empty() || fulfilment.len() > 512
        || fulfilment.chars().any(char::is_control) || recovery_generation == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
        .bind(operation_id.as_slice()).execute(&mut **tx).await?;
    if let Some(row) = sqlx::query("SELECT account_id::text,character_id::text,world_id::text,fulfilment_unit_id,entitlement_key,receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid")
        .bind(operation_id.as_slice()).fetch_optional(&mut **tx).await? {
        if row.try_get::<String,_>("account_id")? != uuid_string(&account)
            || row.try_get::<String,_>("character_id")? != uuid_string(&character)
            || row.try_get::<String,_>("world_id")? != uuid_string(&world)
            || row.try_get::<String,_>("fulfilment_unit_id")? != fulfilment
            || row.try_get::<Option<String>,_>("entitlement_key")?.as_deref() != Some("prey-list-expansion") {
            return Err(DurabilityError::InvalidStoredState);
        }
        return serde_json::from_slice(&row.try_get::<Vec<u8>,_>("receipt")?)
            .map_err(|_| DurabilityError::InvalidStoredState);
    }
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
        .fetch_one(&mut **tx).await?;
    let receipt = DeliveryReceipt {
        operation_id: uuid_string(&operation_id),
        account_id: uuid_string(&account),
        character_id: uuid_string(&character),
        world_id: uuid_string(&world),
        fulfilment_unit_id: fulfilment.to_owned(),
        result: "COMMITTED".to_owned(),
        recovery_generation: recovery_generation.to_string(),
        decided_at: now.to_string(),
    };
    let bytes = encode_receipt(&receipt).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,recovery_generation,decided_at,receipt,item_instance_id,entitlement_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,NULL,'prey-list-expansion')")
        .bind(operation_id.as_slice()).bind(account.as_slice()).bind(character.as_slice())
        .bind(world.as_slice()).bind(fulfilment).bind(recovery_generation.to_string())
        .bind(now).bind(bytes).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO store_receipt_outbox(operation_id) VALUES(encode($1,'hex')::uuid)")
        .bind(operation_id.as_slice()).execute(&mut **tx).await?;
    Ok(receipt)
}

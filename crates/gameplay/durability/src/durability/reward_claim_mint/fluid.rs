//! Initial fluid belongs to the existing complete reward MINT transaction.
use super::*;
use sqlx::{Postgres, Transaction};

pub(super) async fn initialize(
    tx: &mut Transaction<'_, Postgres>,
    root: &Reservation,
    item: [u8; 16],
    transaction: [u8; 16],
    subtype: u8,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_item_fluid_reward_mints(item_instance_id,transaction_id,root_transaction_id,fluid_subtype) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
        .bind(item.as_slice()).bind(transaction.as_slice()).bind(root.transaction_id.as_slice())
        .bind(i16::from(subtype)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
        .bind(item.as_slice()).bind(root.world_id.as_slice()).bind(i16::from(subtype))
        .bind(transaction.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

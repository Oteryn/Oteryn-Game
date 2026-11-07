//! One credit check while the caller holds the (Account, World) balance lock.
//! Reserves keep balance + held custody constant; incoming credits must leave
//! enough room for every already owned escrow return.

use super::DurabilityError;
use super::bank_audit::BANK_BALANCE_MAX;
use crate::foundation::WorldId;
use sqlx::PgConnection;

pub(super) async fn credit_in_transaction(
    connection: &mut PgConnection,
    account: &[u8; 16],
    world: WorldId,
    balance: u64,
    incoming: u64,
) -> Result<Option<u64>, DurabilityError> {
    let remaining: i64 = sqlx::query_scalar(
        "SELECT game_account_bank_credit_headroom(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(account.as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_one(connection)
    .await?;
    let remaining = u64::try_from(remaining).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok((incoming <= remaining)
        .then(|| super::bank::credit(balance, incoming, BANK_BALANCE_MAX))
        .flatten())
}

//! Initial fluid belongs to the existing complete reward MINT transaction.
use super::*;
use serde_json::{Value, json};
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
pub(super) fn plan(request: &RewardClaimMintRequest) -> Value {
    Value::Array(
        std::iter::once((0, request.root_fluid_subtype))
            .chain(
                request
                    .contents
                    .iter()
                    .enumerate()
                    .map(|(index, child)| (index + 1, child.fluid_subtype)),
            )
            .filter_map(|(ordinal, subtype)| {
                subtype.map(|subtype| json!({"ordinal": ordinal, "subtype": subtype}))
            })
            .collect(),
    )
}

pub(super) fn validate(request: &RewardClaimMintRequest) -> Result<()> {
    for (facts, quantity, key, text, subtype) in std::iter::once((
        &request.item,
        request.quantity,
        request.key_number,
        request.root_text.as_ref(),
        request.root_fluid_subtype,
    ))
    .chain(request.contents.iter().map(|child| {
        (
            &child.item,
            child.quantity,
            child.key_number,
            child.text.as_ref(),
            child.fluid_subtype,
        )
    })) {
        if subtype.is_some_and(|subtype| {
            subtype == 0
                || quantity != 1
                || facts.stack != ItemStackClass::NonStackable
                || facts.container_capacity.is_some()
                || key.is_some()
                || text.is_some()
        }) {
            return Err(RewardClaimMintError::InvalidInput);
        }
    }
    Ok(())
}

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenClaim,
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    // Canonical contents::apply initializes children once, with its fluid MINT
    // marker. The lane's ordinal-zero plan proves only this root initialization.
    let Some(subtype) = frozen.request.root_fluid_subtype else {
        return Ok(());
    };
    sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
        .bind(frozen.item_instance_id.as_slice()).bind(reservation.world_id.as_slice())
        .bind(i16::from(subtype)).bind(reservation.transaction_id.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

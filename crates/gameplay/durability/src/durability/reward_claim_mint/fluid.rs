//! Source-resolved fluid rewards initialize the existing fluid owner in the
//! once-claim transaction. Empty rewards retain their legacy intent bytes.
use super::*;
use serde_json::{Value, json};

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
    for (ordinal, subtype) in std::iter::once((0, frozen.request.root_fluid_subtype)).chain(
        frozen
            .request
            .contents
            .iter()
            .enumerate()
            .map(|(index, child)| (index + 1, child.fluid_subtype)),
    ) {
        let Some(subtype) = subtype else {
            continue;
        };
        let (item, transaction) = if ordinal == 0 {
            (frozen.item_instance_id, reservation.transaction_id)
        } else {
            (
                contents::child_id(&reservation.transaction_id, ordinal, 3),
                contents::child_id(&reservation.transaction_id, ordinal, 1),
            )
        };
        sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
            .bind(item.as_slice()).bind(reservation.world_id.as_slice())
            .bind(i16::from(subtype)).bind(transaction.as_slice()).execute(&mut **tx).await?;
    }
    Ok(())
}

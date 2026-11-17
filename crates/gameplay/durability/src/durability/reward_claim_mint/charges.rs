//! Explicit source charges initialize the timed owner in the claim's mint.
//! An absent state still means the definition default for legacy rewards.
use super::*;
use serde_json::{Value, json};

pub(super) fn plan(request: &RewardClaimMintRequest) -> Value {
    Value::Array(
        std::iter::once((0, request.root_charges_remaining))
            .chain(request.contents.iter().enumerate().map(|(index, child)| {
                (index + 1, child.charges_remaining)
            }))
            .filter_map(|(ordinal, charges)| {
                charges.map(|charges| json!({"ordinal": ordinal, "charges_remaining": charges}))
            })
            .collect(),
    )
}

pub(super) fn validate(request: &RewardClaimMintRequest) -> Result<()> {
    for (facts, quantity, key, text, fluid, charges) in std::iter::once((
        &request.item,
        request.quantity,
        request.key_number,
        request.root_text.as_ref(),
        request.root_fluid_subtype,
        request.root_charges_remaining,
    ))
    .chain(request.contents.iter().map(|child| {
        (
            &child.item,
            child.quantity,
            child.key_number,
            child.text.as_ref(),
            child.fluid_subtype,
            child.charges_remaining,
        )
    })) {
        if charges.is_some_and(|charges| {
            charges != 1
                || quantity != 1
                || facts.stack != ItemStackClass::NonStackable
                || facts.container_capacity.is_some()
                || key.is_some()
                || text.is_some()
                || fluid.is_some()
                || facts.definition.family != "Item"
                || facts.definition.revision_ref != "definition-r1"
                || !matches!(
                    facts.definition.production_key.as_str(),
                    "oteryn:item.tibia.i3048"
                        | "oteryn:item.tibia.i3054"
                        | "oteryn:item.tibia.i3081"
                        | "oteryn:item.tibia.i3082"
                        | "oteryn:item.tibia.i3083"
                        | "oteryn:item.tibia.i3085"
                )
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
    for (ordinal, charges) in std::iter::once((0, frozen.request.root_charges_remaining)).chain(
        frozen.request.contents.iter().enumerate().map(|(index, child)| {
            (index + 1, child.charges_remaining)
        }),
    ) {
        let Some(charges) = charges else {
            continue;
        };
        let item = if ordinal == 0 {
            frozen.item_instance_id
        } else {
            contents::child_id(&reservation.transaction_id, ordinal, 3)
        };
        // The initialization guard proves this exact fresh mint against its
        // immutable charges_plan; subsequent writes use timed state revisions.
        sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,remaining_ms,deadline_at,state_revision) VALUES(encode($1,'hex')::uuid,$2,NULL,NULL,1)")
            .bind(item.as_slice())
            .bind(i64::from(charges))
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

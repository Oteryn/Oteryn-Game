//! Fresh top-level siblings share the complete frozen once-claim intent.
use super::*;
use serde_json::Value;

fn item_id(root: &[u8; 16], ordinal: usize, kind: u8) -> [u8; 16] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:reward-claim-additional:v1");
    hash.update(root);
    hash.update((ordinal as u32).to_be_bytes());
    hash.update([kind]);
    let digest = hash.finalize();
    let mut id = *root;
    id[6..].copy_from_slice(&digest[..10]);
    id[6] = (id[6] & 0x0f) | 0x70;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}
pub(super) fn capacity(request: &RewardClaimMintRequest) -> Option<i16> {
    if request.additional_items.is_empty() {
        None
    } else {
        request
            .backpack
            .container_capacity
            .and_then(|v| i16::try_from(v).ok())
    }
}
pub(super) fn validate(request: &RewardClaimMintRequest) -> Result<()> {
    if request.additional_items.is_empty() {
        return Ok(());
    }
    let capacity = request
        .backpack
        .container_capacity
        .ok_or(RewardClaimMintError::InvalidInput)?;
    if capacity == 0
        || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX
        || request.additional_items.len() >= capacity as usize
    {
        return Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::MainBackpackFull,
        ));
    }
    // This first sibling bridge grants ordinary Items. Preserve the existing
    // filled-container fluid/charge/text/key owners; do not silently omit modifiers.
    if request.additional_items.iter().any(|item| {
        item.fluid_subtype.is_some()
            || item.charges_remaining.is_some()
            || item.key_number.is_some()
            || item.text.is_some()
    }) {
        return Err(RewardClaimMintError::InvalidInput);
    }
    contents::validate_items(&request.additional_items)?;
    if contents::plan_items(&request.additional_items, &[0; 16], item_id)
        .to_string()
        .len()
        + request.additional_items.len() * 20
        > 32_768
    {
        return Err(RewardClaimMintError::CapacityExceeded);
    }
    Ok(())
}
pub(super) fn plan(request: &RewardClaimMintRequest, reservation: &Reservation) -> Value {
    contents::plan_items(
        &request.additional_items,
        &reservation.transaction_id,
        item_id,
    )
}
pub(super) fn reservation_matches(
    row: &sqlx::postgres::PgRow,
    request: &RewardClaimMintRequest,
    reservation: &Reservation,
) -> std::result::Result<bool, DurabilityError> {
    let saved: sqlx::types::Json<Value> = row.try_get("additional_plan")?;
    Ok(
        row.try_get::<Option<i16>, _>("additional_capacity")? == capacity(request)
            && saved.0 == plan(request, reservation),
    )
}
pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenClaim,
    reservation: &Reservation,
    envelope: &[u8],
    tuple: SelectedType2Tuple,
    destination: ContainerEntryPosition,
) -> std::result::Result<(), DurabilityError> {
    contents::apply_plan(
        tx,
        frozen,
        reservation,
        envelope,
        tuple,
        contents::MintPlan {
            items: &frozen.request.additional_items,
            parent: destination.parent_item_instance_id,
            ordinal_base: destination.placement_ordinal,
            nested: false,
            derive: item_id,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_ids_are_stable_and_disjoint_from_container_children() {
        let root = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let mut seen = std::collections::BTreeSet::new();
        for ordinal in 1..20 {
            for kind in 1..=3 {
                let id = item_id(&root, ordinal, kind);
                assert_eq!(id, item_id(&root, ordinal, kind));
                assert_eq!(&id[..6], &root[..6]);
                assert_ne!(id, root);
                assert!(mint_audit::check_uuid_v7(&id).is_ok());
                assert!(seen.insert(id));
                for child in 1..=20 {
                    for child_kind in 1..=3 {
                        assert_ne!(id, contents::child_id(&root, child, child_kind));
                    }
                }
            }
        }
    }
}

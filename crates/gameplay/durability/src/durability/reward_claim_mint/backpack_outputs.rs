//! Additional direct backpack outputs share the root's fenced MINT occurrence.
use super::*;

/// The root lock serializes all carried custody. Reserve room and ordinals for
/// the entire output set before any Item, claim, audit or quest effect is written.
pub(super) async fn check_capacity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &RewardClaimMintRequest,
) -> Pass<()> {
    let Some(capacity) = request.backpack.container_capacity else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let (count, maximum): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*)::bigint,max(e.placement_ordinal)::text \
         FROM game_item_container_entries e JOIN game_item_container_slots s \
           ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id \
         WHERE e.character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let outputs = u64::try_from(request.contents.len())
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let occupied = u64::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?;
    let maximum = maximum
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .unwrap_or(0);
    if capacity == 0
        || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX
        || occupied
            .checked_add(outputs)
            .is_none_or(|total| total > u64::from(capacity))
        || maximum.checked_add(outputs).is_none()
    {
        return Ok(Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::MainBackpackFull,
        )));
    }
    Ok(Ok(()))
}

pub(super) async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reservation: &Reservation,
    item: [u8; 16],
    transaction: [u8; 16],
    root: ContainerEntryPosition,
    ordinal: usize,
) -> std::result::Result<ContainerEntryPosition, DurabilityError> {
    let destination = position(root, ordinal)?;
    let placement_ordinal = destination.placement_ordinal;
    sqlx::query(
        "INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,\
         parent_item_instance_id,placement_ordinal,placed_transaction_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
         encode($4,'hex')::uuid,$5::text::numeric(20,0),encode($6,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .bind(reservation.world_id.as_slice())
    .bind(reservation.character_id.as_bytes().as_slice())
    .bind(root.parent_item_instance_id.as_slice())
    .bind(placement_ordinal.to_string())
    .bind(transaction.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(ContainerEntryPosition {
        parent_item_instance_id: root.parent_item_instance_id,
        placement_ordinal,
    })
}

pub(super) fn position(
    root: ContainerEntryPosition,
    ordinal: usize,
) -> std::result::Result<ContainerEntryPosition, DurabilityError> {
    let placement_ordinal = root
        .placement_ordinal
        .checked_add(u64::try_from(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(ContainerEntryPosition {
        parent_item_instance_id: root.parent_item_instance_id,
        placement_ordinal,
    })
}

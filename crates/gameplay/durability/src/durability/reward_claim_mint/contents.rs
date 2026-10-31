//! W15b filled reward containers stay inside the existing claim transaction.
//! The immutable reservation fixes every child identity and item fact. Each
//! child has its own bounded one-item audit event, sharing the root's USE and
//! once-claim cause; the deferred proof requires the whole reserved plan.
use super::*;
use super::super::item_loop_move::{LoopError, carried_weight, pouch_weight};
use super::super::item_transfer::{ItemWeightLookup, read_derived_transfer_capacity_centi_oz};
use serde_json::{Value, json};

pub(super) fn weight_pin_matches(request: &RewardClaimMintRequest, weights: &dyn ItemWeightLookup) -> bool {
    request.content_revision == format!("sha256:{}", weights.generation_digest().iter()
        .map(|byte| format!("{byte:02x}")).collect::<String>())
}

pub(super) fn validate(request: &RewardClaimMintRequest) -> Result<()> {
    if request.contents.is_empty() {
        return Ok(());
    }
    let capacity = request.item.container_capacity.ok_or(RewardClaimMintError::InvalidInput)?;
    if capacity == 0
        || request.quantity != 1 || request.item.stack != ItemStackClass::NonStackable
        || request.key_number.is_some()
        || request.contents.len() > capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX) as usize
    {
        return Err(RewardClaimMintError::InvalidInput);
    }
    for child in &request.contents {
        validate_facts(&child.item).map_err(from_transfer_input)?;
        let (maximum, _) = stack_maximum(child.item.stack)
            .map_err(|refusal| RewardClaimMintError::Refused(from_stack_refusal(refusal)))?;
        if child.quantity == 0 || child.quantity > maximum {
            return Err(RewardClaimMintError::Refused(RewardClaimRefusal::QuantityAboveStackMaximum));
        }
        if child.item.container_capacity.is_some_and(|capacity| {
            capacity == 0
                || child.quantity != 1 || child.item.stack != ItemStackClass::NonStackable
        }) || child.key_number.is_some_and(|number| {
            number == 0 || child.quantity != 1 || child.item.stack != ItemStackClass::NonStackable
                || child.item.container_capacity.is_some()
        }) {
            return Err(RewardClaimMintError::InvalidInput);
        }
        if child.text.as_deref().is_some_and(|text| {
            !super::super::item_text::valid_authored_reward_text(text)
                || child.quantity != 1 || child.item.stack != ItemStackClass::NonStackable
        }) {
            return Err(RewardClaimMintError::InvalidInput);
        }
    }
    // UUID text widths are fixed. PostgreSQL jsonb text adds spaces after
    // commas/colons; twenty bytes per ten-field child conservatively include
    // those separators. Refuse before reservation if the frozen plan may not
    // fit the storage owner's complete-plan byte ceiling.
    if plan_for_transaction(request, &[0; 16]).to_string().len()
        + request.contents.len() * 20 > 32_768 {
        return Err(RewardClaimMintError::CapacityExceeded);
    }
    Ok(())
}

pub(super) fn capacity(request: &RewardClaimMintRequest) -> Option<i16> {
    if request.contents.is_empty() { None }
    else { request.item.container_capacity
        .and_then(|capacity| i16::try_from(capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX)).ok()) }
}

/// UUIDv7-shaped, domain-separated identities retain the reserved timestamp
/// prefix. Retry/restart derives the same children, independently of process RNG.
fn child_id(root: &[u8; 16], ordinal: usize, kind: u8) -> [u8; 16] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:reward-claim-container:v1");
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

fn uuid(id: &[u8; 16]) -> String {
    let hex = id.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

pub(super) fn plan(request: &RewardClaimMintRequest, reservation: &Reservation) -> Value {
    plan_for_transaction(request, &reservation.transaction_id)
}

fn plan_for_transaction(request: &RewardClaimMintRequest, transaction: &[u8; 16]) -> Value {
    Value::Array(request.contents.iter().enumerate().map(|(index, child)| {
        let ordinal = index + 1;
        json!({
            "ordinal": ordinal,
            "transaction_id": uuid(&child_id(transaction, ordinal, 1)),
            "event_id": uuid(&child_id(transaction, ordinal, 2)),
            "item_instance_id": uuid(&child_id(transaction, ordinal, 3)),
            "family": child.item.definition.family,
            "production_key": child.item.definition.production_key,
            "revision_ref": child.item.definition.revision_ref,
            "quantity": child.quantity,
            "key_number": child.key_number.unwrap_or(0),
            "text": child.text,
        })
    }).collect())
}

pub(super) fn reservation_matches(
    row: &sqlx::postgres::PgRow,
    request: &RewardClaimMintRequest,
    reservation: &Reservation,
) -> std::result::Result<bool, DurabilityError> {
    let saved: sqlx::types::Json<Value> = row.try_get("contents_plan")?;
    Ok(row.try_get::<Option<i16>, _>("contents_capacity")? == capacity(request)
        && saved.0 == plan(request, reservation)
        && row.try_get::<Option<String>, _>("root_text")? == request.root_text)
}

/// The character fence already holds its root. This is the complete fresh
/// plan's weight, including the bag itself, against the existing carried tree
/// and managed pouch. The check precedes all claim effects.
pub(super) async fn check_capacity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    world: [u8; 16],
    request: &RewardClaimMintRequest,
    weights: &dyn ItemWeightLookup,
) -> Pass<()> {
    let mut incoming = 0_u64;
    for (facts, quantity) in std::iter::once((&request.item, request.quantity))
        .chain(request.contents.iter().map(|child| (&child.item, child.quantity)))
    {
        let Ok(unit) = weights.weight_of(&facts.definition) else {
            return Ok(Err(RewardClaimMintError::InvalidInput));
        };
        if facts.weight != Some(unit) {
            return Ok(Err(RewardClaimMintError::InvalidInput));
        }
        incoming = incoming.checked_add(u64::from(unit) * u64::from(quantity))
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let carried = match carried_weight(tx, character, world, weights).await {
        Ok(weight) => weight,
        Err(LoopError::Refused(_)) => return Ok(Err(RewardClaimMintError::InvalidInput)),
        Err(LoopError::Durability(error)) => return Err(error),
        Err(LoopError::Database(error)) => return Err(error.into()),
    };
    let pouch = match pouch_weight(tx, character, world, weights).await? {
        Ok(weight) => weight,
        Err(_) => return Ok(Err(RewardClaimMintError::InvalidInput)),
    };
    let total = carried.checked_add(pouch).and_then(|weight| weight.checked_add(incoming))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if total > read_derived_transfer_capacity_centi_oz(tx, character).await? {
        return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::CarryingCapacity)));
    }
    Ok(Ok(()))
}

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenClaim,
    reservation: &Reservation,
    root_envelope: &[u8],
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    if frozen.request.contents.is_empty() { return Ok(()) }
    let (_, root_message) = audit::decode_reward_claim_mint_envelope(root_envelope)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for (index, child) in frozen.request.contents.iter().enumerate() {
        let ordinal = index + 1;
        let transaction = child_id(&reservation.transaction_id, ordinal, 1);
        let event = child_id(&reservation.transaction_id, ordinal, 2);
        let item = child_id(&reservation.transaction_id, ordinal, 3);
        let mut message = root_message.clone();
        message.after = Some(OneItemStateV1 {
            item_instance_id: item.to_vec(), world_id: reservation.world_id.to_vec(),
            definition: Some(definition_message(&child.item.definition)),
            quantity: child.quantity, lifecycle: ITEM_LIFECYCLE_LIVE,
            key_number: u32::from(child.key_number.unwrap_or(0)),
        });
        message.destination.as_mut().ok_or(DurabilityError::InvalidStoredState)?
            .container_entry = Some(entry_message(ContainerEntryPosition {
                parent_item_instance_id: frozen.item_instance_id,
                placement_ordinal: ordinal as u64,
            }));
        let envelope = audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity {
            event_id: event, transaction_id: transaction,
            occurred_at_unix_ms: frozen.occurred_at_unix_ms, channel_id: reservation.channel_id,
            server_build_id: SERVER_BUILD_ID,
        }, message, tuple.tuple()).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,\
            definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid)")
            .bind(item.as_slice()).bind(reservation.world_id.as_slice())
            .bind(&child.item.definition.family).bind(&child.item.definition.production_key)
            .bind(&child.item.definition.revision_ref).bind(i64::from(child.quantity))
            .bind(transaction.as_slice()).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,\
            parent_item_instance_id,placement_ordinal,placed_transaction_id) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
            $4::text::numeric(20,0),encode($5,'hex')::uuid)")
            .bind(item.as_slice()).bind(reservation.world_id.as_slice())
            .bind(frozen.item_instance_id.as_slice()).bind(ordinal.to_string())
            .bind(transaction.as_slice()).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,\
            transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,\
            occurred_at,expires_at,envelope,envelope_sha256,publication_state) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,$3,$4,$5,\
            encode($6,'hex')::uuid,$7,$7+$8,$9,sha256($9),1)")
            .bind(event.as_slice()).bind(transaction.as_slice()).bind(EVENT_TYPE_ID)
            .bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
            .bind(item.as_slice()).bind(frozen.occurred_at_unix_ms)
            .bind(mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope)
            .execute(&mut **tx).await?;
        if let Some(number) = child.key_number {
            sqlx::query("INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id) \
                VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
                .bind(item.as_slice()).bind(i32::from(number)).bind(transaction.as_slice())
                .execute(&mut **tx).await?;
        }
        if let Some(text) = &child.text {
            super::super::item_text::initialize_authored_reward_text(
                tx, item, transaction, text, frozen.occurred_at_unix_ms,
            ).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_identities_are_stable_disjoint_and_keep_the_frozen_timestamp() {
        let root = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let mut seen = std::collections::BTreeSet::new();
        for ordinal in 1..=20 {
            for kind in 1..=3 {
                let id = child_id(&root, ordinal, kind);
                assert_eq!(id, child_id(&root, ordinal, kind));
                assert_eq!(&id[..6], &root[..6]);
                assert_ne!(id, root);
                assert!(mint_audit::check_uuid_v7(&id).is_ok());
                assert!(seen.insert(id));
            }
        }
    }
}

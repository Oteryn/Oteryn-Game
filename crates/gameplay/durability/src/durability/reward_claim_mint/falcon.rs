//! Falcon's durable cycle receipt wraps the existing atomic custody and audit owner.
//! Public Content identity stays unchanged. `:cycle:N` is an INTERNAL operation
//! identity, derived only here after the six-source repeat gate, never a new
//! Content definition. Each cycle retains the original key and frozen draw.
use super::*;
use oteryn_gameplay_quests::quest::falcon::{REWARDS, claim_key, draw_index, repeat_ready};

impl DurabilityRoot {
    pub async fn claim_falcon_chest(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        requests: [RewardClaimMintRequest; 7],
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        validate_pool(&requests, weights)?;
        let public_key = requests[0].claim.production_key.clone();
        let command = requests[0].command;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            lock_cause(&mut tx, command).await?;
            let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
            if !character_item_fence_is_current(&mut tx, &node, &fence, command,
                fence.character_id, *world.as_bytes(), *channel.as_bytes()).await? {
                return Ok(Err(RewardClaimMintError::AuthorityRejected));
            }
            // Current session/lease/scope fences precede both receipt and draw replay.
            if let Some(receipt) = load_receipt(&mut tx, command).await? {
                let transaction: String = receipt.try_get("transaction_id")?;
                let cycle = sqlx::query("SELECT cycle,selected_index,entropy FROM game_character_falcon_chest_cycles WHERE character_id=encode($1,'hex')::uuid AND chest_key=$2 AND transaction_id=$3::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(&public_key).bind(transaction)
                    .fetch_optional(&mut **tx).await?;
                let Some(cycle) = cycle else { return Ok(Err(RewardClaimMintError::ConflictingCause)); };
                let request = cycle_request(&requests, &cycle)?;
                if receipt.try_get::<Vec<u8>,_>("intent_binding")? != intent_binding(&request, fence.character_id).map_err(|_| DurabilityError::InvalidStoredState)? {
                    return Ok(Err(RewardClaimMintError::ConflictingCause));
                }
                let committed = decode_receipt(&receipt)?;
                tx.commit(deadline).await?;
                return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(committed)));
            }
            if load_reservation(&mut tx, command).await?.is_some() { return Ok(Err(RewardClaimMintError::ConflictingCause)); }
            let now: i64 = sqlx::query_scalar("SELECT game_falcon_now_ms()").fetch_one(&mut **tx).await?;
            let previous = sqlx::query("SELECT cycle,selected_index,entropy,claimed_at FROM game_character_falcon_chest_cycles WHERE character_id=encode($1,'hex')::uuid AND chest_key=$2 ORDER BY cycle DESC LIMIT 1 FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&public_key).fetch_optional(&mut **tx).await?;
            let pending = previous.as_ref().is_some_and(|row| row.try_get::<Option<i64>,_>("claimed_at").is_ok_and(|at| at.is_none()));
            if let Some(previous) = &previous
                && let Some(claimed) = previous.try_get::<Option<i64>,_>("claimed_at")?
                && !repeat_ready(claimed, now) {
                return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));
            }
            let cycle = if pending {
                previous.ok_or(DurabilityError::InvalidStoredState)?
            } else {
                let number = match previous { Some(row) => row.try_get::<i64,_>("cycle")?.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?, None => 1 };
                let entropy: Vec<u8> = sqlx::query_scalar("SELECT sha256(uuid_send(gen_random_uuid()) || uuid_send(gen_random_uuid()))").fetch_one(&mut **tx).await?;
                let bytes: [u8;32] = entropy.as_slice().try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_character_falcon_chest_cycles(character_id,chest_key,cycle,entropy,selected_index,drawn_at) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6) RETURNING cycle,selected_index,entropy")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(&public_key).bind(number).bind(entropy)
                    .bind(i16::try_from(draw_index(&bytes)).map_err(|_| DurabilityError::InvalidStoredState)?).bind(now)
                    .fetch_one(&mut **tx).await?
            };
            let request = cycle_request(&requests, &cycle)?;
            let binding = intent_binding(&request, fence.character_id).map_err(|_| DurabilityError::InvalidStoredState)?;
            let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id").fetch_one(&mut **tx).await?;
            let reservation = Reservation { character_id: fence.character_id, world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
                transaction_id: uuid_text(row.try_get("transaction_id")?)?, event_id: uuid_text(row.try_get("event_id")?)?,
                item_instance_id: uuid_text(row.try_get("item_instance_id")?)?, occurred_at_unix_ms: now, work_units_used: 0 };
            let (destination, _) = match admit(&mut tx, &node, &fence, &request, &reservation).await? {
                Ok(admitted) => admitted,
                Err(error) => { tx.commit(deadline).await?; return Ok(Err(error)); }
            };
            if let Err(error) = contents::check_capacity(&mut tx, fence.character_id, reservation.world_id, &request, *weights).await? {
                // A refusal freezes only the draw. No reservation, item or cooldown is written.
                tx.commit(deadline).await?; return Ok(Err(error));
            }
            insert_reservation(&mut tx, &request, &binding, &reservation).await?;
            sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
            let frozen = FrozenClaim { request, character_id: fence.character_id, transaction_id: reservation.transaction_id,
                event_id: reservation.event_id, item_instance_id: reservation.item_instance_id, occurred_at_unix_ms: now, intent_binding: binding };
            let tuple = tx.tuple();
            let message = claim_message(&frozen, &reservation, &fence, destination);
            let envelope = audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity {
                event_id: frozen.event_id, transaction_id: frozen.transaction_id, occurred_at_unix_ms: now,
                channel_id: reservation.channel_id, server_build_id: SERVER_BUILD_ID }, message, tuple.tuple())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let committed = apply_claim(&mut tx, &frozen, &reservation, destination, &envelope, tuple).await?;
            let updated = sqlx::query("UPDATE game_character_falcon_chest_cycles SET claimed_at=$4,transaction_id=encode($5,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid AND chest_key=$2 AND cycle=$3 AND claimed_at IS NULL")
                .bind(fence.character_id.as_bytes().as_slice()).bind(public_key).bind(cycle.try_get::<i64,_>("cycle")?).bind(now).bind(frozen.transaction_id.as_slice()).execute(&mut **tx).await?;
            if updated.rows_affected()!=1 { return Err(DurabilityError::InvalidStoredState); }
            tx.commit(deadline).await?;
            Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
        })).await?
    }
}

fn cycle_request(
    requests: &[RewardClaimMintRequest; 7],
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<RewardClaimMintRequest, DurabilityError> {
    let index = usize::try_from(row.try_get::<i16, _>("selected_index")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entropy: Vec<u8> = row.try_get("entropy")?;
    let entropy: [u8; 32] = entropy
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if index != draw_index(&entropy) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut request = requests
        .get(index)
        .ok_or(DurabilityError::InvalidStoredState)?
        .clone();
    let cycle: i64 = row.try_get("cycle")?;
    if cycle < 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    request.claim.production_key = format!("{}:cycle:{cycle}", request.claim.production_key);
    Ok(request)
}

fn validate_pool(
    requests: &[RewardClaimMintRequest; 7],
    weights: &dyn super::super::item_transfer::ItemWeightLookup,
) -> Result<()> {
    let first = &requests[0];
    let chest = claim_key(&first.claim.production_key).ok_or(RewardClaimMintError::InvalidInput)?;
    let pin = format!(
        "sha256:{}",
        weights
            .generation_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if first.claim.family != "RewardClaim"
        || first.source_placement != chest.source()
        || first.content_revision != pin
        || first.achievement.is_some()
        || first.quest_transition.is_some()
        || first.key_number.is_some()
        || !first.contents.is_empty()
        || first.root_text.is_some()
        || first.root_fluid_subtype.is_some()
        || first.root_charges_remaining.is_some()
    {
        return Err(RewardClaimMintError::InvalidInput);
    }
    for (request, (id, count)) in requests.iter().zip(REWARDS) {
        validate_request(request)?;
        let mut expected = first.clone();
        expected.item = request.item.clone();
        expected.quantity = count;
        if request != &expected
            || request.item.definition.family != "Item"
            || request.item.definition.production_key != format!("oteryn:item.tibia.i{id}")
        {
            return Err(RewardClaimMintError::InvalidInput);
        }
    }
    Ok(())
}

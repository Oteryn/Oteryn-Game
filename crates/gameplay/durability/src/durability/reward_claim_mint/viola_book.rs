//! Book of Wisdom repeat receipt, composed with the existing atomic reward MINT.
use super::*;
const CLAIM: &str = "oteryn:reward-claim.tutorial.viola-book";
const SOURCE: &str = "oteryn:placement.tutorial.viola-book.32614-32419-7";

impl DurabilityRoot {
    pub async fn claim_viola_book(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        validate_book(&request, weights)?;
        let command = request.command;
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
            // Current session/lease/scope fences precede receipt replay.
            if let Some(receipt) = load_receipt(&mut tx, command).await? {
                let transaction: String = receipt.try_get("transaction_id")?;
                let cycle: Option<i64> = sqlx::query_scalar("SELECT cycle FROM game_character_viola_book_cycles WHERE character_id=encode($1,'hex')::uuid AND transaction_id=$2::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(transaction)
                    .fetch_optional(&mut **tx).await?;
                let Some(cycle) = cycle else { return Ok(Err(RewardClaimMintError::ConflictingCause)); };
                let request = cycle_request(&request, cycle)?;
                if receipt.try_get::<Vec<u8>,_>("intent_binding")? != intent_binding(&request, fence.character_id).map_err(|_| DurabilityError::InvalidStoredState)? {
                    return Ok(Err(RewardClaimMintError::ConflictingCause));
                }
                let committed = decode_receipt(&receipt)?;
                tx.commit(deadline).await?;
                return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(committed)));
            }
            if load_reservation(&mut tx, command).await?.is_some() { return Ok(Err(RewardClaimMintError::ConflictingCause)); }
            let now: i64 = sqlx::query_scalar("SELECT game_viola_book_now_ms()").fetch_one(&mut **tx).await?;
            let previous: Option<(i64,i64)> = sqlx::query_as("SELECT cycle,claimed_at FROM game_character_viola_book_cycles WHERE character_id=encode($1,'hex')::uuid ORDER BY cycle DESC LIMIT 1 FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if previous.is_some_and(|(_,at)| i128::from(now) < i128::from(at)+300_000) {
                return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));
            }
            let cycle = match previous { Some((number,_)) => number.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?, None => 1 };
            let request = cycle_request(&request, cycle)?;
            let binding = intent_binding(&request, fence.character_id).map_err(|_| DurabilityError::InvalidStoredState)?;
            let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id").fetch_one(&mut **tx).await?;
            let reservation = Reservation { character_id: fence.character_id, world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
                transaction_id: uuid_text(row.try_get("transaction_id")?)?, event_id: uuid_text(row.try_get("event_id")?)?,
                item_instance_id: uuid_text(row.try_get("item_instance_id")?)?, occurred_at_unix_ms: now, work_units_used: 0 };
            let (destination, _) = match admit(&mut tx, &node, &fence, &request, &reservation).await? {
                Ok(admitted) => admitted,
                Err(error) => return Ok(Err(error)),
            };
            if let Err(error) = contents::check_capacity(&mut tx, fence.character_id, reservation.world_id, &request, *weights).await? {
                return Ok(Err(error));
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
            sqlx::query("INSERT INTO game_character_viola_book_cycles(character_id,cycle,claimed_at,transaction_id) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(cycle).bind(now).bind(frozen.transaction_id.as_slice()).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
        })).await?
    }
}

fn cycle_request(
    request: &RewardClaimMintRequest,
    cycle: i64,
) -> std::result::Result<RewardClaimMintRequest, DurabilityError> {
    if cycle < 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut request = request.clone();
    request.claim.production_key = format!("{}:cycle:{cycle}", request.claim.production_key);
    Ok(request)
}

fn validate_book(
    request: &RewardClaimMintRequest,
    weights: &dyn super::super::item_transfer::ItemWeightLookup,
) -> Result<()> {
    validate_request(request)?;
    let pin = format!(
        "sha256:{}",
        weights
            .generation_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if request.claim.family != "RewardClaim"
        || request.claim.production_key != CLAIM
        || request.claim.revision_ref != "reward-claim-r1"
        || request.source_placement != SOURCE
        || request.item.definition.family != "Item"
        || request.item.definition.production_key != "oteryn:item.tibia.i51977"
        || request.item.definition.revision_ref != "definition-r1"
        || request.quantity != 1
        || request.content_revision != pin
        || request.achievement.is_some()
        || request.quest_transition.is_some()
        || request.key_number.is_some()
        || !request.contents.is_empty()
        || request.root_text.is_some()
        || request.root_fluid_subtype.is_some()
        || request.root_charges_remaining.is_some()
    {
        return Err(RewardClaimMintError::InvalidInput);
    }
    Ok(())
}

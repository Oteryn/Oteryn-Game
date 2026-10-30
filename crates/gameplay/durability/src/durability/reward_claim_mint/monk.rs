//! A durable quest entitlement needs no separately committed MINT reservation.
//! Reservation, existing owner application and entitlement acknowledgement share one
//! type-2 transaction, so cancellation/restart never strands a session-bound attempt.
use super::*;

impl DurabilityRoot {
    pub async fn claim_pending_monk_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        transition: &str,
        request: RewardClaimMintRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        validate_request(&request)?;
        if request.achievement.is_some() || request.quest_transition.is_some()
            || request.key_number.is_some() || request.quantity != 1 {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let binding = intent_binding(&request, fence.character_id)?;
        let weight_pin = format!("sha256:{}", weights.generation_digest().iter().map(|b| format!("{b:02x}")).collect::<String>());
        if request.content_revision != weight_pin { return Err(RewardClaimMintError::InvalidInput) }
        let recovery = authority.record_for(self).map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone(); let transition = transition.to_owned();
        let mut context = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            lock_cause(&mut tx, request.command).await?;
            let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
            if !character_item_fence_is_current(&mut tx, &node, &fence, request.command,
                fence.character_id, *world.as_bytes(), *channel.as_bytes()).await? {
                return Ok(Err(RewardClaimMintError::AuthorityRejected));
            }
            if let Some(row) = load_receipt(&mut tx, request.command).await? {
                if row.try_get::<Vec<u8>,_>("intent_binding")? != binding {
                    return Ok(Err(RewardClaimMintError::ConflictingCause));
                }
                let committed = decode_receipt(&row)?;
                tx.commit(deadline).await?;
                return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(committed)));
            }
            if load_reservation(&mut tx, request.command).await?.is_some() {
                return Ok(Err(RewardClaimMintError::ConflictingCause));
            }
            let entitlement = sqlx::query("SELECT item_source_id,claim_key FROM game_character_monk_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 AND state='PENDING' FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&transition)
                .fetch_optional(&mut **tx).await?;
            let Some(entitlement) = entitlement else {
                return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));
            };
            let source: i32 = entitlement.try_get("item_source_id")?;
            if request.claim.family != "RewardClaim"
                || request.claim.production_key != entitlement.try_get::<String,_>("claim_key")?
                || request.source_placement != transition
                || request.item.definition.family != "Item"
                || request.item.definition.production_key != format!("oteryn:item.tibia.i{source}") {
                return Ok(Err(RewardClaimMintError::InvalidInput));
            }
            let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
                .fetch_one(&mut **tx).await?;
            let reservation = Reservation { character_id: fence.character_id,
                world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
                transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                event_id: uuid_text(row.try_get("event_id")?)?, item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                occurred_at_unix_ms: row.try_get("occurred_at")?, work_units_used: 0 };
            let (destination, _) = match admit(&mut tx, &node, &fence, &request, &reservation).await? {
                Ok(admitted) => admitted, Err(error) => return Ok(Err(error)),
            };
            let incoming = InventoryItem { item_instance_id: reservation.item_instance_id,
                definition: request.item.definition.clone(), quantity: request.quantity };
            if let Err(refusal) = super::super::item_transfer::check_incoming_capacity(&mut tx,
                fence.character_id, world.as_bytes(), &request.item, &incoming, *weights).await? {
                return Ok(Err(if refusal == ItemTransferRefusal::Overweight {
                    RewardClaimMintError::Refused(RewardClaimRefusal::CarryingCapacity)
                } else { RewardClaimMintError::InvalidInput }));
            }
            insert_reservation(&mut tx, &request, &binding, &reservation).await?;
            sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
                .bind(request.command.game_session_id().as_bytes().as_slice())
                .bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let frozen = FrozenClaim { request, character_id: fence.character_id,
                transaction_id: reservation.transaction_id, event_id: reservation.event_id,
                item_instance_id: reservation.item_instance_id, occurred_at_unix_ms: reservation.occurred_at_unix_ms,
                intent_binding: binding };
            let tuple = tx.tuple();
            let message = claim_message(&frozen, &reservation, &fence, destination);
            let envelope = match audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity {
                event_id: frozen.event_id, transaction_id: frozen.transaction_id,
                occurred_at_unix_ms: frozen.occurred_at_unix_ms, channel_id: reservation.channel_id,
                server_build_id: SERVER_BUILD_ID,
            }, message, tuple.tuple()) {
                Ok(envelope) => envelope, Err(error) => return Ok(Err(error.into())),
            };
            let committed = apply_claim(&mut tx, &frozen, &reservation, destination, &envelope, tuple).await?;
            let updated = sqlx::query("UPDATE game_character_monk_reward_entitlements SET state='DELIVERED' WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 AND state='PENDING'")
                .bind(fence.character_id.as_bytes().as_slice()).bind(transition).execute(&mut **tx).await?;
            if updated.rows_affected()!=1 { return Err(DurabilityError::InvalidStoredState) }
            tx.commit(deadline).await?;
            Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
        })).await?
    }
}

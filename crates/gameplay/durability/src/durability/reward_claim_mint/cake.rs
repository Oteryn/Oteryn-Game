//! A durable quest entitlement needs no separately committed MINT reservation.
//! Reservation, existing owner application and entitlement acknowledgement share one
//! type-2 transaction, so cancellation/restart never strands a session-bound attempt.
use super::*;

impl DurabilityRoot {
    pub async fn claim_cake_brutus_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        validate_request(&request)?;
        if request.achievement.is_some()
            || request.quest_transition.is_some()
            || request.key_number.is_some()
            || request.quantity != 1
            || !request.contents.is_empty()
            || !request.additional_items.is_empty()
            || request.root_text.is_some()
        {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let binding = intent_binding(&request, fence.character_id)?;
        let weight_pin = format!(
            "sha256:{}",
            weights
                .generation_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        if request.content_revision != weight_pin {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::super::character_equipment::assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence,
                Some(request.command), weights.generation_digest()).await
                .map_err(|_| DurabilityError::Unavailable)?;
            lock_admission_relations(&mut tx).await?;
            lock_cause(&mut tx, request.command).await?;
            let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
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
            let Some(year) = super::super::cake_event::latest_year(&mut tx, world).await? else {
                return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));
            };
            let category: i16 = match request.item.definition.production_key.as_str() {
                "oteryn:item.tibia.i14756" | "oteryn:item.tibia.i20347" | "oteryn:item.tibia.i20350" => 1,
                "oteryn:item.tibia.i28484" | "oteryn:item.tibia.i28485" | "oteryn:item.tibia.i28486" => 2,
                _ => return Ok(Err(RewardClaimMintError::InvalidInput)),
            };
            let suffix = if category == 1 { "item" } else { "cupcake" };
            if request.claim.family != "RewardClaim"
                || request.claim.production_key != format!("oteryn:reward-claim.cake-brutus-{suffix}.{year}")
                || request.claim.revision_ref != "oteryn:cake-brutus.v1"
                || request.source_placement != "oteryn:npc.brutus_the_baker"
                || request.item.definition.family != "Item"
                || request.item.definition.revision_ref != "definition-r1" {
                return Ok(Err(RewardClaimMintError::InvalidInput));
            }
            let entitled: bool = sqlx::query_scalar("SELECT game_cake_lock_entitlement(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4)")
                .bind(world.as_bytes().as_slice()).bind(year).bind(fence.character_id.as_bytes().as_slice()).bind(category)
                .fetch_one(&mut **tx).await?;
            if !entitled { return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed))); }
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
            sqlx::query("SELECT game_cake_reward_record(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5)")
                .bind(reservation.transaction_id.as_slice()).bind(world.as_bytes().as_slice()).bind(year)
                .bind(fence.character_id.as_bytes().as_slice()).bind(category).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
        })).await?
    }
}

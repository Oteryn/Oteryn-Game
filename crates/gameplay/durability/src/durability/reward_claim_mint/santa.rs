//! Santa's permanent source flag uses the existing once-claim MINT owner.
//! Gift, bag children, once claim, receipt and audit commit in one transaction.
use super::*;

pub const SANTA_PRESENT_CLAIM: &str = "oteryn:reward-claim.santa-claus-present";

#[derive(Debug, Clone)]
pub struct SantaPresentRequest {
    pub command: CommandRef,
    pub reset_epoch: u64,
    pub chance: u8,
    pub group: u8,
    pub gift: Option<RewardClaimMintRequest>,
}

/// Crystal201189f santa_claus.lua:45-56,83-97: only the first <=90 branch is
/// reachable. 91..100 selects an empty table and resolves no reward group.
pub fn santa_present_items(chance: u8, group: u8) -> Option<&'static [u32]> {
    const NORMAL: [&[u32]; 10] = [
        &[7439, 7440, 7443],
        &[3599, 6507],
        &[3599, 6508],
        &[3599, 6506],
        &[3599, 2995],
        &[3599, 2992],
        &[3051, 3097, 3098],
        &[10310],
        &[3039],
        &[3036],
    ];
    if !(1..=90).contains(&chance) {
        return None;
    }
    NORMAL.get(usize::from(group.checked_sub(1)?)).copied()
}

/// Receipt-scoped bookkeeping, as the existing repeatable MINT purchase owner
/// uses: this key is never Santa's permanent840293 entitlement.
pub fn santa_failed_script_claim(command: CommandRef) -> String {
    let session: String = command
        .game_session_id()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!(
        "oteryn:reward.santa-bag.{session}.{}",
        command.command_id().get()
    )
}

fn valid_gift(request: &RewardClaimMintRequest, ids: &[u32], expected_claim: &str) -> bool {
    if request.claim.family != "RewardClaim"
        || request.claim.production_key != expected_claim
        || request.item.definition.production_key != "oteryn:item.tibia.i6510"
        || request.item.definition.family != "Item"
        || request.item.container_capacity != Some(5)
        || request.quantity != 1
        || request.achievement.is_some()
        || request.quest_transition.is_some()
        || request.root_text.is_some()
        || request.key_number.is_some()
        || request.contents.len() != ids.len()
    {
        return false;
    }
    let mut count = 1;
    for (child, id) in request.contents.iter().zip(ids) {
        // Santa:112-119 carries count after candy/snowballs. Container:addItem
        // creates one Item (container_functions.cpp:156-202); Item::Item
        // item.cpp:322-345 applies count only to stackable/charged Items.
        if matches!(id, 2992 | 3599) {
            count = 10;
        }
        let quantity = if matches!(child.item.stack, ItemStackClass::Stackable { .. }) {
            count
        } else {
            1
        };
        if child.item.definition.family != "Item"
            || child.item.definition.production_key != format!("oteryn:item.tibia.i{id}")
            || child.quantity != quantity
            || child.key_number.is_some()
            || child.text.is_some()
        {
            return false;
        }
    }
    true
}

impl DurabilityRoot {
    pub async fn claim_santa_present(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        attempt: SantaPresentRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        if !(1..=100).contains(&attempt.chance)
            || !(1..=10).contains(&attempt.group)
            || attempt.command.game_session_id() != fence.game_session_id
        {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let expected = santa_present_items(attempt.chance, attempt.group);
        match (&attempt.gift, expected) {
            (Some(gift), Some(ids))
                if gift.command == attempt.command
                    && valid_gift(gift, ids, SANTA_PRESENT_CLAIM) =>
            {
                validate_request(gift)?;
                if !contents::weight_pin_matches(gift, weights) {
                    return Err(RewardClaimMintError::InvalidInput);
                }
            }
            // LuaJIT src/lib_math.c:155 makes random(0)=1, so the empty
            // table lookup in santa_claus.lua:97 yields nil. Santa:111 creates
            // this bag before #reward fails at114. There is no reward group,
            // no NPC Merry Christmas reply and no permanent840293 flag.
            (Some(gift), None)
                if gift.command == attempt.command
                    && valid_gift(gift, &[], &santa_failed_script_claim(attempt.command)) =>
            {
                validate_request(gift)?;
                if !contents::weight_pin_matches(gift, weights) {
                    return Err(RewardClaimMintError::InvalidInput);
                }
            }
            _ => return Err(RewardClaimMintError::InvalidInput),
        }
        let binding = attempt
            .gift
            .as_ref()
            .map(|gift| intent_binding(gift, fence.character_id))
            .transpose()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, weights| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            lock_cause(&mut tx, attempt.command).await?;
            let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
            if !character_item_fence_is_current(&mut tx, &node, &fence, attempt.command,
                fence.character_id, *world.as_bytes(), *channel.as_bytes()).await? {
                return Ok(Err(RewardClaimMintError::AuthorityRejected));
            }
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND e.seasonal_calendar_revision=1 AND e.seasonal_event_keys @> ARRAY['christmas']::text[])")
                .bind(world.as_bytes().as_slice()).bind(attempt.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
            if !active { return Ok(Err(RewardClaimMintError::AuthorityRejected)); }
            if let Some(row) = load_receipt(&mut tx, attempt.command).await? {
                if binding.as_ref().is_none_or(|binding| row.try_get::<Vec<u8>, _>("intent_binding").ok().as_deref() != Some(binding.as_slice())) {
                    return Ok(Err(RewardClaimMintError::ConflictingCause));
                }
                let committed = decode_receipt(&row)?;
                tx.commit(deadline).await?;
                return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(committed)));
            }
            // Crystal santa_claus.lua:103-122 checks its permanent flag before
            // drawing. There is no year reset or Premium condition in this rule.
            let claimed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim' AND claim_production_key=$2)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(SANTA_PRESENT_CLAIM).fetch_one(&mut **tx).await?;
            if claimed { return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed))); }
            let request=attempt.gift.ok_or(DurabilityError::InvalidStoredState)?;
            let binding = binding.ok_or(DurabilityError::InvalidStoredState)?;
            let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
                .fetch_one(&mut **tx).await?;
            let reservation = Reservation { character_id: fence.character_id,
                world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
                transaction_id: uuid_text(row.try_get("transaction_id")?)?, event_id: uuid_text(row.try_get("event_id")?)?,
                item_instance_id: uuid_text(row.try_get("item_instance_id")?)?, occurred_at_unix_ms: row.try_get("occurred_at")?, work_units_used: 0 };
            let (destination, _) = match admit(&mut tx, &node, &fence, &request, &reservation).await? {
                Ok(admitted) => admitted, Err(error) => return Ok(Err(error)),
            };
            if let Err(error) = contents::check_capacity(&mut tx, fence.character_id, reservation.world_id, &request, *weights).await? {
                return Ok(Err(error));
            }
            insert_reservation(&mut tx, &request, &binding, &reservation).await?;
            sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
                .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let frozen = FrozenClaim { request, character_id: fence.character_id,
                transaction_id: reservation.transaction_id, event_id: reservation.event_id, item_instance_id: reservation.item_instance_id,
                occurred_at_unix_ms: reservation.occurred_at_unix_ms, intent_binding: binding };
            let tuple = tx.tuple();
            let message = claim_message(&frozen, &reservation, &fence, destination);
            let envelope = match audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity {
                event_id: frozen.event_id, transaction_id: frozen.transaction_id, occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                channel_id: reservation.channel_id, server_build_id: SERVER_BUILD_ID,
            }, message, tuple.tuple()) { Ok(envelope) => envelope, Err(error) => return Ok(Err(error.into())) };
            let committed = apply_claim(&mut tx, &frozen, &reservation, destination, &envelope, tuple).await?;
            tx.commit(deadline).await?;
            Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
        })).await?
    }
}

impl DurabilityRoot {
    /// Resolve the original atomic MINT after its acknowledgement was lost.
    /// This owner never reconstructs a gift or repeats the random decision.
    pub async fn reconcile_santa_present(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        attempt: SantaPresentRequest,
    ) -> Result<Option<CommittedRewardClaimMint>> {
        if attempt.command.game_session_id() != fence.game_session_id {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let binding = attempt
            .gift
            .as_ref()
            .map(|gift| intent_binding(gift, fence.character_id))
            .transpose()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, attempt.command).await?;
                    let (world, channel) =
                        scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
                    if !character_item_fence_is_current(
                        &mut tx,
                        &node,
                        &fence,
                        attempt.command,
                        fence.character_id,
                        *world.as_bytes(),
                        *channel.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(RewardClaimMintError::AuthorityRejected));
                    }
                    let result = if let Some(row) = load_receipt(&mut tx, attempt.command).await? {
                        if binding.as_ref().is_none_or(|binding| {
                            row.try_get::<Vec<u8>, _>("intent_binding").ok().as_deref()
                                != Some(binding.as_slice())
                        }) {
                            return Ok(Err(RewardClaimMintError::ConflictingCause));
                        }
                        Some(decode_receipt(&row)?)
                    } else {
                        None
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn santa_literal_lookup_preserves_unreachable_rare_groups_and_nil() {
        assert_eq!(
            santa_present_items(1, 1),
            Some([7439, 7440, 7443].as_slice())
        );
        assert_eq!(santa_present_items(90, 10), Some([3036].as_slice()));
        assert_eq!(santa_present_items(91, 1), None);
        assert_eq!(santa_present_items(100, 10), None);
        assert_eq!(santa_present_items(0, 1), None);
        assert_eq!(santa_present_items(1, 0), None);
    }
}

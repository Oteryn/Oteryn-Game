//! Arena entitlements participate in the existing complete backpack/container MINT.
use super::*;
use crate::durability::arena_match::{self as arena, MatchState};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_protocol_oteryn::arena::{ArenaDisposition as D, ArenaResult, encode_match_result};

impl DurabilityRoot {
    /// Public description of a live arena award, after LOOK proves visibility.
    pub async fn read_arena_trophy_inscription(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: [u8; 16],
        item: [u8; 16],
    ) -> std::result::Result<Option<String>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let text = sqlx::query_scalar(
                    "SELECT r.root_text FROM game_item_instances i
                     JOIN game_reward_claim_mint_receipts r
                       ON r.item_instance_id=i.item_instance_id AND r.transaction_id=i.minted_transaction_id
                     WHERE i.item_instance_id=encode($1,'hex')::uuid
                       AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1
                       AND r.claim_family='RewardClaim' AND r.root_text IS NOT NULL
                       AND (i.definition_production_key, r.claim_production_key) IN (
                           ('oteryn:item.tibia.i5807', 'oteryn:reward.svargrond-arena.1.4'),
                           ('oteryn:item.tibia.i5806', 'oteryn:reward.svargrond-arena.2.4'),
                           ('oteryn:item.tibia.i5805', 'oteryn:reward.svargrond-arena.3.4')
                       )",
                )
                .bind(item.as_slice())
                .bind(world.as_slice())
                .fetch_optional(&mut **tx)
                .await?;
                tx.commit(deadline).await?;
                Ok(text)
            }))
            .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the existing MINT owner plus the closed donor reward selector"
    )]
    pub async fn claim_arena_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        gameplay: CurrentCharacterGameplayFence,
        mut request: RewardClaimMintRequest,
        rank: u8,
        category: u8,
        choice: u8,
        payload: Vec<u8>,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<Vec<u8>> {
        validate_request(&request)?;
        let expected = arena::donor::reward(rank, category, choice)
            .ok_or(RewardClaimMintError::InvalidInput)?;
        if request.claim.production_key != arena::donor::claim_key(rank, category)
            || request.item.definition.production_key
                != format!("oteryn:item.tibia.i{}", expected.root)
            || request.quantity != 1
            || request.contents.len() != expected.contents.len()
            || request.achievement.is_some()
            || request.quest_transition.is_some()
            || request.key_number.is_some()
            || request.root_text.is_some()
            || !contents::weight_pin_matches(&request, weights)
            || request.contents.iter().zip(&expected.contents).any(
                |(actual, (source, quantity))| {
                    actual.item.definition.production_key != format!("oteryn:item.tibia.i{source}")
                        || actual.quantity != *quantity
                        || actual.key_number.is_some()
                },
            )
        {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let fence = gameplay.item_fence();
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,weights|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            lock_cause(&mut tx,request.command).await?;
            if super::super::character_progression::assert_gameplay_fence(&mut tx,&gameplay,&node).await?.is_err() {return Ok(Err(RewardClaimMintError::AuthorityRejected));}
            if let Some(reply)=arena::replay(&mut tx,gameplay,request.command,&payload).await? {tx.commit(deadline).await?;return Ok(Ok(reply));}
            let state=arena::load_state(&mut tx,gameplay).await?;
            let masks=arena::claimed_masks(&mut tx,gameplay).await?;
            let refusal=if state.completed<rank || state.phase!=3 {Some(D::WrongState)}
                else if masks[usize::from(rank-1)]&(1<<(category-1))!=0 {Some(D::AlreadyClaimed)} else {None};
            if let Some(disposition)=refusal {
                let bytes=finish(&mut tx,gameplay,&request,&payload,&state,disposition).await?;
                tx.commit(deadline).await?;return Ok(Ok(bytes));
            }
            let (world,channel)=scope_of(&fence).map_err(|_|DurabilityError::InvalidStoredState)?;
            let row=sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at").fetch_one(&mut **tx).await?;
            let reservation=Reservation{character_id:fence.character_id,world_id:*world.as_bytes(),channel_id:*channel.as_bytes(),
                transaction_id:uuid_text(row.try_get("transaction_id")?)?,event_id:uuid_text(row.try_get("event_id")?)?,item_instance_id:uuid_text(row.try_get("item_instance_id")?)?,occurred_at_unix_ms:row.try_get("occurred_at")?,work_units_used:0};
            let (destination,_)=match admit(&mut tx,&node,&fence,&request,&reservation).await? {
                Ok(admitted)=>admitted,
                Err(RewardClaimMintError::Refused(_))=>{
                    let bytes=finish(&mut tx,gameplay,&request,&payload,&state,D::InventoryFull).await?;
                    tx.commit(deadline).await?;return Ok(Ok(bytes));
                }
                Err(error)=>return Ok(Err(error)),
            };
            if let Err(error)=contents::check_capacity(&mut tx,fence.character_id,reservation.world_id,&request,*weights).await? {
                if !matches!(error,RewardClaimMintError::Refused(_)) {return Ok(Err(error));}
                let bytes=finish(&mut tx,gameplay,&request,&payload,&state,D::InventoryFull).await?;
                tx.commit(deadline).await?;return Ok(Ok(bytes));
            }
            if category == 4 {
                let name: String = sqlx::query_scalar(
                    "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
                )
                .bind(fence.character_id.as_bytes().as_slice())
                .fetch_one(&mut **tx)
                .await?;
                // Crystal 201189f svargrond_arena.lua:175-190.
                let (difficulty, punctuation) = match rank {
                    1 => ("greenhorn", ""),
                    2 => ("scrapper", "."),
                    3 => ("warlord", "."),
                    _ => return Err(DurabilityError::InvalidStoredState),
                };
                request.root_text = Some(format!(
                    "For victory in the Barbarian Arena at {difficulty} difficulty. Awarded to {name}{punctuation}"
                ));
                validate_request(&request).map_err(|_| DurabilityError::InvalidStoredState)?;
            }
            let binding = intent_binding(&request, fence.character_id)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            insert_reservation(&mut tx,&request,&binding,&reservation).await?;
            sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let frozen=FrozenClaim{request,character_id:fence.character_id,transaction_id:reservation.transaction_id,event_id:reservation.event_id,item_instance_id:reservation.item_instance_id,occurred_at_unix_ms:reservation.occurred_at_unix_ms,intent_binding:binding};
            let tuple=tx.tuple();
            let message=claim_message(&frozen,&reservation,&fence,destination);
            let envelope=audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity{event_id:frozen.event_id,transaction_id:frozen.transaction_id,occurred_at_unix_ms:frozen.occurred_at_unix_ms,channel_id:reservation.channel_id,server_build_id:SERVER_BUILD_ID},message,tuple.tuple())
                .map_err(|_|DurabilityError::InvalidStoredState)?;
            apply_claim(&mut tx,&frozen,&reservation,destination,&envelope,tuple).await?;
            let bytes=finish(&mut tx,gameplay,&frozen.request,&payload,&state,D::Applied).await?;
            tx.commit(deadline).await?;Ok(Ok(bytes))
        })).await?
    }
}
async fn finish(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &RewardClaimMintRequest,
    payload: &[u8],
    state: &MatchState,
    disposition: D,
) -> std::result::Result<Vec<u8>, DurabilityError> {
    let masks = arena::claimed_masks(tx, fence).await?;
    let bytes = encode_match_result(&ArenaResult {
        disposition,
        state: state.wire(arena::now(tx).await?, masks),
    })
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    arena::save(tx, fence, request.command, payload, state, state, &bytes).await?;
    Ok(bytes)
}

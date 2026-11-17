//! Entitlement history is immutable evidence; the live fence is independently re-proved on every action.
use super::super::{
    character_inbox::*,
    db::commit_semantic_transaction,
    item_mint::{TypedDefinitionRef, uuid_text},
    item_transfer::{ItemDefinitionFacts, ItemStackClass},
};
use super::*;
use oteryn_protocol_oteryn::daily_reward::{DailyRewardResult, returner::Reply};
use sha2::{Digest, Sha256};
#[derive(Debug)]
pub enum ReturnerOutcome {
    Announcement(Reply),
    Claim(DailyRewardResult),
}
impl DurabilityRoot {
    pub async fn daily_returner_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        offer: Option<[u8; 16]>,
    ) -> Result<ReturnerOutcome, DailyRewardClaimError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            if let Some(offer)=offer {
                let mut tx=begin_type2_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
                super::super::character_inbox::lock_reward_claim_cause(&mut tx,command).await?;
                if !current(&mut tx,&node,fence,command).await? {return Ok(Err(DailyRewardClaimError::AuthorityRejected));}
                let existing:Option<bool>=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.game_returner_reward_claims c WHERE c.offer_id=e.offer_id) FROM public.game_returner_reward_entitlements e WHERE e.offer_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND e.world_id=encode($3,'hex')::uuid FOR UPDATE OF e")
                    .bind(offer.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                let disposition=match existing {None=>3,Some(true)=>2,Some(false)=>1};
                if disposition!=1 {return Ok(Ok(ReturnerOutcome::Claim(DailyRewardResult{disposition,transaction_id:None})));}
                mint(&mut tx,fence,command,offer).await?;
                tx.commit(deadline).await?;
                Ok(Ok(ReturnerOutcome::Claim(DailyRewardResult{disposition:1,transaction_id:Some(offer)})))
            } else {
                let mut tx=begin_semantic_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
                if !current(&mut tx,&node,fence,command).await? {return Ok(Err(DailyRewardClaimError::AuthorityRejected));}
                let pending:Option<String>=sqlx::query_scalar("SELECT e.offer_id::text FROM public.game_returner_reward_entitlements e WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM public.game_returner_reward_claims c WHERE c.offer_id=e.offer_id) ORDER BY e.created_at_us DESC,e.offer_id DESC LIMIT 1")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                if let Some(id)=pending {return Ok(Ok(ReturnerOutcome::Announcement(Reply{state:1,id:Some(uuid_text(&id)?)})));}
                let saved:Option<String>=sqlx::query_scalar("SELECT offer_id::text FROM public.game_returner_reward_entitlements WHERE source_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid")
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                if let Some(id)=saved {return Ok(Ok(ReturnerOutcome::Announcement(Reply{state:2,id:Some(uuid_text(&id)?)})));}
                let eligible:bool=sqlx::query_scalar("SELECT public.game_returner_session_eligible(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if !eligible {return Ok(Ok(ReturnerOutcome::Announcement(Reply{state:0,id:None})));}
                let id:Option<String>=sqlx::query_scalar("INSERT INTO public.game_returner_reward_entitlements(offer_id,account_id,character_id,world_id,source_session_id,predecessor_session_id) SELECT game_character_uuid_v7(),s.account_id,s.character_id,s.world_id,s.game_session_id,(SELECT p.game_session_id FROM public.game_durability_reconnect_sessions p WHERE p.account_id=s.account_id AND p.training_session_sequence<s.training_session_sequence ORDER BY p.training_session_sequence DESC LIMIT 1) FROM public.game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid ON CONFLICT(account_id,predecessor_session_id) DO NOTHING RETURNING offer_id::text")
                    .bind(command.game_session_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let Some(id)=id else {return Ok(Ok(ReturnerOutcome::Announcement(Reply{state:0,id:None})));};
                commit_semantic_transaction(tx,deadline).await?;
                Ok(Ok(ReturnerOutcome::Announcement(Reply{state:1,id:Some(uuid_text(&id)?)})))
            }
        })).await?
    }
}
async fn mint(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    offer: [u8; 16],
) -> Result<(), DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let text: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    let item = uuid_text(&text)?;
    let (content_revision,ruleset_revision):(String,String)=sqlx::query_as("SELECT content_revision,ruleset_revision FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let binding = Sha256::new()
        .chain_update(b"oteryn:returner:v1\0")
        .chain_update(offer)
        .chain_update(fence.character_id.as_bytes())
        .chain_update(world_id.as_bytes())
        .finalize()
        .into();
    RewardClaimInboxMintCandidate::mint_atomic_in_transaction(
        tx,
        fence,
        RewardClaimInboxPlan {
            command,
            character_id: fence.character_id,
            world_id,
            channel_id,
            transaction_id: offer,
            source_binding: binding,
            item_instance_ids: vec![item],
        },
        RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "ReturnerReward".into(),
                production_key: "oteryn:returner.reward".into(),
                revision_ref: "returner-v1".into(),
            },
            content_revision,
            ruleset_revision,
            sim_revision: "returner-v1".into(),
            outputs: vec![RewardClaimInboxMintOutput {
                source_line: 0,
                item: ItemDefinitionFacts {
                    weight: None,
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i266".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    stack: ItemStackClass::Stackable {
                        proven_maximum: Some(100),
                    },
                    container_capacity: None,
                    container_slot_equip_pattern: false,
                },
                quantity: 1,
                key_number: None,
            }],
        },
    )
    .await
    .map_err(|e| match e {
        RewardClaimInboxError::Unavailable(e) => e,
        _ => DurabilityError::InvalidStoredState,
    })?;
    sqlx::query("INSERT INTO public.game_returner_reward_claims(offer_id,game_session_id,command_id,item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid)")
        .bind(offer.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(item.as_slice()).execute(&mut ***tx).await?;
    Ok(())
}

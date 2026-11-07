//! Closed static12816 rewardbody: actual8133death -> immutableparticipantvisit -> nativeMINT.
use super::super::seasonal_pumpkin;
pub use super::super::seasonal_pumpkin::PumpkinSkinningSource;
use super::*;
use oteryn_gameplay_items::{pumpkin_skinning as rules, reward_bag::reward_bag_bounded_draw};
pub const CLAIM_PREFIX: &str = "oteryn:reward-claim.pumpkin-skinning.";
pub const CLAIM_REVISION: &str =
    "pumpkin-skinning-wiki-c5fbb7ddf71779e795806b92038a209770324cb06e6dd610b5335708765ed742";
pub fn claim_key(command: CommandRef) -> String {
    let session: String = command
        .game_session_id()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{CLAIM_PREFIX}{session}:{}", command.command_id().get())
}
pub fn source_key(source: &PumpkinSkinningSource) -> String {
    fn hex(raw: &[u8]) -> String {
        raw.iter().map(|b| format!("{b:02x}")).collect()
    }
    format!(
        "death:{}:body:{}:revision:{}:map:{}:knife:{}:wire:{}",
        hex(&source.death),
        hex(&source.body_placement),
        source.body_revision,
        hex(&source.map_digest),
        hex(&source.knife),
        hex(&source.wire_binding)
    )
}
impl DurabilityRoot {
    pub async fn probe_pumpkin_skinning(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        wire: [u8; 32],
    ) -> Result<Option<CommittedRewardClaimMint>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let old:Option<Vec<u8>>=sqlx::query_scalar("SELECT wire_binding FROM game_pumpkin_skinning_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid")
    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
   let result=match old {Some(old) if old==wire=>Ok(load_receipt(&mut tx,command).await?.map(|r|decode_receipt(&r)).transpose()?),Some(_)=>Err(RewardClaimMintError::ConflictingCause),None=>Ok(None)};
   tx.commit(deadline).await?;Ok(result)
  })).await?
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "source corpse, original native command, current actor/Node and native Item facts are independent proofs"
    )]
    pub async fn claim_pumpkin_skinning(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        source: PumpkinSkinningSource,
        request: RewardClaimMintRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<RewardClaimMintOutcome> {
        if request.claim.family != "RewardClaim"
            || request.claim.production_key != claim_key(request.command)
            || request.source_placement != source_key(&source)
            || request.claim.revision_ref != CLAIM_REVISION
            || request.item.definition.family != "Item"
            || request.quantity == 0
            || request.quantity > 50
            || !request.contents.is_empty()
            || request.key_number.is_some()
            || request.root_text.is_some()
            || request.achievement.is_some()
            || request.quest_transition.is_some()
            || source.body_placement.len() != 40
            || source.map_digest == [0; 32]
        {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut weights = weights;
        self.try_issue_semantic_pass()?.run_with_context(&mut weights,move|holder,deadline,weights|Box::pin(async move{
   let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
   match seasonal::replay(&mut tx,fence.character_id,&request).await{Ok(Some(old))=>{tx.commit(deadline).await?;return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(old)));},Ok(None)=>{},Err(RewardClaimMintError::Unavailable(e))=>return Err(e),Err(e)=>return Ok(Err(e))};
   let(world,channel)=scope_of(&fence).map_err(|_|DurabilityError::InvalidStoredState)?;
   if !character_item_fence_is_current(&mut tx,&node,&fence,request.command,fence.character_id,*world.as_bytes(),*channel.as_bytes()).await?{return Ok(Err(RewardClaimMintError::AuthorityRejected));}
   sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:pumpkin-skinning:'||encode($1,'hex'),0))").bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
   let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM public.game_daily_effect_now()))::bigint").fetch_one(&mut **tx).await?;
   let cooldown:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_pumpkin_skinning_receipts WHERE character_id=encode($1,'hex')::uuid AND used_at>$2-14400)").bind(fence.character_id.as_bytes().as_slice()).bind(now).fetch_one(&mut **tx).await?;
   if cooldown{return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));}
   let character=crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(|_|DurabilityError::InvalidStoredState)?;
   let Some(visit)=seasonal_pumpkin::live_visit(&mut tx,character,world,Some(source.death)).await? else{return Ok(Err(RewardClaimMintError::AuthorityRejected));};
   let nonce=visit.draw_nonce();
   let index=reward_bag_bounded_draw(visit.source_session,nonce,source.death,rules::DRAW_DOMAIN,13).and_then(|i|usize::try_from(i).ok()).ok_or(DurabilityError::InvalidStoredState)?;
   let reward=rules::select(rules::KNIFE,rules::CORPSE,|domain,upper|reward_bag_bounded_draw(visit.source_session,nonce,source.death,domain,upper)).ok_or(DurabilityError::InvalidStoredState)?;
   if request.item.definition.production_key!=format!("oteryn:item.tibia.i{}",reward.item_id)||request.quantity!=reward.count{return Ok(Err(RewardClaimMintError::InvalidInput));}
   seasonal_pumpkin::arm_source(&mut tx,&node,&fence,source.death,None,Some(request.command),Some(&source),false).await?;
   let command=request.command;
   let minted=match seasonal::mint(&mut tx,&node,&fence,request,*weights).await{Ok(v)=>v,Err(RewardClaimMintError::Unavailable(e))=>return Err(e),Err(e)=>return Ok(Err(e))};
   sqlx::query("INSERT INTO game_pumpkin_skinning_receipts(transaction_id,character_id,world_id,death_id,game_session_id,command_id,knife_id,body_placement,body_revision,map_digest,wire_binding,used_at,draw_index,source_item_id,quantity,created_xact_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,$8,$9::text::numeric,$10,$11,$12,$13,$14,$15,pg_current_xact_id())")
    .bind(minted.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(source.death.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(source.knife.as_slice()).bind(source.body_placement.as_slice()).bind(source.body_revision.to_string()).bind(source.map_digest.as_slice()).bind(source.wire_binding.as_slice()).bind(now).bind(i16::try_from(index).map_err(|_|DurabilityError::InvalidStoredState)?).bind(i64::from(reward.item_id)).bind(i64::from(reward.count)).execute(&mut **tx).await?;
   super::super::account_achievement::grant_missing(&mut tx,fence.character_id,
    &["oteryn:achievement/mutated_presents"],ACHIEVEMENT_SOURCE_KIND,&minted.event_id).await?;
   tx.commit(deadline).await?;Ok(Ok(RewardClaimMintOutcome::Committed(minted)))
  })).await?
    }
}

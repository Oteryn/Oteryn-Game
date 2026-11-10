use super::*;
use oteryn_gameplay_progression::daily_reward::picker::Offer;
pub(super) async fn offer_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    epoch: u64,
    day: u8,
    premium: bool,
) -> Result<Offer, DurabilityError> {
    let vocation:Option<String>=sqlx::query_scalar("SELECT vocation FROM public.game_character_build_state WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    oteryn_gameplay_progression::daily_reward::picker::offer(
        epoch,
        day,
        premium,
        vocation.as_deref().unwrap_or("none"),
    )
    .ok_or(DurabilityError::InvalidStoredState)
}
impl DurabilityRoot {
    pub async fn read_daily_reward_picker<G: DailyRewardPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        gate: &G,
    ) -> Result<oteryn_protocol_oteryn::daily_reward::picker::Offer, DailyRewardClaimError> {
        let view = self
            .read_daily_reward_view(authority, node, fence, command)
            .await?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate,move |holder,deadline,gate|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if !current(&mut tx,&node,fence,command).await? {return Ok(Err(DailyRewardClaimError::AuthorityRejected));}
            let account:String=sqlx::query_scalar("SELECT account_id::text FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let premium=gate.current(&mut tx,super::super::item_mint::uuid_text(&account)?).await?;
            let o=offer_in_transaction(&mut tx,fence,view.epoch,view.lane_day,premium).await?;
            Ok(Ok(oteryn_protocol_oteryn::daily_reward::picker::Offer {epoch:o.epoch,day:o.day,premium:o.premium,maximum:if view.receipt.is_some(){0}else{o.maximum},items:o.items}))
        })).await?
    }
}

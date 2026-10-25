//! Complete DAILY transaction: Character then Account/World serialization, all
//! ordered lines, native Inbox MINT and permanent item bindings in one commit.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_inbox::{
    RewardClaimInboxError, RewardClaimInboxMintOutput, RewardClaimInboxMintRequest,
    RewardClaimInboxOutcome, RewardClaimInboxPlan,
};
use super::daily_reward::{load_receipt, load_state};
use super::daily_reward_claim::{
    DailyRewardClaimError, DailyRewardClaimOutcome, DailyRewardClaimRequest, write_claim,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone)]
pub struct DailyRewardDeliveryPlan {
    pub lane_day: u8,
    pub content_revision: String,
    pub definition_revision: String,
    pub outputs: Vec<RewardClaimInboxMintOutput>,
    pub charges: Vec<Option<u32>>,
    pub item_instance_ids: Vec<[u8; 16]>,
}

#[derive(Debug, Clone)]
pub struct DailyRewardView {
    pub epoch: u64,
    pub lane_day: u8,
    pub streak: u32,
    pub jokers: u8,
    pub receipt: Option<super::daily_reward::DailyRewardReceipt>,
    pub settled: Option<super::daily_reward::DailyRewardReceipt>,
}

impl DurabilityRoot {
    /// A current-session read; neither a caller epoch nor a local clock opens a window.
    pub async fn read_daily_reward_view(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<DailyRewardView, DailyRewardClaimError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !current(&mut tx, &node, fence, command).await? {
                return Ok(Err(DailyRewardClaimError::AuthorityRejected));
            }
            let epoch: Option<String> = sqlx::query_scalar("SELECT h.reset_epoch::text FROM public.game_world_reset_history h WHERE h.world_id=encode($1,'hex')::uuid ORDER BY h.reset_epoch DESC LIMIT 1")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(epoch) = epoch else { return Ok(Err(DailyRewardClaimError::ResetWindowUnavailable)); };
            let epoch = epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let state = load_state(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes()).await?;
            let receipt = load_receipt(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes(), epoch).await?;
            let jokers: Option<i16> = sqlx::query_scalar("SELECT j.jokers FROM public.game_account_daily_reward_jokers j JOIN public.game_character_roots r ON r.account_id=j.account_id AND r.world_id=j.world_id WHERE r.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let saved: Option<String> = sqlx::query_scalar("SELECT c.reset_epoch::text FROM public.game_daily_reward_commands d JOIN public.game_character_daily_reward_claims c ON c.transaction_id=d.claim_id WHERE d.game_session_id=encode($1,'hex')::uuid AND d.command_id=$2::text::numeric AND c.character_id=encode($3,'hex')::uuid AND c.world_id=encode($4,'hex')::uuid")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let settled = match saved { Some(epoch) => load_receipt(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes(), epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?).await?, None => None };
            Ok(Ok(DailyRewardView { epoch, lane_day: state.lane_day, streak: state.streak,
                jokers: u8::try_from(jokers.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?, receipt, settled }))
        })).await?
    }

    /// All value writes share the MAIL-owned transaction when items are present.
    /// The non-item days use the same native recovery/admission/fence boundary.
    pub async fn commit_daily_reward_claim(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: DailyRewardClaimRequest,
        mut plan: DailyRewardDeliveryPlan,
    ) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
        use DailyRewardClaimError as Error;
        if request.reset_epoch == 0
            || request.transaction_id[6] >> 4 != 7
            || request.transaction_id[8] >> 6 != 2
            || !(1..=7).contains(&plan.lane_day)
            || plan.outputs.len() > 20
            || plan
                .outputs
                .iter()
                .enumerate()
                .any(|(index, output)| output.source_line != index as u32)
            || plan
                .charges
                .iter()
                .flatten()
                .any(|charges| !(1..=65535).contains(charges))
            || plan.charges.len() != plan.outputs.len()
            || plan.item_instance_ids.len() != plan.outputs.len()
            || (matches!(plan.lane_day, 3 | 7) != plan.outputs.is_empty())
        {
            return Err(Error::InvalidRequest);
        }
        // Recheck current authority even for a historical claim; it grants no new value.
        let view = self
            .read_daily_reward_view(authority, node, fence, request.command)
            .await?;
        if let Some(receipt) = view.settled {
            return if receipt.request_binding == request.request_binding {
                Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
            } else {
                Err(Error::BindingMismatch)
            };
        }
        if request.reset_epoch != view.epoch {
            return Err(Error::ResetWindowUnavailable);
        }
        if let Some(receipt) = view.receipt {
            return if receipt.request_binding == request.request_binding {
                Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
            } else {
                Err(Error::BindingMismatch)
            };
        }
        if view.lane_day != plan.lane_day {
            return Err(Error::InvalidRequest);
        }
        if !plan.outputs.is_empty() {
            // Reuse a preparation of this CommandRef after an ambiguous attempt/restart.
            // These immutable identities define the plan, never the current fence.
            if let Some((transaction, items)) = self
                .daily_prepared_ids(authority, request.command, request.request_binding)
                .await?
            {
                plan.item_instance_ids = items;
                let request = DailyRewardClaimRequest {
                    transaction_id: transaction,
                    ..request
                };
                return self
                    .commit_daily_items(authority, node, fence, request, plan)
                    .await;
            }
            return self
                .commit_daily_items(authority, node, fence, request, plan)
                .await;
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !current(&mut tx, &node, fence, request.command).await? {
                        return Ok(Err(Error::AuthorityRejected));
                    }
                    let outcome = apply(&mut tx, fence, request, &plan).await;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }

    async fn commit_daily_items(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: DailyRewardClaimRequest,
        plan: DailyRewardDeliveryPlan,
    ) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DailyRewardClaimError::AuthorityRejected);
        };
        let source = RewardClaimInboxPlan {
            command: request.command,
            character_id: fence.character_id,
            world_id,
            channel_id,
            transaction_id: request.transaction_id,
            source_binding: request.request_binding,
            item_instance_ids: plan.item_instance_ids.clone(),
        };
        let mint = RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: "oteryn:daily-reward".into(),
                revision_ref: plan.definition_revision.clone(),
            },
            content_revision: plan.content_revision.clone(),
            ruleset_revision: "daily-reward-r1".into(),
            sim_revision: "daily-reward-r1".into(),
            outputs: plan.outputs.clone(),
        };
        let mut candidate = self
            .prepare_reward_claim_inbox_mint(authority, node, fence, &source, mint)
            .await
            .map_err(inbox_error)?;
        let mut outcome = None;
        let result = self
            .commit_reward_claim_inbox_mint(
                authority,
                node,
                fence,
                &mut candidate,
                &mut outcome,
                move |context, outcome| {
                    Box::pin(async move {
                        let result = apply(context.transaction(), fence, request, &plan).await;
                        let committed = matches!(result, Ok(DailyRewardClaimOutcome::Committed(_)));
                        *outcome = Some(result);
                        if committed {
                            Ok(())
                        } else {
                            Err(RewardClaimInboxError::SourceRefused)
                        }
                    })
                },
            )
            .await;
        match (result, outcome) {
            (Ok(RewardClaimInboxOutcome::Committed { .. }), Some(result)) => result,
            (Err(RewardClaimInboxError::SourceRefused), Some(result)) => result,
            (Err(error), _) => Err(inbox_error(error)),
            (Ok(RewardClaimInboxOutcome::AlreadyCommitted(_)), _) => {
                let view = self
                    .read_daily_reward_view(authority, node, fence, request.command)
                    .await?;
                view.receipt
                    .filter(|r| r.request_binding == request.request_binding)
                    .map(DailyRewardClaimOutcome::AlreadyClaimed)
                    .ok_or(DailyRewardClaimError::BindingMismatch)
            }
            _ => Err(DailyRewardClaimError::InvalidRequest),
        }
    }

    async fn daily_prepared_ids(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        command: CommandRef,
        binding: [u8; 32],
    ) -> Result<Option<([u8; 16], Vec<[u8; 16]>)>, DailyRewardClaimError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT transaction_id::text, item_instance_ids::text[], source_binding FROM public.game_reward_claim_inbox_mint_reservations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { return Ok(Ok(None)); };
            if row.try_get::<Vec<u8>,_>("source_binding")? != binding { return Ok(Err(DailyRewardClaimError::BindingMismatch)); }
            let items = row.try_get::<Vec<String>,_>("item_instance_ids")?.iter().map(|s| super::item_mint::uuid_text(s)).collect::<Result<Vec<_>,_>>()?;
            Ok(Ok(Some((super::item_mint::uuid_text(row.try_get("transaction_id")?)?, items))))
        })).await?
    }
}

fn inbox_error(error: RewardClaimInboxError) -> DailyRewardClaimError {
    match error {
        RewardClaimInboxError::Unavailable(e) => DailyRewardClaimError::Durability(e),
        RewardClaimInboxError::ConflictingDelivery => DailyRewardClaimError::BindingMismatch,
        _ => DailyRewardClaimError::AuthorityRejected,
    }
}

async fn current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
) -> Result<bool, DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    character_item_fence_is_current(
        tx,
        node,
        &fence,
        command,
        fence.character_id,
        *world_id.as_bytes(),
        *channel_id.as_bytes(),
    )
    .await
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    plan: &DailyRewardDeliveryPlan,
) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
    let world = fence.runtime_scope.world_id();
    // A competing claim must not retain the mint performed before this callback.
    if let Some(receipt) = load_receipt(
        tx,
        fence.character_id.as_bytes(),
        world.as_bytes(),
        request.reset_epoch,
    )
    .await?
    {
        return if receipt.request_binding == request.request_binding {
            Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
        } else {
            Err(DailyRewardClaimError::BindingMismatch)
        };
    }
    let state = load_state(tx, fence.character_id.as_bytes(), world.as_bytes()).await?;
    if state.lane_day != plan.lane_day {
        return Err(DailyRewardClaimError::InvalidRequest);
    }
    let joker = super::daily_reward_jokers::grant(tx, fence, request).await?;
    let outcome = write_claim(tx, fence, request, joker.balance).await?;
    let DailyRewardClaimOutcome::Committed(receipt) = &outcome else {
        return Err(DailyRewardClaimError::InvalidRequest);
    };
    super::daily_reward_jokers::spend(tx, fence, request, joker, receipt.jokers_spent).await?;
    sqlx::query("INSERT INTO public.game_daily_reward_commands VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
    if plan.outputs.is_empty() {
        let (kind, quantity) = if plan.lane_day == 3 {
            ("PREY_WILDCARD", 1)
        } else {
            ("XP_BOOST", 10)
        };
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status) VALUES(encode($1,'hex')::uuid,0,$2,$3,'PENDING')")
            .bind(request.transaction_id.as_slice()).bind(kind).bind(quantity).execute(&mut **tx).await?;
    }
    for (index, ((output, item), charges)) in plan
        .outputs
        .iter()
        .zip(&plan.item_instance_ids)
        .zip(&plan.charges)
        .enumerate()
    {
        sqlx::query("INSERT INTO public.game_item_bindings VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'DAILY_REWARD')")
            .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,definition,quantity,status,delivering_transaction_id,item_instance_id,charges) VALUES(encode($1,'hex')::uuid,$2,'ITEM',$3,$4,'DELIVERED',encode($1,'hex')::uuid,encode($5,'hex')::uuid,$6)")
            .bind(request.transaction_id.as_slice()).bind(index as i16).bind(&output.item.definition.production_key)
            .bind(i32::try_from(output.quantity).map_err(|_| DailyRewardClaimError::InvalidRequest)?)
            .bind(item.as_slice()).bind(charges.map(|c| c as i32)).execute(&mut **tx).await?;
    }
    Ok(outcome)
}

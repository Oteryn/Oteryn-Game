//! Live Forge adapter. Rules, native item/fee/dust owners and retained outcomes
//! share one session-fenced Type-2 transaction; reservation fixes every random draw.
use super::*;
use crate::domain::forge::ItemTier;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::forge::{
    ForgeAttempt, ForgeItem, ForgeOperation, ForgePlan, ForgeQuote, ForgeRequest, ForgeResources,
    ForgeRuleError,
};
use oteryn_protocol_oteryn::item_forge::{Operation, Request};
use serde_json::{Value, json};

#[path = "item_forge_live/actions.rs"]
mod actions;
#[path = "item_forge_live/audit.rs"]
pub(crate) mod audit;
#[path = "item_forge_live/dust_limit.rs"]
mod dust_limit;
#[path = "item_forge_live/inventory.rs"]
mod inventory;
#[path = "item_forge_live/storage.rs"]
mod storage;
#[cfg(test)]
#[path = "item_forge_live/tests.rs"]
mod tests;

fn reply(receipt: &ForgeDustLimitReceipt) -> Value {
    json!({"operation":"dust_limit",
        "succeeded":receipt.disposition==ForgeDustLimitDisposition::Raised,
        "result":receipt.entry.map(|entry| json!({"dust":entry.after.balance(),"dust_limit":entry.after.limit().get()}))})
}

async fn retained_reply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: Request,
    intent: &[u8],
) -> LimitResult<Option<Value>> {
    storage::check_command(tx, fence, command, intent).await?;
    if let Some(reply) =
        storage::read_reply(tx, fence.character_id, request.occurrence, intent).await?
    {
        return Ok(Some(reply));
    }
    // Retain pre-P18 DustLimit receipts under the same occurrence namespace.
    if let Some(frozen) =
        load_dust_limit_reservation(tx, fence.character_id, request.occurrence).await?
    {
        if request.operation != Operation::DustLimit {
            return Err(ForgeDustLimitError::ConflictingOccurrence);
        }
        return Ok(load_dust_limit_receipt(tx, &frozen)
            .await?
            .as_ref()
            .map(reply));
    }
    Ok(None)
}

impl DurabilityRoot {
    /// Receipt-only probe. It checks the current session/scope independently but needs
    /// neither today's item custody nor today's content to return a retained outcome.
    pub async fn read_item_forge_reply(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: Request,
    ) -> LimitResult<Option<Value>> {
        let intent = request.encode().ok_or(ForgeDustLimitError::InvalidInput)?;
        if request.operation == Operation::QueryHistory {
            return Ok(None);
        }
        check_uuid_v7(&request.occurrence).map_err(|_| ForgeDustLimitError::InvalidInput)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    storage::lock(&mut tx, fence.character_id, request.occurrence).await?;
                    lock_dust_limit_cause(&mut tx, fence.character_id, request.occurrence).await?;
                    let Ok((world, channel)) = scope_of(&fence) else {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    };
                    if !lock_dust_limit_current(
                        &mut tx,
                        &node,
                        &fence,
                        command,
                        *world.as_bytes(),
                        *channel.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    let result = retained_reply(&mut tx, fence, command, request, &intent).await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }

    /// The mutation authority is explicit: the caller holds the live spell lane,
    /// while the transaction independently checks session, lease and scope ownership.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub async fn execute_item_forge(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: Request,
        resolved_items: Option<ForgeResolvedItems>,
        native: &NativeGameplayState,
        content: &dyn crate::durability::item_loop_move::ItemLoopContent,
    ) -> LimitResult<Value> {
        let intent = request
            .encode()
            .ok_or(ForgeDustLimitError::InvalidInput)?
            .to_vec();
        if request.operation != Operation::QueryHistory {
            check_uuid_v7(&request.occurrence).map_err(|_| ForgeDustLimitError::InvalidInput)?;
        }
        let resolved_items = resolved_items.filter(|items| items.first != items.second);
        if let Some(items) = resolved_items {
            if !request.operation.requires_pair() {
                return Err(ForgeDustLimitError::InvalidInput);
            }
            check_uuid_v7(&items.first).map_err(|_| ForgeDustLimitError::InvalidInput)?;
            check_uuid_v7(&items.second).map_err(|_| ForgeDustLimitError::InvalidInput)?;
        }
        permit.check_scope(fence.runtime_scope)?;
        if content.generation_digest() != native.source_digest() {
            return Err(ForgeDustLimitError::MissingActiveContent);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let reserve_node = node.clone();
        let digest = native.source_digest();
        let reserve_intent = intent.clone();
        let reserve_recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let prepared = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &reserve_recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    storage::lock(&mut tx, fence.character_id, request.occurrence).await?;
                    lock_dust_limit_cause(&mut tx, fence.character_id, request.occurrence).await?;
                    let Ok((world, channel)) = scope_of(&fence) else {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    };
                    if !lock_dust_limit_current(
                        &mut tx,
                        &reserve_node,
                        &fence,
                        command,
                        *world.as_bytes(),
                        *channel.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    if request.operation == Operation::QueryHistory {
                        let now = storage::now(&mut tx).await?;
                        let result =
                            if storage::charge_cooldown(&mut tx, fence.character_id, now).await? {
                                actions::refusal(request, "COOLDOWN")
                            } else {
                                storage::history(&mut tx, fence.character_id, request.history_limit)
                                    .await?
                            };
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(Err(result)));
                    }
                    match retained_reply(&mut tx, fence, command, request, &reserve_intent).await {
                        Ok(Some(reply)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(Err(reply)));
                        }
                        Err(error) => return Ok(Err(error)),
                        Ok(None) => {}
                    }
                    let reserved = if let Some(reserved) =
                        storage::load(&mut tx, fence.character_id, request.occurrence).await?
                    {
                        if reserved.intent != reserve_intent {
                            return Ok(Err(ForgeDustLimitError::ConflictingOccurrence));
                        }
                        reserved
                    } else {
                        let Some(content) = current_dust_limit_content(
                            &mut tx,
                            *world.as_bytes(),
                            *channel.as_bytes(),
                        )
                        .await?
                        else {
                            return Ok(Err(ForgeDustLimitError::MissingActiveContent));
                        };
                        if content != digest {
                            return Ok(Err(ForgeDustLimitError::MissingActiveContent));
                        }
                        storage::insert(
                            &mut tx,
                            fence,
                            request,
                            reserve_intent,
                            resolved_items,
                            content,
                        )
                        .await?
                    };
                    if reserved.world != *world.as_bytes()
                        || reserved.channel != *channel.as_bytes()
                    {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    if !storage::charge_attempt(&mut tx, &reserved).await? {
                        return Ok(Err(ForgeDustLimitError::CapacityExceeded));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Ok(reserved)))
                })
            })
            .await??;
        let reserved = match prepared {
            Ok(reserved) => reserved,
            Err(reply) => return Ok(reply),
        };
        let node = node.clone();
        let mut context = (native, content);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (native, content) = *context;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    storage::lock(&mut tx, fence.character_id, request.occurrence).await?;
                    lock_dust_limit_cause(&mut tx, fence.character_id, request.occurrence).await?;
                    let Ok((world, channel)) = scope_of(&fence) else {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    };
                    if !lock_dust_limit_current(
                        &mut tx,
                        &node,
                        &fence,
                        command,
                        *world.as_bytes(),
                        *channel.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    match retained_reply(&mut tx, fence, command, request, &intent).await {
                        Ok(Some(reply)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(reply));
                        }
                        Err(error) => return Ok(Err(error)),
                        Ok(None) => {}
                    }
                    if reserved.world != *world.as_bytes()
                        || reserved.channel != *channel.as_bytes()
                    {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    let active =
                        current_dust_limit_content(&mut tx, *world.as_bytes(), *channel.as_bytes())
                            .await?;
                    let reason = if active != Some(reserved.content)
                        || native.source_digest() != reserved.content
                        || reserved.ruleset != storage::RULESET
                        || reserved.simulation_revision
                            != oteryn_simulation_determinism::active_profile()
                                .revision()
                                .get()
                    {
                        Some("REVISION_CHANGED")
                    } else if reserved.exhausted {
                        Some("COOLDOWN")
                    } else if request.operation.requires_pair() && reserved.resolved_items.is_none()
                    {
                        Some("INVALID_ITEM_HANDLE")
                    } else {
                        None
                    };
                    let result = if let Some(reason) = reason {
                        let reply = actions::refusal(request, reason);
                        actions::record(&mut tx, fence, command, request, &reserved, &reply, None)
                            .await?;
                        reply
                    } else if request.operation == Operation::DustLimit {
                        dust_limit::apply(&mut tx, fence, command, request, &reserved).await?
                    } else {
                        let planned = match inventory::read(&mut tx, fence, native).await? {
                            Some(inventory) => {
                                match actions::plan(
                                    &mut tx, fence, request, &reserved, native, &inventory, content,
                                )
                                .await?
                                {
                                    Ok(effects) => {
                                        sqlx::query("SAVEPOINT forge_effects")
                                            .execute(&mut **tx)
                                            .await?;
                                        actions::record(
                                            &mut tx,
                                            fence,
                                            command,
                                            request,
                                            &reserved,
                                            &effects.reply,
                                            Some(&effects),
                                        )
                                        .await?;
                                        match actions::apply(
                                            &mut tx, fence, command, request, &reserved, native,
                                            &inventory, &effects,
                                        )
                                        .await?
                                        {
                                            Ok(()) => {
                                                sqlx::query("RELEASE SAVEPOINT forge_effects")
                                                    .execute(&mut **tx)
                                                    .await?;
                                                Ok(effects.reply)
                                            }
                                            Err(reason) => {
                                                sqlx::query("ROLLBACK TO SAVEPOINT forge_effects")
                                                    .execute(&mut **tx)
                                                    .await?;
                                                sqlx::query("RELEASE SAVEPOINT forge_effects")
                                                    .execute(&mut **tx)
                                                    .await?;
                                                Err(reason)
                                            }
                                        }
                                    }
                                    Err(reason) => Err(reason),
                                }
                            }
                            None => Err("WRONG_ITEMS"),
                        };
                        match planned {
                            Ok(reply) => reply,
                            Err(reason) => {
                                let reply = actions::refusal(request, reason);
                                actions::record(
                                    &mut tx, fence, command, request, &reserved, &reply, None,
                                )
                                .await?;
                                reply
                            }
                        }
                    };
                    tx.commit(deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }
}

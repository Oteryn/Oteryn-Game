//! Live transport adapter for the admitted native DustLimit owner and its history.
//! Fusion, transfer and item conversions await native item/fee/ForgeCause composition.
use super::*;
use oteryn_protocol_oteryn::item_forge::{Operation, Request};
use serde_json::{Value, json};

fn reply(receipt: &ForgeDustLimitReceipt) -> Value {
    json!({
        "operation": "dust_limit",
        "succeeded": receipt.disposition == ForgeDustLimitDisposition::Raised,
        "result": receipt.entry.map(|entry| json!({
            "dust": entry.after.balance(), "dust_limit": entry.after.limit().get()
        })),
    })
}

impl super::super::db::DurabilityRoot {
    /// Run the admitted dust-only slice through its existing native, fenced receipt owner.
    /// Other wire operations refuse without touching the staged migration-0320 stores.
    pub async fn execute_item_forge(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: Request,
    ) -> LimitResult<Value> {
        if !matches!(
            request.operation,
            Operation::DustLimit | Operation::QueryHistory
        ) || request.first_item.is_some()
            || request.second_item.is_some()
            || request.history_limit > oteryn_protocol_oteryn::item_forge::MAX_HISTORY
            || (request.operation == Operation::DustLimit
                && (check_uuid_v7(&request.occurrence).is_err() || request.history_limit != 0))
        {
            return Err(ForgeDustLimitError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let node_for_read = node.clone();
        let prepared = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let Ok((world, channel)) = scope_of(&fence) else {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    };
                    if request.operation == Operation::DustLimit {
                        lock_dust_limit_cause(&mut tx, fence.character_id, request.occurrence)
                            .await?;
                    }
                    // All reads and reservations bind independently current session/scope facts.
                    if !lock_dust_limit_current(
                        &mut tx,
                        &node_for_read,
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
                        let entries = if request.history_limit == 0 {
                            Vec::new()
                        } else {
                            match read_forge_dust_limit_history(
                                &mut tx,
                                fence.character_id,
                                None,
                                u16::from(request.history_limit),
                            )
                            .await
                            {
                                Ok(page) => page.entries,
                                Err(error) => return Ok(Err(error)),
                            }
                        };
                        let result = Value::Array(entries.iter().map(|entry| {
                        json!({"ordinal": entry.ordinal, "receipt": reply(&entry.receipt)})
                    }).collect());
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(Err(result)));
                    }
                    // Reuse a retained reservation's quote rather than quoting a retry at the
                    // newer balance/limit. The native commit also returns terminal receipts first.
                    let expected_upgrade_count = match load_dust_limit_reservation(
                        &mut tx,
                        fence.character_id,
                        request.occurrence,
                    )
                    .await?
                    {
                        Some(frozen) => frozen.request.expected_upgrade_count,
                        None => read_forge_dust(&mut tx, fence.character_id)
                            .await?
                            .limit()
                            .upgrade_count(),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Ok(expected_upgrade_count)))
                })
            })
            .await??;
        let expected_upgrade_count = match prepared {
            Ok(count) => count,
            Err(history) => return Ok(history),
        };
        let candidate = self
            .freeze_forge_dust_limit(
                authority,
                node,
                fence,
                command,
                ForgeDustLimitRequest {
                    occurrence_id: request.occurrence,
                    expected_upgrade_count,
                },
            )
            .await?;
        let outcome = self
            .commit_forge_dust_limit(authority, node, fence, command, &candidate)
            .await?;
        Ok(reply(&outcome.receipt))
    }
}

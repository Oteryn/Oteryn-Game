//! Claim an exact native Inbox item using the Platform-issued CLAIM identity.
use super::{ComposedFreshAdmission, gameplay_fence, receipt_outbox, uuid_text, wire};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::native_admission_source::store::{OperationCursor, OperationKind};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};

const FOREGROUND_PLATFORM_BUDGET: Duration = Duration::from_secs(2);

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: wire::StoreInboxClaimRequest,
) -> Option<wire::StoreClaimReceipt> {
    let deadline = Instant::now() + FOREGROUND_PLATFORM_BUDGET;
    let account = admitted.controller?.account_id;
    let fence = gameplay_fence(owner, admitted).await?;
    let world = *admitted.world_id.as_bytes();
    let character = *fence.character_id.as_bytes();
    let read = || {
        owner.root.read_store_claim_operation(
            account,
            character,
            world,
            request.fulfilment_unit_id.clone(),
        )
    };
    let platform = owner.store.as_ref()?;
    let (delivery, operation, item) = if let Some(binding) = read().await.ok()? {
        binding
    } else {
        // The outbox worker discovers claims across all pages. A command can
        // find a newly issued claim on the first page within its remaining budget.
        let account = uuid_text(&account);
        let intent = timeout_at(deadline, async {
            let page = platform
                .lock()
                .await
                .list_pending(&OperationCursor::default())
                .await
                .ok()?;
            for pending in page.entries {
                if pending.operation_kind != OperationKind::Claim {
                    continue;
                }
                let Ok(intent) = platform
                    .lock()
                    .await
                    .read_intent(OperationKind::Claim, &pending.operation_id)
                    .await
                else {
                    continue;
                };
                if intent.account_id == account
                    && intent.entitlement_id == request.fulfilment_unit_id
                {
                    return Some(intent);
                }
            }
            None
        })
        .await
        .ok()??;
        receipt_outbox::remember_claim(owner.root, &intent).await?;
        read().await.ok()??
    };
    if owner
        .root
        .has_store_restore_conflict(operation)
        .await
        .ok()?
    {
        return None;
    }

    let (receipt, unit) = if let Some(source) =
        owner.root.read_store_claim_source(delivery).await.ok()?
    {
        source
    } else {
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
        let native = owner.active_generation?.native_gameplay()?;
        let permit = owner.spell_lane_permit().await?;
        owner
            .root
            .commit_store_inbox_retrieval(
                &permit,
                owner.character,
                owner.holder,
                fence.item_fence(),
                command,
                &NativeItemWeights(native).with_wheel(owner.premium_coordinator),
                item,
                delivery,
            )
            .await
            .ok()?
            .ok()??
    };

    // The worker republishes from the durable move when this attempt runs out
    // of time or the original generation needs reconciliation.
    let _ = timeout_at(deadline, async {
        let intent = platform
            .lock()
            .await
            .read_intent(OperationKind::Claim, &uuid_text(&operation))
            .await
            .ok()?;
        if intent.account_id != receipt.account_id
            || intent.entitlement_id != receipt.fulfilment_unit_id
        {
            return None;
        }
        let mut client = platform.lock().await;
        let bytes = client
            .claim_receipt(
                &intent,
                &receipt.character_id,
                &receipt.world_id,
                &unit,
                &receipt.recovery_generation,
                &receipt.decided_at,
            )
            .ok()?;
        client.publish_receipt(&bytes).await.ok()
    })
    .await;

    Some(wire::StoreClaimReceipt {
        operation_id: operation,
        fulfilment_unit_id: request.fulfilment_unit_id,
        result: wire::StoreClaimCode::Committed,
        recovery_generation: receipt.recovery_generation.parse().ok()?,
        decided_at_unix_ms: receipt.decided_at.parse::<u64>().ok()?.checked_mul(1_000)?,
    })
}

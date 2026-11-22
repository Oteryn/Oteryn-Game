//! Claim an exact native Inbox item using the Platform-issued CLAIM identity.
use super::{ComposedFreshAdmission, gameplay_fence, receipt_outbox, uuid_text, wire};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::native_admission_source::store::{OperationCursor, OperationKind};

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: wire::StoreInboxClaimRequest,
) -> Option<wire::StoreClaimReceipt> {
    let account = admitted.controller?.account_id;
    let fence = gameplay_fence(owner, admitted).await?;
    let world = *admitted.world_id.as_bytes();
    let character = *fence.character_id.as_bytes();
    let read = || {
        owner.root.read_store_claim_operation(
            account, character, world, request.fulfilment_unit_id.clone(),
        )
    };
    let platform = owner.store.as_ref()?;
    let (delivery, operation, item) = if let Some(binding) = read().await.ok()? {
        binding
    } else {
        // The worker discovers later pages if the claim is not on the first page yet.
        let mut client = platform.lock().await;
        let page = client.list_pending(&OperationCursor::default()).await.ok()?;
        for pending in page.entries {
            if pending.operation_kind != OperationKind::Claim {
                continue;
            }
            let Ok(intent) = client.read_intent(OperationKind::Claim, &pending.operation_id).await else {
                continue;
            };
            if intent.account_id == uuid_text(&account) && intent.entitlement_id == request.fulfilment_unit_id {
                receipt_outbox::remember_claim(owner.root, &intent).await?;
                break;
            }
        }
        read().await.ok()??
    };
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    if owner.root.read_store_claim_source(delivery).await.ok()?.is_none() {
        let native = owner.active_generation?.native_gameplay()?;
        let permit = owner.spell_lane_permit().await?;
        let _ = owner.root.commit_store_inbox_retrieval(
            &permit, owner.character, owner.holder, fence.item_fence(), command,
            &NativeItemWeights(native).with_wheel(owner.premium_coordinator), item,
        ).await.ok()?;
    }
    let source = owner.root.read_store_claim_source(delivery).await.ok()?;
    let (result, generation, decided_at) = if let Some((receipt, _)) = source {
        let mut client = platform.lock().await;
        let _ = receipt_outbox::publish_claim(owner.root, &mut client, &uuid_text(&operation), false).await;
        (
            wire::StoreClaimCode::Committed,
            receipt.recovery_generation.parse::<u64>().ok()?,
            receipt.decided_at.parse::<u64>().ok()?.checked_mul(1_000)?,
        )
    } else {
        let generation = owner.root.store_publication_generation().await.ok()??;
        (wire::StoreClaimCode::NotClaimable, generation.parse::<u64>().ok()?, 0)
    };
    Some(wire::StoreClaimReceipt {
        operation_id: operation,
        fulfilment_unit_id: request.fulfilment_unit_id,
        result,
        recovery_generation: generation,
        decided_at_unix_ms: decided_at,
    })
}

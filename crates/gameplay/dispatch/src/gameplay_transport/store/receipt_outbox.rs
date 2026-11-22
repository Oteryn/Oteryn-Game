//! Republish committed deliveries without an active buyer session.
use super::{StoreClient, uuid_bytes};
use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
    store::{DeliveryReceipt, PaidStoreCosmeticIntent},
};
use crate::native_admission_source::store::{DeliveryIntent, OperationCursor, OperationKind, ProtocolError};
use std::time::Duration;

pub async fn run_store_receipt_outbox(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    mut client: StoreClient,
) {
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut cursor = OperationCursor::default();
    loop {
        tick.tick().await;
        publish_pending(root, &mut client).await;
        reconcile_platform_operations(root, authority, &mut client, &mut cursor).await;
    }
}

pub(in crate::gameplay_transport) async fn publish_pending(
    root: &DurabilityRoot,
    client: &mut StoreClient,
) {
    let Ok(receipts) = root.list_unacknowledged_store_receipts(16).await else {
        return;
    };
    for receipt in receipts {
        let Some(id) = uuid_bytes(&receipt.operation_id) else {
            continue;
        };
        let acknowledged = async {
            let intent = client
                .read_intent(OperationKind::Deliver, &receipt.operation_id)
                .await
                .ok()?;
            if intent.account_id != receipt.account_id
                || intent.entitlement_id != receipt.fulfilment_unit_id
            {
                return None;
            }
            publish_delivery(root, client, &intent, &receipt).await.ok()
        }
        .await
        .is_some();
        let _ = root.record_store_receipt_attempt(id, acknowledged).await;
    }
}

pub(super) async fn publish_delivery(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    intent: &DeliveryIntent,
    receipt: &DeliveryReceipt,
) -> Result<(), ProtocolError> {
    publish_at_current_generation(root, client, receipt, |client, generation| {
        client.delivery_receipt(intent, generation, &receipt.decided_at)
    }).await
}

async fn publish_at_current_generation(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    receipt: &DeliveryReceipt,
    encode: impl Fn(&StoreClient, &str) -> Result<Vec<u8>, ProtocolError>,
) -> Result<(), ProtocolError> {
    let bytes = encode(client, &receipt.recovery_generation)?;
    let client_result = client.publish_receipt(&bytes).await;
    if client_result != Err(ProtocolError::Conflict) {
        // An identical prior publication is accepted before the restore fence check.
        return client_result;
    }
    let generation = root.store_publication_generation().await.ok().flatten()
        .filter(|current| {
            current.parse::<u64>().ok() > receipt.recovery_generation.parse::<u64>().ok()
        })
        .ok_or(ProtocolError::Conflict)?;
    let bytes = encode(client, &generation)?;
    client.publish_receipt(&bytes).await
}

/// Platform allocates CLAIM ids only after accepting DELIVER. The durable delivery
/// receipt remains the proof even when the buyer has gone offline.
async fn reconcile_platform_operations(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: &mut StoreClient,
    cursor: &mut OperationCursor,
) {
    let Ok(page) = client.list_pending(cursor).await else {
        return;
    };
    for pending in &page.entries {
        match pending.operation_kind {
            OperationKind::Claim => { let _ = publish_claim(root, client, &pending.operation_id).await; }
            OperationKind::Deliver => { let _ = recover_cosmetic(root, authority, client, &pending.operation_id).await; }
            _ => {}
        }
    }
    *cursor = page.entries.last().map_or_else(OperationCursor::default, OperationCursor::from);
}

async fn publish_claim(root: &DurabilityRoot, client: &mut StoreClient, id: &str) -> Option<()> {
    let intent = client.read_intent(OperationKind::Claim, id).await.ok()?;
    let delivery_id = uuid_bytes(intent.delivery_operation_id.as_deref()?)?;
    let (receipt, unit) = root.read_store_claim_source(delivery_id).await.ok()??;
    if intent.account_id != receipt.account_id || intent.entitlement_id != receipt.fulfilment_unit_id {
        return None;
    }
    publish_at_current_generation(root, client, &receipt, |client, generation| {
        client.claim_receipt(&intent, &receipt.character_id, &receipt.world_id, &unit, generation, &receipt.decided_at)
    }).await.ok()
}

async fn recover_cosmetic(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: &mut StoreClient,
    operation: &str,
) -> Option<()> {
    let source = client.read_intent(OperationKind::Deliver, operation).await.ok()?;
    let offer = source.offer_key?;
    if source.quantity != Some(1) || !matches!(source.unit_kind.as_str(), "ITEM" | "COSMETIC")
        || crate::content::store_cosmetics::offer(&offer).is_none()
    {
        return None;
    }
    root.recover_paid_store_cosmetic(authority, PaidStoreCosmeticIntent {
        operation: uuid_bytes(operation)?,
        account: uuid_bytes(&source.account_id)?,
        order: uuid_bytes(source.order_id.as_deref()?)?,
        fulfilment: source.entitlement_id,
        offer,
        catalogue: source.catalog_revision?,
    }).await.ok()?;
    Some(())
}

//! Republish committed deliveries without an active buyer session.
use super::{COSMETIC_CATALOGUE, PREDECESSOR_CATALOGUES, RetainedAnswer,
    StoreClient, rules, uuid_bytes, uuid_text};
use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
    store::{DeliveryReceipt, PaidStoreCosmeticIntent, PurchaseRequest, StorePurchaseHistoryEntry},
};
use crate::native_admission_source::store::{DeliveryIntent, OperationCursor, OperationKind, ProtocolError, PurchaseResult, PurchaseStoreOffer};
use std::time::Duration;

pub async fn run_store_receipt_outbox(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: StoreClient,
) {
    run_reconciliation(root, Some(authority), None, client).await;
}

/// Reconcile reopened native decisions while gameplay nodes are stopped.
pub async fn run_retained_store_reconciliation(root: &DurabilityRoot, client: StoreClient, generation: u64) {
    run_reconciliation(root, None, Some(generation), client).await;
}

async fn run_reconciliation(
    root: &DurabilityRoot,
    authority: Option<&ReconciledCharacterAuthority<'_, '_>>,
    restore_generation: Option<u64>,
    mut client: StoreClient,
) {
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut cursor = OperationCursor::default();
    loop {
        tick.tick().await;
        if authority.is_some() {
            publish_pending(root, &mut client).await;
        }
        reconcile_platform_operations(root, authority, restore_generation, &mut client, &mut cursor).await;
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
    authority: Option<&ReconciledCharacterAuthority<'_, '_>>,
    restore_generation: Option<u64>,
    client: &mut StoreClient,
    cursor: &mut OperationCursor,
) {
    let Ok(page) = client.list_pending(cursor).await else {
        return;
    };
    for pending in &page.entries {
        match pending.operation_kind {
            OperationKind::Claim => { let _ = publish_claim(root, client, &pending.operation_id, authority.is_some(), restore_generation).await; }
            OperationKind::Deliver => {
                let Ok(intent) = client.read_intent(OperationKind::Deliver, &pending.operation_id).await else {
                    continue;
                };
                let Some(operation) = uuid_bytes(&pending.operation_id) else {
                    continue;
                };
                match root.read_retained_store_delivery(operation).await {
                    Ok(Some(receipt)) if intent.account_id == receipt.account_id
                        && intent.entitlement_id == receipt.fulfilment_unit_id =>
                    {
                        let acknowledged = publish_delivery(root, client, &intent, &receipt).await.is_ok();
                        if acknowledged && authority.is_some() {
                            let _ = root.mark_store_receipt_acknowledged(operation).await;
                        }
                    }
                    Ok(None) => {
                        if let Some(generation) = restore_generation {
                            let _ = publish_restore_conflict(root, client, &intent, generation).await;
                        } else if let Some(authority) = authority
                            && root.has_store_restore_conflict(operation).await.ok() == Some(false)
                        {
                            let _ = recover_cosmetic(root, authority, client, intent).await;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    *cursor = page.entries.last().map_or_else(OperationCursor::default, OperationCursor::from);
}

pub(super) async fn remember_claim(root: &DurabilityRoot, intent: &DeliveryIntent) -> Option<()> {
    if intent.unit_kind != "ITEM" || intent.quantity != Some(1) {
        return None;
    }
    root.remember_store_claim_operation(
        uuid_bytes(intent.delivery_operation_id.as_deref()?)?,
        uuid_bytes(&intent.operation_id)?,
        intent.account_id.clone(),
        intent.entitlement_id.clone(),
    ).await.ok()
}

async fn publish_claim(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    id: &str,
    discover: bool,
    restore_generation: Option<u64>,
) -> Option<()> {
    let intent = client.read_intent(OperationKind::Claim, id).await.ok()?;
    if discover {
        remember_claim(root, &intent).await?;
    }
    let delivery_id = uuid_bytes(intent.delivery_operation_id.as_deref()?)?;
    let Some((receipt, unit)) = root.read_store_claim_source(delivery_id).await.ok()? else {
        return publish_restore_conflict(root, client, &intent, restore_generation?).await;
    };
    if intent.account_id != receipt.account_id || intent.entitlement_id != receipt.fulfilment_unit_id {
        return None;
    }
    publish_at_current_generation(root, client, &receipt, |client, generation| {
        client.claim_receipt(&intent, &receipt.character_id, &receipt.world_id, &unit, generation, &receipt.decided_at)
    }).await.ok()
}

async fn publish_restore_conflict(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    intent: &DeliveryIntent,
    generation: u64,
) -> Option<()> {
    if !matches!(intent.unit_kind.as_str(), "ITEM" | "COSMETIC") {
        return None;
    }
    // Start immediately before this Platform sequence number. Pending lists
    // also contain new operations, which must never be decided by this path.
    let previous = intent.source_revision.parse::<u64>().ok()?.checked_sub(1)?;
    let cursor = if previous == 0 {
        OperationCursor::default()
    } else {
        OperationCursor {
            after_source_revision: previous.to_string(),
            after_operation_id: intent.operation_id.clone(),
        }
    };
    let states = client.reconcile(&cursor).await.ok()?;
    if !states.operations.iter().any(|state| {
        state.operation_id == intent.operation_id
            && state.operation_kind == intent.operation_kind
            && state.source_revision == intent.source_revision
            && state.state == "REOPENED"
            && state.receipt_digest.is_none()
            && state.recovery_generation.is_none()
    }) {
        return None;
    }
    let operation = uuid_bytes(&intent.operation_id)?;
    let decided_at = root.record_store_restore_conflict(operation, generation).await.ok()??;
    let bytes = client.restore_conflict_receipt(intent, generation, decided_at).ok()?;
    client.publish_receipt(&bytes).await.ok()
}

async fn recover_cosmetic(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: &mut StoreClient,
    source: DeliveryIntent,
) -> Option<()> {
    let offer = source.offer_key.as_ref()?;
    if source.quantity != Some(1) || !matches!(source.unit_kind.as_str(), "ITEM" | "COSMETIC")
        || crate::content::store_cosmetics::offer(offer).is_none()
    {
        return None;
    }
    let account = uuid_bytes(&source.account_id)?;
    let order = uuid_bytes(source.order_id.as_deref()?)?;
    let operation = uuid_bytes(&source.operation_id)?;
    let revision = source.catalog_revision.as_ref()?;
    let intent = || PaidStoreCosmeticIntent {
        operation, account, order, fulfilment: source.entitlement_id.clone(),
        offer: offer.clone(), catalogue: revision.clone(),
    };
    if root.recover_paid_store_cosmetic(authority, intent()).await.ok()? {
        return Some(());
    }

    // A lost purchase reply leaves a reservation but no purchase-history row.
    // Replay its original confirmation and require Platform to name this order.
    let pending = root.pending_store_cosmetic_purchase(account, offer.clone()).await.ok()??;
    let catalogue = COSMETIC_CATALOGUE.iter().chain(PREDECESSOR_CATALOGUES.iter().flatten())
        .find(|catalogue| catalogue.rules.revision() == revision && catalogue.rules.offer(offer).is_some())?;
    let quote = rules::build_quote(&catalogue.rules, offer, &rules::CharacterStoreView::default()).ok()?;
    let binding = rules::PurchaseStoreOffer::from_quote(
        rules::PurchaseRequestId::from_bytes(pending.purchase_id).ok()?,
        crate::domain::AccountId::from_bytes(account).ok()?,
        crate::domain::CharacterId::from_bytes(pending.character_id).ok()?,
        quote,
    );
    let request = PurchaseRequest {
        purchase_request_id: pending.purchase_id, account_id: account,
        character_id: pending.character_id, binding: binding.binding_digest().into(),
    };
    if pending.binding_digest != request.binding_digest()
        || root.read_store_purchase_answer(request.clone()).await.ok()?.is_some()
    {
        return None;
    }
    let unit = super::platform_unit_kind(revision, offer)?;
    let answer = client.purchase(&PurchaseStoreOffer {
        purchase_request_id: uuid_text(&pending.purchase_id),
        account_id: source.account_id.clone(),
        character_id: uuid_text(&pending.character_id),
        world_id: uuid_text(&pending.world_id),
        catalog_revision: revision.clone(),
        offer_key: offer.clone(),
        unit_price: binding.price.to_string(),
        quantity: 1,
        currency: super::platform_currency(binding.currency).ok()?.into(),
    }, unit).await.ok()?;
    if !matches!(&answer, PurchaseResult::Accepted { order_id, operation_ids }
        if uuid_bytes(order_id) == Some(order) && operation_ids.as_slice() == [source.operation_id.as_str()])
    {
        return None;
    }
    root.record_store_answer_with_history(
        request, serde_json::to_vec(&RetainedAnswer::of(&answer)).ok()?,
        Some(StorePurchaseHistoryEntry {
            order_id: order, offer_key: offer.clone(), catalog_revision: revision.clone(),
            price: binding.price, currency: super::currency_text(binding.currency).into(),
        }),
    ).await.ok()?;
    root.recover_paid_store_cosmetic(authority, intent()).await.ok()?;
    Some(())
}

//! Republish committed deliveries without an active buyer session.
use super::{
    COSMETIC_CATALOGUE, PREDECESSOR_CATALOGUES, RetainedAnswer, StoreClient, rules, uuid_bytes,
    uuid_text,
};
use crate::durability::{
    DurabilityRoot,
    character_authority::ReconciledCharacterAuthority,
    store::{
        DeliveryReceipt, PaidStoreCosmeticIntent, PendingStorePurchase, PurchaseRequest,
        StorePurchaseHistoryEntry,
    },
};
use crate::native_admission_source::store::{
    DeliveryIntent, OperationCursor, OperationKind, ProtocolError, PurchaseResult,
    PurchaseStoreOffer,
};
use std::time::Duration;

pub async fn run_store_receipt_outbox(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: StoreClient,
) {
    run_store_receipt_outbox_with_mode(root, authority, client, super::StorePurchaseMode::Enabled)
        .await;
}

pub async fn run_store_receipt_outbox_with_mode(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    mut client: StoreClient,
    purchases: super::StorePurchaseMode,
) {
    let mut tick = worker_tick();
    let mut purchase_cursor = [0; 16];
    let mut cursor = OperationCursor::default();
    loop {
        tick.tick().await;
        publish_pending(root, &mut client).await;
        if purchases == super::StorePurchaseMode::Enabled
            && let Ok(purchases) = root
                .pending_store_hunting_purchases(authority, purchase_cursor)
                .await
        {
            purchase_cursor = purchases
                .last()
                .map_or([0; 16], |row| row.request.purchase_request_id);
            for purchase in purchases {
                super::hunting_offers::replay(root, &mut client, purchase).await;
            }
        }
        let Ok(page) = client.list_pending(&cursor).await else {
            continue;
        };
        for entry in &page.entries {
            reconcile_operation(
                root,
                Some(authority),
                None,
                &mut client,
                entry.operation_kind,
                &entry.operation_id,
            )
            .await;
        }
        cursor = page
            .entries
            .last()
            .map_or_else(OperationCursor::default, OperationCursor::from);
    }
}

/// Reconcile reopened native decisions while gameplay nodes are stopped.
pub async fn run_retained_store_reconciliation(
    root: &DurabilityRoot,
    mut client: StoreClient,
    generation: u64,
) {
    let mut tick = worker_tick();
    let mut cursor = OperationCursor::default();
    loop {
        tick.tick().await;
        // A restore notice has a finite retry budget. Drain pages before
        // waiting for another poll, and skip states unrelated to this restore.
        while let Ok(page) = client.reconcile(&cursor).await {
            let Some(last) = page.operations.last() else {
                cursor = OperationCursor::default();
                break;
            };
            for entry in page
                .operations
                .iter()
                .filter(|entry| entry.state == "REOPENED")
            {
                reconcile_operation(
                    root,
                    None,
                    Some(generation),
                    &mut client,
                    entry.operation_kind,
                    &entry.operation_id,
                )
                .await;
            }
            cursor = OperationCursor {
                after_source_revision: last.source_revision.clone(),
                after_operation_id: last.operation_id.clone(),
            };
            tokio::task::yield_now().await;
        }
    }
}

fn worker_tick() -> tokio::time::Interval {
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tick
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
    publish_at_current_generation(
        root,
        client,
        &receipt.recovery_generation,
        |client, generation| client.delivery_receipt(intent, generation, &receipt.decided_at),
    )
    .await
}

async fn publish_at_current_generation(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    original_generation: &str,
    encode: impl Fn(&StoreClient, &str) -> Result<Vec<u8>, ProtocolError>,
) -> Result<(), ProtocolError> {
    let bytes = encode(client, original_generation)?;
    let client_result = client.publish_receipt(&bytes).await;
    if client_result != Err(ProtocolError::Conflict) {
        // An identical prior publication is accepted before the restore fence check.
        return client_result;
    }
    let generation = root
        .store_publication_generation()
        .await
        .ok()
        .flatten()
        .filter(|current| current.parse::<u64>().ok() > original_generation.parse::<u64>().ok())
        .ok_or(ProtocolError::Conflict)?;
    let bytes = encode(client, &generation)?;
    client.publish_receipt(&bytes).await
}

/// Platform allocates CLAIM ids only after accepting DELIVER. The durable delivery
/// receipt remains the proof even when the buyer has gone offline.
async fn reconcile_operation(
    root: &DurabilityRoot,
    authority: Option<&ReconciledCharacterAuthority<'_, '_>>,
    restore_generation: Option<u64>,
    client: &mut StoreClient,
    kind: OperationKind,
    id: &str,
) -> Option<()> {
    match kind {
        OperationKind::Claim => {
            publish_claim(root, client, id, authority.is_some(), restore_generation).await
        }
        OperationKind::Deliver => {
            let intent = client.read_intent(OperationKind::Deliver, id).await.ok()?;
            let operation = uuid_bytes(id)?;
            if intent.unit_kind == "PREMIUM_TIME" {
                return register_premium_delivery(
                    root,
                    client,
                    &intent,
                    operation,
                    restore_generation,
                )
                .await;
            }
            match root.read_retained_store_delivery(operation).await.ok()? {
                Some(receipt)
                    if intent.account_id == receipt.account_id
                        && intent.entitlement_id == receipt.fulfilment_unit_id =>
                {
                    publish_delivery(root, client, &intent, &receipt)
                        .await
                        .ok()?;
                    if authority.is_some() {
                        let _ = root.mark_store_receipt_acknowledged(operation).await;
                    }
                    Some(())
                }
                None if restore_generation.is_some() => {
                    publish_restore_conflict(root, client, &intent, restore_generation?).await
                }
                None if !root.has_store_restore_conflict(operation).await.ok()? => {
                    let authority = authority?;
                    if intent
                        .offer_key
                        .as_deref()
                        .is_some_and(super::hunting_offers::is_offer)
                    {
                        super::hunting_offers::recover(root, authority, client, &intent).await
                    } else {
                        recover_cosmetic(root, authority, client, intent).await
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

async fn register_premium_delivery(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    intent: &DeliveryIntent,
    operation: [u8; 16],
    restore_generation: Option<u64>,
) -> Option<()> {
    if !matches!(intent.quantity, Some(1..=366)) {
        return None;
    }
    let (generation, decided_at) = root
        .record_premium_store_registration(
            operation,
            intent.binding_digest.clone(),
            restore_generation,
        )
        .await
        .ok()??;
    let decided_at = decided_at.to_string();
    publish_at_current_generation(root, client, &generation, |client, generation| {
        client.delivery_receipt(intent, generation, &decided_at)
    })
    .await
    .ok()
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
    )
    .await
    .ok()
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
    if intent.account_id != receipt.account_id
        || intent.entitlement_id != receipt.fulfilment_unit_id
    {
        return None;
    }
    publish_at_current_generation(
        root,
        client,
        &receipt.recovery_generation,
        |client, generation| {
            client.claim_receipt(
                &intent,
                &receipt.character_id,
                &receipt.world_id,
                &unit,
                generation,
                &receipt.decided_at,
            )
        },
    )
    .await
    .ok()
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
    let operation = uuid_bytes(&intent.operation_id)?;
    let decided_at = root
        .record_store_restore_conflict(
            operation,
            generation,
            intent.delivery_operation_id.as_deref().and_then(uuid_bytes),
        )
        .await
        .ok()??;
    let bytes = client
        .restore_conflict_receipt(intent, generation, decided_at)
        .ok()?;
    client.publish_receipt(&bytes).await.ok()
}

async fn recover_cosmetic(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: &mut StoreClient,
    source: DeliveryIntent,
) -> Option<()> {
    let offer = source.offer_key.as_ref()?;
    if source.quantity != Some(1)
        || !matches!(source.unit_kind.as_str(), "ITEM" | "COSMETIC")
        || crate::content::store_cosmetics::offer(offer).is_none()
    {
        return None;
    }
    let account = uuid_bytes(&source.account_id)?;
    let order = uuid_bytes(source.order_id.as_deref()?)?;
    let operation = uuid_bytes(&source.operation_id)?;
    let revision = source.catalog_revision.as_ref()?;
    let intent = || PaidStoreCosmeticIntent {
        operation,
        account,
        order,
        fulfilment: source.entitlement_id.clone(),
        offer: offer.clone(),
        catalogue: revision.clone(),
    };
    if root
        .recover_paid_store_cosmetic(authority, intent())
        .await
        .ok()?
    {
        return Some(());
    }

    // The intent has no Character or request ID. Replay only retained
    // confirmations, and require the original Platform order and operation.
    let mut after = [0; 16];
    loop {
        let candidates = root
            .unanswered_store_cosmetic_purchases(account, offer.clone(), after)
            .await
            .ok()?;
        after = candidates.last()?.purchase_id;
        for pending in candidates {
            if retain_cosmetic_answer(root, client, &source, pending)
                .await
                .is_some()
            {
                return root
                    .recover_paid_store_cosmetic(authority, intent())
                    .await
                    .ok()?
                    .then_some(());
            }
        }
    }
}

async fn retain_cosmetic_answer(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    source: &DeliveryIntent,
    pending: PendingStorePurchase,
) -> Option<()> {
    let account = uuid_bytes(&source.account_id)?;
    let offer = source.offer_key.as_ref()?;
    let revision = source.catalog_revision.as_ref()?;
    let order = uuid_bytes(source.order_id.as_deref()?)?;
    let catalogue = COSMETIC_CATALOGUE
        .iter()
        .chain(PREDECESSOR_CATALOGUES.iter().flatten())
        .find(|catalogue| {
            catalogue.rules.revision() == revision && catalogue.rules.offer(offer).is_some()
        })?;
    let quote = rules::build_quote(
        &catalogue.rules,
        offer,
        &rules::CharacterStoreView::default(),
    )
    .ok()?;
    let binding = rules::PurchaseStoreOffer::from_quote(
        rules::PurchaseRequestId::from_bytes(pending.purchase_id).ok()?,
        crate::domain::AccountId::from_bytes(account).ok()?,
        crate::domain::CharacterId::from_bytes(pending.character_id).ok()?,
        quote,
    );
    let request = PurchaseRequest {
        purchase_request_id: pending.purchase_id,
        account_id: account,
        character_id: pending.character_id,
        binding: binding.binding_digest().into(),
    };
    if pending.binding_digest != request.binding_digest()
        || root
            .read_store_purchase_answer(request.clone())
            .await
            .ok()?
            .is_some()
    {
        return None;
    }
    let unit = super::platform_unit_kind(revision, offer)?;
    let answer = client
        .purchase(
            &PurchaseStoreOffer {
                purchase_request_id: uuid_text(&pending.purchase_id),
                account_id: source.account_id.clone(),
                character_id: uuid_text(&pending.character_id),
                world_id: uuid_text(&pending.world_id),
                catalog_revision: revision.clone(),
                offer_key: offer.clone(),
                unit_price: binding.price.to_string(),
                quantity: 1,
                currency: super::platform_currency(binding.currency).ok()?.into(),
            },
            unit,
        )
        .await
        .ok()?;
    if !matches!(&answer, PurchaseResult::Accepted { order_id, operation_ids }
        if uuid_bytes(order_id) == Some(order) && operation_ids.as_slice() == [source.operation_id.as_str()])
    {
        return None;
    }
    root.record_store_answer_with_history(
        request,
        serde_json::to_vec(&RetainedAnswer::of(&answer)).ok()?,
        Some(StorePurchaseHistoryEntry {
            order_id: order,
            offer_key: offer.clone(),
            catalog_revision: revision.clone(),
            price: binding.price,
            currency: super::currency_text(binding.currency).into(),
        }),
    )
    .await
    .ok()?;
    Some(())
}

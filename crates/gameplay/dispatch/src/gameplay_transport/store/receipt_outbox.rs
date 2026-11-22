//! Republish committed deliveries without an active buyer session.
use super::{StoreClient, uuid_bytes};
use crate::durability::DurabilityRoot;
use crate::native_admission_source::store::OperationKind;
use std::time::Duration;

pub async fn run_store_receipt_outbox(root: &DurabilityRoot, mut client: StoreClient) {
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        publish_pending(root, &mut client).await;
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
            let bytes = client
                .delivery_receipt(&intent, &receipt.recovery_generation, &receipt.decided_at)
                .ok()?;
            client.publish_receipt(&bytes).await.ok()
        }
        .await
        .is_some();
        let _ = root.record_store_receipt_attempt(id, acknowledged).await;
    }
}

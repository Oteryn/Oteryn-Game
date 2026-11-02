//! Resume only a durably reserved Prey purchase across catalogue retunes or disablement.
//! The caller's old quote is never authority by itself: every byte must match its journal.
use super::*;

pub(super) const OFFER_KEY: &str = "store:prey-list-expansion";

pub(super) enum Resume {
    Fresh,
    Completed(wire::StorePurchaseResult),
}

pub(super) async fn resume(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    request: &wire::StorePurchaseRequest,
) -> Option<Resume> {
    if request.offer_key != OFFER_KEY || request.currency != "TIBIA_COIN" || request.price == 0 {
        return None;
    }
    let fence = gameplay_fence(owner, admitted).await?;
    let account = admitted.controller?.account_id;
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id).ok()?,
        account_id: crate::domain::AccountId::from_bytes(account).ok()?,
        character_id: fence.character_id,
        offer_key: request.offer_key.clone(),
        catalog_revision: request.catalog_revision.clone(),
        price: request.price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let journal = PurchaseRequest {
        purchase_request_id: request.purchase_request_id,
        account_id: account,
        character_id: *fence.character_id.as_bytes(),
        binding: binding.binding_digest().to_vec(),
    };
    let Some(retained) = owner.root.read_prey_expansion_purchase(
        owner.character, owner.holder, fence, journal.clone(),
    ).await.ok()? else {
        return Some(Resume::Fresh);
    };
    let saved = match retained {
        PurchaseRequestState::Answered(answer) => answer,
        PurchaseRequestState::Sent => {
            // The first answer may have been lost after Platform debited the purchase.
            // Reconcile its original request id and immutable quote, never create a new id.
            let answer = platform.lock().await.purchase(&PurchaseStoreOffer {
                purchase_request_id: uuid_text(&request.purchase_request_id),
                account_id: uuid_text(&account),
                character_id: uuid_text(fence.character_id.as_bytes()),
                offer_key: binding.offer_key.clone(),
                catalog_revision: binding.catalog_revision.clone(),
                price: binding.price,
                currency: currency_text(binding.currency).into(),
            }).await.ok()?;
            let answer = serde_json::to_vec(&RetainedAnswer::of(&answer)).ok()?;
            owner.root.record_store_answer(journal.clone(), answer).await.ok()?
        }
    };
    let answer: RetainedAnswer = serde_json::from_slice(&saved).ok()?;
    let code = answer.code()?;
    if code != wire::StorePurchaseCode::Accepted {
        owner.root.release_refused_prey_expansion_purchase(owner.character, journal).await.ok()?;
        return Some(Resume::Completed(refusal(code)));
    }
    let (order_id, operation_id) = delivery_ids(&answer)?;
    deliver(owner, platform, admitted, &binding, &answer.operation_ids[0]).await?;
    Some(Resume::Completed(wire::StorePurchaseResult {
        code,
        order_id: Some(order_id),
        operation_id: Some(operation_id),
    }))
}

/// The permanent offer has one entitlement effect, so its accepted order has one operation.
pub(super) fn delivery_ids(answer: &RetainedAnswer) -> Option<([u8; 16], [u8; 16])> {
    let [operation] = answer.operation_ids.as_slice() else { return None; };
    let canonical = |text: &str| {
        let bytes = uuid_bytes(text)?;
        (bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80 && uuid_text(&bytes) == text).then_some(bytes)
    };
    Some((canonical(answer.order_id.as_deref()?)?, canonical(operation)?))
}

async fn deliver(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    binding: &rules::PurchaseStoreOffer,
    operation: &str,
) -> Option<()> {
    let operation_id = uuid_bytes(operation)?;
    let intent = platform.lock().await.read_intent(operation).await.ok()?;
    let account_id = *binding.account_id.as_bytes();
    let world_id = *admitted.world_id.as_bytes();
    if intent.operation_id != operation
        || intent.account_id != uuid_text(&account_id)
        || intent.character_id.as_deref() != Some(uuid_text(binding.character_id.as_bytes()).as_str())
        || intent.world_id.as_deref().is_some_and(|world| world != uuid_text(&world_id))
        || intent.offer_key != OFFER_KEY
        || intent.offer_key != binding.offer_key
        || intent.catalog_revision != binding.catalog_revision
    {
        return None;
    }
    // The retained reservation already fixes this product's permanent effect. Consulting
    // a newer catalogue here would strand an accepted old operation or change its effect.
    let receipt = owner.deliver_prey_expansion(admitted, crate::durability::prey::PreyStoreDelivery {
        operation_id, account_id, world_id, fulfilment_unit_id: intent.fulfilment_unit_id,
    }).await?;
    let bytes = serde_json::to_vec(&receipt).ok()?;
    if platform.lock().await.publish_receipt(&bytes).await.is_ok() {
        let _ = owner.root.mark_store_receipt_acknowledged(operation_id).await;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prey_delivery_validates_one_complete_operation_before_any_effect() {
        let order = "01010101-0101-7101-8101-010101010101";
        let operation = "02020202-0202-7202-8202-020202020202";
        let answer = RetainedAnswer {
            result: "ACCEPTED".into(), order_id: Some(order.into()), operation_ids: vec![operation.into()],
        };
        assert!(delivery_ids(&answer).is_some());
        for field in 0..7 {
            let mut invalid = answer.clone();
            match field {
                0 => invalid.order_id = None,
                1 => invalid.order_id = Some("invalid-order".into()),
                2 => invalid.operation_ids.clear(),
                3 => invalid.operation_ids.push(operation.into()),
                4 => invalid.operation_ids[0] = "invalid-operation".into(),
                5 => invalid.order_id = Some("01010101-0101-4101-8101-010101010101".into()),
                _ => invalid.operation_ids[0] = "02020202-0202-7202-0202-020202020202".into(),
            }
            assert!(delivery_ids(&invalid).is_none());
        }
    }
}

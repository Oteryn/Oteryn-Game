//! Local earned currency never crosses the commercial Platform port.
use super::*;
pub(super) async fn purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
) -> Option<wire::StorePurchaseResult> {
    let request = wire::decode_purchase_request(payload).ok()?;
    if owner.store_purchases != StorePurchaseMode::Enabled {
        let fence = admitted.item_fence?;
        let retained = owner.root.has_store_purchase_request(request.purchase_request_id, admitted.controller?.account_id, *fence.character_id.as_bytes()).await.ok()?;
        if !retained { return Some(refusal(wire::StorePurchaseCode::PurchaseDisabled)); }
    }
    let Some(catalogue) = catalogue_for_key(&request.offer_key) else {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    };
    let Some(offer) = catalogue.rules.offer(&request.offer_key) else {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    };
    if !wire::cosmetic_tokens::eligible(&offer.category) {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    }
    if request.currency != wire::cosmetic_tokens::CURRENCY
        || request.price != offer.price_coins
        || request.catalog_revision != catalogue.rules.revision()
    {
        return Some(refusal(wire::StorePurchaseCode::StaleQuote));
    }
    let Some(cosmetic) = crate::content::store_cosmetics::offer(&request.offer_key) else {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    };
    let native = owner.active_generation?.native_gameplay()?.cosmetics()?;
    if !crate::content::store_cosmetics::compatible(native, cosmetic) {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    }
    let fence = gameplay_fence(owner, admitted).await?;
    let mut binding = wire::encode_purchase_request(&request).ok()?;
    binding.extend(admitted.world_id.as_bytes());
    owner
        .root
        .purchase_store_cosmetic_tokens(
            owner.character,
            owner.holder,
            fence,
            PurchaseRequest {
                purchase_request_id: request.purchase_request_id,
                account_id: admitted.controller?.account_id,
                character_id: *fence.character_id.as_bytes(),
                binding,
            },
            request.offer_key,
            request.catalog_revision,
        )
        .await
        .ok()
}

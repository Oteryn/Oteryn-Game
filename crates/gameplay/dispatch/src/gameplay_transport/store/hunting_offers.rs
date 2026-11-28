//! The Platform charges an immutable price per key. Only the currently purchasable
//! XP tier is projected as the XP Boost, and the durable owner rechecks it at debit.
use super::*;
use crate::durability::store::{
    PaidStoreHuntingIntent, PendingStoreHuntingPurchase, StoreHuntingDelivery, StoreHuntingOffer,
    StoreHuntingPurchase,
};
use crate::durability::{DurabilityRoot, character_authority::ReconciledCharacterAuthority};
use crate::native_admission_source::store::{DeliveryIntent, OperationKind};

pub(super) const HIDDEN_XP_TIERS: usize = 5;

pub(super) fn xp_tier(key: &str) -> Option<i16> {
    match StoreHuntingOffer::from_key(key)? {
        StoreHuntingOffer::XpBoost(tier) => Some(tier),
        StoreHuntingOffer::PermanentPreySlot | StoreHuntingOffer::PreyWildcards => None,
    }
}

pub(super) fn is_offer(key: &str) -> bool {
    StoreHuntingOffer::from_key(key).is_some()
}

pub(super) fn visible(key: &str, tier: Option<i16>) -> bool {
    xp_tier(key).is_none_or(|offer_tier| tier == Some(offer_tier))
}

pub(super) async fn current_tier(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> Option<i16> {
    let fence = gameplay_fence(owner, admitted).await?;
    owner
        .root
        .next_store_xp_boost_tier(
            owner.character,
            owner.holder,
            fence,
            *admitted.channel_id.as_bytes(),
        )
        .await
        .ok()?
}

/// Only definite refusals become replies. Once the journal may contain a paid
/// answer, any unavailable step leaves this command outstanding.
pub(super) async fn purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: wire::StorePurchaseRequest,
) -> Option<wire::StorePurchaseResult> {
    let account = admitted.controller?.account_id;
    let character = admitted.item_fence?.character_id;
    let world = *admitted.world_id.as_bytes();
    let enabled = owner.store_purchases == StorePurchaseMode::Enabled;
    let platform = owner.store.as_ref();
    let active = CATALOGUE.as_ref()?;
    let is_new_quote = request.catalog_revision == active.rules.revision()
        && rules::build_quote(
            &active.rules,
            &request.offer_key,
            &rules::CharacterStoreView::default(),
        )
        .is_ok_and(|quote| {
            quote.price == request.price && request.currency == currency_text(quote.currency)
        });
    if request.currency != currency_text(rules::CurrencyKind::TibiaCoin) {
        return Some(refusal(wire::StorePurchaseCode::StaleQuote));
    }
    // The saved request validates the immutable quote on replay, including revisions
    // that have since left the browse catalogue. Only a new debit needs an active quote.
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id)
            .ok()?,
        account_id: crate::domain::AccountId::from_bytes(account).ok()?,
        character_id: character,
        offer_key: request.offer_key.clone(),
        catalog_revision: request.catalog_revision.clone(),
        price: request.price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let mut pending = PendingStoreHuntingPurchase {
        request: PurchaseRequest {
            purchase_request_id: request.purchase_request_id,
            account_id: account,
            character_id: *character.as_bytes(),
            binding: binding.binding_digest().to_vec(),
        },
        world_id: world,
        offer_key: request.offer_key,
        catalog_revision: request.catalog_revision,
        price: i64::try_from(request.price).ok()?,
    };
    let retained = owner
        .root
        .has_store_purchase_request(request.purchase_request_id, account, *character.as_bytes())
        .await
        .ok()?;
    let retry = owner
        .root
        .read_store_purchase_retry(pending.request.clone())
        .await
        .ok()?;
    if retry.is_some() || !retained {
        let is_new_retry = retry.is_none();
        let original = if let Some(retry) = retry {
            if retry.world_id != world {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            }
            Some(PurchaseRequest {
                purchase_request_id: retry.purchase_id,
                binding: retry.binding_digest,
                ..pending.request.clone()
            })
        } else {
            owner
                .root
                .pending_store_hunting_confirmation(PendingStoreHuntingPurchase {
                    request: pending.request.clone(),
                    world_id: pending.world_id,
                    offer_key: pending.offer_key.clone(),
                    catalog_revision: pending.catalog_revision.clone(),
                    price: pending.price,
                })
                .await
                .ok()?
        };
        if let Some(mut original) = original {
            let mut original_binding = binding.clone();
            original_binding.purchase_request_id =
                rules::PurchaseRequestId::from_bytes(original.purchase_request_id).ok()?;
            let digest = original.binding;
            original.binding = original_binding.binding_digest().to_vec();
            if digest != original.binding_digest() {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            }
            if is_new_retry {
                owner
                    .root
                    .record_store_purchase_retry(
                        pending.request,
                        original.purchase_request_id,
                        world,
                    )
                    .await
                    .ok()?;
            }
            pending.request = original;
        } else if !enabled || platform.is_none() {
            return Some(refusal(wire::StorePurchaseCode::PurchaseDisabled));
        } else if !is_new_quote {
            return Some(refusal(wire::StorePurchaseCode::StaleQuote));
        }
    }

    let state = owner
        .root
        .record_purchase_request(pending.request.clone())
        .await
        .ok()?;
    let saved = match state {
        PurchaseRequestState::Answered(answer) => answer,
        PurchaseRequestState::Sent if !enabled || platform.is_none() => {
            // Restore reconciliation owns unproven debits in this mode.
            return None;
        }
        PurchaseRequestState::Sent => {
            let fence = gameplay_fence(owner, admitted).await?;
            let reserved = owner
                .root
                .reserve_store_hunting_purchase(
                    owner.character,
                    owner.holder,
                    fence,
                    StoreHuntingPurchase {
                        request: pending.request.clone(),
                        offer_key: pending.offer_key.clone(),
                        allow_new: is_new_quote,
                        catalog_revision: pending.catalog_revision.clone(),
                        price: pending.price,
                    },
                    &crate::gameplay_transport::prey::Premium(owner.premium_coordinator),
                    *admitted.channel_id.as_bytes(),
                )
                .await
                .ok()??;
            if !reserved {
                if let Some(answer) = owner
                    .root
                    .read_store_purchase_answer(pending.request.clone())
                    .await
                    .ok()?
                {
                    answer
                } else {
                    let answer = RetainedAnswer::of(&PurchaseResult::OfferUnavailable);
                    owner
                        .root
                        .record_store_answer_with_history(
                            pending.request.clone(),
                            serde_json::to_vec(&answer).ok()?,
                            None,
                        )
                        .await
                        .ok()?
                }
            } else if owner
                .root
                .mark_store_hunting_purchase_sent(
                    owner.character,
                    pending.request.purchase_request_id,
                )
                .await
                .ok()?
            {
                let mut client = platform?.lock().await;
                send(owner.root, &mut client, &pending).await?
            } else {
                owner
                    .root
                    .read_store_purchase_answer(pending.request.clone())
                    .await
                    .ok()??
            }
        }
    };
    let answer: RetainedAnswer = serde_json::from_slice(&saved).ok()?;
    let code = answer.code()?;
    if code != wire::StorePurchaseCode::Accepted {
        return Some(refusal(code));
    }
    let (order, operation) = prey_retry::delivery_ids(&answer)?;
    if owner
        .root
        .has_store_restore_conflict(operation)
        .await
        .ok()?
    {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    }
    let platform = platform?;
    let source = platform
        .lock()
        .await
        .read_intent(OperationKind::Deliver, &uuid_text(&operation))
        .await
        .ok()?;
    let paid = paid_intent(&source)?;
    if paid.order != order
        || paid.operation != operation
        || paid.account != account
        || paid.offer != pending.offer_key
        || paid.catalogue != pending.catalog_revision
    {
        return None;
    }
    let receipt = {
        let _slot = owner.revision_sequencer.acquire(character).await;
        let fence = gameplay_fence(owner, admitted).await?;
        owner
            .root
            .grant_store_hunting_offer(
                owner.character,
                owner.holder,
                fence,
                StoreHuntingDelivery {
                    identity: DeliveryIdentity {
                        account_id: account,
                        character_id: *character.as_bytes(),
                        world_id: world,
                        fulfilment_unit_id: paid.fulfilment,
                    },
                    operation_id: operation,
                    purchase_request_id: pending.request.purchase_request_id,
                    offer_key: pending.offer_key,
                },
                *admitted.channel_id.as_bytes(),
            )
            .await
            .ok()?
    };
    publish(owner, platform, operation, &source, &receipt).await?;
    Some(wire::StorePurchaseResult {
        code,
        order_id: Some(order),
        operation_id: Some(operation),
    })
}

fn original_request(pending: &PendingStoreHuntingPurchase) -> Option<PurchaseRequest> {
    let request = &pending.request;
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id)
            .ok()?,
        account_id: crate::domain::AccountId::from_bytes(request.account_id).ok()?,
        character_id: crate::domain::CharacterId::from_bytes(request.character_id).ok()?,
        offer_key: pending.offer_key.clone(),
        catalog_revision: pending.catalog_revision.clone(),
        price: u64::try_from(pending.price).ok()?,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let mut request = request.clone();
    request.binding = binding.binding_digest().to_vec();
    Some(request)
}

pub(super) async fn replay(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    mut pending: PendingStoreHuntingPurchase,
) -> Option<Vec<u8>> {
    let reconstructed = original_request(&pending)?;
    if pending.request.binding != reconstructed.binding_digest() {
        return None;
    }
    pending.request = reconstructed;
    send(root, client, &pending).await
}

pub(super) async fn send(
    root: &DurabilityRoot,
    client: &mut StoreClient,
    pending: &PendingStoreHuntingPurchase,
) -> Option<Vec<u8>> {
    let request = &pending.request;
    let answer = purchase_answer(
        client
            .purchase(
                &PurchaseStoreOffer {
                    purchase_request_id: uuid_text(&request.purchase_request_id),
                    account_id: uuid_text(&request.account_id),
                    character_id: uuid_text(&request.character_id),
                    world_id: uuid_text(&pending.world_id),
                    catalog_revision: pending.catalog_revision.clone(),
                    offer_key: pending.offer_key.clone(),
                    currency: "COINS".into(),
                    unit_price: pending.price.to_string(),
                    quantity: 1,
                },
                "COSMETIC",
            )
            .await,
    )?;
    let retained = RetainedAnswer::of(&answer);
    let history = if matches!(answer, PurchaseResult::Accepted { .. }) {
        let (order, _) = prey_retry::delivery_ids(&retained)?;
        Some(StorePurchaseHistoryEntry {
            order_id: order,
            offer_key: pending.offer_key.clone(),
            catalog_revision: pending.catalog_revision.clone(),
            price: u64::try_from(pending.price).ok()?,
            currency: "TIBIA_COIN".into(),
        })
    } else {
        None
    };
    root.record_store_answer_with_history(
        request.clone(),
        serde_json::to_vec(&retained).ok()?,
        history,
    )
    .await
    .ok()
}

pub(super) fn paid_intent(source: &DeliveryIntent) -> Option<PaidStoreHuntingIntent> {
    let offer = source.offer_key.as_ref()?;
    if source.operation_kind != OperationKind::Deliver
        || source.quantity != Some(1)
        || source.unit_kind != "COSMETIC"
        || source.grant_key.as_ref() != Some(offer)
        || source.grant_revision.as_deref() != Some("1")
        || !is_offer(offer)
    {
        return None;
    }
    Some(PaidStoreHuntingIntent {
        operation: uuid_bytes(&source.operation_id)?,
        account: uuid_bytes(&source.account_id)?,
        order: uuid_bytes(source.order_id.as_deref()?)?,
        fulfilment: source.entitlement_id.clone(),
        offer: offer.clone(),
        catalogue: source.catalog_revision.clone()?,
    })
}

pub(super) async fn recover(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    client: &mut StoreClient,
    source: &DeliveryIntent,
) -> Option<()> {
    let intent = paid_intent(source)?;
    let operation = intent.operation;
    let receipt = root
        .recover_store_hunting_offer(authority, intent)
        .await
        .ok()??;
    super::receipt_outbox::publish_delivery(root, client, source, &receipt)
        .await
        .ok()?;
    root.mark_store_receipt_acknowledged(operation).await.ok()?;
    Some(())
}

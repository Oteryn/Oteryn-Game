use super::catalogue::StoreService;
use super::*;
use crate::durability::store::{StoreDurabilityError, StorePreyDelivery};
use crate::native_admission_source::store::OperationKind;

pub(super) async fn reserve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: PurchaseRequest,
    service: StoreService,
) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
    match service {
        StoreService::PreyWildcards { count } => {
            owner
                .root
                .reserve_store_prey_pack(owner.character, owner.holder, fence, request, count)
                .await
        }
    }
}

pub(super) async fn release(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    request: PurchaseRequest,
    service: StoreService,
) -> Result<(), StoreDurabilityError> {
    match service {
        StoreService::PreyWildcards { .. } => {
            owner
                .root
                .release_refused_store_prey_pack(owner.character, request)
                .await
        }
    }
}

pub(super) async fn deliver(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    binding: &rules::PurchaseStoreOffer,
    request: PurchaseRequest,
    service: StoreService,
    (order_id, operation_id): ([u8; 16], [u8; 16]),
) -> Option<()> {
    let intent = platform
        .lock()
        .await
        .read_intent(OperationKind::Deliver, &uuid_text(&operation_id))
        .await
        .ok()?;
    if intent.quantity != Some(1)
        || intent.unit_kind != "ITEM"
        || intent.order_id.as_deref() != Some(uuid_text(&order_id).as_str())
        || intent.account_id != uuid_text(&request.account_id)
        || intent.offer_key.as_deref() != Some(binding.offer_key.as_str())
        || intent.catalog_revision.as_deref() != Some(binding.catalog_revision.as_str())
    {
        return None;
    }
    let fence = gameplay_fence(owner, admitted).await?;
    let identity = DeliveryIdentity {
        account_id: request.account_id,
        character_id: request.character_id,
        world_id: *admitted.world_id.as_bytes(),
        fulfilment_unit_id: intent.entitlement_id.clone(),
    };
    let receipt = match service {
        StoreService::PreyWildcards { count } => owner
            .root
            .apply_store_prey_pack(
                owner.character,
                owner.holder,
                fence,
                StorePreyDelivery {
                    identity,
                    request,
                    operation_id,
                    pack: count,
                },
            )
            .await
            .ok()?,
    };
    publish(owner, platform, operation_id, &intent, &receipt).await
}

use super::catalogue::StoreService;
use super::*;
use crate::durability::store::{StoreDurabilityError, StoreServiceDelivery};
use crate::native_admission_source::store::OperationKind;

pub(super) async fn visible_xp_tier(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
) -> Option<i16> {
    if !catalogue.has_xp_boost() {
        return None;
    }
    let fence = gameplay_fence(owner, admitted).await?;
    owner
        .root
        .read_store_xp_tier(owner.character, owner.holder, fence)
        .await
        .ok()?
}

pub(super) async fn reserve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: PurchaseRequest,
    service: StoreService,
) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
    if matches!(service, StoreService::WeeklyTaskExpansion)
        && owner.task_board_catalogue().is_none()
    {
        return Ok(None);
    }
    owner
        .root
        .reserve_store_service(owner.character, owner.holder, fence, request, service)
        .await
}

pub(super) async fn release(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    request: PurchaseRequest,
) -> Result<(), StoreDurabilityError> {
    owner
        .root
        .release_refused_store_service(owner.character, request)
        .await
}

pub(super) async fn resume_old_quote(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: &wire::StorePurchaseRequest,
    command_id: u64,
) -> Option<wire::StorePurchaseResult> {
    if request.currency != "TIBIA_COIN" || request.price == 0 {
        return None;
    }
    let fence = gameplay_fence(owner, admitted).await?;
    let account = admitted.controller?.account_id;
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id)
            .ok()?,
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
    let service = owner
        .root
        .read_retained_store_service(owner.character, owner.holder, fence, journal.clone())
        .await
        .ok()??;
    let platform = owner.store.as_ref()?;
    let bytes = match reserve(owner, fence, journal.clone(), service)
        .await
        .ok()??
    {
        PurchaseRequestState::Answered(bytes) => bytes,
        PurchaseRequestState::Sent => {
            let answer = platform_purchase(platform, admitted, &binding).await?;
            retain_answer(owner, &binding, journal.clone(), answer).await?
        }
    };
    let answer: RetainedAnswer = serde_json::from_slice(&bytes).ok()?;
    let code = answer.code()?;
    if code != wire::StorePurchaseCode::Accepted {
        release(owner, journal).await.ok()?;
        return Some(refusal(code));
    }
    let ids = prey_retry::delivery_ids(&answer)?;
    deliver(
        owner,
        platform,
        admitted,
        &binding,
        journal,
        service,
        (ids, command_id),
    )
    .await?;
    Some(wire::StorePurchaseResult {
        code,
        order_id: Some(ids.0),
        operation_id: Some(ids.1),
    })
}

pub(super) async fn deliver(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    binding: &rules::PurchaseStoreOffer,
    request: PurchaseRequest,
    service: StoreService,
    ((order_id, operation_id), command_id): (([u8; 16], [u8; 16]), u64),
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
    let identity = DeliveryIdentity {
        account_id: request.account_id,
        character_id: request.character_id,
        world_id: *admitted.world_id.as_bytes(),
        fulfilment_unit_id: intent.entitlement_id.clone(),
    };
    let delivery = StoreServiceDelivery {
        identity,
        request,
        operation_id,
        service,
    };
    if matches!(service, StoreService::WeeklyTaskExpansion) {
        let catalogue = Arc::new(owner.task_board_catalogue()?);
        let character = admitted.item_fence?.character_id;
        let _guard = owner.revision_sequencer.acquire(character).await;
        let fence = gameplay_fence(owner, admitted).await?;
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
        let result = owner
            .root
            .grant_store_task_board_expansion(
                owner.character,
                owner.holder,
                fence,
                command,
                delivery.clone(),
                catalogue,
            )
            .await
            .ok()?;
        if result.refusal.is_some() {
            return None;
        }
    }
    let fence = gameplay_fence(owner, admitted).await?;
    let receipt = owner
        .root
        .apply_store_service(owner.character, owner.holder, fence, delivery)
        .await
        .ok()?;
    publish(owner, platform, operation_id, &intent, &receipt).await
}

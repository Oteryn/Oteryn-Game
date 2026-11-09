//! STORE_V1 (capability 154): the admitted catalogue page with the Platform balance, and a
//! purchase the Platform debits and the native Inbox mint delivers. Platform owns the balance,
//! the debit and the order ledger; Game retains only the request binding and its answer
//! (`store_purchase_requests`) and the delivery receipt of each operation.
#[path = "store/coin_history.rs"]
mod coin_history;
use super::connection::AdmittedSession;
use super::item_ref_admission::{generation_item_facts,generation_inbox_item_facts};
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::durability::character_inbox::{RewardClaimInboxMintOutput, RewardClaimInboxMintRequest};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::store::{DeliveryIdentity,DeliveryPlan,HirelingServiceDeliveryPlan,HirelingStoreAction,HirelingStorePurchase,PurchaseRequest,PurchaseRequestState,StorePurchaseHistoryEntry};
use crate::foundation::{CommandId, CommandRef};
use crate::native_admission_source::store::{ProtocolError, PurchaseResult, PurchaseStoreOffer, StoreClient};
use oteryn_gameplay_economy::store as rules;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::store as wire;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, LazyLock};

#[path = "store/catalogue.rs"]
mod catalogue;
#[path = "store/prey_retry.rs"]
mod prey_retry;
#[path = "store/hireling.rs"]
mod hireling;
use catalogue::{RuntimeCatalogue, currency_text};

/// The node's Platform Store client; `None` refuses every purchase as disabled.
pub type StorePlatform = Arc<tokio::sync::Mutex<StoreClient>>;

/// Operator-owned restore switch. Read/reconciliation/delivery availability is independent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorePurchaseMode {
    #[default]
    Enabled,
    DisabledRestoredBackup,
}

impl StorePurchaseMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::DisabledRestoredBackup => "disabled_restored_backup",
        }
    }
}

/// Add STORE_V1 only when its client is composed. Cache the bounded static base sets,
/// as the other conditional capability offers do.
pub fn with_store_offer(
    base: &'static [super::capabilities::OfferedCapability],
    configured: bool,
) -> &'static [super::capabilities::OfferedCapability] {
    if !configured { return base; }
    type Set = &'static [super::capabilities::OfferedCapability];
    static CACHE: std::sync::Mutex<Vec<(Set, Set)>> = std::sync::Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, set)) = cache.iter().find(|(key, _)| *key == base) { return set; }
    let mut set = base.to_vec();
    if !set.iter().any(|row| row.id == wire::CAPABILITY_STORE_V1) {
        set.push(super::capabilities::OfferedCapability { id: wire::CAPABILITY_STORE_V1, requires: &[] });
    }
    if !set.iter().any(|row| row.id == wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1) {
        set.push(super::capabilities::OfferedCapability {
            id: wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
            requires: &[wire::CAPABILITY_STORE_V1],
        });
    }
    set.sort_by_key(|row| row.id);
    let set: Set = Box::leak(set.into_boxed_slice());
    cache.push((base, set));
    set
}

async fn purchase_with_mode(
    mode: StorePurchaseMode,
    retained: impl std::future::Future<Output = Option<bool>>,
    purchase: impl std::future::Future<Output = Option<wire::StorePurchaseResult>>,
) -> Option<wire::StorePurchaseResult> {
    // Sent may already have debited Platform: reconcile the original request ID even
    // after restore. Already-debited delivery/polling is deliberately outside this gate.
    if mode == StorePurchaseMode::DisabledRestoredBackup && !retained.await? {
        return Some(refusal(wire::StorePurchaseCode::PurchaseDisabled));
    }
    purchase.await
}


static CATALOGUE: LazyLock<Option<RuntimeCatalogue>> = LazyLock::new(|| {
    RuntimeCatalogue::from_json(include_bytes!(
        "../../../../../rulesets/economy/store/catalogue.json"
    ))
 });

static HIRELING_CATALOGUE: LazyLock<Option<RuntimeCatalogue>> = LazyLock::new(|| {
    RuntimeCatalogue::from_json(include_bytes!("../../../../../rulesets/economy/store/hirelings.json"))
});

fn catalogue_for_key(key: &str)->Option<&'static RuntimeCatalogue> {
    if key.starts_with("store:hireling:") {HIRELING_CATALOGUE.as_ref()} else {CATALOGUE.as_ref()}
}

macro_rules! authority_methods {
    () => {
        fn store_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
            _command_type: u32,
            _payload: Vec<u8>,
        ) -> impl std::future::Future<Output = (oteryn_protocol_oteryn::CommandStatus, Vec<u8>)>
        {
            async { (oteryn_protocol_oteryn::CommandStatus::Rejected, Vec::new()) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn store_command(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command_id: u64,
            command_type: u32,
            payload: Vec<u8>,
        ) -> (oteryn_protocol_oteryn::CommandStatus, Vec<u8>) {
            store::execute(self, admitted, command_id, command_type, &payload).await
        }
    };
}
pub(super) use composed_methods;

const REJECTED: (CommandStatus, Vec<u8>) = (CommandStatus::Rejected, Vec::new());

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> (CommandStatus, Vec<u8>) {
    let encoded = match command_type {
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE => {
            let Ok(request) = wire::decode_catalogue_request(payload) else {
                return REJECTED;
            };
            let catalogue=if matches!(request.category.as_str(),"houses.hirelings"|"houses.hireling-dresses") {
                if !admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::hireling::management::CAPABILITY_HIRELING_MANAGEMENT_V1) {return REJECTED}
                HIRELING_CATALOGUE.as_ref()
            } else {CATALOGUE.as_ref()};
            let Some(catalogue)=catalogue else {return REJECTED};
            let Some(page) = catalogue_page(owner, admitted, catalogue, &request).await else {
                return REJECTED;
            };
            wire::encode_catalogue_page(&page)
        }
        wire::COMMAND_TYPE_STORE_PURCHASE => {
            let Ok((request,customization)) = wire::decode_purchase_request_with_hireling(payload) else {
                return REJECTED;
            };
            let Some(catalogue)=catalogue_for_key(&request.offer_key) else {return REJECTED};
            let Some(result) = purchase(owner, admitted, catalogue, command_id, request,customization).await
            else {
                return REJECTED;
            };
            wire::encode_purchase_result(&result)
        }
        wire::COMMAND_TYPE_STORE_HISTORY_PAGE => {
            let Ok(request) = wire::decode_coin_history_request(payload) else { return REJECTED; };
            let Some(account) = admitted.controller.map(|controller| controller.account_id) else { return REJECTED; };
            let Some(platform) = owner.store.as_ref() else { return REJECTED; };
            let mut client = platform.lock().await;
            let Some(page) = coin_history::page(&mut client, &uuid_text(&account), &request).await else { return REJECTED; };
            wire::encode_coin_history_page(&page)
        }
        wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE => {
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1)
            {
                return REJECTED;
            }
            let Ok(request) = wire::decode_history_request(payload) else {
                return REJECTED;
            };
            let after = if request.page_cursor.is_empty() {
                None
            } else {
                let Some(cursor) = parse_history_cursor(&request.page_cursor) else {
                    return REJECTED;
                };
                Some(cursor)
            };
            let Some(account) = admitted.controller.map(|controller| controller.account_id) else {
                return REJECTED;
            };
            let Ok(mut rows) = owner.root.store_purchase_history_page(account, after, 33).await
            else {
                return REJECTED;
            };
            let next_page_cursor = if rows.len() == 33 {
                format_history_cursor(rows[31].purchased_at_ms, &rows[31].purchase_request_id)
            } else {
                String::new()
            };
            rows.truncate(32);
            wire::encode_history_page(&wire::StoreHistoryPage {
                entries: rows
                    .into_iter()
                    .map(|row| wire::StoreHistoryEntry {
                        order_id: row.entry.order_id,
                        offer_key: row.entry.offer_key,
                        catalog_revision: row.entry.catalog_revision,
                        price: row.entry.price,
                        currency: row.entry.currency,
                        purchased_at_unix_ms: row.purchased_at_ms,
                    })
                    .collect(),
                next_page_cursor,
            })
        }
        _ => return REJECTED,
    };
    encoded.map_or(REJECTED, |bytes| (CommandStatus::Accepted, bytes))
}

/// One page of a category, keyed after the cursor's offer key, with the Platform balances.
async fn catalogue_page(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    request: &wire::StoreCatalogueRequest,
) -> Option<wire::StoreCataloguePage> {
    let account = uuid_text(&admitted.controller?.account_id);
    let after = (!request.page_cursor.is_empty()).then_some(request.page_cursor.as_str());
    let mut rows = catalogue
        .rules
        .offers_in_category(&request.category, after, wire::MAX_STORE_PAGE_ENTRIES + 1)
        .ok()?;
    let next_page_cursor = if rows.len() > wire::MAX_STORE_PAGE_ENTRIES {
        rows.truncate(wire::MAX_STORE_PAGE_ENTRIES);
        rows.last().map(|offer| offer.key.clone()).unwrap_or_default()
    } else {
        String::new()
    };
    let platform = owner.store.as_ref()?;
    let total_coins = {
        let mut client = platform.lock().await;
        let page = client.balance(&account).await.ok()?;
        let [balance] = page.balances.as_slice() else {
            return None;
        };
        if balance.currency != "COINS" {
            return None;
        }
        balance.available.parse::<u64>().ok()?
    };
    Some(wire::StoreCataloguePage {
        catalog_revision: catalogue.rules.revision().to_owned(),
        offers: rows
            .into_iter()
            .map(|offer| wire::StoreOffer {
                offer_key: offer.key.clone(),
                title: catalogue.title(&offer.key).to_owned(),
                price: offer.price_coins,
                currency: currency_text(offer.currency).to_owned(),
            })
            .collect(),
        next_page_cursor,
        total_coins,
        transferable_coins: total_coins,
    })
}

/// The Platform answer retained for a purchase request id, replayed on every retry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedAnswer {
    result: String,
    order_id: Option<String>,
    operation_ids: Vec<String>,
}

impl RetainedAnswer {
    fn of(result: &PurchaseResult) -> Self {
        let (code, order_id, operation_ids) = match result {
            PurchaseResult::Accepted {
                order_id,
                operation_ids,
            } => ("ACCEPTED", Some(order_id.clone()), operation_ids.clone()),
            PurchaseResult::StaleQuote => ("STALE_QUOTE", None, Vec::new()),
            PurchaseResult::OfferUnavailable => ("OFFER_UNAVAILABLE", None, Vec::new()),
            PurchaseResult::PurchaseDisabled => ("PURCHASE_DISABLED", None, Vec::new()),
            PurchaseResult::InsufficientBalance => ("INSUFFICIENT_BALANCE", None, Vec::new()),
            PurchaseResult::AccountRestricted => ("ACCOUNT_RESTRICTED", None, Vec::new()),
        };
        Self {
            result: code.into(),
            order_id,
            operation_ids,
        }
    }

    fn code(&self) -> Option<wire::StorePurchaseCode> {
        Some(match self.result.as_str() {
            "ACCEPTED" => wire::StorePurchaseCode::Accepted,
            "STALE_QUOTE" => wire::StorePurchaseCode::StaleQuote,
            "OFFER_UNAVAILABLE" => wire::StorePurchaseCode::OfferUnavailable,
            "PURCHASE_DISABLED" => wire::StorePurchaseCode::PurchaseDisabled,
            "INSUFFICIENT_BALANCE" => wire::StorePurchaseCode::InsufficientBalance,
            "ACCOUNT_RESTRICTED" => wire::StorePurchaseCode::AccountRestricted,
            _ => return None,
        })
    }
}

fn refusal(code: wire::StorePurchaseCode) -> wire::StorePurchaseResult {
    wire::StorePurchaseResult {
        code,
        order_id: None,
        operation_id: None,
    }
}

/// InvalidInput is produced only by local purchase validation/encoding, before transport.
/// Retain a definite refusal so the normal answer path releases native reservations.
/// Every post-send error remains uncertain and must reconcile the original request id.
fn purchase_answer(answer: Result<PurchaseResult, ProtocolError>) -> Option<PurchaseResult> {
    match answer {
        Ok(answer) => Some(answer),
        Err(ProtocolError::InvalidInput) => Some(PurchaseResult::PurchaseDisabled),
        Err(_) => None,
    }
}

/// A server quote mismatch never reaches Platform. Otherwise the first binding of the request id
/// is retained before the debit, and its answer is saved once: a retry with the same id returns
/// the saved answer and delivers again, idempotently per operation, never a second debit.
async fn purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    command_id: u64,
    request: wire::StorePurchaseRequest,
    customization: Option<wire::StoreHirelingCustomization>,
) -> Option<wire::StorePurchaseResult> {
    let account = admitted.controller?.account_id;
    let fence = admitted.item_fence?;
    purchase_with_mode(
        owner.store_purchases,
        async { owner.root.has_store_purchase_request(
            request.purchase_request_id, account, *fence.character_id.as_bytes(),
        ).await.ok() },
        purchase_retained_or_new(owner, admitted, catalogue, command_id, &request, customization),
    ).await
}

async fn purchase_retained_or_new(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    command_id: u64,
    request: &wire::StorePurchaseRequest,
    customization: Option<wire::StoreHirelingCustomization>,
) -> Option<wire::StorePurchaseResult> {
    let account = admitted.controller?.account_id;
    let fence = admitted.item_fence?;
    let Some(platform) = owner.store.clone() else {
        return Some(refusal(wire::StorePurchaseCode::PurchaseDisabled));
    };
    // A retained Prey purchase may already have debited Platform under an older quote.
    // Resolve only that exact binding before applying today's catalogue to fresh requests.
    if request.offer_key == prey_retry::OFFER_KEY
        && let prey_retry::Resume::Completed(result) =
            prey_retry::resume(owner, &platform, admitted, request).await?
    {
        return Some(result);
    }
    let hireling_action=if let Some(offer)=catalogue.hireling(&request.offer_key) {
        if !admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::hireling::management::CAPABILITY_HIRELING_MANAGEMENT_V1) {return None}
        let Some(action)=hireling::action(offer,customization.as_ref()) else {return Some(refusal(wire::StorePurchaseCode::OfferUnavailable))};
        Some(action)
    } else {
        if customization.is_some() {return None}
        None
    };
    let Ok(quote) = rules::build_quote(
        &catalogue.rules,
        &request.offer_key,
        &rules::CharacterStoreView::default(),
    ) else {
        return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
    };
    if request.catalog_revision != quote.catalog_revision
        || request.price != quote.price
        || request.currency != currency_text(quote.currency)
    {
        return Some(refusal(wire::StorePurchaseCode::StaleQuote));
    }
    let platform_currency = match platform_currency(quote.currency) {
        Ok(currency) => currency,
        Err(code) => return Some(refusal(code)),
    };
    let id = rules::PurchaseRequestId::from_bytes(request.purchase_request_id).ok()?;
    let character_id = *fence.character_id.as_bytes();
    let binding = rules::PurchaseStoreOffer::from_quote(
        id,
        crate::domain::AccountId::from_bytes(account).ok()?,
        fence.character_id,
        quote,
    );
    let journal = PurchaseRequest {
        purchase_request_id: request.purchase_request_id,
        account_id: account,
        character_id,
        binding: hireling::request_binding(binding.binding_digest(),hireling_action.as_ref())?,
    };
    let prey_expansion = matches!(
        &catalogue.rules.offer(&binding.offer_key)?.delivery,
        rules::DeliveryKind::Unlock { unlock_key } if unlock_key == catalogue::PREY_EXPANSION_UNLOCK
    );
    let charm_expansion = matches!(
        &catalogue.rules.offer(&binding.offer_key)?.delivery,
        rules::DeliveryKind::Unlock { unlock_key } if unlock_key == catalogue::CHARM_EXPANSION_UNLOCK
    );
    let hireling_purchase=hireling_action.as_ref().map(|action|HirelingStorePurchase {purchase_request_id:request.purchase_request_id,origin_channel:*admitted.channel_id.as_bytes(),action:action.clone()});
    let request_state=if let Some(action)=hireling_action {
        let current=gameplay_fence(owner,admitted).await?;
        let weights=if matches!(&action,HirelingStoreAction::Hire {..}) {
            owner.active_generation.and_then(|g|g.native_gameplay()).map(|native|super::item_ref_admission::NativeInboxItemWeights(native,owner.premium_coordinator))
        } else {None};
        let Some(state)=owner.root.reserve_hireling_store_purchase(owner.character,owner.holder,journal.clone(),action,current,*admitted.channel_id.as_bytes(),weights.as_ref().map(|w|w as &dyn crate::durability::item_transfer::ItemWeightLookup)).await.ok()? else {
            return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
        };
        state
    } else {owner.root.record_purchase_request(journal.clone()).await.ok()?};
    let saved = match request_state {
        PurchaseRequestState::Answered(saved) => saved,
        PurchaseRequestState::Sent => {
            let reserved = if prey_expansion {
                let fence = gameplay_fence(owner, admitted).await?;
                owner.root.reserve_prey_expansion_purchase(
                    owner.character, owner.holder, fence, journal.clone()
                ).await.ok()?
            } else if charm_expansion {
                let fence = gameplay_fence(owner, admitted).await?;
                owner.root.reserve_charm_expansion_purchase(
                    owner.character, owner.holder, fence, journal.clone()
                ).await.ok()?
            } else { true };
            let answer = if reserved {
                // Every admitted runtime offer uses Platform ITEM grants, including
                // native permanent unlocks and Hireling service effects.
                purchase_answer(platform.lock().await.purchase(&PurchaseStoreOffer {
                    purchase_request_id: uuid_text(&request.purchase_request_id),
                    account_id: uuid_text(&account),
                    character_id: uuid_text(&character_id),
                    offer_key: binding.offer_key.clone(),
                    catalog_revision: binding.catalog_revision.clone(),
                    unit_price: binding.price.to_string(),
                    world_id: uuid_text(admitted.world_id.as_bytes()),
                    quantity: 1,
                    currency: platform_currency.into(),
                }, "ITEM").await)?
            } else {
                PurchaseResult::OfferUnavailable
            };
            let history = match &answer {
                PurchaseResult::Accepted { order_id, .. } => {
                    uuid_bytes(order_id).map(|order_id| StorePurchaseHistoryEntry {
                        order_id,
                        offer_key: binding.offer_key.clone(),
                        catalog_revision: binding.catalog_revision.clone(),
                        price: binding.price,
                        currency: currency_text(binding.currency).into(),
                    })
                }
                _ => None,
            };
            let bytes = serde_json::to_vec(&RetainedAnswer::of(&answer)).ok()?;
            owner.root
                .record_store_answer_with_history(journal.clone(), bytes, history)
                .await
                .ok()?
        }
    };
    let answer: RetainedAnswer = serde_json::from_slice(&saved).ok()?;
    let code = answer.code()?;
    if code != wire::StorePurchaseCode::Accepted {
        if prey_expansion {
            owner.root.release_refused_prey_expansion_purchase(owner.character, journal).await.ok()?;
        } else if charm_expansion {
            owner.root.release_refused_charm_expansion_purchase(owner.character, journal).await.ok()?;
        }
        return Some(refusal(code));
    }
    if prey_expansion || charm_expansion {
        // This permanent product has exactly one effect; reject a malformed retained
        // order or several operations before committing even the first entitlement.
        prey_retry::delivery_ids(&answer)?;
    }
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    if hireling_purchase.is_some() && answer.operation_ids.len()!=1 {return None}
    for operation in &answer.operation_ids {
        deliver(owner, &platform, admitted, catalogue, &binding, command, operation,hireling_purchase.as_ref()).await?;
    }
    Some(wire::StorePurchaseResult {
        code,
        order_id: Some(uuid_bytes(answer.order_id.as_deref()?)?),
        operation_id: Some(uuid_bytes(answer.operation_ids.first()?)?),
    })
}

/// Read the Platform delivery intent of one operation of this purchase and mint its item into
/// the Character Inbox under the current gameplay fence; a delivered operation replays its
/// receipt. The receipt is published to Platform and acknowledged in the outbox.
#[allow(clippy::too_many_arguments, reason = "the existing Store delivery owner receives the optional immutable Hireling reservation")]
async fn deliver(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    binding: &rules::PurchaseStoreOffer,
    command: CommandRef,
    operation: &str,
    purchase: Option<&HirelingStorePurchase>,
) -> Option<()> {
    let operation_id = uuid_bytes(operation)?;
    let intent = platform.lock().await.read_intent(crate::native_admission_source::store::OperationKind::Deliver, operation).await.ok()?;
    let account = *binding.account_id.as_bytes();
    let character_id = *binding.character_id.as_bytes();
    let world_id = *admitted.world_id.as_bytes();
    if intent.quantity != Some(1)
        || intent.account_id != uuid_text(&account)
        || intent.offer_key.as_deref() != Some(binding.offer_key.as_str())
        || intent.catalog_revision.as_deref() != Some(binding.catalog_revision.as_str())
    {
        return None;
    }
    if let Some(purchase)=purchase.filter(|p|!matches!(p.action,HirelingStoreAction::Hire {..})) {
        let fence=gameplay_fence(owner,admitted).await?;
        let plan=HirelingServiceDeliveryPlan {identity:DeliveryIdentity {account_id:account,character_id,world_id,fulfilment_unit_id:intent.entitlement_id.clone()},command,purchase:purchase.clone()};
        let receipt=owner.root.apply_store_hireling_service(owner.character,owner.holder,operation_id,plan,fence).await.ok()?;
        publish(owner,platform,operation_id,&intent,&receipt).await?;
        return Some(());
    }
    let effects = rules::plan_delivery(
        &catalogue.rules,
        &rules::DeliveryIntent {
            operation_id: rules::OperationId(operation_id),
            account_id: binding.account_id,
            character_id: binding.character_id,
            world_id: crate::domain::WorldId::from_bytes(world_id).ok()?,
            fulfilment_unit_id: intent.entitlement_id.clone(),
            offer_key: intent.offer_key.clone()?,
            catalog_revision: intent.catalog_revision.clone()?,
            quantity: 1,
        },
        &rules::CharacterStoreView::default(),
    )
    .ok()?;
    if let [rules::DeliveryEffect::GrantUnlock { unlock_key }] = effects.as_slice() {
        let receipt = if unlock_key == catalogue::PREY_EXPANSION_UNLOCK {
            owner.deliver_prey_expansion(admitted, crate::durability::prey::PreyStoreDelivery {
                operation_id, account_id: account, world_id,
                fulfilment_unit_id: intent.entitlement_id.clone(),
            }).await?
        } else if unlock_key == catalogue::CHARM_EXPANSION_UNLOCK {
            let fence = gameplay_fence(owner, admitted).await?;
            owner.root.grant_charm_expansion(owner.character, owner.holder, fence, operation_id,
                DeliveryIdentity { account_id: account, character_id, world_id, fulfilment_unit_id: intent.entitlement_id.clone() }).await.ok()?
        } else { return None };
        let bytes = platform.lock().await.delivery_receipt(&intent, &receipt.recovery_generation, &receipt.decided_at).ok()?;
        if platform.lock().await.publish_receipt(&bytes).await.is_ok() {
            let _ = owner.root.mark_store_receipt_acknowledged(operation_id).await;
        }
        return Some(());
    }
    let item_key=if purchase.is_some() {"oteryn:item.tibia.i29432"} else {
        let [rules::DeliveryEffect::MintStoreInbox {item_key,count:1}]=effects.as_slice() else {return None};
        item_key.as_str()
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: item_key.to_owned(),
        revision_ref: catalogue.item_revision(&binding.offer_key)?.to_owned(),
    };
    let native = owner.active_generation?.native_gameplay()?;
    let item = if matches!(purchase.map(|p| &p.action),Some(HirelingStoreAction::Hire {..})) {
        generation_inbox_item_facts(native, &definition).ok()?
    } else {
        generation_item_facts(native, &definition).ok()?
    };
    let fence = gameplay_fence(owner, admitted).await?;
    let plan = DeliveryPlan {
        account_id: account,
        character_id,
        world_id,
        fulfilment_unit_id: intent.entitlement_id.clone(),
        item_instance_id: item_instance_id(&operation_id),
        command,
        mint: RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "StoreOffer".into(),
                production_key: binding.offer_key.clone(),
                revision_ref: binding.catalog_revision.clone(),
            },
            content_revision: binding.catalog_revision.clone(),
            ruleset_revision: "store-v1".into(),
            sim_revision: "store-v1".into(),
            outputs: vec![RewardClaimInboxMintOutput {
                source_line: 0,
                quantity: 1,
                key_number: None,
                item,
            }],
        },
    };
    let receipt=if let Some(purchase)=purchase {
        owner.root.apply_store_hireling_delivery(owner.character,owner.holder,operation_id,plan,fence,purchase.clone()).await.ok()?
    } else {
        owner.root.apply_store_delivery(owner.character,owner.holder,operation_id,plan,fence).await.ok()?
    };
    publish(owner,platform,operation_id,&intent,&receipt).await
}

async fn publish(owner:&ComposedFreshAdmission<'_, '_, '_>,platform:&StorePlatform,operation_id:[u8;16],intent:&crate::native_admission_source::store::DeliveryIntent,receipt:&crate::durability::store::DeliveryReceipt)->Option<()> {
    let bytes = platform.lock().await.delivery_receipt(intent, &receipt.recovery_generation, &receipt.decided_at).ok()?;
    // Publication is best effort here: the outbox keeps the receipt until Platform accepts it.
    if platform.lock().await.publish_receipt(&bytes).await.is_ok() {
        let _ = owner.root.mark_store_receipt_acknowledged(operation_id).await;
    }
    Some(())
}

/// The current gameplay fence of an Active session whose item fence is still current.
async fn gameplay_fence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> Option<CurrentCharacterGameplayFence> {
    let expected = admitted.item_fence?;
    let (current, _) = FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(admitted.game_session_id)
        .await
        .ok()?;
    if current.session_state() != super::GameSessionState::Active
        || super::item_fence_of(current) != Some(expected)
    {
        return None;
    }
    let revision = owner
        .root
        .read_current_character(owner.character, expected.character_id)
        .await
        .ok()?
        .revision;
    Some(CurrentCharacterGameplayFence {
        expected_character_revision: revision,
        character_id: expected.character_id,
        game_session_id: expected.game_session_id,
        connection_generation: expected.connection_generation,
        character_lease_generation: expected.character_lease_generation,
        runtime_scope: expected.runtime_scope,
        scope_ownership_generation: expected.scope_ownership_generation,
    })
}

/// The delivered item's UUIDv7 identity, fixed by its operation so every retry mints the same item.
fn item_instance_id(operation_id: &[u8; 16]) -> [u8; 16] {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:store-delivery-item:v1\0");
    digest.update(operation_id);
    let mut id = [0; 16];
    id.copy_from_slice(&digest.finalize()[..16]);
    id[6] = 0x70 | (id[6] & 0x0f);
    id[8] = 0x80 | (id[8] & 0x3f);
    id
}

fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

fn parse_history_cursor(text: &str) -> Option<(u64, [u8; 16])> {
    let (milliseconds, identifier) = text.split_once('.')?;
    if milliseconds.is_empty()
        || !milliseconds.bytes().all(|byte| byte.is_ascii_digit())
        || identifier.len() != 32
        || !identifier
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let milliseconds = milliseconds.parse().ok()?;
    let mut id = [0; 16];
    for (index, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(identifier.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some((milliseconds, id))
}

fn format_history_cursor(purchased_at_ms: u64, purchase_request_id: &[u8; 16]) -> String {
    let identifier: String = purchase_request_id
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{purchased_at_ms}.{identifier}")
}

pub(super) fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || text.len() != 36 {
        return None;
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(bytes)
}

#[cfg(test)]
#[path = "store/tests.rs"]
mod tests;

// Platform has one currency: Oteryn Coins. Every coin is transferable.
fn platform_currency(
    currency: rules::CurrencyKind,
) -> Result<&'static str, wire::StorePurchaseCode> {
    match currency {
        rules::CurrencyKind::TibiaCoin => Ok("COINS"),
        rules::CurrencyKind::TransferableTibiaCoin => Err(wire::StorePurchaseCode::OfferUnavailable),
    }
}

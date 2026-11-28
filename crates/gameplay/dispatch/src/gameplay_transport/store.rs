//! STORE_V1 (capability 154): the admitted catalogue page with the Platform balance, and a
//! purchase the Platform debits and the appropriate Game owner fulfills. Platform owns the
//! balance, debit and order ledger. Game retains the request binding and its answer
//! (`store_purchase_requests`) and the delivery receipt of each operation.
#[path = "store/cosmetic_tokens.rs"]
mod cosmetic_tokens;
#[path = "store/claim.rs"]
mod claim;
#[path = "store/coin_history.rs"]
mod coin_history;
use super::connection::AdmittedSession;
use super::item_ref_admission::{generation_item_facts,generation_inbox_item_facts};
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::durability::character_inbox::{RewardClaimInboxMintOutput, RewardClaimInboxMintRequest};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::store::{DeliveryIdentity,DeliveryPlan,HirelingServiceDeliveryPlan,HirelingStoreAction,HirelingStorePurchase,PurchaseRequest,PurchaseRequestState,StoreBlessingDelivery,StorePurchaseHistoryEntry};
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
#[path = "store/browse.rs"]
mod browse;
#[path = "store/prey_retry.rs"]
mod prey_retry;
#[path = "store/receipt_outbox.rs"]
pub(super) mod receipt_outbox;
pub use receipt_outbox::{run_retained_store_reconciliation, run_store_receipt_outbox, run_store_receipt_outbox_with_mode};
#[path = "store/hireling.rs"]
mod hireling;
#[path = "store/hunting_offers.rs"]
mod hunting_offers;
#[path = "store/inbox.rs"]
mod inbox;
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
    if !set.iter().any(|row| row.id == wire::cosmetic_tokens::CAPABILITY) {
        set.push(super::capabilities::OfferedCapability { id: wire::cosmetic_tokens::CAPABILITY, requires: &[wire::CAPABILITY_STORE_V1] });
    }
    for id in [
        wire::browse::CAPABILITY_HOME_V1,
        wire::CAPABILITY_NAME_INPUT_V1,
    ] {
        if !set.iter().any(|row| row.id == id) {
            set.push(super::capabilities::OfferedCapability {
                id,
                requires: &[wire::CAPABILITY_STORE_V1],
            });
        }
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


const ITEM_CATALOGUE_JSON: &[u8] =
    include_bytes!("../../../../../rulesets/economy/store/catalogue.json");
const HIRELING_CATALOGUE_JSON: &[u8] =
    include_bytes!("../../../../../rulesets/economy/store/hirelings.json");
const COSMETIC_CATALOGUE_JSON: &[u8] = crate::content::store_cosmetics::DOCUMENT;
const PLATFORM_CATALOGUE_JSON: &[u8] =
    include_bytes!("../../../../../rulesets/economy/store/platform-catalogue.json");

const PLATFORM_PREDECESSORS_JSON: &[u8] =
    include_bytes!("../../../../../rulesets/economy/store/platform-predecessors.json");

fn platform_revision(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

static PLATFORM_REVISION: LazyLock<String> =
    LazyLock::new(|| platform_revision(PLATFORM_CATALOGUE_JSON));

static PLATFORM_PREDECESSORS: LazyLock<Vec<(String, serde_json::Value)>> = LazyLock::new(|| {
    serde_json::from_slice::<Vec<String>>(PLATFORM_PREDECESSORS_JSON)
        .into_iter()
        .flatten()
        .filter_map(|snapshot| {
            let document = serde_json::from_str(&snapshot).ok()?;
            Some((platform_revision(snapshot.as_bytes()), document))
        })
        .collect()
});

static PLATFORM_OFFERS: LazyLock<Option<serde_json::Value>> =
    LazyLock::new(|| serde_json::from_slice(PLATFORM_CATALOGUE_JSON).ok());

pub(super) fn platform_unit_kind(revision: &str, offer_key: &str) -> Option<&'static str> {
    let document = if revision == PLATFORM_REVISION.as_str() {
        PLATFORM_OFFERS.as_ref()?
    } else if let Some((_, document)) = PLATFORM_PREDECESSORS
        .iter()
        .find(|(old, _)| old == revision)
    {
        document
    } else {
        // Pre-Platform runtime catalogue revisions used ITEM for all grants,
        // including native outfit and mount unlocks.
        return Some("ITEM");
    };
    let offers = document.get("offers")?.as_array()?;
    let offer = offers
        .iter()
        .find(|offer| offer["offer_key"] == offer_key)?;
    let [unit] = offer["units"].as_array()?.as_slice() else {
        return None;
    };
    unit["unit_kind"]
        .as_str()
        .filter(|kind| matches!(*kind, "ITEM" | "COSMETIC"))
}

fn load_active_catalogue(bytes: &[u8]) -> Option<RuntimeCatalogue> {
    RuntimeCatalogue::from_json_with_revision(bytes, Some(&PLATFORM_REVISION))
}

static CATALOGUE: LazyLock<Option<RuntimeCatalogue>> =
    LazyLock::new(|| load_active_catalogue(ITEM_CATALOGUE_JSON));
static HIRELING_CATALOGUE: LazyLock<Option<RuntimeCatalogue>> =
    LazyLock::new(|| load_active_catalogue(HIRELING_CATALOGUE_JSON));
static COSMETIC_CATALOGUE: LazyLock<Option<RuntimeCatalogue>> =
    LazyLock::new(|| load_active_catalogue(COSMETIC_CATALOGUE_JSON));

static PREDECESSOR_CATALOGUES: LazyLock<Vec<Option<RuntimeCatalogue>>> = LazyLock::new(|| {
    [
        RuntimeCatalogue::from_json(include_bytes!(
            "../../../../../rulesets/economy/store/predecessors/catalogue.json"
        )),
        RuntimeCatalogue::from_json(include_bytes!(
            "../../../../../rulesets/economy/store/predecessors/cosmetics.json"
        )),
        RuntimeCatalogue::from_json(include_bytes!(
            "../../../../../rulesets/economy/store/predecessors/charged-catalogue.json"
        )),
        RuntimeCatalogue::from_json(COSMETIC_CATALOGUE_JSON),
        RuntimeCatalogue::from_json(HIRELING_CATALOGUE_JSON),
    ]
    .into_iter()
    .chain(
        PLATFORM_PREDECESSORS
            .iter()
            .map(|(revision, document)| RuntimeCatalogue::from_platform(document, revision)),
    )
    .collect()
});

#[cfg(test)]
pub(super) fn active_catalogue_revision() -> &'static str {
    &PLATFORM_REVISION
}

static BLESSING_CATALOGUE: LazyLock<Option<RuntimeCatalogue>> =
    LazyLock::new(RuntimeCatalogue::from_blessings);

fn catalogue_for_key(key: &str) -> Option<&'static RuntimeCatalogue> {
    if key.starts_with(catalogue::BLESSING_OFFER_PREFIX) {
        BLESSING_CATALOGUE.as_ref()
    } else if crate::content::store_cosmetics::offer(key).is_some() {
        COSMETIC_CATALOGUE.as_ref()
    } else if key.starts_with("store:hireling:") {
        HIRELING_CATALOGUE.as_ref()
    } else {
        CATALOGUE.as_ref()
    }
}

macro_rules! authority_methods {
    () => {
        fn store_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
            _command_type: u32,
            _payload: Vec<u8>,
        ) -> impl std::future::Future<Output = Option<(oteryn_protocol_oteryn::CommandStatus, Vec<u8>)>>
        {
            async { Some((oteryn_protocol_oteryn::CommandStatus::Rejected, Vec::new())) }
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
        ) -> Option<(oteryn_protocol_oteryn::CommandStatus, Vec<u8>)> {
            store::execute_command(self, admitted, command_id, command_type, &payload).await
        }
    };
}
pub(super) use composed_methods;

const REJECTED: (CommandStatus, Vec<u8>) = (CommandStatus::Rejected, Vec::new());

pub(super) async fn execute_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> Option<(CommandStatus, Vec<u8>)> {
    if command_type == wire::COMMAND_TYPE_STORE_PURCHASE
        && let Ok((request, customization)) = wire::decode_purchase_request_with_hireling(payload)
        && hunting_offers::is_offer(&request.offer_key)
    {
        if customization.is_some() {
            return Some(REJECTED);
        }
        let result = hunting_offers::purchase(owner, admitted, request).await?;
        return Some((CommandStatus::Accepted, wire::encode_purchase_result(&result).ok()?));
    }
    Some(execute(owner, admitted, command_id, command_type, payload).await)
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> (CommandStatus, Vec<u8>) {
    let encoded = match command_type {
        wire::cosmetic_tokens::BALANCE => {
            if !payload.is_empty() || !admitted.continuity.selected_capabilities.contains(wire::cosmetic_tokens::CAPABILITY) { return REJECTED; }
            let Some(fence) = gameplay_fence(owner, admitted).await else { return REJECTED; };
            let Ok(balance) = owner.root.read_cosmetic_token_balance(owner.character, owner.holder, fence).await else { return REJECTED; };
            Ok(balance.to_le_bytes().to_vec())
        }
        wire::cosmetic_tokens::PURCHASE => {
            if !admitted.continuity.selected_capabilities.contains(wire::cosmetic_tokens::CAPABILITY) { return REJECTED; }
            let Some(result) = cosmetic_tokens::purchase(owner, admitted, payload).await else { return REJECTED; };
            wire::encode_purchase_result(&result)
        }
        wire::browse::COMMAND_TYPE | wire::browse::COMMAND_TYPE_RECENT => {
            let recent = command_type == wire::browse::COMMAND_TYPE_RECENT;
            if recent
                && !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::browse::CAPABILITY_HOME_V1)
            {
                return REJECTED;
            }
            let Ok(query) = wire::browse::Query::decode(payload) else {
                return REJECTED;
            };
            if recent && query.category != "featured" {
                return REJECTED;
            }
            let Some(page) = browse::execute(owner, admitted, &query, recent).await else {
                return REJECTED;
            };
            if !recent && query.cursor.is_empty() && let Some(active) = CATALOGUE.as_ref() {
                let _ = resume_removed_item(owner, admitted, active, command_id).await;
            }
            page.encode()
        }
        wire::browse::COMMAND_TYPE_BANNERS => {
            if !payload.is_empty()
                || !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::browse::CAPABILITY_HOME_V1)
            {
                return REJECTED;
            }
            browse::banners().encode()
        }
        wire::COMMAND_TYPE_NAME_INPUT => {
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_NAME_INPUT_V1)
            {
                return REJECTED;
            }
            let Ok(name) = wire::decode_name_input(payload) else {
                return REJECTED;
            };
            let valid = crate::domain::character_name::CharacterName::parse(name).is_ok();
            Ok(vec![u8::from(valid)])
        }
        wire::COMMAND_TYPE_STORE_INBOX_CLAIM => {
            let query = if payload == wire::STORE_INBOX_QUERY {
                Some((None, false))
            } else {
                wire::decode_inbox_query(payload).map(|after| (after, true))
            };
            if let Some((after, paged)) = query {
                let Some(page) = Box::pin(inbox::project(owner, admitted, after)).await else {
                    return REJECTED;
                };
                if paged {
                    wire::encode_inbox_page(&page)
                } else {
                    wire::encode_inbox_entries(&page.entries)
                }
            } else {
                let Ok(request) = wire::decode_inbox_claim_request(payload) else {
                    return REJECTED;
                };
                let Some(receipt) = claim::execute(owner, admitted, command_id, request).await else {
                    return REJECTED;
                };
                wire::encode_claim_receipt(&receipt)
            }
        }
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE => {
            let Ok(request) = wire::decode_catalogue_request(payload) else {
                return REJECTED;
            };
            let catalogue=if matches!(request.category.as_str(),"houses.hirelings"|"houses.hireling-dresses") {
                if !admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::hireling::management::CAPABILITY_HIRELING_MANAGEMENT_V1) {return REJECTED}
                HIRELING_CATALOGUE.as_ref()
            } else if request.category == catalogue::BLESSING_CATEGORY {
                BLESSING_CATALOGUE.as_ref()
            } else if wire::cosmetic_tokens::eligible(&request.category) {COSMETIC_CATALOGUE.as_ref()} else {CATALOGUE.as_ref()};
            let Some(catalogue)=catalogue else {return REJECTED};
            if let Some(active) = CATALOGUE.as_ref() {
                let _ = resume_removed_item(owner, admitted, active, command_id).await;
            }
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
        .offers_in_category(&request.category, after, wire::MAX_STORE_PAGE_ENTRIES + 1 + hunting_offers::HIDDEN_XP_TIERS)
        .ok()?;
    if rows.iter().any(|offer| hunting_offers::xp_tier(&offer.key).is_some()) {
        let tier = hunting_offers::current_tier(owner, admitted).await;
        rows.retain(|offer| hunting_offers::visible(&offer.key, tier));
    }
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
    catalogue: &'static RuntimeCatalogue,
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

/// A removed offer cannot be selected again. Opening the Store can finish its
/// exact retained purchase using the buyer's current fenced Inbox authority.
async fn resume_removed_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    active: &'static RuntimeCatalogue,
    command_id: u64,
) -> Option<()> {
    let account = crate::domain::AccountId::from_bytes(admitted.controller?.account_id).ok()?;
    let character = admitted.item_fence?.character_id;
    // The digest excludes the request ID. Only an ID returned by the journal is sent.
    let probe_id = rules::PurchaseRequestId::from_bytes(*account.as_bytes()).ok()?;
    let removed: Vec<_> = PREDECESSOR_CATALOGUES.iter().flatten().flat_map(|old| {
        old.entries().filter_map(move |offer| {
            if !matches!(offer.delivery, rules::DeliveryKind::Item { .. })
                || active.rules.offer(&offer.key).is_some()
            {
                return None;
            }
            let quote = rules::build_quote(&old.rules, &offer.key, &rules::CharacterStoreView::default()).ok()?;
            let binding = rules::PurchaseStoreOffer::from_quote(probe_id, account, character, quote);
            let digest = PurchaseRequest {
                purchase_request_id: *probe_id.as_bytes(),
                account_id: *account.as_bytes(),
                character_id: *character.as_bytes(),
                binding: binding.binding_digest().into(),
            }.binding_digest();
            Some((binding, digest))
        })
    }).collect();
    if removed.is_empty() {
        return Some(());
    }
    let candidates = owner.root.pending_store_item_purchases(
        *account.as_bytes(), *character.as_bytes(), *admitted.world_id.as_bytes(),
        removed.iter().map(|(_, digest)| digest.to_vec()).collect(),
        true,
    ).await.ok()?;
    for pending in candidates {
        if let Some((binding, _)) = removed.iter().find(|(_, digest)| pending.binding_digest == *digest) {
            purchase(owner, admitted, active, command_id, wire::StorePurchaseRequest {
                purchase_request_id: pending.purchase_id,
                offer_key: binding.offer_key.clone(),
                catalog_revision: binding.catalog_revision.clone(),
                price: binding.price,
                currency: currency_text(binding.currency).into(),
            }, None).await?;
            break;
        }
    }
    Some(())
}

/// A new confirmation can resume an unfinished purchase for its buyer. Verify the
/// exact original quote and price before repeating a potentially unanswered debit.
fn retained_binding(
    active: &'static RuntimeCatalogue,
    confirmed: &rules::PurchaseStoreOffer,
    pending: &crate::durability::store::PendingStorePurchase,
) -> Option<(&'static RuntimeCatalogue, rules::PurchaseStoreOffer)> {
    let purchase_request_id = rules::PurchaseRequestId::from_bytes(pending.purchase_id).ok()?;
    let character_id = crate::domain::CharacterId::from_bytes(pending.character_id).ok()?;
    std::iter::once(active).chain(PREDECESSOR_CATALOGUES.iter().flatten()).find_map(|catalogue| {
        let (binding, digest) = matching_quote_binding(catalogue, confirmed, purchase_request_id, character_id)?;
        (pending.binding_digest == digest).then_some((catalogue, binding))
    })
}

fn matching_quote_binding(
    catalogue: &RuntimeCatalogue,
    confirmed: &rules::PurchaseStoreOffer,
    purchase_request_id: rules::PurchaseRequestId,
    character_id: crate::domain::CharacterId,
) -> Option<(rules::PurchaseStoreOffer, [u8; 32])> {
    let quote = rules::build_quote(&catalogue.rules, &confirmed.offer_key, &rules::CharacterStoreView::default()).ok()?;
    if quote.price != confirmed.price || quote.currency != confirmed.currency {
        return None;
    }
    let binding = rules::PurchaseStoreOffer::from_quote(
        purchase_request_id, confirmed.account_id, character_id, quote,
    );
    let digest = PurchaseRequest {
        purchase_request_id: *purchase_request_id.as_bytes(),
        account_id: *confirmed.account_id.as_bytes(),
        character_id: *character_id.as_bytes(),
        binding: binding.binding_digest().into(),
    }.binding_digest();
    Some((binding, digest))
}

/// Legacy accepted journals have no history row to supply the offer. Match the
/// retained quote against the original buyer before deciding who can deliver it.
async fn pending_paid_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    catalogue: &'static RuntimeCatalogue,
    confirmed: &rules::PurchaseStoreOffer,
    world: [u8; 16],
) -> Result<Option<crate::durability::store::PendingStorePurchase>, crate::durability::store::StoreDurabilityError> {
    let account = *confirmed.account_id.as_bytes();
    if let Some(paid) = owner.root.pending_paid_store_item(account, confirmed.offer_key.clone(), world).await? {
        return Ok(Some(paid));
    }
    let mut after = None;
    loop {
        let page = owner.root.pending_legacy_paid_store_items(account, world, after).await?;
        let Some(last) = page.last() else {
            return Ok(None);
        };
        after = Some(last.purchase_id);
        if let Some(paid) = page.into_iter().find(|pending| retained_binding(catalogue, confirmed, pending).is_some()) {
            return Ok(Some(paid));
        }
    }
}

async fn purchase_retained_or_new(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &'static RuntimeCatalogue,
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
    let stale_quote = catalogue.rules.revision() != request.catalog_revision;
    let removed_offer = stale_quote && catalogue.rules.offer(&request.offer_key).is_none();
    let retained = if stale_quote {
        PREDECESSOR_CATALOGUES.iter().flatten().find(|old| {
            old.rules.revision() == request.catalog_revision && old.rules.offer(&request.offer_key).is_some()
        })
    } else {
        Some(catalogue)
    };
    let Some(catalogue) = retained else {
        return Some(refusal(wire::StorePurchaseCode::StaleQuote));
    };
    let cosmetic = crate::content::store_cosmetics::offer(&request.offer_key);
    if let Some(offer) = cosmetic {
        let native = owner.active_generation?.native_gameplay()?.cosmetics()?;
        if !crate::content::store_cosmetics::compatible(native, offer) { return Some(refusal(wire::StorePurchaseCode::OfferUnavailable)); }
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
    let confirmed = rules::PurchaseStoreOffer::from_quote(
        id,
        crate::domain::AccountId::from_bytes(account).ok()?,
        fence.character_id,
        quote,
    );
    let ordinary_item = matches!(
        &catalogue.rules.offer(&request.offer_key)?.delivery, rules::DeliveryKind::Item { .. }
    );
    let mut retained_request = false;
    let (catalogue, binding, pending) = if cosmetic.is_some() || ordinary_item || stale_quote {
        retained_request = owner.root.has_store_purchase_request(
            request.purchase_request_id, account, *fence.character_id.as_bytes(),
        ).await.ok()?;

        let retry = PurchaseRequest {
            purchase_request_id: request.purchase_request_id, account_id: account,
            character_id: *fence.character_id.as_bytes(), binding: confirmed.binding_digest().into(),
        };
        let pending = owner.root.read_store_purchase_retry(retry.clone()).await.ok()?;
        let pending = if pending.is_none() && !retained_request {
            if ordinary_item {
                let world = *admitted.world_id.as_bytes();
                let paid = pending_paid_item(owner, catalogue, &confirmed, world).await.ok()?;
                if paid.is_some() {
                    paid
                } else {
                    let legacy = if removed_offer {
                        vec![retry.binding_digest().to_vec()]
                    } else if stale_quote {
                        Vec::new()
                    } else {
                        std::iter::once(catalogue).chain(PREDECESSOR_CATALOGUES.iter().flatten())
                            .filter_map(|old| matching_quote_binding(old, &confirmed, id, fence.character_id))
                            .map(|(_, digest)| digest.to_vec())
                            .collect()
                    };
                    owner.root.pending_store_item_purchases(account, *fence.character_id.as_bytes(), world, legacy, false)
                        .await.ok()?.into_iter()
                        .find(|candidate| retained_binding(catalogue, &confirmed, candidate).is_some())
                }
            } else if cosmetic.is_some() {
                owner.root.pending_store_cosmetic_purchase(account, *fence.character_id.as_bytes(), request.offer_key.clone()).await.ok()?
            } else {
                None
            }
        } else {
            pending
        };
        if let Some(pending) = pending {
            if pending.character_id != *fence.character_id.as_bytes()
                || (ordinary_item && pending.world_id != *admitted.world_id.as_bytes())
            {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            }
            let Some((catalogue, binding)) = retained_binding(catalogue, &confirmed, &pending) else {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            };
            owner.root.record_store_purchase_retry(retry, pending.purchase_id, pending.world_id).await.ok()?;
            (catalogue, binding, Some(pending))
        } else {
            (catalogue, confirmed, None)
        }
    } else {
        (catalogue, confirmed, None)
    };
    if stale_quote && !retained_request && pending.is_none() {
        return Some(refusal(wire::StorePurchaseCode::StaleQuote));
    }
    let purchase_world = pending.as_ref().map_or(*admitted.world_id.as_bytes(), |pending| pending.world_id);
    let character_id = *binding.character_id.as_bytes();
    let unit_kind = platform_unit_kind(catalogue.rules.revision(), &binding.offer_key)?;
    let journal = PurchaseRequest {
        purchase_request_id: *binding.purchase_request_id.as_bytes(),
        account_id: account,
        character_id,
        binding: hireling::request_binding(binding.binding_digest(),hireling_action.as_ref())?,
    };
    let delivery = &catalogue.rules.offer(&binding.offer_key)?.delivery;
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
    } else if let rules::DeliveryKind::Item { item_key, .. } = delivery {
        if let Some(answer) = owner.root.read_store_purchase_answer(journal.clone()).await.ok()? {
            PurchaseRequestState::Answered(answer)
        } else {
            let current = gameplay_fence(owner, admitted).await?;
            if !matches!(current.runtime_scope, crate::foundation::RuntimeScopeRefV1::Channel { .. }) {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            }
            let native = owner.active_generation?.native_gameplay()?;
            let weights = super::item_ref_admission::NativeInboxItemWeights(native, owner.premium_coordinator);
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: item_key.clone(),
                revision_ref: catalogue.item_revision(&binding.offer_key)?.into(),
            };
            let Some(state) = owner.root.reserve_store_item_purchase(
                owner.character, owner.holder, current, journal.clone(), definition, &weights,
            ).await.ok()? else {
                return Some(refusal(wire::StorePurchaseCode::OfferUnavailable));
            };
            state
        }
    } else {owner.root.record_purchase_request(journal.clone()).await.ok()?};
    let saved = match request_state {
        PurchaseRequestState::Answered(saved) => saved,
        PurchaseRequestState::Sent => {
            let reserved = if cosmetic.is_some() {
                let fence = gameplay_fence(owner, admitted).await?;
                let content = owner.active_generation?.native_gameplay()?.cosmetics()?.clone();
                pending.is_some() || owner.root.reserve_store_cosmetic(
                    owner.character, owner.holder, fence, journal.clone(), request.offer_key.clone(), content,
                ).await.ok()?
            } else if prey_expansion {
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
                purchase_answer(platform.lock().await.purchase(&PurchaseStoreOffer {
                    purchase_request_id: uuid_text(&journal.purchase_request_id),
                    account_id: uuid_text(&account),
                    character_id: uuid_text(&character_id),
                    offer_key: binding.offer_key.clone(),
                    catalog_revision: binding.catalog_revision.clone(),
                    unit_price: binding.price.to_string(),
                    world_id: uuid_text(&purchase_world),
                    quantity: 1,
                    currency: platform_currency.into(),
                }, unit_kind).await)?
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
        if cosmetic.is_some() {
            owner.root.release_store_cosmetic(journal).await.ok()?;
        } else if prey_expansion {
            owner.root.release_refused_prey_expansion_purchase(owner.character, journal).await.ok()?;
        } else if charm_expansion {
            owner.root.release_refused_charm_expansion_purchase(owner.character, journal).await.ok()?;
        }
        return Some(refusal(code));
    }
    if prey_expansion || charm_expansion || binding.offer_key.starts_with(catalogue::BLESSING_OFFER_PREFIX) {
        // Each of these offers has exactly one effect, so validate the retained order
        // before committing its entitlement.
        prey_retry::delivery_ids(&answer)?;
    }
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    if hireling_purchase.is_some() && answer.operation_ids.len()!=1 {return None}
    for operation in &answer.operation_ids {
        deliver(owner, &platform, admitted, catalogue, &binding, command, operation, hireling_purchase.as_ref(), purchase_world).await?;
    }
    Some(wire::StorePurchaseResult {
        code,
        order_id: Some(uuid_bytes(answer.order_id.as_deref()?)?),
        operation_id: Some(uuid_bytes(answer.operation_ids.first()?)?),
    })
}

/// Read the Platform intent and apply its effect under the current gameplay fence.
/// Already fulfilled operations replay their receipt for publication to Platform.
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
    world_id: [u8; 16],
) -> Option<()> {
    let operation_id = uuid_bytes(operation)?;
    if owner.root.has_store_restore_conflict(operation_id).await.ok()? {
        return None;
    }
    let intent = platform.lock().await.read_intent(crate::native_admission_source::store::OperationKind::Deliver, operation).await.ok()?;
    let account = *binding.account_id.as_bytes();
    let character_id = *binding.character_id.as_bytes();
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
        } else if let Some(blessing_key) = catalogue::blessing_storage_key(unlock_key) {
            let fence = gameplay_fence(owner, admitted).await?;
            owner.root.grant_store_blessing(
                owner.character,
                owner.holder,
                fence,
                StoreBlessingDelivery {
                    operation_id,
                    purchase_request_id: *binding.purchase_request_id.as_bytes(),
                    identity: DeliveryIdentity {
                        account_id: account,
                        character_id,
                        world_id,
                        fulfilment_unit_id: intent.entitlement_id.clone(),
                    },
                    blessing_key: blessing_key.to_owned(),
                },
            ).await.ok()?
        } else {
            let offer = crate::content::store_cosmetics::offer(&binding.offer_key)?;
            let native = owner.active_generation?.native_gameplay()?.cosmetics()?;
            if !crate::content::store_cosmetics::compatible(native, offer) { return None; }
            let fence = gameplay_fence(owner, admitted).await?;
            owner.root.grant_store_cosmetic(owner.character, owner.holder, fence, operation_id,
                DeliveryIdentity { account_id: account, character_id, world_id, fulfilment_unit_id: intent.entitlement_id.clone() }, *binding.purchase_request_id.as_bytes(), binding.offer_key.clone()).await.ok()?
        };
        return publish(owner, platform, operation_id, &intent, &receipt).await;
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
        purchase_request_id: Some(*binding.purchase_request_id.as_bytes()),
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
    // Publication is best effort here: the outbox keeps the receipt until Platform accepts it.
    let acknowledged = {
        let mut client = platform.lock().await;
        if let Ok(bytes) = client.delivery_receipt(intent, &receipt.recovery_generation, &receipt.decided_at) {
            client.publish_receipt(&bytes).await.is_ok()
        } else {
            false
        }
    };
    if acknowledged {
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
#[path = "store/platform_price_list_tests.rs"]
mod platform_price_list_tests;

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

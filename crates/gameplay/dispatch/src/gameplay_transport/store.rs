//! STORE_V1 (capability 154): the admitted catalogue page with the Platform balance, and a
//! purchase the Platform debits and the native Inbox mint delivers. Platform owns the balance,
//! the debit and the order ledger; Game retains only the request binding and its answer
//! (`store_purchase_requests`) and the delivery receipt of each operation.
use super::connection::AdmittedSession;
use super::item_ref_admission::generation_item_facts;
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::durability::character_inbox::{RewardClaimInboxMintOutput, RewardClaimInboxMintRequest};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::store::{DeliveryPlan, PurchaseRequest, PurchaseRequestState};
use crate::foundation::{CommandId, CommandRef};
use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer, StoreClient};
use oteryn_gameplay_economy::store as rules;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::store as wire;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, LazyLock};

#[path = "store/catalogue.rs"]
mod catalogue;
use catalogue::{RuntimeCatalogue, currency_text};

/// The node's Platform Store client; `None` refuses every purchase as disabled.
pub type StorePlatform = Arc<tokio::sync::Mutex<StoreClient>>;

static CATALOGUE: LazyLock<Option<RuntimeCatalogue>> = LazyLock::new(|| {
    RuntimeCatalogue::from_json(include_bytes!(
        "../../../../../rulesets/economy/store/catalogue.json"
    ))
});

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
    let Some(catalogue) = CATALOGUE.as_ref() else {
        return REJECTED;
    };
    let encoded = match command_type {
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE => {
            let Ok(request) = wire::decode_catalogue_request(payload) else {
                return REJECTED;
            };
            let Some(page) = catalogue_page(owner, admitted, catalogue, &request).await else {
                return REJECTED;
            };
            wire::encode_catalogue_page(&page)
        }
        wire::COMMAND_TYPE_STORE_PURCHASE => {
            let Ok(request) = wire::decode_purchase_request(payload) else {
                return REJECTED;
            };
            let Some(result) = purchase(owner, admitted, catalogue, command_id, request).await
            else {
                return REJECTED;
            };
            wire::encode_purchase_result(&result)
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
    let (total_coins, transferable_coins) = {
        let mut client = platform.lock().await;
        let total = client.balance(&account, "TIBIA_COIN").await.ok()?;
        let transferable = client
            .balance(&account, "TRANSFERABLE_TIBIA_COIN")
            .await
            .ok()?;
        (total.balance, transferable.balance)
    };
    if transferable_coins > total_coins {
        return None;
    }
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
        transferable_coins,
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

/// A server quote mismatch never reaches Platform. Otherwise the first binding of the request id
/// is retained before the debit, and its answer is saved once: a retry with the same id returns
/// the saved answer and delivers again, idempotently per operation, never a second debit.
async fn purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    command_id: u64,
    request: wire::StorePurchaseRequest,
) -> Option<wire::StorePurchaseResult> {
    let account = admitted.controller?.account_id;
    let fence = admitted.item_fence?;
    let Some(platform) = owner.store.clone() else {
        return Some(refusal(wire::StorePurchaseCode::PurchaseDisabled));
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
        binding: binding.binding_digest().to_vec(),
    };
    let saved = match owner.root.record_purchase_request(journal.clone()).await.ok()? {
        PurchaseRequestState::Answered(saved) => saved,
        PurchaseRequestState::Sent => {
            let answer = platform
                .lock()
                .await
                .purchase(&PurchaseStoreOffer {
                    purchase_request_id: uuid_text(&request.purchase_request_id),
                    account_id: uuid_text(&account),
                    character_id: uuid_text(&character_id),
                    offer_key: binding.offer_key.clone(),
                    catalog_revision: binding.catalog_revision.clone(),
                    price: binding.price,
                    currency: currency_text(binding.currency).into(),
                })
                .await
                // No answer: the debit may have happened, so only a retry of this id resolves it.
                .ok()?;
            let bytes = serde_json::to_vec(&RetainedAnswer::of(&answer)).ok()?;
            owner.root.record_store_answer(journal, bytes).await.ok()?
        }
    };
    let answer: RetainedAnswer = serde_json::from_slice(&saved).ok()?;
    let code = answer.code()?;
    if code != wire::StorePurchaseCode::Accepted {
        return Some(refusal(code));
    }
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    for operation in &answer.operation_ids {
        deliver(owner, &platform, admitted, catalogue, &binding, command, operation).await?;
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
async fn deliver(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    platform: &StorePlatform,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    binding: &rules::PurchaseStoreOffer,
    command: CommandRef,
    operation: &str,
) -> Option<()> {
    let operation_id = uuid_bytes(operation)?;
    let intent = platform.lock().await.read_intent(operation).await.ok()?;
    let account = *binding.account_id.as_bytes();
    let character_id = *binding.character_id.as_bytes();
    let world_id = *admitted.world_id.as_bytes();
    if intent.account_id != uuid_text(&account)
        || intent.character_id.as_deref() != Some(uuid_text(&character_id).as_str())
        || intent
            .world_id
            .as_deref()
            .is_some_and(|world| world != uuid_text(&world_id))
        || intent.offer_key != binding.offer_key
        || intent.catalog_revision != binding.catalog_revision
    {
        return None;
    }
    let effects = rules::plan_delivery(
        &catalogue.rules,
        &rules::DeliveryIntent {
            operation_id: rules::OperationId(operation_id),
            account_id: binding.account_id,
            character_id: binding.character_id,
            world_id: crate::domain::WorldId::from_bytes(world_id).ok()?,
            fulfilment_unit_id: intent.fulfilment_unit_id.clone(),
            offer_key: intent.offer_key.clone(),
            catalog_revision: intent.catalog_revision.clone(),
            quantity: 1,
        },
        &rules::CharacterStoreView::default(),
    )
    .ok()?;
    let [rules::DeliveryEffect::MintStoreInbox { item_key, count: 1 }] = effects.as_slice() else {
        return None;
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: item_key.clone(),
        revision_ref: catalogue.item_revision(&binding.offer_key)?.to_owned(),
    };
    let native = owner.active_generation?.native_gameplay()?;
    let item = generation_item_facts(native, &definition).ok()?;
    let fence = gameplay_fence(owner, admitted).await?;
    let plan = DeliveryPlan {
        account_id: account,
        character_id,
        world_id,
        fulfilment_unit_id: intent.fulfilment_unit_id,
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
    let receipt = owner
        .root
        .apply_store_delivery(owner.character, owner.holder, operation_id, plan, fence)
        .await
        .ok()?;
    let bytes = serde_json::to_vec(&receipt).ok()?;
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

fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
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

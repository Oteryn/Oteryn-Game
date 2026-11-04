//! Closed Store v1 shapes and canonical member order. No permissive JSON decoding.
use super::ProtocolError;
use serde_json::Value;

pub(super) const ISSUER: &str = "OTERYN_PLATFORM_STORE_AUTHORITY";
pub(super) const NIL: &str = "00000000-0000-0000-0000-000000000000";
const HEAD: &str = "contract_version operation source_authority";
const INTENT_HEAD: &str =
    "contract_version operation_kind issuer_authority source_revision operation_id";
const ECHO: &str = "entitlement_id account_id product_id product_version unit_kind";
const RECEIPT_HEAD: &str = "contract_version operation source_authority issuer_authority operation_kind operation_id binding_digest outcome result_code account_id entitlement_id product_id product_version";

pub(super) fn decimal(s: &str, zero: bool, max: u64) -> bool {
    (zero && s == "0")
        || (!s.is_empty()
            && !s.starts_with('0')
            && s.bytes().all(|b| b.is_ascii_digit())
            && s.parse::<u64>().is_ok_and(|v| v <= max))
}
fn key(s: &str, grant: bool) -> bool {
    (1..=64).contains(&s.len())
        && (s.as_bytes()[0].is_ascii_lowercase() || s.as_bytes()[0].is_ascii_digit())
        && s.bytes().all(|b| {
            (b.is_ascii_lowercase() || b.is_ascii_digit())
                || b"._-".contains(&b)
                || (grant && b == b':')
        })
}
fn offer_key(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && (s.as_bytes()[0].is_ascii_lowercase() || s.as_bytes()[0].is_ascii_digit())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.:-".contains(&b))
}
fn product(s: &str) -> bool {
    let parts: Vec<_> = s.split('.').collect();
    s.len() <= 64
        && (2..=4).contains(&parts.len())
        && parts.iter().all(|p| {
            (1..=32).contains(&p.len())
                && p.as_bytes()[0].is_ascii_lowercase()
                && p.bytes()
                    .all(|b| (b.is_ascii_lowercase() || b.is_ascii_digit()) || b == b'_')
        })
}
fn one(s: &str, set: &str) -> bool {
    set.split(' ').any(|v| v == s)
}
fn check(ok: bool) -> Result<(), ProtocolError> {
    if ok {
        Ok(())
    } else {
        Err(ProtocolError::Malformed)
    }
}
fn primitive(k: &str, v: &Value, shape: &str) -> bool {
    let s = v.as_str().unwrap_or("");
    match k {
        "contract_version" => v.as_u64() == Some(1),
        "operation" => s == operation(shape),
        "source_authority" => super::token(s, 128),
        "issuer_authority" => s == ISSUER,
        "audience" => s == "OTERYN_GAME_STORE",
        "operation_kind" => one(s, "DELIVER CLAIM REVOKE_UNCLAIMED RECLAIM FLAG"),
        "unit_kind" => one(s, "PREMIUM_TIME COSMETIC ITEM"),
        "currency" => s == "COINS",
        "reversal_reason" => one(s, "REFUND CHARGEBACK SUPPORT_CORRECTION"),
        "outcome" => one(s, "COMMITTED REJECTED"),
        "state" => one(s, "PENDING REOPENED COMMITTED REJECTED"),
        "kind" => one(
            s,
            "STORE_PURCHASE STORE_REVERSAL COIN_PACKAGE_CREDIT COIN_PACKAGE_REVERSAL AUCTION_RESERVE AUCTION_RELEASE AUCTION_SETTLE OPERATOR_ADJUSTMENT OTHER",
        ),
        "result" => match shape {
            "purchase-answer" => one(
                s,
                "ACCEPTED STALE_QUOTE OFFER_UNAVAILABLE PURCHASE_DISABLED INSUFFICIENT_BALANCE ACCOUNT_RESTRICTED",
            ),
            "receipt-answer" => s == "accepted",
            _ => s == "OK",
        },
        "product_id" => product(s),
        "offer_key" => (shape == "history-entry" && v.is_null()) || offer_key(s),
        "grant_key" => key(s, true),
        "after_operation_id" => s == NIL || super::uuid7(s),
        "order_id" if shape == "history-entry" && v.is_null() => true,
        "receipt_digest" | "recovery_generation" if shape == "state-entry" && v.is_null() => true,
        "catalog_revision" => {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        _ if k.ends_with("_digest") => {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        _ if k.ends_with("_id")
            && k != "entry_id"
            && k != "next_before_entry_id"
            && k != "before_entry_id" =>
        {
            super::uuid7(s)
        }
        "quantity" if shape == "history-entry" && v.is_null() => true,
        "max_entries" => v.as_u64().is_some_and(|v| (1..=32).contains(&v)),
        "quantity" | "delivered_quantity" | "minted_quantity" | "revoked_quantity"
        | "shortfall_quantity" | "reclaimed_quantity" => {
            let min = u64::from(
                k != "reclaimed_quantity" && !(k == "shortfall_quantity" && shape == "receipt"),
            );
            let max = if shape == "purchase-request" || shape == "history-entry" {
                100
            } else {
                65535
            };
            v.as_u64().is_some_and(|v| (min..=max).contains(&v))
        }
        "available" | "reserved" | "available_after" => decimal(s, true, i64::MAX as u64),
        "unit_price" | "total_price" => decimal(s, false, i64::MAX as u64),
        "available_delta" | "reserved_delta" => decimal(
            s.strip_prefix('-').unwrap_or(s),
            !s.starts_with('-'),
            i64::MAX as u64,
        ),
        "before_entry_id" | "next_before_entry_id" | "after_source_revision" => {
            decimal(s, true, u64::MAX)
        }
        "result_code" => true, // The outcome/kind matrix is checked below.
        _ => decimal(s, false, u64::MAX),
    }
}
pub(super) fn operation(shape: &str) -> &'static str {
    match shape {
        "balance-request" => "ReadStoreBalanceV1",
        "history-request" => "ReadStoreHistoryPageV1",
        "purchase-request" => "PurchaseStoreOfferV1",
        "pending-request" => "ListPendingStoreOperationsV1",
        "reconcile-request" => "ListStoreOperationStatesV1",
        "read-request" => "ReadStoreOperationIntentV1",
        "receipt" => "PublishStoreDeliveryReceiptV1",
        _ => "",
    }
}
fn fields(shape: &str, v: &Value) -> Result<String, ProtocolError> {
    Ok(match shape {
        "balance-request" => format!("{HEAD} account_id"),
        "history-request" => format!("{HEAD} account_id before_entry_id max_entries"),
        "purchase-request" => format!("{HEAD} purchase_request_id account_id character_id world_id catalog_revision offer_key currency unit_price quantity"),
        "pending-request" | "reconcile-request" => format!("{HEAD} after_source_revision after_operation_id max_entries"),
        "read-request" => format!("{HEAD} issuer_authority operation_kind operation_id"),
        "balance-response" => "contract_version result account_id balances read_at".into(),
        "balance-entry" => "currency available reserved".into(),
        "history-response" => "contract_version result account_id entries next_before_entry_id".into(),
        "history-entry" => "entry_id kind currency available_delta reserved_delta order_id offer_key quantity created_at".into(),
        "pending-response" => "contract_version pending".into(),
        "pending-entry" => "operation_kind operation_id source_revision".into(),
        "reconcile-response" => "contract_version operations".into(),
        "state-entry" => "operation_kind operation_id source_revision state receipt_digest recovery_generation".into(),
        "unit" => "unit_kind entitlement_id delivery_operation_id".into(),
        "purchase-answer" => format!("contract_version purchase_request_id request_digest result {}decided_at", if v["result"] == "ACCEPTED" {"order_id currency total_price available_after units "} else {""}),
        "receipt-answer" => "contract_version result".into(),
        "intent" => {
            let middle = match v["operation_kind"].as_str() {
                Some("DELIVER") => format!("order_id {ECHO} catalog_revision offer_key grant_key grant_revision quantity"),
                Some("CLAIM") => format!("delivery_operation_id {ECHO} grant_key grant_revision quantity"),
                Some("REVOKE_UNCLAIMED") => format!("delivery_operation_id {ECHO} reversal_reason"),
                Some("RECLAIM") => format!("delivery_operation_id claim_operation_id claim_receipt_digest {ECHO} reversal_reason"),
                Some("FLAG") => format!("delivery_operation_id reversal_operation_id {ECHO} reversal_reason shortfall_quantity"),
                _ => return Err(ProtocolError::UnknownResult),
            };
            format!("{INTENT_HEAD} {middle} issued_at_source audience")
        }
        "receipt" => {
            let kind = v["operation_kind"].as_str().ok_or(ProtocolError::Malformed)?;
            let committed = v["outcome"] == "COMMITTED";
            let code = v["result_code"].as_str().ok_or(ProtocolError::Malformed)?;
            let (members, codes) = if committed {
                match kind {
                    "DELIVER" => ("delivered_quantity ", "STORE_OK"),
                    "CLAIM" => ("character_id world_id minted_quantity minted_units_digest ", "STORE_OK"),
                    "REVOKE_UNCLAIMED" => ("revoked_quantity ", "STORE_OK"),
                    "RECLAIM" => ("character_id world_id reclaimed_quantity shortfall_quantity ", if v["shortfall_quantity"] == 0 {"STORE_RECLAIMED"} else {"STORE_RECLAIM_SHORTFALL"}),
                    "FLAG" => ("", "STORE_OK"),
                    _ => return Err(ProtocolError::UnknownResult),
                }
            } else {
                let specific = match kind {
                    "DELIVER" => "STORE_ACCOUNT_NOT_FOUND STORE_GRANT_UNKNOWN",
                    "CLAIM" => "STORE_GRANT_UNKNOWN STORE_LINE_REVOKED STORE_LINE_NOT_FOUND",
                    "REVOKE_UNCLAIMED" => "STORE_LINE_ALREADY_CLAIMED STORE_LINE_NOT_FOUND",
                    "RECLAIM" | "FLAG" => "STORE_LINE_NOT_FOUND",
                    _ => return Err(ProtocolError::UnknownResult),
                };
                check(one(code, specific) || one(code, "STORE_INTENT_INVALID STORE_OPERATION_CONFLICT STORE_UNSUPPORTED"))?;
                ("", code)
            };
            check(one(code, codes))?;
            format!("{RECEIPT_HEAD} {members}recovery_generation decided_at")
        }
        _ => return Err(ProtocolError::Malformed),
    })
}
/// Encode in the contract's order while validating closed shapes and field types.
pub(super) fn encode(shape: &str, v: &Value) -> Result<String, ProtocolError> {
    let fields = fields(shape, v)?;
    let object = v.as_object().ok_or(ProtocolError::Malformed)?;
    check(object.len() == fields.split_whitespace().count())?;
    let mut parts = Vec::new();
    for k in fields.split_whitespace() {
        let value = object.get(k).ok_or(ProtocolError::Malformed)?;
        let nested = match k {
            "balances" => Some(("balance-entry", 1, 1)),
            "entries" => Some(("history-entry", 0, 32)),
            "pending" => Some(("pending-entry", 0, 32)),
            "operations" => Some(("state-entry", 0, 32)),
            "units" => Some(("unit", 1, 8)),
            _ => None,
        };
        let encoded = if let Some((child, min, max)) = nested {
            let array = value.as_array().ok_or(ProtocolError::Malformed)?;
            check((min..=max).contains(&array.len()))?;
            let rows = array
                .iter()
                .map(|v| encode(child, v))
                .collect::<Result<Vec<_>, _>>()?;
            format!("[{}]", rows.join(","))
        } else {
            check(primitive(k, value, shape))?;
            serde_json::to_string(value).map_err(|_| ProtocolError::Malformed)?
        };
        parts.push(format!("\"{k}\":{encoded}"));
    }
    if shape == "intent" && v["operation_kind"] != "DELIVER" {
        check(v["unit_kind"] == "ITEM")?;
    }
    if shape == "state-entry" {
        let pending = v["state"] == "PENDING" || v["state"] == "REOPENED";
        check(
            v["receipt_digest"].is_null() == pending
                && v["recovery_generation"].is_null() == pending,
        )?;
    }
    if shape == "pending-request" || shape == "reconcile-request" {
        check((v["after_source_revision"] == "0") == (v["after_operation_id"] == NIL))?;
    }
    let raw = format!("{{{}}}", parts.join(","));
    check(!raw.contains('\\'))?;
    Ok(raw)
}
pub(super) fn decode(shape: &str, raw: &[u8], max: usize) -> Result<Value, ProtocolError> {
    super::plain(raw, max)?;
    let v: Value = serde_json::from_slice(raw).map_err(|_| ProtocolError::Malformed)?;
    check(encode(shape, &v)?.as_bytes() == raw)?;
    Ok(v)
}

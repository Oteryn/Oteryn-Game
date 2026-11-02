#![allow(clippy::unwrap_used)]
use super::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

type Calls = Arc<Mutex<Vec<(Operation, String)>>>;
type Receipts = Arc<Mutex<Vec<Vec<u8>>>>;

const OP: &str = "0192b6a0-4c00-7000-8000-000000000701";
const RECEIPT: &[u8] =
    br#"{"operation_id":"01890f3e-7b1c-7abc-8def-0123456789ab","result":"COMMITTED"}"#;

struct Fake {
    replies: VecDeque<Result<Vec<u8>, TransportFailure>>,
    calls: Calls,
}
impl StoreTransport for Fake {
    fn exchange<'a>(
        &'a mut self,
        op: Operation,
        body: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, TransportFailure>> {
        self.calls.lock().unwrap().push((op, body.to_owned()));
        let value = self.replies.pop_front().unwrap();
        Box::pin(async move { value })
    }
}
fn client(replies: Vec<Result<Vec<u8>, TransportFailure>>) -> (StoreClient, Calls) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        StoreClient::new(
            Box::new(Fake {
                replies: replies.into(),
                calls: calls.clone(),
            }),
            "oteryn-game/store-consumer".into(),
        ),
        calls,
    )
}
fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/store-wire-v1")
        .join(name);
    let text = std::fs::read_to_string(path).unwrap();
    compact(&text)
}
fn compact(text: &str) -> Vec<u8> {
    let mut quoted = false;
    text.bytes()
        .filter(|b| {
            if *b == b'"' {
                quoted = !quoted;
            }
            quoted || !b.is_ascii_whitespace()
        })
        .collect()
}
fn list() -> Vec<u8> {
    format!(r#"{{"contract_version":1,"pending":[{{"operation_kind":"DELIVER","operation_id":"{OP}","source_revision":"101"}}]}}"#).into_bytes()
}
fn intent() -> Vec<u8> {
    fixture("intent.deliver-premium-time.json")
}
struct Apply {
    seen: Arc<Mutex<usize>>,
}
struct Publisher {
    replies: VecDeque<Result<(), ProtocolError>>,
    receipts: Receipts,
}
impl PublishReceipt for Publisher {
    fn publish<'a>(&'a mut self, receipt: &'a [u8]) -> BoxFuture<'a, Result<(), ProtocolError>> {
        self.receipts.lock().unwrap().push(receipt.to_vec());
        let reply = self.replies.pop_front().unwrap();
        Box::pin(async move { reply })
    }
}
fn publisher(replies: Vec<Result<(), ProtocolError>>) -> (Box<dyn PublishReceipt>, Receipts) {
    let receipts = Arc::new(Mutex::new(Vec::new()));
    (
        Box::new(Publisher {
            replies: replies.into(),
            receipts: receipts.clone(),
        }),
        receipts,
    )
}
impl ApplyDelivery for Apply {
    fn apply<'a>(&'a mut self, _: &'a DeliveryIntent) -> BoxFuture<'a, Result<Vec<u8>, ()>> {
        *self.seen.lock().unwrap() += 1;
        Box::pin(async { Ok(RECEIPT.to_vec()) })
    }
}
#[derive(Default)]
struct Events {
    unknown: Arc<Mutex<Vec<String>>>,
}
impl ReceiptEvents for Events {
    fn unknown_operation(&mut self, id: &str) {
        self.unknown.lock().unwrap().push(id.to_owned());
    }
}

#[test]
fn replay_of_same_intent_reapplies_and_republishes_stored_receipt() {
    block_on(async {
        let (c, calls) = client(vec![
            Ok(list()),
            Ok(intent()),
            Ok(br#"{"contract_version":1,"pending":[]}"#.to_vec()),
            Ok(list()),
            Ok(intent()),
        ]);
        let (publisher, receipts) = publisher(vec![Ok(()), Ok(())]);
        let seen = Arc::new(Mutex::new(0));
        let mut p = StorePoller::new(
            c,
            Box::new(Apply { seen: seen.clone() }),
            publisher,
            Box::new(Events::default()),
        );
        p.poll_once().await;
        p.poll_once().await;
        p.poll_once().await;
        assert_eq!(*seen.lock().unwrap(), 2);
        assert_eq!(*receipts.lock().unwrap(), vec![RECEIPT.to_vec(); 2]);
        assert_eq!(calls.lock().unwrap().len(), 5);
    });
}

#[test]
fn lost_publish_response_retries_without_changing_operation() {
    block_on(async {
        let (c, calls) = client(vec![Ok(list()), Ok(intent()), Ok(list()), Ok(intent())]);
        let (publisher, _) = publisher(vec![Err(ProtocolError::Transport), Ok(())]);
        let mut p = StorePoller::new(
            c,
            Box::new(Apply {
                seen: Arc::new(Mutex::new(0)),
            }),
            publisher,
            Box::new(Events::default()),
        );
        assert_eq!(p.poll_once().await, POLL);
        assert_eq!(p.poll_once().await, POLL);
        let reads: Vec<_> = calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(op, _)| *op == Operation::ReadStoreOperationIntentV1)
            .map(|(_, b)| b.clone())
            .collect();
        assert_eq!(reads.len(), 2);
        assert!(reads.iter().all(|b| b.contains(OP)));
    });
}

#[test]
fn receipt_404_stops_that_publication_and_raises_alarm() {
    block_on(async {
        let (c, _) = client(vec![Ok(list()), Ok(intent())]);
        let (publisher, _) = publisher(vec![Err(ProtocolError::NotFound)]);
        let events = Events::default();
        let unknown = events.unknown.clone();
        let mut p = StorePoller::new(
            c,
            Box::new(Apply {
                seen: Arc::new(Mutex::new(0)),
            }),
            publisher,
            Box::new(events),
        );
        assert_eq!(p.poll_once().await, POLL);
        assert_eq!(&*unknown.lock().unwrap(), &[OP.to_owned()]);
    });
}

fn request() -> PurchaseStoreOffer {
    let v: serde_json::Value =
        serde_json::from_slice(&fixture("purchase-request.valid.json")).unwrap();
    serde_json::from_value(v).unwrap()
}

#[test]
fn purchase_closed_failures_are_typed() {
    block_on(async {
        for (code, want) in [
            ("STALE_QUOTE", PurchaseResult::StaleQuote),
            ("INSUFFICIENT_BALANCE", PurchaseResult::InsufficientBalance),
        ] {
            let raw = fixture(&format!(
                "purchase-answer.{}.json",
                code.to_lowercase().replace('_', "-")
            ));
            let (mut c, _) = client(vec![Ok(raw)]);
            assert_eq!(c.purchase(&request()).await.unwrap(), want);
        }
    });
}

#[test]
fn accepted_purchase_decodes_bound_ids() {
    block_on(async {
        let raw = fixture("purchase-answer.accepted.json");
        let (mut c, _) = client(vec![Ok(raw)]);
        assert!(matches!(
            c.purchase(&request()).await.unwrap(),
            PurchaseResult::Accepted { .. }
        ));
    });
}

#[test]
fn malformed_and_unknown_results_fail_closed() {
    block_on(async {
        for raw in [
        br#"{"contract_version":1,"result":"NEW_SUCCESS","order_id":null,"operation_ids":null}"#
            .to_vec(),
        br#"{"contract_version":1,"entries":[],"next_cursor":null,"extra":1}"#.to_vec(),
    ] {
        let (mut c, _) = client(vec![Ok(raw)]);
        assert!(c.purchase(&request()).await.is_err() || c.list_pending(&OperationCursor::default()).await.is_err());
    }
    });
}

#[test]
fn store_wire_conformance_pending_response() {
    assert!(decode_pending(&fixture("pending-response.valid.json")).is_ok());
}

#[test]
fn store_wire_conformance_valid_responses_and_requests() {
    let manifest: serde_json::Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for (file, example) in manifest["valid"].as_object().unwrap() {
        let shape = example["schema"]
            .as_str()
            .unwrap()
            .trim_end_matches(".schema.json");
        if shape.starts_with("restore-") {
            continue;
        } // Existing, separate restore contract.
        let raw = fixture(file);
        let decoded = wire::decode(
            shape,
            &raw,
            match shape {
                "balance-request" | "history-request" | "pending-request" | "reconcile-request"
                | "read-request" | "balance-response" => 512,
                "purchase-request" | "receipt" => 1024,
                "purchase-answer" => 2048,
                "intent" => 4096,
                "pending-response" => 8192,
                "receipt-answer" => 64,
                _ => 12288,
            },
        );
        assert!(decoded.is_ok(), "{file}: {decoded:?}");
        let value = decoded.unwrap();
        assert_eq!(
            wire::encode(shape, &value).unwrap().as_bytes(),
            raw,
            "{file}"
        );
        assert_eq!(digest(&raw), example["sha256"], "{file}");
        match shape {
            "intent" => {
                assert_eq!(decode_intent(&raw).unwrap().binding_digest, digest(&raw));
            }
            "purchase-answer" => {
                decode_purchase(&raw).unwrap();
            }
            "pending-response" => {
                decode_pending(&raw).unwrap();
            }
            "balance-response" => {
                typed::<BalancePage>(value).unwrap();
            }
            "history-response" => {
                typed::<HistoryPage>(value).unwrap();
            }
            "reconcile-response" => {
                typed::<ReconcilePage>(value).unwrap();
            }
            _ => {}
        }
    }
}
#[test]
fn store_wire_conformance_invalid_cases() {
    let manifest: serde_json::Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for case in manifest["invalid_cases"].as_array().unwrap() {
        let shape = case["schema"]
            .as_str()
            .unwrap()
            .trim_end_matches(".schema.json");
        if shape.starts_with("restore-") {
            continue;
        }
        assert!(
            wire::encode(shape, &case["body"]).is_err(),
            "{}",
            case["case"]
        );
    }
    for case in manifest["rule_cases"].as_array().unwrap() {
        let shape = case["message"].as_str().unwrap();
        if shape.ends_with("-request") {
            assert!(
                wire::encode(shape, &case["body"]).is_err(),
                "{}",
                case["case"]
            );
        }
    }
}
#[test]
fn store_wire_conformance_rejects_noncanonical_bytes_and_bounds() {
    for (file, shape, max) in [
        ("intent.deliver-item.json", "intent", 4096),
        ("balance-response.valid.json", "balance-response", 512),
        ("purchase-answer.accepted.json", "purchase-answer", 2048),
    ] {
        let raw = fixture(file);
        let mut whitespace = raw.clone();
        whitespace.push(b' ');
        assert!(wire::decode(shape, &whitespace, max).is_err());
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert!(wire::decode(shape, &serde_json::to_vec(&value).unwrap(), max).is_err());
        assert!(wire::decode(shape, &raw, raw.len() - 1).is_err());
    }
}

#[test]
fn store_wire_conformance_client_sends_exact_request_examples() {
    block_on(async {
        let request = request();
        let account = request.account_id.as_str();
        let (mut c, calls) = client(vec![
            Ok(fixture("balance-response.valid.json")),
            Ok(fixture("history-response.valid.json")),
            Ok(fixture("purchase-answer.accepted.json")),
            Ok(fixture("pending-response.valid.json")),
            Ok(fixture("intent.claim.json")),
            Ok(fixture("receipt-answer.valid.json")),
            Ok(fixture("reconcile-response.valid.json")),
        ]);
        c.balance(account).await.unwrap();
        c.history(account, "0", 2).await.unwrap();
        c.purchase(&request).await.unwrap();
        let cursor: OperationCursor =
            serde_json::from_slice(&fixture("pending-request.valid.json")).unwrap();
        c.list_pending(&cursor).await.unwrap();
        let read: serde_json::Value =
            serde_json::from_slice(&fixture("read-request.valid.json")).unwrap();
        c.read_intent(OperationKind::Claim, read["operation_id"].as_str().unwrap())
            .await
            .unwrap();
        c.publish_receipt(&fixture("receipt.claim.committed.json"))
            .await
            .unwrap();
        let reconcile: OperationCursor =
            serde_json::from_slice(&fixture("reconcile-request.valid.json")).unwrap();
        c.reconcile(&reconcile).await.unwrap();
        let calls = calls.lock().unwrap();
        for ((op, body), (expected_op, file)) in calls.iter().zip([
            (Operation::ReadStoreBalanceV1, "balance-request.valid.json"),
            (
                Operation::ReadStoreHistoryPageV1,
                "history-request.valid.json",
            ),
            (
                Operation::PurchaseStoreOfferV1,
                "purchase-request.valid.json",
            ),
            (
                Operation::ListPendingStoreOperationsV1,
                "pending-request.valid.json",
            ),
            (
                Operation::ReadStoreOperationIntentV1,
                "read-request.valid.json",
            ),
            (
                Operation::PublishStoreDeliveryReceiptV1,
                "receipt.claim.committed.json",
            ),
            (
                Operation::ListStoreOperationStatesV1,
                "reconcile-request.valid.json",
            ),
        ]) {
            assert_eq!(*op, expected_op);
            assert_eq!(body.as_bytes(), fixture(file), "{file}");
        }
        assert_eq!(calls.len(), 7);
    });
}
#[test]
fn store_wire_conformance_receipt_binds_exact_intent_bytes() {
    let (c, _) = client(Vec::new());
    for (intent_file, receipt_file) in [
        (
            "intent.deliver-item.json",
            "receipt.deliver-item.committed.json",
        ),
        (
            "intent.deliver-premium-time.json",
            "receipt.deliver-premium-time.committed.json",
        ),
    ] {
        let intent = decode_intent(&fixture(intent_file)).unwrap();
        let receipt = fixture(receipt_file);
        let expected: serde_json::Value = serde_json::from_slice(&receipt).unwrap();
        let bytes = c
            .delivery_receipt(
                &intent,
                expected["recovery_generation"].as_str().unwrap(),
                expected["decided_at"].as_str().unwrap(),
            )
            .unwrap();
        assert_eq!(bytes, receipt);
    }
}
#[test]
fn store_wire_conformance_purchase_refuses_wrong_request_binding() {
    block_on(async {
        for member in [
            "purchase_request_id",
            "request_digest",
            "currency",
            "total_price",
        ] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fixture("purchase-answer.accepted.json")).unwrap();
            value[member] = match member {
                "purchase_request_id" => OP.into(),
                "request_digest" => "0".repeat(64).into(),
                "currency" => "TRANSFERABLE_COINS".into(),
                _ => "1".into(),
            };
            let raw = fixture("purchase-answer.accepted.json");
            let original: serde_json::Value = serde_json::from_slice(&raw).unwrap();
            let raw = String::from_utf8(raw)
                .unwrap()
                .replace(
                    &format!("\"{member}\":{}", original[member]),
                    &format!("\"{member}\":{}", value[member]),
                )
                .into_bytes();
            let (mut c, _) = client(vec![Ok(raw)]);
            assert_eq!(
                c.purchase(&request()).await,
                Err(ProtocolError::Malformed),
                "{member}"
            );
        }
    });
}
#[test]
fn store_wire_conformance_manifest_provenance() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/store-wire-v1/manifest.json"),
    )
    .unwrap();
    assert_eq!(
        digest(&bytes),
        "fb46ea97101f2f6ff0356076d0beb9058633b87075861123ad3f68cf50a688f5"
    );
}

#[test]
fn store_wire_conformance_http_status_mapping() {
    for (status, expected) in [
        (404, TransportFailure::NotFound),
        (409, TransportFailure::Conflict),
        (503, TransportFailure::Unavailable),
        (429, TransportFailure::RateLimited(Duration::from_secs(17))),
    ] {
        let operation = if status == 404 {
            Operation::ReadStoreOperationIntentV1
        } else {
            Operation::PurchaseStoreOfferV1
        };
        assert_eq!(
            classify_response(operation, status, Vec::new(), Some(Duration::from_secs(17))),
            Err(expected)
        );
        assert_eq!(
            classify_response(
                operation,
                status,
                b"{}".to_vec(),
                Some(Duration::from_secs(17))
            ),
            Err(TransportFailure::InvalidResponse)
        );
    }
    assert_eq!(
        classify_response(Operation::PurchaseStoreOfferV1, 429, Vec::new(), None),
        Err(TransportFailure::InvalidResponse)
    );
    assert_eq!(
        classify_response(
            Operation::PurchaseStoreOfferV1,
            200,
            b"stored answer".to_vec(),
            None
        ),
        Ok(b"stored answer".to_vec())
    );
}

#[test]
fn store_wire_conformance_retry_retains_exact_purchase_bytes() {
    block_on(async {
        let (mut c, calls) = client(vec![
            Err(TransportFailure::Unavailable),
            Err(TransportFailure::RateLimited(Duration::from_secs(17))),
            Ok(fixture("purchase-answer.accepted.json")),
        ]);
        let request = request();
        assert_eq!(c.purchase(&request).await, Err(ProtocolError::Transport));
        assert_eq!(
            c.purchase(&request).await,
            Err(ProtocolError::RateLimited(Duration::from_secs(17)))
        );
        assert!(matches!(
            c.purchase(&request).await,
            Ok(PurchaseResult::Accepted { .. })
        ));
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().all(|call| call == &calls[0]));
    });
}

#[test]
fn store_wire_conformance_routes_and_bounds() {
    for (op, route, request, response) in [
        (Operation::ReadStoreBalanceV1, "balance/read", 512, 512),
        (
            Operation::ReadStoreHistoryPageV1,
            "history/read",
            512,
            12288,
        ),
        (Operation::PurchaseStoreOfferV1, "purchases", 1024, 2048),
        (
            Operation::ListPendingStoreOperationsV1,
            "operations/pending",
            512,
            8192,
        ),
        (
            Operation::ReadStoreOperationIntentV1,
            "operations/read",
            512,
            4096,
        ),
        (
            Operation::PublishStoreDeliveryReceiptV1,
            "operations/receipt",
            1024,
            64,
        ),
        (
            Operation::ListStoreOperationStatesV1,
            "operations/reconcile",
            512,
            12288,
        ),
    ] {
        assert_eq!(
            op.path(),
            format!("/internal/v1/products-entitlements/store/{route}")
        );
        assert_eq!(op.request_bytes_max(), request);
        assert_eq!(response_bytes_max(op), Some(response));
    }
}

#[test]
fn store_wire_conformance_invalid_catalog_quote_never_reaches_platform() {
    block_on(async {
        for field in ["catalog_revision", "offer_key", "currency"] {
            let (mut c, calls) = client(Vec::new());
            let mut request = request();
            if field == "catalog_revision" {
                request.catalog_revision = "legacy-r1".into();
            } else if field == "offer_key" {
                request.offer_key = "Store:exercise-sword".into();
            } else {
                request.currency = "TRANSFERABLE_COINS".into();
            }
            assert_eq!(c.purchase(&request).await, Err(ProtocolError::InvalidInput));
            assert!(calls.lock().unwrap().is_empty());
        }
    });
}

#[test]
fn store_wire_offer_keys_keep_colons_and_reject_uppercase() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("purchase-request.valid.json")).unwrap();
    value["catalog_revision"] = "a".repeat(64).into();
    for key in [
        "store:exercise-sword".to_owned(),
        format!("s{}", ":".repeat(127)),
    ] {
        value["offer_key"] = key.into();
        assert!(wire::encode("purchase-request", &value).is_ok());
    }
    for key in [
        "Store:exercise-sword".to_owned(),
        format!("s{}", ":".repeat(128)),
    ] {
        value["offer_key"] = key.into();
        assert!(wire::encode("purchase-request", &value).is_err());
    }
}

#[test]
fn store_wire_balance_requires_exactly_one_coins_entry() {
    block_on(async {
        let raw = fixture("balance-response.valid.json");
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let account = value["account_id"].as_str().unwrap();
        let (mut c, _) = client(vec![Ok(raw.clone())]);
        assert_eq!(c.balance(account).await.unwrap().balances.len(), 1);
        for currency in ["TRANSFERABLE_COINS", "GOLD", "", "TIBIA_COIN"] {
            let raw = String::from_utf8(raw.clone())
                .unwrap()
                .replace("COINS", currency)
                .into_bytes();
            let (mut c, _) = client(vec![Ok(raw)]);
            assert_eq!(
                c.balance(account).await,
                Err(ProtocolError::Malformed),
                "{currency}"
            );
        }
        for balances in [
            serde_json::json!([]),
            serde_json::json!([value["balances"][0], value["balances"][0]]),
            serde_json::json!([value["balances"][0], {"currency":"TRANSFERABLE_COINS","available":"1","reserved":"0"}]),
        ] {
            let mut invalid = value.clone();
            invalid["balances"] = balances;
            assert_eq!(
                wire::encode("balance-response", &invalid),
                Err(ProtocolError::Malformed)
            );
        }
    });
}

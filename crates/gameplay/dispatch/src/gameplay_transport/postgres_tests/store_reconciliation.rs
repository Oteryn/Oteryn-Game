#![allow(clippy::expect_used)]

use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{platform_bytes, sha256, uuid};
use crate::gameplay_transport::{run_retained_store_reconciliation, run_store_receipt_outbox};
use crate::native_admission_source::descriptor::Operation;
use crate::native_admission_source::store::{StoreClient, StoreTransport, TransportFailure};
use serde_json::{Value, json};
use std::time::Duration;

const PREMIUM: &[u8] = include_bytes!(
    "../../../../native-admission-source/tests/fixtures/store-wire-v1/intent.deliver-premium-time.json"
);
const BACKLOG: u64 = 8_000;

struct PremiumPlatform {
    intent: Value,
    backlog: u64,
    generation: String,
    receipts: tokio::sync::mpsc::UnboundedSender<Value>,
    reads: tokio::sync::mpsc::UnboundedSender<()>,
}

impl StoreTransport for PremiumPlatform {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, TransportFailure>> + Send + 'a>,
    > {
        let request: Value = serde_json::from_str(body).expect("request");
        let answer = match operation {
            Operation::ListPendingStoreOperationsV1 | Operation::ListStoreOperationStatesV1 => {
                let states = operation == Operation::ListStoreOperationStatesV1;
                let after: u64 = request["after_source_revision"]
                    .as_str()
                    .expect("cursor")
                    .parse()
                    .expect("revision");
                let max = request["max_entries"].as_u64().expect("page size");
                let mut entries = Vec::new();
                for revision in after + 1..=(after + max).min(self.backlog + 1) {
                    let premium = revision > self.backlog;
                    let mut older_id = id(73);
                    older_id[12..].copy_from_slice(&(revision as u32).to_be_bytes());
                    let mut entry = json!({
                        "operation_kind": if premium { "DELIVER" } else { "CLAIM" },
                        "operation_id": if premium {
                            self.intent["operation_id"].clone()
                        } else {
                            uuid(&older_id).into()
                        },
                        "source_revision": revision.to_string(),
                    });
                    if states {
                        entry["state"] = if premium { "REOPENED" } else { "PENDING" }.into();
                        entry["receipt_digest"] = Value::Null;
                        entry["recovery_generation"] = Value::Null;
                    }
                    entries.push(entry);
                }
                if states {
                    json!({"contract_version": 1, "operations": entries})
                } else {
                    json!({"contract_version": 1, "pending": entries})
                }
            }
            Operation::ReadStoreOperationIntentV1
                if request["operation_id"] == self.intent["operation_id"] =>
            {
                self.reads.send(()).expect("intent subscriber");
                self.intent.clone()
            }
            Operation::PublishStoreDeliveryReceiptV1 => {
                assert_eq!(
                    request["binding_digest"],
                    sha256(&platform_bytes(&self.intent))
                );
                assert_eq!(request["operation_id"], self.intent["operation_id"]);
                assert_eq!(request["outcome"], "COMMITTED");
                assert_eq!(request["delivered_quantity"], self.intent["quantity"]);
                if request["recovery_generation"] != self.generation.as_str() {
                    return Box::pin(async { Err(TransportFailure::Conflict) });
                }
                self.receipts.send(request).expect("receipt subscriber");
                json!({"contract_version": 1, "result": "accepted"})
            }
            _ => return Box::pin(async { Err(TransportFailure::NotFound) }),
        };
        Box::pin(async move { Ok(platform_bytes(&answer)) })
    }
}

fn registration(backlog: u64, restored: bool) -> TestResult {
    run(async move |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "store_premium_registration", true, false)
                .await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let (tx, mut receipts) = tokio::sync::mpsc::unbounded_channel();
        let (read, mut reads) = tokio::sync::mpsc::unbounded_channel();
        let generation = if restored {
            let previous = seal.record().recovery_generation;
            let issued_at = seal.record().issued_at + 1;
            drop(seal);
            let transition = harness
                .recovery
                .begin_recovery(previous, id(181), issued_at)
                .map_err(debug)?;
            harness
                .root
                .reconcile_character_recovery(&transition)
                .await
                .map_err(debug)?;
            transition.record().recovery_generation
        } else {
            drop(seal);
            harness
                .recovery
                .seal_current()
                .map_err(debug)?
                .record()
                .recovery_generation
        };
        let mut intent: Value = serde_json::from_slice(PREMIUM)?;
        intent["source_revision"] = (backlog + 1).to_string().into();
        let client = || {
            StoreClient::new(
                Box::new(PremiumPlatform {
                    intent: intent.clone(),
                    backlog,
                    generation: generation.to_string(),
                    receipts: tx.clone(),
                    reads: read.clone(),
                }),
                "oteryn-game/store-consumer".into(),
            )
        };
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        if backlog > 0 {
            let work = run_retained_store_reconciliation(&harness.root, client(), generation);
            tokio::select! {
                () = work => return Err("Store worker exited".into()),
                result = tokio::time::timeout(Duration::from_secs(4), reads.recv()) => {
                    result.map_err(|_| "reopened delivery still behind the pending backlog")?
                        .ok_or("intent channel closed")?;
                }
            }
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let mut first = None;
        for _ in 0..2 {
            let work = async {
                if restored {
                    run_retained_store_reconciliation(&harness.root, client(), generation).await;
                } else {
                    run_store_receipt_outbox(&harness.root, &authority, client()).await;
                }
            };
            let receipt = tokio::select! {
                () = work => return Err("Store worker exited".into()),
                receipt = tokio::time::timeout(Duration::from_secs(4), receipts.recv()) =>
                    receipt.map_err(|_| "Store worker did not register Premium Time")?.ok_or("receipt channel closed")?,
            };
            if let Some(first) = &first {
                assert_eq!(first, &receipt, "registration retry changed its receipt");
            } else {
                first = Some(receipt);
            }
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn store_worker_registers_web_premium_time() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-web-premium", || {
        registration(0, false)
    })
}

#[test]
fn store_restore_drains_older_pending_pages_before_notice_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-restore-backlog", || {
        registration(BACKLOG, true)
    })
}

#[test]
fn store_restore_registers_reopened_web_premium_time() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-restore-premium", || {
        registration(0, true)
    })
}

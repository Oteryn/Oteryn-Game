//! S4: Platform purchase -> durable Inbox -> backpack -> depot -> successor admission.
use super::*;
use crate::native_admission_source::descriptor::Operation;
use crate::native_admission_source::store::{StoreClient, StoreTransport, TransportFailure};
use oteryn_protocol_oteryn::store as wire;
use std::sync::Mutex;

#[derive(Default)]
struct PlatformLedger {
    purchases: u64,
    balance: u64,
    receipts: u64,
    intent: Option<serde_json::Value>,
}
struct Platform(Arc<Mutex<PlatformLedger>>);
impl StoreTransport for Platform {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, TransportFailure>> + Send + 'a>,
    > {
        let result = (|| {
            let mut ledger = self.0.lock().map_err(|_| TransportFailure::NotFound)?;
            let request: serde_json::Value =
                serde_json::from_str(body).map_err(|_| TransportFailure::NotFound)?;
            let answer = match operation {
                Operation::ReadStoreBalanceV1 => {
                    serde_json::json!({"contract_version":1,"balance":{"currency":request["currency"],"balance":ledger.balance}})
                }
                Operation::PurchaseStoreOfferV1 => {
                    let price = request["price"]
                        .as_u64()
                        .ok_or(TransportFailure::NotFound)?;
                    ledger.purchases += 1;
                    ledger.balance = ledger
                        .balance
                        .checked_sub(price)
                        .ok_or(TransportFailure::NotFound)?;
                    ledger.intent = Some(
                        serde_json::json!({"operation_id":crate::gameplay_transport::canonical_uuid(&id(101)),"account_id":request["account_id"],"character_id":request["character_id"],"world_id":crate::gameplay_transport::canonical_uuid(&id(42)),"fulfilment_unit_id":"unit-1","catalog_revision":request["catalog_revision"],"offer_key":request["offer_key"]}),
                    );
                    serde_json::json!({"contract_version":1,"result":"ACCEPTED","order_id":crate::gameplay_transport::canonical_uuid(&id(121)),"operation_ids":[crate::gameplay_transport::canonical_uuid(&id(101))]})
                }
                Operation::ReadStoreDeliveryIntentV1 => {
                    serde_json::json!({"contract_version":1,"intent":ledger.intent})
                }
                Operation::PublishStoreReceiptV1 => {
                    ledger.receipts += 1;
                    serde_json::json!({"contract_version":1,"result":"accepted"})
                }
                _ => return Err(TransportFailure::NotFound),
            };
            Ok(answer.to_string().into_bytes())
        })();
        Box::pin(async { result })
    }
}

#[test]
fn s4_store_inbox_inventory_depot_replay_and_relogin() -> TestResult {
    timed("s4-store", scenario)
}

fn scenario() -> TestResult {
    world_scenario!("xmech_s4", |h,
                                 request,
                                 seal,
                                 authority,
                                 world_id,
                                 native,
                                 world,
                                 cells,
                                 room,
                                 pin,
                                 deps,
                                 actor,
                                 session,
                                 owner,
                                 fence,
                                 admitted| {
        let platform = Arc::new(Mutex::new(PlatformLedger {
            balance: 60,
            ..PlatformLedger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(Platform(platform.clone())),
        ))));
        let account: Vec<u8> = sqlx::query_scalar("SELECT decode(replace(account_id::text,'-',''),'hex') FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let mut controller = admitted.controller.ok_or("controller")?;
        controller.account_id = account.as_slice().try_into()?;
        admitted.controller = Some(controller);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[crate::gameplay_transport::capabilities::OfferedCapability {
                id: wire::CAPABILITY_STORE_V1,
                requires: &[],
            }],
            &[wire::CAPABILITY_STORE_V1],
        )
        .ok_or("Store capability")?;
        let purchase = wire::encode_purchase_request(&wire::StorePurchaseRequest {
            purchase_request_id: id(62),
            offer_key: "store:exercise-sword".into(),
            catalog_revision: "store-catalogue-r1".into(),
            price: 25,
            currency: "TIBIA_COIN".into(),
        })
        .map_err(debug)?;
        let result = exchange(
            &owner,
            &mut admitted,
            wire::COMMAND_TYPE_STORE_PURCHASE,
            &purchase,
        )
        .await?;
        assert_eq!(result.0, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(
            wire::decode_purchase_result(&result.1).map_err(debug)?.code,
            wire::StorePurchaseCode::Accepted
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM store_delivery_receipts").await?,
            1
        );
        let item: Vec<u8> = sqlx::query_scalar("SELECT decode(replace(item_instance_id::text,'-',''),'hex') FROM game_item_character_inbox_locations").fetch_one(&h.pool).await?;
        let item: [u8; 16] = item.as_slice().try_into()?;
        assert_eq!(count(&h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i28552' AND quantity=1 AND lifecycle=1").await?,1);
        let bought = snapshot(&h).await?;
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                wire::COMMAND_TYPE_STORE_PURCHASE,
                &purchase
            )
            .await?,
            result
        );
        assert_eq!(snapshot(&h).await?, bought);
        {
            let p = platform.lock().map_err(|_| "Platform ledger")?;
            assert_eq!((p.purchases, p.balance, p.receipts), (1, 35, 2));
        }
        at_locker(&mut owner, admitted).await?;
        let retrieve = oteryn_protocol_oteryn::depot_search::Request::Retrieve {
            client_id: 28552,
            all: true,
        };
        let moved = owner
            .depot_search_command(actor, admitted, 70, retrieve)
            .await;
        assert_eq!(
            (moved.outcome, moved.retrieved),
            (oteryn_protocol_oteryn::depot_search::Outcome::Retrieved, 1)
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_item_character_inbox_locations"
            )
            .await?,
            0
        );
        let inventory = ledger(&h).await?;
        assert!(
            inventory
                .backpack
                .iter()
                .any(|(_, q, i)| *i == item && *q == 1)
        );
        let moved_state = snapshot(&h).await?;
        assert_eq!(
            owner
                .depot_search_command(actor, admitted, 70, retrieve)
                .await,
            moved
        );
        assert_eq!(snapshot(&h).await?, moved_state);
        // XMECH-E2E-1 GAP: all enabled Store item products have UNKNOWN equipment
        // semantics (rulesets/economy/store/catalogue.json and native Item policies).
        // Delivery and retrieval above are exercised; only equip/hotkey lacks a valid input.
        let policy = native
            .item_policy("oteryn:item.tibia.i28552", "definition-r1")
            .ok_or("exercise sword policy")?;
        assert!(!matches!(
            policy.record().semantics.equipment,
            crate::content::ReferenceItemField::Known(_)
        ));
        deposit(&h, &owner, admitted, item, 71).await?;
        let next = logout_relogin(&h, &mut owner, admitted).await?;
        let page = owner
            .depot(
                next.runtime_actor.ok_or("next actor")?,
                command(next, 1),
                DepotIntent {
                    town: Some(0),
                    box_number: DEPOT_BOX,
                    page: 0,
                    action: DepotAction::Browse,
                },
                None,
            )
            .await
            .ok_or("depot reload")?;
        assert_eq!(page.reply.entries.len(), 1);
        assert_eq!(page.reply.entries[0].count, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}

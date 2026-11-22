//! Crystal's five-wildcard Store pack through the native purchase command.
use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{FakePlatform, Ledger, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::store::{RuntimeCatalogue, execute_with_catalogue};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex};

const PACK: &str = "store:prey-wildcards-5";

#[test]
fn store_wildcards_replay_once_and_refuse_a_pack_above_the_cap() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-wildcards", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "store_services", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let account: Vec<u8> =
            sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots")
                .fetch_one(&h.pool)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("fence")?);
        let mut controller = admitted.controller.ok_or("controller")?;
        controller.account_id = account.try_into().map_err(|_| "account length")?;
        admitted.controller = Some(controller);
        let ledger = Arc::new(Mutex::new(Ledger {
            balance: 300,
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(FakePlatform(ledger.clone())),
            "oteryn-game/store-consumer".into(),
        ))));
        let catalogue = RuntimeCatalogue::from_json(&serde_json::to_vec(&serde_json::json!({
            "schema":"OTERYN_STORE_RUNTIME_CATALOGUE/v1",
            "source":{"crystal":"201189f871d23597b82caf88fd611454a2ddefb8"},
            "offers":[{
                "key":PACK, "category":"extras.useful-things", "title":"Prey Wildcard",
                "price":25, "currency":"TIBIA_COIN", "source_line":6536,
                "service":{"kind":"prey_wildcards","count":5}
            }]
        }))?)
        .ok_or("admit Crystal wildcard pack")?;
        for (index, request_seed) in [60, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70]
            .into_iter()
            .enumerate()
        {
            let request = wire::StorePurchaseRequest {
                purchase_request_id: id(request_seed),
                offer_key: PACK.into(),
                catalog_revision: catalogue.rules.revision().into(),
                price: 25,
                currency: "TIBIA_COIN".into(),
            };
            let payload = wire::encode_purchase_request(&request).map_err(debug)?;
            let (status, bytes) =
                execute_with_catalogue(&owner, admitted, (index + 1) as u64, &payload, |_| {
                    Some(&catalogue)
                })
                .await;
            assert_eq!(status, CommandStatus::Accepted);
            let result = wire::decode_purchase_result(&bytes).map_err(debug)?;
            let expected = if request_seed == 70 {
                wire::StorePurchaseCode::OfferUnavailable
            } else {
                wire::StorePurchaseCode::Accepted
            };
            assert_eq!(result.code, expected);
        }
        let settled: (i16, i64, i64, i64) = sqlx::query_as(
            "SELECT balance,(SELECT count(*) FROM game_account_prey_wildcard_entries WHERE kind='STORE'),
                    (SELECT count(*) FROM store_delivery_receipts),(SELECT count(*) FROM store_receipt_outbox)
             FROM game_account_prey_wildcards",
        ).fetch_one(&h.pool).await?;
        assert_eq!(settled, (50, 10, 10, 10));
        assert_eq!(
            ledger.lock().map_err(|_| "ledger poisoned")?.debits,
            [25; 10]
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

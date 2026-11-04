//! Game-owned display history contains only accepted in-game Store purchases.
#![allow(clippy::expect_used)]

use super::fixture::{Harness, debug, id, run};
use super::store::{FakePlatform, Ledger, purchase, purchase_request, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex as SyncMutex};

const SWORD: &str = "store:exercise-sword";

#[test]
fn store_purchase_history_lists_in_game_purchases_newest_first() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("store-history", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "store_history", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let account: String =
            sqlx::query_scalar("SELECT replace(account_id::text,'-','') FROM game_character_roots")
                .fetch_one(&harness.pool)
                .await?;
        let mut account_id = [0; 16];
        for (index, byte) in account_id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&account[index * 2..index * 2 + 2], 16)?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("fence")?);
        let mut controller_binding = admitted.controller.ok_or("controller")?;
        controller_binding.account_id = account_id;
        admitted.controller = Some(controller_binding);
        let ledger = Arc::new(SyncMutex::new(Ledger {
            balance: 60,
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(FakePlatform(ledger)),
            "oteryn-game/store-consumer".into(),
        ))));
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                wire::CAPABILITY_STORE_V1,
                wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
            ],
        )
        .ok_or("Store history selection")?;

        let purchase_62 = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
        let purchase_63 = purchase(&owner, &mut admitted, purchase_request(63, SWORD, 25)?).await?;
        assert_eq!(purchase_62.code, wire::StorePurchaseCode::Accepted);
        assert_eq!(purchase_63.code, wire::StorePurchaseCode::Accepted);
        assert_eq!(
            purchase(&owner, &mut admitted, purchase_request(64, SWORD, 25)?)
                .await?
                .code,
            wire::StorePurchaseCode::InsufficientBalance,
        );
        assert_eq!(
            purchase(&owner, &mut admitted, purchase_request(60, SWORD, 24)?)
                .await?
                .code,
            wire::StorePurchaseCode::StaleQuote,
        );
        let history_request = || {
            wire::encode_history_request(&wire::StoreHistoryRequest {
                page_cursor: String::new(),
            })
            .map(|payload| (wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE, payload))
            .map_err(debug)
        };
        let values = serve(&owner, &mut admitted, &[history_request()?]).await?;
        assert_eq!(values[0].0, CommandStatus::Accepted);
        let page = wire::decode_history_page(&values[0].1).map_err(debug)?;
        assert_eq!(page.entries.len(), 2);
        assert_eq!(
            page.entries[0].order_id,
            purchase_63.order_id.ok_or("order 63")?
        );
        assert_eq!(
            page.entries[1].order_id,
            purchase_62.order_id.ok_or("order 62")?
        );
        assert!(page.entries[0].purchased_at_unix_ms >= page.entries[1].purchased_at_unix_ms);
        if page.entries[0].purchased_at_unix_ms == page.entries[1].purchased_at_unix_ms {
            assert!(id(63) > id(62));
        }
        for entry in &page.entries {
            assert_eq!(entry.offer_key, SWORD);
            assert_eq!(entry.price, 25);
            assert_eq!(entry.currency, "TIBIA_COIN");
        }
        assert!(page.next_page_cursor.is_empty());
        let count = || async {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_store_purchase_history")
                .fetch_one(&harness.pool)
                .await
        };
        assert_eq!(count().await?, 2);
        let replay = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
        assert_eq!(replay.order_id, purchase_62.order_id);
        assert_eq!(count().await?, 2);

        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_STORE_V1],
        )
        .ok_or("Store-only selection")?;
        assert_eq!(
            serve(&owner, &mut admitted, &[history_request()?]).await?[0],
            (CommandStatus::Rejected, vec![])
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                wire::CAPABILITY_STORE_V1,
                wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
            ],
        )
        .ok_or("Store history reselection")?;
        let malformed = wire::encode_history_request(&wire::StoreHistoryRequest {
            page_cursor: "x".into(),
        })
        .map_err(debug)?;
        assert_eq!(
            serve(
                &owner,
                &mut admitted,
                &[(wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE, malformed)]
            )
            .await?[0],
            (CommandStatus::Rejected, vec![])
        );
        Ok(())
    })
}

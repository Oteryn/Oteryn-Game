//! A restore after Platform acceptance and before the native grant.
use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;
use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer};
use oteryn_gameplay_economy::store as rules;

pub(super) async fn prepare(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    cosmetic: &wire::StorePurchaseRequest,
) -> TestResult {
    use super::super::hireling_store_fixtures::{command, new_hire};
    use super::super::store_fixtures::{purchase, purchase_request};

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &[
            OfferedCapability { id: 3, requires: &[] },
            OfferedCapability { id: 51, requires: &[] },
            OfferedCapability { id: 154, requires: &[] },
            OfferedCapability { id: 165, requires: &[3, 51] },
            OfferedCapability { id: 250, requires: &[165] },
        ],
        &[3, 51, 154, 165, 250],
    ).ok_or("Store and Hireling capabilities")?;
    ledger.lock().map_err(|_| "ledger")?.balance = 10_000;
    let first = purchase(owner, admitted, command(74, 25440, 150, Some(new_hire("Alex Moon")))?).await?;
    let second = purchase(owner, admitted, command(75, 25440, 150, Some(new_hire("Taylor Moon")))?).await?;
    assert_eq!(first.code, wire::StorePurchaseCode::Accepted);
    assert_eq!(second.code, wire::StorePurchaseCode::Accepted);
    {
        let mut platform = ledger.lock().map_err(|_| "ledger")?;
        platform.intents.clear();
        platform.receipts = 0;
    }
    sqlx::raw_sql(
        "CREATE FUNCTION interrupt_restored_store_grant() RETURNS trigger LANGUAGE plpgsql AS
         $$ BEGIN RAISE EXCEPTION 'fixture interruption before native grant'; END; $$;
         CREATE TRIGGER interrupt_restored_store_grant BEFORE INSERT ON store_delivery_receipts
         FOR EACH ROW EXECUTE FUNCTION interrupt_restored_store_grant();",
    ).execute(&harness.pool).await?;
    let commands = [
        (185, wire::encode_purchase_request(cosmetic).map_err(debug)?),
        purchase_request(81, "store:charm-expansion", 450)?,
        command(82, 25440, 150, Some(new_hire("Morgan Moon")))?,
        command(83, 1001, 250, None)?,
        command(84, 25438, 250, Some(wire::StoreHirelingCustomization {
            hireling_id: first.operation_id,
            name: Some("Jamie Moon".into()),
            sex: None,
        }))?,
        command(85, 25437, 120, Some(wire::StoreHirelingCustomization {
            hireling_id: second.operation_id,
            name: None,
            sex: None,
        }))?,
    ];
    for command in commands {
        let rejected = serve(owner, admitted, &[command]).await?;
        assert_eq!(rejected, [(CommandStatus::Rejected, Vec::new())]);
    }
    retain_legacy_prey_debit(owner, *admitted).await?;
    assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits.len(), 9);
    assert_eq!(reservations(harness).await?, (1, 1, 1, 4));
    Ok(())
}

async fn retain_legacy_prey_debit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let offer = "store:prey-list-expansion";
    let mut catalogue: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../rulesets/economy/store/catalogue.json"
    ))?;
    let source = catalogue["offers"].as_array_mut().ok_or("Store offers")?
        .iter_mut().find(|row| row["key"] == offer).ok_or("legacy Prey offer")?;
    source["enabled"] = true.into();
    let price = 37; // Retained quote from the existing Prey recovery fixture.
    source["price"] = price.into();
    catalogue.sort_all_objects();
    let revision = super::super::store_fixtures::sha256(&serde_json::to_vec(&catalogue)?);
    let account = admitted.controller.ok_or("controller")?.account_id;
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner, admitted.game_session_id, false,
    ).await.ok_or("current fence")?;
    let purchase_id = id(86);
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(purchase_id).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: fence.character_id,
        offer_key: offer.into(),
        catalog_revision: revision.clone(),
        price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let journal = crate::durability::store::PurchaseRequest {
        purchase_request_id: purchase_id,
        account_id: account,
        character_id: *fence.character_id.as_bytes(),
        binding: binding.binding_digest().to_vec(),
    };
    owner.root.record_purchase_request(journal.clone()).await.map_err(debug)?;
    assert!(owner.root.reserve_prey_expansion_purchase(
        owner.character, owner.holder, fence, journal.clone(),
    ).await.map_err(debug)?);
    let answer = owner.store.as_ref().ok_or("Platform")?.lock().await.purchase(
        &PurchaseStoreOffer {
            purchase_request_id: uuid(&purchase_id),
            account_id: uuid(&account),
            character_id: uuid(fence.character_id.as_bytes()),
            world_id: uuid(admitted.world_id.as_bytes()),
            offer_key: offer.into(),
            catalog_revision: revision,
            unit_price: price.to_string(),
            quantity: 1,
            currency: "COINS".into(),
        },
        "ITEM",
    ).await.map_err(debug)?;
    let PurchaseResult::Accepted { order_id, operation_ids } = answer else {
        return Err("legacy Prey debit was not accepted".into());
    };
    owner.root.record_store_answer(journal, serde_json::to_vec(&serde_json::json!({
        "result": "ACCEPTED", "order_id": order_id, "operation_ids": operation_ids,
    }))?).await.map_err(debug)?;
    Ok(())
}

pub(super) async fn reconcile(
    harness: &Harness,
    ledger: &Arc<SyncMutex<Ledger>>,
    predecessor: u64,
    issued_at: u64,
) -> TestResult {
    let transition = harness.recovery.begin_recovery(predecessor, id(181), issued_at).map_err(debug)?;
    harness.root.reconcile_character_recovery(&transition).await.map_err(debug)?;
    let generation = transition.record().recovery_generation;
    drop(transition);
    let ready = {
        let mut platform = ledger.lock().map_err(|_| "ledger")?;
        platform.recovery_generation = Some(generation.to_string());
        platform.advertise_deliveries = true;
        platform.reopened = true;
        platform.receipt_ready.clone()
    };
    let client = StoreClient::new(Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into());
    let reported = async {
        loop {
            if ledger.lock().map_err(|_| "ledger")?.restore_conflicts.len() == 7 {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            ready.notified().await;
        }
    };
    tokio::select! {
        () = crate::gameplay_transport::run_retained_store_reconciliation(&harness.root, client, generation) => {
            return Err("restore worker ended before reporting conflicts".into());
        }
        result = tokio::time::timeout(std::time::Duration::from_secs(10), reported) => result??,
    }
    let fences: i64 = sqlx::query_scalar("SELECT count(*) FROM game_store_restore_conflicts")
        .fetch_one(&harness.pool).await?;
    assert_eq!(fences, 7);
    assert_eq!(reservations(harness).await?, (0, 0, 0, 0), "a reported restore conflict must release its pending reservation");
    let deliveries: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
        .fetch_one(&harness.pool).await?;
    assert_eq!(deliveries, 2, "only the two original Hirelings were delivered");
    Ok(())
}

async fn reservations(harness: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_character_store_cosmetics WHERE operation_id IS NULL),
         (SELECT count(*) FROM game_prey_expansion_purchases),
         (SELECT count(*) FROM game_charm_expansion_purchases),
         (SELECT count(*) FROM game_hireling_store_reservations WHERE state = 1)",
    ).fetch_one(&harness.pool).await?)
}

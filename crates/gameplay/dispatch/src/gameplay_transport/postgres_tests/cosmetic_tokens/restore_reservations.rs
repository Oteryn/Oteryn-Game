//! A restore after Platform acceptance and before the native grant.
use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;
use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer};
use oteryn_gameplay_economy::store as rules;

#[derive(Clone, Copy)]
pub(super) enum Snapshot {
    Answered,
    Sent,
    Fenced,
}

pub(super) type LateAnswer = (crate::durability::store::PurchaseRequest, Vec<u8>);

pub(super) async fn prepare(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    cosmetic: &wire::StorePurchaseRequest,
    snapshot: Snapshot,
) -> TestResult<Option<LateAnswer>> {
    use super::super::hireling_store_fixtures::{command, new_hire};
    use super::super::store_fixtures::purchase;

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
    if matches!(snapshot, Snapshot::Sent) {
        sqlx::raw_sql(
            "CREATE FUNCTION interrupt_store_answer() RETURNS trigger LANGUAGE plpgsql AS
             $$ BEGIN RAISE EXCEPTION 'fixture interruption before answer retention'; END; $$;
             CREATE TRIGGER interrupt_store_answer BEFORE UPDATE ON store_purchase_requests
             FOR EACH ROW EXECUTE FUNCTION interrupt_store_answer();",
        ).execute(&harness.pool).await?;
    }
    for command in purchase_commands(
        cosmetic.clone(), 80, first.operation_id, second.operation_id,
    )? {
        let rejected = serve(owner, admitted, &[command]).await?;
        assert_eq!(rejected, [(CommandStatus::Rejected, Vec::new())]);
    }
    if matches!(snapshot, Snapshot::Sent) {
        sqlx::query("DROP TRIGGER interrupt_store_answer ON store_purchase_requests")
            .execute(&harness.pool).await?;
    }
    let late = retain_legacy_prey_debit(owner, *admitted, snapshot).await?;
    assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits.len(), 9);
    assert_eq!(reservations(harness).await?, (1, 1, 1, 4));
    Ok(late)
}

async fn retain_legacy_prey_debit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    snapshot: Snapshot,
) -> TestResult<Option<LateAnswer>> {
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
    let answer = serde_json::to_vec(&serde_json::json!({
        "result": "ACCEPTED", "order_id": order_id, "operation_ids": operation_ids,
    }))?;
    if matches!(snapshot, Snapshot::Sent) {
        return Ok(Some((journal, answer)));
    }
    owner.root.record_store_answer(journal, answer).await.map_err(debug)?;
    Ok(None)
}

pub(super) async fn reconcile(
    harness: &Harness,
    ledger: &Arc<SyncMutex<Ledger>>,
    predecessor: u64,
    issued_at: u64,
    snapshot: Snapshot,
    late: Option<LateAnswer>,
) -> TestResult {
    if matches!(snapshot, Snapshot::Fenced) {
        // Recreate the database just before 3300 and run the real migration
        // after the restore worker has written the permanent fences.
        sqlx::raw_sql(
            "DROP TRIGGER game_store_restore_release_reservations ON game_store_restore_conflicts;
             DROP TRIGGER IF EXISTS game_store_answer_release_reservations ON store_purchase_requests;
             DROP FUNCTION game_store_release_restored_reservations();
             DROP FUNCTION IF EXISTS game_store_release_restored_reservation(uuid);
             DROP INDEX IF EXISTS store_purchase_accepted_account;
             DROP INDEX store_purchase_accepted_operation;
             DROP FUNCTION game_store_accepted_operation(bytea);",
        ).execute(&harness.pool).await?;
    }
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
    if matches!(snapshot, Snapshot::Fenced) {
        assert_eq!(reservations(harness).await?, (1, 1, 1, 4));
        sqlx::raw_sql(include_str!("../../../../../../../apps/game-server/migrations/3351_store_restore_reservations.sql"))
            .execute(&harness.pool).await?;
    }
    if let Some((request, answer)) = late {
        let sent: i64 = sqlx::query_scalar("SELECT count(*) FROM store_purchase_requests WHERE state=1")
            .fetch_one(&harness.pool).await?;
        assert_eq!(sent, 7);
        let paid = ledger.lock().map_err(|_| "ledger")?.answers.clone();
        for seed in 80..=85 {
            let id = id(seed);
            let (_, answer) = paid.get(&uuid(&id)).ok_or("Platform answer for original purchase")?;
            let retained = serde_json::to_vec(&serde_json::json!({
                "result": "ACCEPTED",
                "order_id": answer["order_id"],
                "operation_ids": [answer["units"][0]["delivery_operation_id"].clone()],
            }))?;
            sqlx::query("UPDATE store_purchase_requests SET state=2,answer=$2 WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1")
                .bind(id.as_slice()).bind(retained).execute(&harness.pool).await?;
        }
        harness.root.record_store_answer(request, answer).await.map_err(debug)?;
    }
    assert_eq!(reservations(harness).await?, (0, 0, 0, 0), "a reported restore conflict must release its pending reservation");
    let deliveries: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
        .fetch_one(&harness.pool).await?;
    assert_eq!(deliveries, 2, "only the two original Hirelings were delivered");
    Ok(())
}

pub(super) async fn buy_again(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    cosmetic: wire::StorePurchaseRequest,
) -> TestResult {
    use super::super::store_fixtures::purchase;
    sqlx::query("DROP TRIGGER interrupt_restored_store_grant ON store_delivery_receipts")
        .execute(&harness.pool).await?;
    let targets: Vec<sqlx::types::Uuid> = sqlx::query_scalar(
        "SELECT hireling_id FROM game_house_hirelings ORDER BY name",
    ).fetch_all(&harness.pool).await?;
    let [first, second] = targets.as_slice() else { return Err("expected the two retained Hirelings".into()); };
    for command in purchase_commands(cosmetic, 90, Some(*first.as_bytes()), Some(*second.as_bytes()))? {
        assert_eq!(purchase(owner, admitted, command).await?.code, wire::StorePurchaseCode::Accepted);
    }
    let fenced_grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM store_delivery_receipts JOIN game_store_restore_conflicts USING(operation_id)",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(fenced_grants, 0);
    Ok(())
}

fn purchase_commands(
    mut cosmetic: wire::StorePurchaseRequest,
    seed: u8,
    first: Option<[u8; 16]>,
    second: Option<[u8; 16]>,
) -> TestResult<[(u32, Vec<u8>); 6]> {
    use super::super::hireling_store_fixtures::{command, new_hire};
    use super::super::store_fixtures::purchase_request;
    cosmetic.purchase_request_id = id(seed);
    Ok([
        (wire::COMMAND_TYPE_STORE_PURCHASE, wire::encode_purchase_request(&cosmetic).map_err(debug)?),
        purchase_request(seed + 1, "store:charm-expansion", 450)?,
        command(seed + 2, 25440, 150, Some(new_hire("Morgan Moon")))?,
        command(seed + 3, 1001, 250, None)?,
        command(seed + 4, 25438, 250, Some(wire::StoreHirelingCustomization {
            hireling_id: first,
            name: Some("Jamie Moon".into()),
            sex: None,
        }))?,
        command(seed + 5, 25437, 120, Some(wire::StoreHirelingCustomization {
            hireling_id: second,
            name: None,
            sex: None,
        }))?,
    ])
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

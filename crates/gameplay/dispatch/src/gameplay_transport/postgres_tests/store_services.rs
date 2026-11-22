//! Crystal Store services through the same handler used by command 185.
use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{FakePlatform, Ledger, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::store::{RuntimeCatalogue, execute_with_catalogue};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
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
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-wildcards", || {
        scenario(Scenario::Wildcards)
    })
}

#[test]
fn store_xp_boost_replays_and_refuses_wrong_tier_and_an_active_boost() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-xp-boost", || {
        scenario(Scenario::XpBoost)
    })
}

#[test]
fn store_blessing_bundle_grants_every_blessing_and_refuses_while_any_is_at_cap() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-blessings", || {
        scenario(Scenario::Blessings)
    })
}

#[test]
fn store_service_effect_failure_rolls_back_receipt_and_retries_without_another_debit() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-service-rollback", || {
        scenario(Scenario::Rollback)
    })
}

enum Scenario {
    Wildcards,
    XpBoost,
    Blessings,
    Rollback,
}

fn scenario(scenario: Scenario) -> TestResult {
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
            balance: 800,
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(FakePlatform(ledger.clone())),
            "oteryn-game/store-consumer".into(),
        ))));
        let debits = match scenario {
            Scenario::XpBoost => {
                xp_boost(&h, &owner, admitted).await?;
                vec![30]
            }
            Scenario::Wildcards => {
                wildcards(&h, &owner, admitted).await?;
                vec![25; 10]
            }
            Scenario::Blessings => {
                blessings(&h, &owner, admitted).await?;
                vec![650]
            }
            Scenario::Rollback => {
                rollback(&h, &owner, admitted).await?;
                vec![25]
            }
        };
        assert_eq!(ledger.lock().map_err(|_| "ledger poisoned")?.debits, debits);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

const ALL_BLESSINGS: &str = "store:all-blessings-5";
const XP_TIER_ONE: &str = "store:xp-boost-1";
const XP_TIER_TWO: &str = "store:xp-boost-2";

fn catalogue() -> TestResult<RuntimeCatalogue> {
    RuntimeCatalogue::from_json(&serde_json::to_vec(&serde_json::json!({
        "schema":"OTERYN_STORE_RUNTIME_CATALOGUE/v1",
        "source":{"crystal":"201189f871d23597b82caf88fd611454a2ddefb8"},
        "offers":[{
            "key":PACK, "category":"extras.useful-things", "title":"Prey Wildcard",
            "price":25, "currency":"TIBIA_COIN", "source_line":6536,
            "service":{"kind":"prey_wildcards","count":5}
        }, {
            "key":XP_TIER_ONE, "category":"boosts", "title":"XP Boost",
            "price":30, "currency":"TIBIA_COIN", "source_line":6485,
            "service":{"kind":"xp_boost","tier":1}
        }, {
            "key":XP_TIER_TWO, "category":"boosts", "title":"XP Boost",
            "price":45, "currency":"TIBIA_COIN", "source_line":6485,
            "service":{"kind":"xp_boost","tier":2}
        }, {
            "key":ALL_BLESSINGS, "category":"consumables.blessings", "title":"All Regular Blessings",
            "price":650, "currency":"TIBIA_COIN", "source_line":127,
            "service":{"kind":"blessings","count":5}
        }]
    }))?)
    .ok_or_else(|| "admit Crystal Store service offers".into())
}

async fn buy(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    command: u64,
    seed: u8,
    offer: (&str, u64),
) -> TestResult<wire::StorePurchaseResult> {
    let (status, bytes) =
        command_purchase(owner, admitted, catalogue, command, seed, offer).await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok(wire::decode_purchase_result(&bytes).map_err(debug)?)
}

async fn command_purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    catalogue: &RuntimeCatalogue,
    command: u64,
    seed: u8,
    offer: (&str, u64),
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let request = wire::StorePurchaseRequest {
        purchase_request_id: id(seed),
        offer_key: offer.0.into(),
        catalog_revision: catalogue.rules.revision().into(),
        price: offer.1,
        currency: "TIBIA_COIN".into(),
    };
    let payload = wire::encode_purchase_request(&request).map_err(debug)?;
    let (status, bytes) =
        execute_with_catalogue(owner, admitted, command, &payload, |_| Some(catalogue)).await;
    Ok((status, bytes))
}

async fn wildcards(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let catalogue = catalogue()?;
    for (index, seed) in [60, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70]
        .into_iter()
        .enumerate()
    {
        let result = buy(
            owner,
            admitted,
            &catalogue,
            (index + 1) as u64,
            seed,
            (PACK, 25),
        )
        .await?;
        let expected = if seed == 70 {
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
    Ok(())
}

async fn xp_boost(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let catalogue = catalogue()?;
    sqlx::query(
        "INSERT INTO game_world_reset_history(world_id,reset_epoch,reset_day,reset_at_unix_ms,committed_at_unix_ms)
         SELECT world_id,1,1,118800000,118800000 FROM game_character_roots",
    ).execute(&h.pool).await?;
    let wrong_tier = buy(owner, admitted, &catalogue, 1, 60, (XP_TIER_TWO, 45)).await?;
    assert_eq!(wrong_tier.code, wire::StorePurchaseCode::OfferUnavailable);
    let purchase = buy(owner, admitted, &catalogue, 2, 61, (XP_TIER_ONE, 30)).await?;
    assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
    let replay = buy(owner, admitted, &catalogue, 3, 61, (XP_TIER_ONE, 30)).await?;
    assert_eq!(replay, purchase);
    let active = buy(owner, admitted, &catalogue, 4, 62, (XP_TIER_TWO, 45)).await?;
    assert_eq!(active.code, wire::StorePurchaseCode::OfferUnavailable);
    let settled: (i16, i16, i64) = sqlx::query_as(
        "SELECT b.remaining_minutes,p.amount,(SELECT count(*) FROM store_receipt_outbox)
         FROM game_store_hunting_xp_boosts b JOIN game_store_service_purchases p USING(operation_id)"
    ).fetch_one(&h.pool).await?;
    assert_eq!(settled, (60, 1, 1));
    Ok(())
}

async fn blessings(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let catalogue = catalogue()?;
    let purchase = buy(owner, admitted, &catalogue, 1, 60, (ALL_BLESSINGS, 650)).await?;
    assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
    let replay = buy(owner, admitted, &catalogue, 2, 60, (ALL_BLESSINGS, 650)).await?;
    assert_eq!(replay, purchase);
    let capped = buy(owner, admitted, &catalogue, 3, 61, (ALL_BLESSINGS, 650)).await?;
    assert_eq!(capped.code, wire::StorePurchaseCode::OfferUnavailable);
    let settled: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_blessings),
                (SELECT count(*) FROM game_character_blessing_extra_charges WHERE remaining=4),
                (SELECT count(*) FROM store_receipt_outbox)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(settled, (8, 8, 1));
    Ok(())
}

async fn rollback(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let catalogue = catalogue()?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_store_credit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected Store effect failure'; END $$;
         CREATE TRIGGER reject_store_credit BEFORE INSERT ON game_account_prey_wildcard_entries
         FOR EACH ROW EXECUTE FUNCTION reject_store_credit();",
    )
    .execute(&h.pool)
    .await?;
    let interrupted = command_purchase(owner, admitted, &catalogue, 1, 60, (PACK, 25)).await?;
    assert_eq!(interrupted, (CommandStatus::Rejected, Vec::new()));
    let rolled_back: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_account_prey_wildcards),
                (SELECT count(*) FROM store_delivery_receipts),
                (SELECT count(*) FROM store_receipt_outbox),
                (SELECT count(*) FROM game_store_service_purchases WHERE operation_id IS NULL)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rolled_back, (0, 0, 0, 1));
    sqlx::query("DROP TRIGGER reject_store_credit ON game_account_prey_wildcard_entries")
        .execute(&h.pool)
        .await?;
    let replay = buy(owner, admitted, &catalogue, 2, 60, (PACK, 25)).await?;
    assert_eq!(replay.code, wire::StorePurchaseCode::Accepted);
    let settled: (i16,i64,i64) = sqlx::query_as(
        "SELECT balance,(SELECT count(*) FROM store_delivery_receipts),(SELECT count(*) FROM store_receipt_outbox)
         FROM game_account_prey_wildcards",
    ).fetch_one(&h.pool).await?;
    assert_eq!(settled, (5, 1, 1));
    Ok(())
}

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
fn store_service_effect_failure_rolls_back_and_replays_after_catalogue_change() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-service-rollback", || {
        scenario(Scenario::Rollback)
    })
}

#[test]
fn store_weekly_expansion_commits_the_task_board_and_refuses_a_second_purchase() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-weekly-expansion", || {
        scenario(Scenario::Weekly)
    })
}

#[test]
fn store_reward_access_is_spent_once_by_a_daily_claim_away_from_the_shrine() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-reward-access", || {
        scenario(Scenario::RewardAccess)
    })
}

enum Scenario {
    Wildcards,
    XpBoost,
    Blessings,
    Rollback,
    Weekly,
    RewardAccess,
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
        let content = matches!(scenario, Scenario::Weekly | Scenario::RewardAccess).then(|| {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            )
        });
        let mut owner = deps.owner(&h, &authority);
        if let Some((controller, _)) = &content {
            owner.active_generation = Some(controller.active().ok_or("active generation")?);
        }
        if let Some((_, pin)) = content
            .as_ref()
            .filter(|_| matches!(scenario, Scenario::Weekly))
        {
            let base =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::task_board::extend(base.catalogue())
                    .map_err(debug)?,
            ));
            let world = deps.admitted.world_id;
            let channel = deps.admitted.channel_id;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.activation_sequence().to_string())
                .bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice())
                .execute(&h.pool).await?;
            sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
                .bind(world.as_bytes().as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind("taskboard-test-map")
                .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice())
                .execute(&h.pool).await?;
        }
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
            Scenario::Weekly => {
                weekly_expansion(&h, &owner, admitted).await?;
                vec![450]
            }
            Scenario::RewardAccess => {
                reward_access(&h, &owner, admitted).await?;
                vec![100]
            }
        };
        assert_eq!(ledger.lock().map_err(|_| "ledger poisoned")?.debits, debits);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

const ACCESS: &str = "store:instant-reward-access";
const WEEKLY: &str = "store:permanent-weekly-task-expansion";
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
        }, {
            "key":WEEKLY, "category":"extras.useful-things", "title":"Permanent Weekly Task Expansion",
            "price":450, "currency":"TIBIA_COIN", "source_line":6570,
            "service":{"kind":"weekly_task_expansion"}
        }, {
            "key":ACCESS, "category":"extras.useful-things", "title":"Instant Reward Access",
            "price":100, "currency":"TIBIA_COIN", "source_line":6546,
            "service":{"kind":"instant_reward_access"}
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
    let retained = wire::StorePurchaseRequest {
        purchase_request_id: id(60),
        offer_key: PACK.into(),
        catalog_revision: catalogue.rules.revision().into(),
        price: 25,
        currency: "TIBIA_COIN".into(),
    };
    let payload = wire::encode_purchase_request(&retained).map_err(debug)?;
    let (status, bytes) = execute_with_catalogue(owner, admitted, 2, &payload, |_| None).await;
    assert_eq!(status, CommandStatus::Accepted);
    let replay = wire::decode_purchase_result(&bytes).map_err(debug)?;
    assert_eq!(replay.code, wire::StorePurchaseCode::Accepted);
    let settled: (i16,i64,i64) = sqlx::query_as(
        "SELECT balance,(SELECT count(*) FROM store_delivery_receipts),(SELECT count(*) FROM store_receipt_outbox)
         FROM game_account_prey_wildcards",
    ).fetch_one(&h.pool).await?;
    assert_eq!(settled, (5, 1, 1));
    Ok(())
}

async fn weekly_expansion(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let catalogue = catalogue()?;
    let purchase = buy(owner, admitted, &catalogue, 1, 60, (WEEKLY, 450)).await?;
    assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
    assert_eq!(
        buy(owner, admitted, &catalogue, 2, 60, (WEEKLY, 450)).await?,
        purchase
    );
    assert_eq!(
        buy(owner, admitted, &catalogue, 3, 61, (WEEKLY, 450))
            .await?
            .code,
        wire::StorePurchaseCode::OfferUnavailable
    );
    let settled: (bool,i64,i64) = sqlx::query_as(
        "SELECT (body->>'expansion')::boolean,(SELECT count(*) FROM game_task_board_expansion_deliveries),
                (SELECT count(*) FROM store_receipt_outbox) FROM game_character_task_boards",
    ).fetch_one(&h.pool).await?;
    assert_eq!(settled, (true, 1, 1));
    Ok(())
}

async fn reward_access(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::daily_reward as daily;
    let catalogue = catalogue()?;
    sqlx::query(
        "INSERT INTO game_world_reset_history(world_id,reset_epoch,reset_day,reset_at_unix_ms,committed_at_unix_ms)
         SELECT world_id,1,1,118800000,118800000 FROM game_character_roots",
    ).execute(&h.pool).await?;
    let purchase = buy(owner, admitted, &catalogue, 1, 60, (ACCESS, 100)).await?;
    assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 2,
        item_fence: admitted.item_fence,
    };
    let first = owner
        .daily_command(actor, command, Some(daily::DailyRewardClaim))
        .await
        .ok_or("claim result")?;
    assert!(first.starts_with(&[8, 1, 18, 16]));
    let replay = owner
        .daily_command(actor, command, Some(daily::DailyRewardClaim))
        .await
        .ok_or("claim replay")?;
    assert_eq!(
        replay,
        daily::DailyRewardResult {
            disposition: 2,
            transaction_id: None
        }
        .encode()
        .map_err(debug)?,
    );
    let uses: i64 = sqlx::query_scalar("SELECT count(*) FROM game_store_reward_access_uses")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(uses, 1);
    Ok(())
}

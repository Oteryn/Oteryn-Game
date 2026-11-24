//! Real Store dispatch: character-bound earned currency, grant, replay and refusals.

use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{FakePlatform, Ledger, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex as SyncMutex};

#[test]
fn cosmetic_tokens_real_dispatch_debits_character_and_unlocks_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("cosmetic-tokens", || scenario(true, false, None))
}

#[test]
fn cosmetic_tokens_real_dispatch_coin_payment_grants_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("cosmetic-coins", || scenario(false, false, None))
}

#[test]
fn cosmetic_tokens_replay_answer_from_previous_catalogue() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("cosmetic-tokens-old-quote", || {
        scenario(true, true, None)
    })
}

#[derive(Clone, Copy)]
enum Regression {
    Abandoned,
    LegacyRefusal,
    PaidBeforeGrant,
    OfflinePaid,
    OfflineLostAnswer,
    Transfer,
    Earned,
    AccountOwnedToken,
}

fn regression(name: &str, case: Regression) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        scenario(false, false, Some(case))
    })
}

#[test]
fn coin_cosmetic_recovers_abandoned_confirmation_under_a_new_id() -> TestResult {
    regression("cosmetic-abandoned", Regression::Abandoned)
}

#[test]
fn coin_cosmetic_legacy_refusal_releases_offer_for_a_new_id() -> TestResult {
    regression("cosmetic-legacy-refusal", Regression::LegacyRefusal)
}

#[test]
fn coin_cosmetic_paid_grant_recovers_under_a_new_id() -> TestResult {
    regression("cosmetic-paid-grant", Regression::PaidBeforeGrant)
}

#[test]
fn coin_cosmetic_worker_recovers_before_game_grant() -> TestResult {
    regression("cosmetic-offline-paid", Regression::OfflinePaid)
}

#[test]
fn coin_cosmetic_worker_recovers_a_lost_platform_answer() -> TestResult {
    regression("cosmetic-offline-lost-answer", Regression::OfflineLostAnswer)
}

#[test]
fn coin_cosmetic_stays_with_the_paid_account_after_transfer() -> TestResult {
    regression("cosmetic-transfer", Regression::Transfer)
}

#[test]
fn already_earned_mount_refuses_coin_payment() -> TestResult {
    regression("cosmetic-earned", Regression::Earned)
}

#[test]
fn account_owned_coin_cosmetic_refuses_token_payment_on_another_character() -> TestResult {
    regression("cosmetic-account-owned-token", Regression::AccountOwnedToken)
}

fn scenario(tokens: bool, previous_revision: bool, regression: Option<Regression>) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "cosmetic_tokens").await?;
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
            balance: if tokens { 60 } else { 1000 },
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(FakePlatform(ledger.clone())),
            "oteryn-game/store-consumer".into(),
        ))));
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_STORE_V1, 479],
        )
        .ok_or("Store history selection")?;

        let values = serve(&owner, &mut admitted, &[(6187, vec![])]).await?;
        assert_eq!(values[0].0, CommandStatus::Accepted);
        assert_eq!(values[0].1, 0_u64.to_le_bytes());
        let pages = serve(
            &owner,
            &mut admitted,
            &[(
                183,
                wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
                    category: "cosmetics.outfits".into(),
                    page_cursor: String::new(),
                })
                .map_err(debug)?,
            )],
        )
        .await?;
        assert_eq!(pages[0].0, CommandStatus::Accepted);
        let page = wire::decode_catalogue_page(&pages[0].1).map_err(debug)?;
        let offer = page
            .offers
            .iter()
            .find(|o| o.offer_key == "store:outfit:field-surgeon")
            .ok_or("Field Surgeon")?;
        assert_eq!(offer.price, 870);
        let request = wire::StorePurchaseRequest {
            purchase_request_id: id(80),
            offer_key: offer.offer_key.clone(),
            catalog_revision: page.catalog_revision.clone(),
            price: offer.price,
            currency: if tokens {
                "COSMETIC_TOKEN"
            } else {
                "TIBIA_COIN"
            }
            .into(),
        };
        let command = |request: &wire::StorePurchaseRequest| -> TestResult<_> {
            Ok((
                if request.currency == "COSMETIC_TOKEN" {
                    6188
                } else {
                    185
                },
                wire::encode_purchase_request(request).map_err(debug)?,
            ))
        };
        if let Some(case) = regression {
            cosmetic_regression(case, &harness, &owner, &mut admitted, &ledger, request).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let earned =
            if tokens {
                let result = serve(&owner, &mut admitted, &[command(&request)?]).await?;
                assert_eq!(
                    wire::decode_purchase_result(&result[0].1)
                        .map_err(debug)?
                        .code,
                    wire::StorePurchaseCode::InsufficientBalance
                );
                let ordinary = serve(
                    &owner,
                    &mut admitted,
                    &[(
                        183,
                        wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
                            category: "consumables.exercise-weapons".into(),
                            page_cursor: String::new(),
                        })
                        .map_err(debug)?,
                    )],
                )
                .await?;
                let ordinary = wire::decode_catalogue_page(&ordinary[0].1).map_err(debug)?;
                let mut non_cosmetic = request.clone();
                non_cosmetic.offer_key = ordinary.offers[0].offer_key.clone();
                non_cosmetic.price = ordinary.offers[0].price;
                non_cosmetic.catalog_revision = ordinary.catalog_revision;
                non_cosmetic.currency = "COSMETIC_TOKEN".into();
                let result = serve(&owner, &mut admitted, &[command(&non_cosmetic)?]).await?;
                assert_eq!(
                    wire::decode_purchase_result(&result[0].1)
                        .map_err(debug)?
                        .code,
                    wire::StorePurchaseCode::OfferUnavailable
                );
                // Earn the funds through the existing daily owner, preserving every native credit guard.
                let content = crate::content::daily_reward::DailyRewardContent::catalogue()
                    .ok_or("daily content")?;
                let mut earned = 0_u64;
                // WU26-07: a Free seven-day cycle earns 5 tokens, so 870 needs 1218 claims.
                for epoch in 1..=1300_u64 {
                    super::daily_reward_fixtures::epoch(&harness, epoch).await?;
                    let fence = crate::gameplay_transport::character_gameplay_fence::current(
                        &owner,
                        admitted.game_session_id,
                        false,
                    )
                    .await
                    .ok_or("current")?;
                    owner
                .root
                .commit_daily_reward_claim(
                    owner.character,
                    owner.holder,
                    fence.item_fence(),
                    crate::durability::daily_reward_claim::DailyRewardClaimRequest {
                        command: crate::foundation::CommandRef::new(
                            admitted.game_session_id,
                            crate::foundation::CommandId::new(10_000 + epoch).map_err(debug)?,
                        ),
                        reset_epoch: epoch,
                        transaction_id: crate::operator_files::uuid_v7().map_err(debug)?,
                        request_binding: [7; 32],
                    },
                    crate::durability::daily_reward_delivery::DailyRewardDeliveryPlan {
                        remote: false,
                        content_revision: content.revision.clone(),
                    },
                    &crate::gameplay_transport::spell_premium_coordinator::DailyRewardPremium(None),
                )
                .await
                .map_err(debug)?;
                    earned = owner
                        .root
                        .read_cosmetic_token_balance(owner.character, owner.holder, fence)
                        .await
                        .map_err(debug)?;
                    if earned >= 870 {
                        break;
                    }
                }
                assert!(earned >= 870);
                earned
            } else {
                0
            };
        let before = serve(&owner, &mut admitted, &[(6187, vec![])]).await?;
        assert_eq!(
            wire::cosmetic_tokens::decode_balance(&before[0].1).map_err(debug)?,
            earned
        );
        if previous_revision {
            let mut previous = request.clone();
            let mut snapshot: serde_json::Value = serde_json::from_slice(include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/cosmetics.json"
            ))?;
            snapshot.sort_all_objects();
            previous.catalog_revision = super::store_fixtures::sha256(&serde_json::to_vec(&snapshot)?);
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                &owner, admitted.game_session_id, false,
            ).await.ok_or("current")?;
            let mut binding = wire::encode_purchase_request(&previous).map_err(debug)?;
            binding.extend(admitted.world_id.as_bytes());
            let committed = owner.root.purchase_store_cosmetic_tokens(
                owner.character, owner.holder, fence,
                crate::durability::store::PurchaseRequest {
                    purchase_request_id: previous.purchase_request_id,
                    account_id, character_id: *fence.character_id.as_bytes(), binding,
                },
                previous.offer_key.clone(), previous.catalog_revision.clone(),
                active.native_gameplay().and_then(|state| state.cosmetics()).ok_or("cosmetics")?.clone(),
            ).await.map_err(debug)?;
            assert_eq!(committed.code, wire::StorePurchaseCode::Accepted);
            let replay = serve(&owner, &mut admitted, &[command(&previous)?]).await?;
            assert_eq!(wire::decode_purchase_result(&replay[0].1).map_err(debug)?, committed);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let result = serve(&owner, &mut admitted, &[command(&request)?]).await?;
        let purchase = wire::decode_purchase_result(&result[0].1).map_err(debug)?;
        assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
        let replay = serve(&owner, &mut admitted, &[command(&request)?]).await?;
        assert_eq!(
            wire::decode_purchase_result(&replay[0].1).map_err(debug)?,
            purchase
        );
        owner.store_purchases =
            crate::gameplay_transport::StorePurchaseMode::DisabledRestoredBackup;
        let restored_replay = serve(&owner, &mut admitted, &[command(&request)?]).await?;
        assert_eq!(
            wire::decode_purchase_result(&restored_replay[0].1).map_err(debug)?,
            purchase
        );
        let mut fresh = request.clone();
        fresh.purchase_request_id = id(81);
        let disabled = serve(&owner, &mut admitted, &[command(&fresh)?]).await?;
        assert_eq!(
            wire::decode_purchase_result(&disabled[0].1)
                .map_err(debug)?
                .code,
            wire::StorePurchaseCode::PurchaseDisabled
        );
        owner.store_purchases = crate::gameplay_transport::StorePurchaseMode::Enabled;
        let after = serve(&owner, &mut admitted, &[(6187, vec![])]).await?;
        assert_eq!(
            wire::cosmetic_tokens::decode_balance(&after[0].1).map_err(debug)?,
            if tokens { earned - 870 } else { 0 }
        );
        let mut stale = request.clone();
        stale.price += 1;
        let stale = serve(&owner, &mut admitted, &[command(&stale)?]).await?;
        assert_eq!(
            wire::decode_purchase_result(&stale[0].1)
                .map_err(debug)?
                .code,
            wire::StorePurchaseCode::StaleQuote
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cosmetic_token_purchases")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(count, i64::from(tokens));
        let cosmetic = owner
            .active_generation
            .ok_or("generation")?
            .native_gameplay()
            .ok_or("native")?
            .cosmetics()
            .ok_or("cosmetics")?;
        use crate::domain::appearance::{
            AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact,
        };
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let state = owner
            .root
            .read_current_character_appearance_state(
                owner.character,
                owner.holder,
                fence,
                cosmetic.earned_grant_keys(),
            )
            .await
            .map_err(debug)?;
        let unlocks = EarnedCosmetics {
            content: cosmetic,
            facts: &state.earned,
        };
        let target = "oteryn:outfit.field_surgeon".to_owned();
        assert_eq!(unlocks.outfit(&target), UnlockFact::Compatible);
        assert_eq!(unlocks.addon(&target, 1), UnlockFact::Compatible);
        assert_eq!(unlocks.addon(&target, 2), UnlockFact::Compatible);
        let mut wrong = fence;
        wrong.character_id = crate::domain::CharacterId::from_bytes(id(99)).map_err(debug)?;
        assert!(
            owner
                .root
                .read_cosmetic_token_balance(owner.character, owner.holder, wrong)
                .await
                .is_err()
        );
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        assert!(
            owner
                .root
                .read_cosmetic_token_balance(owner.character, owner.holder, stale_fence)
                .await
                .is_err()
        );
        let mut stale_connection = fence;
        stale_connection.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        assert!(
            owner
                .root
                .read_cosmetic_token_balance(owner.character, owner.holder, stale_connection)
                .await
                .is_err()
        );
        let mut forbidden = harness.pool.begin().await?;
        assert!(
            sqlx::query("UPDATE game_character_store_cosmetics SET operation_id=NULL")
                .execute(&mut *forbidden)
                .await
                .is_err()
        );
        forbidden.rollback().await?;
        if tokens {
            let mut corrupt = harness.pool.begin().await?;
            sqlx::query("UPDATE game_character_cosmetic_token_spends SET spent=spent-1")
                .execute(&mut *corrupt)
                .await?;
            assert!(
                sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                    .execute(&mut *corrupt)
                    .await
                    .is_err()
            );
            corrupt.rollback().await?;
        }
        assert_eq!(
            ledger.lock().map_err(|_| "ledger poisoned")?.purchases,
            usize::from(!tokens)
        );
        assert_eq!(
            ledger.lock().map_err(|_| "ledger poisoned")?.debits,
            if tokens { vec![] } else { vec![870] }
        );
        if !tokens {
            ledger.lock().map_err(|_| "ledger poisoned")?.balance += 30_000;
            buy_catalogue_expansion(&owner, &mut admitted).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn cosmetic_regression(
    case: Regression,
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    mut request: wire::StorePurchaseRequest,
) -> TestResult {
    use super::store_fixtures::purchase;
    let command = |request: &wire::StorePurchaseRequest| -> TestResult<_> {
        Ok((185, wire::encode_purchase_request(request).map_err(debug)?))
    };
    let character = *admitted.item_fence.ok_or("fence")?.character_id.as_bytes();
    let account = admitted.controller.ok_or("controller")?.account_id;
    match case {
        Regression::Earned => {
            let source: serde_json::Value = serde_json::from_slice(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"
            ))?;
            let lottery = source["rules"].as_array().ok_or("rules")?.iter()
                .find(|r| r["item_id"] == 37746).ok_or("lottery")?;
            let grant = lottery["choices"].as_array().ok_or("choices")?.iter()
                .find(|r| r["feedback"].as_str().is_some_and(|text| text.contains("Armoured War Horse")))
                .and_then(|row| row["grants"][0].as_str()).ok_or("earned grant")?;
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            sqlx::query(
                "WITH receipt AS (
                   INSERT INTO game_account_appearance_unlock_receipts
                   VALUES(uuidv7(),encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,'item_action',$4)
                   RETURNING account_id,unlock_key,grant_occurrence_id
                 )
                 INSERT INTO game_account_appearance_unlocks SELECT * FROM receipt"
            ).bind(account.as_slice()).bind(grant).bind(source["provenance"].as_str())
                .bind(character.as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let key = "store:mount:armoured-war-horse";
            let offer = crate::content::store_cosmetics::offer(key).ok_or("mount offer")?;
            request.offer_key = key.into();
            request.price = offer.price;
            let result = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::OfferUnavailable);
            assert!(ledger.lock().map_err(|_| "ledger")?.debits.is_empty());
        }
        Regression::AccountOwnedToken => {
            let accepted = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            // Retain the paid account grant with a different character as its
            // historical recipient. Give the current character enough tokens.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            for sql in [
                "UPDATE game_character_store_cosmetics SET character_id=encode($1,'hex')::uuid",
                "UPDATE store_delivery_receipts SET character_id=encode($1,'hex')::uuid",
                "UPDATE store_purchase_requests SET character_id=encode($1,'hex')::uuid",
            ] {
                sqlx::query(sql).bind(id(93).as_slice()).execute(&mut *tx).await?;
            }
            sqlx::query(
                "INSERT INTO game_character_cosmetic_tokens(character_id,world_id,balance,last_claim_id)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,uuidv7())"
            ).bind(character.as_slice()).bind(admitted.world_id.as_bytes().as_slice())
                .bind(i64::try_from(request.price)?).execute(&mut *tx).await?;
            tx.commit().await?;
            request.purchase_request_id = id(81);
            request.currency = "COSMETIC_TOKEN".into();
            let result = purchase(owner, admitted, (6188, wire::encode_purchase_request(&request).map_err(debug)?)).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::OfferUnavailable);
            let spends: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cosmetic_token_purchases")
                .fetch_one(&harness.pool).await?;
            assert_eq!(spends, 0);
        }
        Regression::Transfer => {
            use crate::domain::appearance::{AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact};
            let accepted = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            let new_account = id(95);
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            for sql in [
                "UPDATE game_character_roots SET account_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid",
                "UPDATE game_durability_reconnect_sessions SET account_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid",
            ] {
                sqlx::query(sql).bind(new_account.as_slice()).bind(character.as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            let fence = crate::gameplay_transport::character_gameplay_fence::current(owner, admitted.game_session_id, false)
                .await.ok_or("new owner's fence")?;
            let content = owner.active_generation.and_then(|g| g.native_gameplay())
                .and_then(|g| g.cosmetics()).ok_or("cosmetics")?;
            let state = owner.root.read_current_character_appearance_state(
                owner.character, owner.holder, fence, content.earned_grant_keys(),
            ).await.map_err(debug)?;
            let ownership = EarnedCosmetics { content, facts: &state.earned };
            assert_eq!(ownership.outfit(&"oteryn:outfit.field_surgeon".into()), UnlockFact::Absent);
            let mut controller = admitted.controller.ok_or("controller")?;
            controller.account_id = new_account;
            admitted.controller = Some(controller);
            ledger.lock().map_err(|_| "ledger")?.balance = 1_000;
            request.purchase_request_id = id(81);
            let accepted = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
        }
        Regression::Abandoned | Regression::LegacyRefusal | Regression::PaidBeforeGrant | Regression::OfflinePaid | Regression::OfflineLostAnswer => {
            let paid = matches!(case, Regression::PaidBeforeGrant | Regression::OfflinePaid);
            let lost = matches!(case, Regression::OfflineLostAnswer);
            if lost {
                ledger.lock().map_err(|_| "ledger")?.malformed = true;
            } else if paid {
                sqlx::raw_sql(
                    "CREATE FUNCTION fail_cosmetic_grant() RETURNS trigger LANGUAGE plpgsql AS
                    $$ BEGIN RAISE EXCEPTION 'fixture interruption before cosmetic grant'; END; $$;
                    CREATE TRIGGER fail_cosmetic_grant BEFORE INSERT ON store_delivery_receipts
                    FOR EACH ROW EXECUTE FUNCTION fail_cosmetic_grant();"
                ).execute(&harness.pool).await?;
            } else {
                ledger.lock().map_err(|_| "ledger")?.fail_before_debit = true;
            }
            let first = serve(owner, admitted, &[command(&request)?]).await?;
            assert_eq!(first, [(CommandStatus::Rejected, Vec::new())]);
            if lost {
                ledger.lock().map_err(|_| "ledger")?.malformed = false;
                let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_store_purchase_history")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(rows, 0);
            } else if paid {
                assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, [request.price]);
                sqlx::query("DROP TRIGGER fail_cosmetic_grant ON store_delivery_receipts").execute(&harness.pool).await?;
            } else {
                ledger.lock().map_err(|_| "ledger")?.fail_before_debit = false;
            }
            if matches!(case, Regression::OfflinePaid | Regression::OfflineLostAnswer) {
                let ready = {
                    let mut platform = ledger.lock().map_err(|_| "ledger")?;
                    platform.advertise_deliveries = true;
                    platform.receipt_ready.clone()
                };
                let worker = crate::gameplay_transport::run_store_receipt_outbox(
                    owner.root, owner.character, StoreClient::new(Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into()),
                );
                let published = tokio::select! {
                    () = worker => false,
                    signal = tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified()) => signal.is_ok(),
                };
                assert!(published, "a disconnected buyer's paid cosmetic must be granted");
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(receipts, 1);
                assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, [request.price]);
                return Ok(());
            }
            if matches!(case, Regression::LegacyRefusal) {
                let refusal = serde_json::to_vec(&serde_json::json!({
                    "result":"INSUFFICIENT_BALANCE","order_id":null,"operation_ids":[]
                }))?;
                sqlx::query("UPDATE store_purchase_requests SET state=2,answer=$2 WHERE purchase_request_id=encode($1,'hex')::uuid")
                    .bind(request.purchase_request_id.as_slice()).bind(refusal).execute(&harness.pool).await?;
            }
            request.purchase_request_id = id(81);
            let recovered = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(recovered.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(purchase(owner, admitted, command(&request)?).await?, recovered);
            assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, [request.price]);
        }
    }
    Ok(())
}

async fn buy_catalogue_expansion(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use crate::domain::appearance::{
        AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact,
    };
    use wire::browse::{Artwork, Page, Query};

    let mut request_seed = 160;
    for (category, kind, prefix, picks, total) in [
        ("cosmetics.mounts", 3, "mount", MOUNTS, 16),
        ("cosmetics.outfits", 2, "outfit", OUTFITS, 58),
    ] {
        let mut query = Query {
            category: category.into(),
            search: String::new(),
            sort: 1,
            cursor: String::new(),
        };
        let mut products = Vec::new();
        let page_size = wire::MAX_STORE_PAGE_ENTRIES;
        for offset in (0..total).step_by(page_size) {
            let rows = serve(
                owner,
                admitted,
                &[(wire::browse::COMMAND_TYPE, query.encode().map_err(debug)?)],
            )
            .await?;
            assert_eq!(rows[0].0, CommandStatus::Accepted, "{category}");
            let page = Page::decode(&rows[0].1).map_err(debug)?;
            let expected = (total - offset).min(page_size);
            assert_eq!(page.products.len(), expected, "{category}");
            assert_eq!(
                page.next.is_empty(),
                offset + expected == total,
                "{category}"
            );
            query.cursor = page.next;
            products.extend(page.products);
        }
        assert_eq!(products.len(), total, "{category}");
        let keys: std::collections::BTreeSet<_> =
            products.iter().map(|product| &product.offer.offer_key).collect();
        assert_eq!(keys.len(), total, "{category}");
        for (slug, price, looks, _) in picks {
            let key = format!("store:{prefix}:{}", slug.replace('_', "-"));
            let product = products
                .iter()
                .find(|product| product.offer.offer_key == key)
                .ok_or_else(|| format!("missing Store card {key}"))?;
            assert_eq!(product.category, category);
            assert_eq!(product.artwork, Artwork { kind, looks: *looks }, "{key}");
            assert_eq!(product.offer.price, *price, "{key}");
            assert_eq!(product.offer.currency, "TIBIA_COIN");
            let request = wire::StorePurchaseRequest {
                purchase_request_id: id(request_seed),
                offer_key: product.offer.offer_key.clone(),
                catalog_revision: product.revision.clone(),
                price: product.offer.price,
                currency: product.offer.currency.clone(),
            };
            let result = super::store_fixtures::purchase(
                owner,
                admitted,
                (
                    wire::COMMAND_TYPE_STORE_PURCHASE,
                    wire::encode_purchase_request(&request).map_err(debug)?,
                ),
            )
            .await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted, "{key}");
            request_seed += 1;
        }
    }

    let cosmetics = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.cosmetics())
        .ok_or("cosmetics")?;
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("fence")?;
    let state = owner
        .root
        .read_current_character_appearance_state(
            owner.character,
            owner.holder,
            fence,
            cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    let unlocks = EarnedCosmetics {
        content: cosmetics,
        facts: &state.earned,
    };
    assert_eq!(
        unlocks.outfit(&"oteryn:outfit.field_surgeon".to_owned()),
        UnlockFact::Compatible
    );
    for (slug, _, _, _) in MOUNTS {
        let target = format!("oteryn:content.mount.{slug}");
        assert_eq!(unlocks.mount(&target), UnlockFact::Compatible, "{target}");
    }
    for (slug, _, _, addons) in OUTFITS {
        let target = format!("oteryn:outfit.{slug}");
        assert_eq!(unlocks.outfit(&target), UnlockFact::Compatible, "{target}");
        for addon in 1..=*addons {
            assert_eq!(
                unlocks.addon(&target, addon),
                UnlockFact::Compatible,
                "{target} addon {addon}"
            );
        }
        assert_eq!(
            unlocks.addon(&target, addons + 1),
            UnlockFact::Incompatible,
            "{target}"
        );
    }
    Ok(())
}

// Crystal 201189f Store rows and prices, with TibiaWiki BR prices for the five unlisted mounts.
// Each tuple pins the cosmetic slug, Tibia Coin price, male/female artwork and addon count.
const MOUNTS: &[(&str, u64, [u32; 2], u8)] = &[
    ("armoured_war_horse", 870, [426, 426], 0),
    ("battle_werewolf", 870, [1873, 1873], 0),
    ("boisterous_bull", 690, [1672, 1672], 0),
    ("crimson_ray", 870, [521, 521], 0),
    ("death_crawler", 600, [624, 624], 0),
    ("desert_king", 450, [572, 572], 0),
    ("flamesteed", 900, [626, 626], 0),
    ("jade_lion", 450, [627, 627], 0),
    ("jade_pincer", 600, [628, 628], 0),
    ("magma_skull", 750, [1686, 1686], 0),
    ("nethersteed", 900, [629, 629], 0),
    ("obstinate_ox", 690, [1674, 1674], 0),
    ("shadow_draptor", 870, [427, 427], 0),
    ("steelbeak", 870, [522, 522], 0),
    ("surly_steer", 690, [1673, 1673], 0),
    ("tombstinger", 600, [546, 546], 0),
];

const OUTFITS: &[(&str, u64, [u32; 2], u8)] = &[
    ("arena_champion", 870, [884, 885], 2),
    ("evoker", 840, [725, 724], 2),
    ("grove_keeper", 870, [908, 909], 2),
    ("lion_of_war", 1750, [1206, 1207], 2),
    ("lupine_warden", 840, [899, 900], 2),
    ("pharaoh", 750, [955, 956], 2),
    ("philosopher", 750, [873, 874], 2),
    ("retro_nobleman", 870, [966, 967], 0),
    ("retro_summoner", 870, [964, 965], 0),
    ("retro_warrior", 870, [962, 963], 0),
    ("royal_pumpkin", 840, [760, 759], 2),
    ("sea_dog", 600, [750, 749], 2),
    ("seaweaver", 570, [733, 732], 2),
    ("spirit_caller", 600, [699, 698], 2),
    ("trophy_hunter", 870, [957, 958], 2),
    ("winter_warden", 870, [853, 852], 2),
];

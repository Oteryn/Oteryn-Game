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

const FIELD_SURGEON: &str = "store:outfit:field-surgeon";
const FORMER_OWNER: u8 = 40;
const NEW_OWNER: u8 = 95;

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
    OfflineLockOrder,
    CoinRetryLockOrder,
    TokenRetryLockOrder,
    OfflineLostAnswer,
    LegacyItem { worker: bool },
    Inbox,
    RestoreReservations(restore_reservations::Snapshot),
    Transfer,
    Earned,
    AccountOwnedToken,
    SiblingPending { tokens: bool },
    BlessingImport,
}

fn regression(name: &str, case: Regression) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        scenario(false, false, Some(case))
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED {name}");
    Ok(())
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
fn coin_cosmetic_recovery_locks_the_buyer_before_its_grant() -> TestResult {
    regression("cosmetic-recovery-lock-order", Regression::OfflineLockOrder)
}

#[test]
fn coin_cosmetic_reservation_leaves_buyer_unlocked_while_answer_writer_holds_purchase() -> TestResult {
    regression("coin-cosmetic-lock-order", Regression::CoinRetryLockOrder)
}

#[test]
fn token_cosmetic_purchase_leaves_buyer_unlocked_while_answer_writer_holds_purchase() -> TestResult {
    regression("token-cosmetic-lock-order", Regression::TokenRetryLockOrder)
}

#[test]
fn coin_cosmetic_worker_recovers_a_lost_platform_answer() -> TestResult {
    regression("cosmetic-offline-lost-answer", Regression::OfflineLostAnswer)
}

#[test]
fn store_review_legacy_item_replays_through_confirmation() -> TestResult {
    regression("cosmetic-legacy-item-confirmation", Regression::LegacyItem { worker: false })
}

#[test]
fn store_review_legacy_item_replays_through_worker() -> TestResult {
    regression("cosmetic-legacy-item-worker", Regression::LegacyItem { worker: true })
}

#[test]
fn store_review_inbox_projects_entitlement_independent_of_delivery_and_claim() -> TestResult {
    regression("store-inbox-entitlement", Regression::Inbox)
}

#[test]
fn store_review_restore_conflict_retires_accepted_reservations() -> TestResult {
    regression("store-restore-reservations", Regression::RestoreReservations(restore_reservations::Snapshot::Answered))
}

#[test]
fn store_restore_releases_reservations_after_the_original_answer_is_retained() -> TestResult {
    regression("store-restore-sent", Regression::RestoreReservations(restore_reservations::Snapshot::Sent))
}

#[test]
fn store_restore_backfills_reservations_for_existing_delivery_fences() -> TestResult {
    regression("store-restore-upgrade", Regression::RestoreReservations(restore_reservations::Snapshot::Fenced))
}

#[path = "cosmetic_tokens/restore_reservations.rs"]
mod restore_reservations;

#[test]
fn coin_cosmetic_stays_with_the_paid_account_after_transfer() -> TestResult {
    regression("cosmetic-transfer", Regression::Transfer)
}

#[test]
fn already_earned_mount_refuses_coin_payment() -> TestResult {
    regression("cosmetic-earned", Regression::Earned)
}

#[test]
fn sibling_completed_coin_cosmetic_allows_token_payment() -> TestResult {
    regression("cosmetic-account-owned-token", Regression::AccountOwnedToken)
}

#[test]
fn sibling_pending_coin_cosmetic_allows_a_separate_coin_purchase() -> TestResult {
    regression("cosmetic-sibling-coins", Regression::SiblingPending { tokens: false })
}

#[test]
fn sibling_pending_coin_cosmetic_allows_a_separate_token_purchase() -> TestResult {
    regression("cosmetic-sibling-tokens", Regression::SiblingPending { tokens: true })
}

#[test]
fn store_blessing_page_quotes_buy_each_imported_blessing() -> TestResult {
    regression("store-blessing-import", Regression::BlessingImport)
}

fn scenario(tokens: bool, previous_revision: bool, regression: Option<Regression>) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "cosmetic_tokens").await?;
        let ledger = Arc::new(SyncMutex::new(Ledger {
            balance: if tokens { 60 } else { 1000 },
            world: uuid(&id(42)),
            ..Ledger::default()
        }));
        let mut platform = StoreClient::new(
            Box::new(FakePlatform(ledger.clone())),
            "oteryn-game/store-consumer".into(),
        );
        if matches!(regression, Some(Regression::Transfer)) {
            prepare_transferred_cosmetic_session(&harness, &mut platform).await?;
        } else {
            prepare_current_session(&harness).await?;
        }
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, pin) =
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
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(platform)));
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_STORE_V1, 479],
        )
        .ok_or("Store history selection")?;

        if matches!(regression, Some(Regression::BlessingImport)) {
            blessing_import(&harness, &owner, &mut admitted, &ledger).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

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
            .find(|o| o.offer_key == FIELD_SURGEON)
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
        if let Some(Regression::RestoreReservations(snapshot)) = regression {
            let late = restore_reservations::prepare(&harness, &owner, &mut admitted, &ledger, &request, snapshot).await?;
            let platform = owner.store.clone();
            let states = owner.states.clone();
            drop(owner);
            drop(authority);
            let predecessor = seal.record().recovery_generation;
            let issued_at = seal.record().issued_at.checked_add(1).ok_or("issued at")?;
            drop(seal);
            restore_reservations::reconcile(&harness, &ledger, predecessor, issued_at, snapshot, late).await?;
            if matches!(snapshot, restore_reservations::Snapshot::Sent | restore_reservations::Snapshot::Fenced) {
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut restored = deps.owner(&harness, &authority);
                restored.active_generation = Some(active);
                restored.store = platform;
                restored.states = states;
                restore_reservations::buy_again(&harness, &restored, &mut admitted, request).await?;
            }
            return harness.cleanup().await;
        }
        if let Some(case) = regression {
            cosmetic_regression(case, &harness, &owner, &mut admitted, &ledger, request, pin.frame_binding().digest()).await?;
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

// Model the old owner's already-paid, offline delivery. The character
// transfers while offline, then admits for the first time under the new owner.
async fn prepare_transferred_cosmetic_session(
    h: &Harness,
    platform: &mut StoreClient,
) -> TestResult {
    use crate::durability::store::{
        PaidStoreCosmeticIntent, PurchaseRequest, PurchaseRequestState, StorePurchaseHistoryEntry,
    };
    use crate::native_admission_source::store::{
        OperationKind, PurchaseResult, PurchaseStoreOffer,
    };
    use oteryn_gameplay_economy::store as rules;

    let account = id(FORMER_OWNER);
    let character = id(41);
    let offer = crate::content::store_cosmetics::offer(FIELD_SURGEON).ok_or("cosmetic offer")?;
    let revision = crate::gameplay_transport::store::active_catalogue_revision();
    let purchase_id = id(81);
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(purchase_id).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: crate::domain::CharacterId::from_bytes(character).map_err(debug)?,
        offer_key: offer.key.clone(),
        catalog_revision: revision.into(),
        price: offer.price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let journal = PurchaseRequest {
        purchase_request_id: purchase_id,
        account_id: account,
        character_id: character,
        binding: binding.binding_digest().to_vec(),
    };
    assert_eq!(
        h.root
            .record_purchase_request(journal.clone())
            .await
            .map_err(debug)?,
        PurchaseRequestState::Sent
    );
    sqlx::query(
        "INSERT INTO game_character_store_cosmetics(character_id, world_id, offer_key, purchase_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(sqlx::types::Uuid::from_bytes(character))
    .bind(sqlx::types::Uuid::from_bytes(id(42)))
    .bind(&offer.key)
    .bind(sqlx::types::Uuid::from_bytes(purchase_id))
    .execute(&h.pool)
    .await?;
    let answer = platform
        .purchase(
            &PurchaseStoreOffer {
                purchase_request_id: uuid(&purchase_id),
                account_id: uuid(&account),
                character_id: uuid(&character),
                world_id: uuid(&id(42)),
                offer_key: offer.key.clone(),
                catalog_revision: revision.into(),
                unit_price: offer.price.to_string(),
                quantity: 1,
                currency: "COINS".into(),
            },
            "COSMETIC",
        )
        .await
        .map_err(debug)?;
    let PurchaseResult::Accepted {
        order_id,
        operation_ids,
    } = answer
    else {
        return Err(format!("former owner's Store purchase: {answer:?}").into());
    };
    let [operation] = operation_ids.as_slice() else {
        return Err("expected one cosmetic operation".into());
    };
    let intent = platform
        .read_intent(OperationKind::Deliver, operation)
        .await
        .map_err(debug)?;
    let order = *sqlx::types::Uuid::parse_str(&order_id)?.as_bytes();
    h.root
        .record_store_answer_with_history(
            journal,
            serde_json::to_vec(&serde_json::json!({
                "result": "ACCEPTED", "order_id": order_id, "operation_ids": operation_ids,
            }))?,
            Some(StorePurchaseHistoryEntry {
                order_id: order,
                offer_key: offer.key.clone(),
                catalog_revision: revision.into(),
                price: offer.price,
                currency: "TIBIA_COIN".into(),
            }),
        )
        .await
        .map_err(debug)?;

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert!(
        h.root
            .recover_paid_store_cosmetic(
                &authority,
                PaidStoreCosmeticIntent {
                    operation: *sqlx::types::Uuid::parse_str(operation)?.as_bytes(),
                    account,
                    order,
                    fulfilment: intent.entitlement_id,
                    offer: offer.key.clone(),
                    catalogue: revision.into(),
                },
            )
            .await
            .map_err(debug)?
    );
    transfer_cosmetic_character(h, &authority, character).await?;
    drop(authority);
    drop(seal);
    super::fresh_session_fixtures::prepare_current_session_for_account(h, id(NEW_OWNER)).await?;
    Ok(())
}

async fn transfer_cosmetic_character(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    character: [u8; 16],
) -> TestResult {
    use crate::native_admission_source::character_commands::{Decision, Input};
    let source = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    let new_account = uuid(&id(NEW_OWNER));
    source
        .evidence
        .refresh_account(&h.root, &h.node, &new_account)
        .await
        .map_err(debug)?;
    let now = super::chat_world_fixture::clock(h).await?;
    let operation = uuid(&id(85));
    // The command decoder checks this canonical field order byte for byte.
    let intent = format!(
        concat!(
            r#"{{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","#,
            r#""issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"85","#,
            r#""operation_id":"{}","character_id":"{}","expected_from_account_id":"{}","to_account_id":"{}","#,
            r#""purpose":"BAZAAR_LISTING_ESCROW","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,
        ),
        uuid(&id(185)),
        operation,
        uuid(&character),
        uuid(&id(FORMER_OWNER)),
        new_account,
        now,
        now + 300,
    );
    let outcome = h
        .root
        .decide_character_command(
            authority,
            &h.node,
            "oteryn:character-authority:primary",
            "TransferCharacterOwnership",
            &operation,
            Input::Intent(intent.as_bytes()),
        )
        .await;
    assert!(
        matches!(&outcome, Decision::Publish { result_code, .. } if result_code == "CHAR_CMD_OK"),
        "offline Character transfer: {outcome:?}"
    );
    Ok(())
}

async fn blessing_import(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
) -> TestResult {
    ledger.lock().map_err(|_| "ledger")?.enforce_active_import = true;
    let replies = serve(
        owner,
        admitted,
        &[(
            wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
            wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
                category: "consumables.blessings".into(),
                page_cursor: String::new(),
            })
            .map_err(debug)?,
        )],
    )
    .await?;
    assert_eq!(replies[0].0, CommandStatus::Accepted);
    let page = wire::decode_catalogue_page(&replies[0].1).map_err(debug)?;
    assert!(page.next_page_cursor.is_empty());
    let catalogue = crate::content::blessing_catalogue::BlessingCatalogue::load().map_err(debug)?;
    assert_eq!(page.offers.len(), catalogue.blessings.len());
    for (index, blessing) in catalogue.blessings.iter().enumerate() {
        let suffix = blessing
            .storage_key
            .strip_prefix("oteryn:blessing.")
            .ok_or("blessing key")?;
        let key = format!("store:blessing:{suffix}");
        let offer = page
            .offers
            .iter()
            .find(|offer| offer.offer_key == key)
            .ok_or("blessing offer")?;
        let request = wire::StorePurchaseRequest {
            purchase_request_id: id(80 + u8::try_from(index)?),
            offer_key: offer.offer_key.clone(),
            catalog_revision: page.catalog_revision.clone(),
            price: offer.price,
            currency: offer.currency.clone(),
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
        let copies: i64 = sqlx::query_scalar(
            "SELECT copies FROM game_character_store_blessing_copies
             WHERE character_id=$1 AND blessing_key=$2",
        )
        .bind(sqlx::types::Uuid::from_bytes(
            *admitted.item_fence.ok_or("fence")?.character_id.as_bytes(),
        ))
        .bind(&blessing.storage_key)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(copies, 1, "{key}");
    }
    assert_eq!(
        ledger.lock().map_err(|_| "ledger")?.debits.len(),
        catalogue.blessings.len()
    );
    Ok(())
}

async fn seed_sibling(harness: &Harness, character: [u8; 16]) -> TestResult<sqlx::types::Uuid> {
    let sibling = sqlx::types::Uuid::from_bytes(id(93));
    sqlx::query(
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,
             character_revision,profile_revision,ruleset_revision,content_revision,
             starter_template_revision,name,sex)
         SELECT $1,account_id,world_id,lifecycle,character_revision,profile_revision,
             ruleset_revision,content_revision,starter_template_revision,'Store Sibling',sex
           FROM game_character_roots WHERE character_id=$2",
    )
    .bind(sibling)
    .bind(sqlx::types::Uuid::from_bytes(character))
    .execute(&harness.pool)
    .await?;
    Ok(sibling)
}

fn token_command(request: &wire::StorePurchaseRequest) -> TestResult<(u32, Vec<u8>)> {
    Ok((wire::cosmetic_tokens::PURCHASE, wire::encode_purchase_request(request).map_err(debug)?))
}

async fn fund_tokens(harness: &Harness, admitted: &AdmittedSession, price: u64) -> TestResult {
    let character = *admitted.item_fence.ok_or("fence")?.character_id.as_bytes();
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_cosmetic_tokens(character_id,world_id,balance,last_claim_id)
         VALUES($1,$2,$3,uuidv7())",
    )
    .bind(sqlx::types::Uuid::from_bytes(character))
    .bind(sqlx::types::Uuid::from_bytes(*admitted.world_id.as_bytes()))
    .bind(i64::try_from(price)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn assert_token_grant(
    harness: &Harness,
    character: [u8; 16],
    request: &wire::StorePurchaseRequest,
) -> TestResult {
    let spends: Vec<(sqlx::types::Uuid, i64)> =
        sqlx::query_as("SELECT character_id, price FROM game_cosmetic_token_purchases")
            .fetch_all(&harness.pool)
            .await?;
    assert_eq!(
        spends,
        [(
            sqlx::types::Uuid::from_bytes(character),
            i64::try_from(request.price)?
        )]
    );
    assert!(has_cosmetic_grant(harness, character, request.purchase_request_id).await?);
    Ok(())
}

async fn has_cosmetic_grant(
    harness: &Harness,
    character: [u8; 16],
    purchase: [u8; 16],
) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_store_cosmetics
         WHERE character_id=$1 AND purchase_id=$2 AND operation_id IS NOT NULL)",
    )
    .bind(sqlx::types::Uuid::from_bytes(character))
    .bind(sqlx::types::Uuid::from_bytes(purchase))
    .fetch_one(&harness.pool)
    .await?)
}

async fn sibling_pending(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    mut request: wire::StorePurchaseRequest,
    tokens: bool,
) -> TestResult {
    use super::store_fixtures::purchase;
    use crate::durability::store::{
        PaidStoreCosmeticIntent, PurchaseRequest, PurchaseRequestState, StorePurchaseHistoryEntry,
    };
    use crate::native_admission_source::store::{
        OperationKind, PurchaseResult, PurchaseStoreOffer,
    };
    use oteryn_gameplay_economy::store as rules;

    let character = *admitted.item_fence.ok_or("fence")?.character_id.as_bytes();
    let account = admitted.controller.ok_or("controller")?.account_id;
    let sibling = seed_sibling(harness, character).await?;
    let original = request.purchase_request_id;
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(original).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: crate::domain::CharacterId::from_bytes(*sibling.as_bytes()).map_err(debug)?,
        offer_key: request.offer_key.clone(),
        catalog_revision: request.catalog_revision.clone(),
        price: request.price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let journal = PurchaseRequest {
        purchase_request_id: original,
        account_id: account,
        character_id: *sibling.as_bytes(),
        binding: binding.binding_digest().into(),
    };
    assert_eq!(
        owner
            .root
            .record_purchase_request(journal.clone())
            .await
            .map_err(debug)?,
        PurchaseRequestState::Sent
    );
    sqlx::query(
        "INSERT INTO game_character_store_cosmetics(character_id,world_id,offer_key,purchase_id)
         VALUES($1,$2,$3,$4)",
    )
    .bind(sibling)
    .bind(sqlx::types::Uuid::from_bytes(*admitted.world_id.as_bytes()))
    .bind(&request.offer_key)
    .bind(sqlx::types::Uuid::from_bytes(original))
    .execute(&harness.pool)
    .await?;
    ledger.lock().map_err(|_| "ledger")?.balance = request.price * 2;
    let mut platform = StoreClient::new(
        Box::new(FakePlatform(ledger.clone())),
        "oteryn-game/store-consumer".into(),
    );
    let answer = platform
        .purchase(
            &PurchaseStoreOffer {
                purchase_request_id: uuid(&original),
                account_id: uuid(&account),
                character_id: sibling.to_string(),
                world_id: uuid(admitted.world_id.as_bytes()),
                catalog_revision: request.catalog_revision.clone(),
                offer_key: request.offer_key.clone(),
                unit_price: request.price.to_string(),
                quantity: 1,
                currency: "COINS".into(),
            },
            "COSMETIC",
        )
        .await
        .map_err(debug)?;
    let PurchaseResult::Accepted {
        order_id,
        operation_ids,
    } = answer
    else {
        return Err(format!("sibling debit: {answer:?}").into());
    };
    let [operation] = operation_ids.as_slice() else {
        return Err("expected one cosmetic operation".into());
    };
    let order = *sqlx::types::Uuid::parse_str(&order_id)?.as_bytes();
    owner
        .root
        .record_store_answer_with_history(
            journal,
            serde_json::to_vec(&serde_json::json!({
                "result": "ACCEPTED", "order_id": order_id, "operation_ids": operation_ids,
            }))?,
            Some(StorePurchaseHistoryEntry {
                order_id: order,
                offer_key: request.offer_key.clone(),
                catalog_revision: request.catalog_revision.clone(),
                price: request.price,
                currency: "TIBIA_COIN".into(),
            }),
        )
        .await
        .map_err(debug)?;

    request.purchase_request_id = id(81);
    let command = if tokens {
        fund_tokens(harness, admitted, request.price).await?;
        request.currency = "COSMETIC_TOKEN".into();
        token_command(&request)?
    } else {
        (
            wire::COMMAND_TYPE_STORE_PURCHASE,
            wire::encode_purchase_request(&request).map_err(debug)?,
        )
    };
    let result = purchase(owner, admitted, command.clone()).await?;
    assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
    if tokens {
        assert_token_grant(harness, character, &request).await?;
    } else {
        assert!(
            has_cosmetic_grant(harness, character, request.purchase_request_id).await?,
            "a fresh coin confirmation must grant the cosmetic to its buyer"
        );
    }
    assert!(!has_cosmetic_grant(harness, *sibling.as_bytes(), original).await?);
    assert_eq!(purchase(owner, admitted, command).await?, result);

    let intent = platform
        .read_intent(OperationKind::Deliver, operation)
        .await
        .map_err(debug)?;
    assert!(
        owner
            .root
            .recover_paid_store_cosmetic(
                owner.character,
                PaidStoreCosmeticIntent {
                    operation: *sqlx::types::Uuid::parse_str(operation)?.as_bytes(),
                    account,
                    order,
                    fulfilment: intent.entitlement_id,
                    offer: request.offer_key,
                    catalogue: request.catalog_revision,
                }
            )
            .await
            .map_err(debug)?
    );
    assert!(has_cosmetic_grant(harness, *sibling.as_bytes(), original).await?);
    let debits = if tokens {
        vec![request.price]
    } else {
        vec![request.price; 2]
    };
    assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, debits);
    Ok(())
}

async fn cosmetic_regression(
    case: Regression,
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    mut request: wire::StorePurchaseRequest,
    frame_digest: [u8; 32],
) -> TestResult {
    use super::store_fixtures::purchase;
    let command = |request: &wire::StorePurchaseRequest| -> TestResult<_> {
        Ok((185, wire::encode_purchase_request(request).map_err(debug)?))
    };
    let character = *admitted.item_fence.ok_or("fence")?.character_id.as_bytes();
    let account = admitted.controller.ok_or("controller")?.account_id;
    match case {
        Regression::RestoreReservations(_) => return Err("restore needs the recovery transition fixture".into()),
        Regression::BlessingImport => return Err("blessing import requires its own catalogue page".into()),
        Regression::SiblingPending { tokens } => {
            sibling_pending(harness, owner, admitted, ledger, request, tokens).await?;
        }
        Regression::CoinRetryLockOrder | Regression::TokenRetryLockOrder => {
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                owner, admitted.game_session_id, false,
            ).await.ok_or("current fence")?;
            let content = owner.active_generation
                .and_then(|generation| generation.native_gameplay())
                .and_then(|native| native.cosmetics()).ok_or("cosmetics content")?.clone();
            let journal = crate::durability::store::PurchaseRequest {
                purchase_request_id: request.purchase_request_id,
                account_id: account,
                character_id: character,
                binding: vec![1; 32],
            };
            let purchase_id = journal.purchase_request_id;
            if matches!(case, Regression::CoinRetryLockOrder) {
                // The retry reaches this owner after its initial journal transaction.
                owner.root.record_purchase_request(journal.clone()).await.map_err(debug)?;
                let reserved = super::store_fixtures::assert_store_retry_lock_order(
                    &harness.pool, purchase_id, character,
                    owner.root.reserve_store_cosmetic(owner.character, owner.holder, fence, journal, request.offer_key, content),
                ).await?.map_err(debug)?;
                assert!(reserved);
            } else {
                let refused = super::store_fixtures::assert_store_retry_lock_order(
                    &harness.pool, purchase_id, character,
                    owner.root.purchase_store_cosmetic_tokens(
                        owner.character, owner.holder, fence, journal, request.offer_key,
                        request.catalog_revision, content,
                    ),
                ).await?.map_err(debug)?;
                assert_eq!(refused.code, wire::StorePurchaseCode::InsufficientBalance);
            }
        }
        Regression::Inbox => {
            let entitlement = id(201);
            {
                let mut platform = ledger.lock().map_err(|_| "ledger")?;
                platform.advertise_claims = true;
                platform.entitlement_override = Some(uuid(&entitlement));
            }
            let paid = purchase(owner, admitted, super::store_fixtures::purchase_request(
                81, "store:exercise-sword", 25,
            )?).await?;
            assert_eq!(paid.code, wire::StorePurchaseCode::Accepted);
            let delivery = paid.operation_id.ok_or("delivery")?;
            let claim = id(113);
            assert_ne!(delivery, entitlement);
            assert_ne!(claim, entitlement);
            assert_ne!(claim, delivery);
            let query = (wire::COMMAND_TYPE_STORE_INBOX_CLAIM, vec![16, 1]);
            let replies = serve(owner, admitted, &[query]).await?;
            assert_eq!(replies[0].0, CommandStatus::Accepted, "the Store Inbox must project the buyer's claim key");
            let mut expected = vec![10, 22, 10, 16];
            expected.extend_from_slice(&entitlement);
            expected.extend([16, 0x88, 0xdf, 0x01]); // Exercise Sword (28552).
            assert_eq!(replies[0].1, expected);
            super::item_look_fixtures::seed_main_backpack(harness, &[]).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
                .bind(admitted.world_id.as_bytes().as_slice())
                .bind(admitted.channel_id.as_bytes().as_slice())
                .execute(&harness.pool).await?;
            let generation = owner.active_generation.ok_or("native generation")?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(admitted.world_id.as_bytes().as_slice())
                .bind(admitted.channel_id.as_bytes().as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(frame_digest.as_slice())
                .execute(&harness.pool).await?;
            let claim_request = wire::StoreInboxClaimRequest { fulfilment_unit_id: uuid(&entitlement) };
            let claimed = serve(owner, admitted, &[(
                wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
                wire::encode_inbox_claim_request(&claim_request).map_err(debug)?,
            )]).await?;
            assert_eq!(claimed[0].0, CommandStatus::Accepted);
            let receipt = wire::decode_claim_receipt(&claimed[0].1).map_err(debug)?;
            assert_eq!(receipt.operation_id, claim);
            assert_eq!(receipt.fulfilment_unit_id, claim_request.fulfilment_unit_id);
            assert_eq!(receipt.result, wire::StoreClaimCode::Committed);
            let empty = serve(owner, admitted, &[
                (wire::COMMAND_TYPE_STORE_INBOX_CLAIM, vec![16, 1]),
            ]).await?;
            assert_eq!(empty, [(CommandStatus::Accepted, Vec::new())]);
        }
        Regression::LegacyItem { worker } => {
            use crate::durability::store::{PurchaseRequest, PurchaseRequestState};
            use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer};
            use oteryn_gameplay_economy::store as rules;

            let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/cosmetics.json"
            ))?;
            let legacy = document["offers"].as_array().ok_or("legacy offers")?
                .iter().find(|offer| offer["key"] == FIELD_SURGEON).ok_or("legacy Field Surgeon")?;
            let price = legacy["price"].as_u64().ok_or("legacy price")?;
            document.sort_all_objects();
            let revision = super::store_fixtures::sha256(&serde_json::to_vec(&document)?);
            let binding = rules::PurchaseStoreOffer {
                purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id).map_err(debug)?,
                account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
                character_id: crate::domain::CharacterId::from_bytes(character).map_err(debug)?,
                offer_key: request.offer_key.clone(),
                catalog_revision: revision.clone(),
                price,
                currency: rules::CurrencyKind::TibiaCoin,
            };
            let journal = PurchaseRequest {
                purchase_request_id: request.purchase_request_id,
                account_id: account,
                character_id: character,
                binding: binding.binding_digest().to_vec(),
            };
            assert_eq!(owner.root.record_purchase_request(journal.clone()).await.map_err(debug)?, PurchaseRequestState::Sent);
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                owner, admitted.game_session_id, false,
            ).await.ok_or("gameplay fence")?;
            let cosmetics = owner.active_generation.and_then(|g| g.native_gameplay())
                .and_then(|g| g.cosmetics()).ok_or("cosmetics")?;
            assert!(owner.root.reserve_store_cosmetic(
                owner.character, owner.holder, fence, journal, request.offer_key.clone(), cosmetics.clone(),
            ).await.map_err(debug)?);

            let mut client = StoreClient::new(
                Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into(),
            );
            {
                let mut platform = ledger.lock().map_err(|_| "ledger")?;
                platform.unit_kind_override = Some("ITEM");
                platform.advertise_deliveries = true;
            }
            let answer = client.purchase(&PurchaseStoreOffer {
                purchase_request_id: uuid(&request.purchase_request_id),
                account_id: uuid(&account),
                character_id: uuid(&character),
                world_id: uuid(admitted.world_id.as_bytes()),
                catalog_revision: revision,
                offer_key: request.offer_key.clone(),
                unit_price: price.to_string(),
                quantity: 1,
                currency: "COINS".into(),
            }, "ITEM").await.map_err(debug)?;
            assert!(matches!(answer, PurchaseResult::Accepted { .. }));
            // The legacy Game node lost the reply after Platform committed the debit.
            if worker {
                let ready = ledger.lock().map_err(|_| "ledger")?.receipt_ready.clone();
                tokio::select! {
                    () = crate::gameplay_transport::run_store_receipt_outbox(owner.root, owner.character, client) => {
                        return Err("Store worker stopped before publishing the legacy ITEM grant".into());
                    }
                    result = tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified()) => {
                        result.map_err(|_| "Store worker did not publish the legacy ITEM grant")?;
                    }
                }
            } else {
                request.purchase_request_id = id(81);
                let accepted = purchase(owner, admitted, command(&request)?).await?;
                assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            }
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
                .fetch_one(&harness.pool).await?;
            assert_eq!(receipts, 1);
            assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, [price]);
        }
        Regression::Earned => {
            use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
            let key = "store:mount:armoured-war-horse";
            let offer = crate::content::store_cosmetics::offer(key).ok_or("mount offer")?;
            let content = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|g| g.cosmetics())
                .ok_or("cosmetics")?;
            let source: serde_json::Value = serde_json::from_slice(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"
            ))?;
            let lottery = source["rules"]
                .as_array()
                .ok_or("rules")?
                .iter()
                .find(|r| r["item_id"] == 37746)
                .ok_or("lottery")?;
            let provenance = source["provenance"]
                .as_str()
                .ok_or("item grant provenance")?;
            // Crystal's lottery feedback uses names from a different ID table.
            // Resolve its awarded mount through the pinned grant mapping.
            let grant = lottery["choices"]
                .as_array()
                .ok_or("choices")?
                .iter()
                .filter_map(|row| row["grants"][0].as_str())
                .find(|grant| {
                    content.earned_unlock(
                        grant,
                        provenance,
                        EarnedCosmetic::Mount(&offer.unlock_key),
                    ) == UnlockFact::Compatible
                })
                .ok_or("lottery grant for the Store mount")?;
            assert_eq!(grant, "oteryn:item-grant.mount.23");
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "WITH receipt AS (
                   INSERT INTO game_account_appearance_unlock_receipts
                   VALUES(uuidv7(),encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,'item_action',$4)
                   RETURNING account_id,unlock_key,grant_occurrence_id
                 )
                 INSERT INTO game_account_appearance_unlocks SELECT * FROM receipt"
            ).bind(account.as_slice()).bind(grant).bind(provenance)
                .bind(character.as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            request.offer_key = key.into();
            request.price = offer.price;
            let result = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::OfferUnavailable);
            let ledger = ledger.lock().map_err(|_| "ledger")?;
            assert_eq!(ledger.purchases, 0);
            assert!(ledger.debits.is_empty());
        }
        Regression::AccountOwnedToken => {
            let accepted = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            fund_tokens(harness, admitted, request.price).await?;
            request.purchase_request_id = id(81);
            request.currency = "COSMETIC_TOKEN".into();
            let duplicate = purchase(owner, admitted, token_command(&request)?).await?;
            assert_eq!(duplicate.code, wire::StorePurchaseCode::OfferUnavailable);

            // Cosmetic tokens review P1: coin ownership follows the purchasing
            // character, so a sibling with funds can buy the same offer.
            let sibling = seed_sibling(harness, character).await?;
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            for sql in [
                "UPDATE game_character_store_cosmetics SET character_id=$1",
                "UPDATE store_delivery_receipts SET character_id=$1",
                "UPDATE store_purchase_requests SET character_id=$1",
                "UPDATE game_store_purchase_history SET character_id=$1",
            ] {
                sqlx::query(sql).bind(sibling).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            request.purchase_request_id = id(82);
            let result = purchase(owner, admitted, token_command(&request)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            assert_token_grant(harness, character, &request).await?;
        }
        Regression::Transfer => {
            use crate::domain::appearance::{
                AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact,
            };
            assert_eq!(account, id(NEW_OWNER));
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                owner,
                admitted.game_session_id,
                false,
            )
            .await
            .ok_or("new owner's fence")?;
            let content = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|g| g.cosmetics())
                .ok_or("cosmetics")?;
            let state = owner
                .root
                .read_current_character_appearance_state(
                    owner.character,
                    owner.holder,
                    fence,
                    content.earned_grant_keys(),
                )
                .await
                .map_err(debug)?;
            let ownership = EarnedCosmetics {
                content,
                facts: &state.earned,
            };
            assert_eq!(
                ownership.outfit(&"oteryn:outfit.field_surgeon".into()),
                UnlockFact::Absent
            );
            ledger.lock().map_err(|_| "ledger")?.balance = 1_000;
            let accepted = purchase(owner, admitted, command(&request)?).await?;
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            let owners: Vec<sqlx::types::Uuid> = sqlx::query_scalar(
                "SELECT account_id FROM store_delivery_receipts
                 WHERE character_id = $1 AND entitlement_key = $2 ORDER BY account_id",
            )
            .bind(sqlx::types::Uuid::from_bytes(character))
            .bind(format!("store-cosmetic:{FIELD_SURGEON}"))
            .fetch_all(&harness.pool)
            .await?;
            assert_eq!(
                owners,
                [FORMER_OWNER, NEW_OWNER].map(|tag| sqlx::types::Uuid::from_bytes(id(tag)))
            );
            assert_eq!(
                ledger.lock().map_err(|_| "ledger")?.debits,
                [request.price; 2]
            );
        }
        Regression::Abandoned | Regression::LegacyRefusal | Regression::PaidBeforeGrant | Regression::OfflinePaid | Regression::OfflineLostAnswer | Regression::OfflineLockOrder => {
            let paid = matches!(case, Regression::PaidBeforeGrant | Regression::OfflinePaid | Regression::OfflineLockOrder);
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
            if matches!(case, Regression::OfflineLockOrder) {
                sqlx::raw_sql(
                    "CREATE FUNCTION pause_cosmetic_grant() RETURNS trigger LANGUAGE plpgsql AS
                     $$ BEGIN PERFORM pg_advisory_xact_lock(3485,1); RETURN NEW; END $$;
                     CREATE TRIGGER pause_cosmetic_grant BEFORE UPDATE ON game_character_store_cosmetics
                     FOR EACH ROW WHEN (OLD.operation_id IS NULL AND NEW.operation_id IS NOT NULL)
                     EXECUTE FUNCTION pause_cosmetic_grant();"
                ).execute(&harness.pool).await?;
                let mut barrier = harness.pool.begin().await?;
                let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *barrier).await?;
                sqlx::query("SELECT pg_advisory_xact_lock(3485,1)").execute(&mut *barrier).await?;
                let ready = {
                    let mut platform = ledger.lock().map_err(|_| "ledger")?;
                    platform.advertise_deliveries = true;
                    platform.receipt_ready.clone()
                };
                let client = StoreClient::new(
                    Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into(),
                );
                let worker = crate::gameplay_transport::run_store_receipt_outbox(owner.root, owner.character, client);
                tokio::pin!(worker);
                tokio::select! {
                    () = &mut worker => return Err("cosmetic worker exited".into()),
                    queued = super::store_fixtures::wait_for_advisory_waiter(&harness.pool, pid) => queued?,
                }
                let competing = sqlx::query("SELECT character_id FROM game_character_roots FOR UPDATE NOWAIT")
                    .fetch_one(&harness.pool).await;
                let buyer_locked = competing.as_ref().err().and_then(sqlx::Error::as_database_error)
                    .and_then(|error| error.code()).is_some_and(|code| code == "55P03");
                barrier.commit().await?;
                assert!(buyer_locked, "cosmetic recovery reached the grant before locking the buyer root");
                tokio::select! {
                    () = &mut worker => return Err("cosmetic worker exited".into()),
                    replay = async {
                        tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified()).await?;
                        purchase(owner, admitted, command(&request)?).await
                    } => assert_eq!(replay?.code, wire::StorePurchaseCode::Accepted),
                }
                assert_eq!(ledger.lock().map_err(|_| "ledger")?.debits, [request.price]);
                let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(grants, 1);
                return Ok(());
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

    // The train can carry the initial S06 catalogue before the archive continuation.
    // Pin the exact totals for each revision independently of the served offer list.
    let catalogue: serde_json::Value =
        serde_json::from_slice(crate::content::store_cosmetics::DOCUMENT)?;
    let has_archive_cohort = catalogue
        .pointer("/source/tibia_fandom_archive/cohort")
        .is_some();
    let mut request_seed = 160;
    for (category, kind, prefix, picks, initial_total, archive_additions) in [
        // Global Store additions: Arctic Unicorn, Batcat and Jungle Tiger.
        ("cosmetics.mounts", 3, "mount", MOUNTS, 19, 143),
        // Ceremonial Garb and Retro Citizen, Hunter, Knight and Mage.
        ("cosmetics.outfits", 2, "outfit", OUTFITS, 63, 8),
    ] {
        let total = initial_total + if has_archive_cohort { archive_additions } else { 0 };
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

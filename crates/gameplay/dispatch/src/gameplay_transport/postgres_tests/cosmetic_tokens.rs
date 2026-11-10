//! Real Store dispatch: character-bound earned currency, grant, replay and refusals.

use super::fixture::{Harness, debug, id, run};
use super::store::{FakePlatform, Ledger, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex as SyncMutex};

#[test]
fn cosmetic_tokens_real_dispatch_debits_character_and_unlocks_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("cosmetic-tokens", || scenario(true))
}

#[test]
fn cosmetic_tokens_real_dispatch_coin_payment_grants_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("cosmetic-coins", || scenario(false))
}

fn scenario(tokens: bool) -> TestResult {
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
                for epoch in 1..=1000_u64 {
                    super::daily_reward::epoch(&harness, epoch).await?;
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
                        content_revision: content.revision.clone(),
                        selection: None,
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
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

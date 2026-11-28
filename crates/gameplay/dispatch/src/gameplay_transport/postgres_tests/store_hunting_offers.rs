//! Real framed Store commands with the Platform's imported prices and the runtime database role.
use super::fixture::{Harness, debug, id};
use super::store_fixtures::{FakePlatform, Ledger, purchase, purchase_request, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::party_kill_xp::{self, PartyKillXpMember, PartyKillXpRequest};
use crate::foundation::RuntimeScopeRefV1;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::{CommandStatus, potion_durations as durations, store as wire};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum Offer {
    XpBoost,
    PermanentSlot,
    Wildcards,
    LostDebit,
    UnavailableGrant,
    AbandonedHold,
    RefusedHold,
    RestoredHold,
    RestoredUnboundHold,
    ExpiredDailyHold,
}

#[test]
fn store_xp_boost_purchases_use_hunting_time_and_server_save_tiers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::XpBoost)
}

#[test]
fn store_permanent_prey_slot_unlocks_the_character_and_replays_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::PermanentSlot)
}

#[test]
fn store_prey_wildcard_packs_share_the_ledger_and_stop_at_fifty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::Wildcards)
}

#[test]
fn store_hunting_refusal_restores_daily_wildcards() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::RefusedHold)
}

#[test]
fn store_hunting_expired_unsent_reservation_does_not_block_purchase() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::AbandonedHold)
}

#[test]
fn store_hunting_lost_grant_commit_stays_outstanding_and_worker_publishes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::UnavailableGrant)
}

#[test]
fn store_hunting_worker_recovers_a_lost_debit_without_a_client_retry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::LostDebit)
}

#[test]
fn store_hunting_restore_worker_releases_a_paid_hold_without_a_receipt() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::RestoredHold)
}

#[test]
fn store_hunting_restore_releases_sent_hold_before_operation_binding() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::RestoredUnboundHold)
}

#[test]
fn store_hunting_expired_unsent_hold_cannot_reduce_daily_reward() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Offer::ExpiredDailyHold)
}

#[test]
fn store_hunting_offers_checkout_from_named_house() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::hireling_lifecycle_fixtures::run_case_with_admin(
        super::hireling_lifecycle_fixtures::Case::HuntingStore,
        admin,
    )
}

#[test]
fn store_hunting_migration_can_be_reapplied() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "store_hunting_rerun", true, false).await?;
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/3347_store_hunting_offers.sql"
        ))
        .execute(&h.pool)
        .await?;
        eprintln!("POSTGRES_EXECUTED migration 3347 reapplied");
        h.cleanup().await
    })
}

fn scenario(admin: String, offer: Offer) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "store_hunting_offers", true, false).await?;
        prepare_current_session(&h).await?;
        super::daily_reward_fixtures::epoch(&h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (generation, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        if !matches!(offer, Offer::Wildcards) {
            owner.admit_progression_session(&mut admitted).await;
            assert!(
                owner
                    .progression_binding(admitted.game_session_id)
                    .is_some()
            );
        }
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        let account: Vec<u8> =
            sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        admitted.controller.as_mut().ok_or("controller")?.account_id =
            account.try_into().map_err(|_| "account UUID length")?;
        let ledger = Arc::new(Mutex::new(Ledger {
            balance: 4000,
            world: uuid(admitted.world_id.as_bytes()),
            enforce_active_import: true,
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
            Box::new(FakePlatform(ledger.clone())),
            "oteryn-game/store-consumer".into(),
        ))));
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::prey::offered(
                crate::gameplay_transport::store::with_store_offer(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    true,
                ),
                true,
            ),
            &[
                wire::CAPABILITY_STORE_V1,
                durations::CAPABILITY_POTION_DURATIONS_V1,
                oteryn_protocol_oteryn::prey::CAPABILITY_PREY_V1,
            ],
        )
        .ok_or("Store and potion capabilities")?;
        super::journal_inventory::runtime_role(owner.root).await?;

        if matches!(offer, Offer::RestoredHold | Offer::RestoredUnboundHold) {
            let unbound = matches!(offer, Offer::RestoredUnboundHold);
            if unbound {
                ledger.lock().map_err(debug)?.lose_debit_answer_once = true;
            } else {
                fail_wildcard_grant(&h).await?;
            }
            serve(
                &owner,
                &mut admitted,
                &[purchase_request(80, "store:prey-wildcards:1", 10)?],
            )
            .await?;
            let bound: bool = sqlx::query_scalar(
                "SELECT operation_id IS NOT NULL FROM game_store_hunting_pending WHERE sent",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(bound, !unbound);
            drop(owner);
            drop(authority);
            let predecessor = seal.record().recovery_generation;
            let issued_at = seal.record().issued_at.checked_add(1).ok_or("issued at")?;
            drop(seal);
            restore_paid_hold(&h, &ledger, predecessor, issued_at).await?;
            return h.cleanup().await;
        }

        match offer {
            Offer::XpBoost => xp_boost(&h, &owner, &mut admitted).await?,
            Offer::PermanentSlot => permanent_slot(&h, &owner, &mut admitted).await?,
            Offer::Wildcards => wildcards(&h, &owner, &mut admitted).await?,
            Offer::LostDebit | Offer::UnavailableGrant => {
                heal_paid_purchase(
                    &h,
                    &owner,
                    &mut admitted,
                    &ledger,
                    matches!(offer, Offer::LostDebit),
                )
                .await?
            }
            Offer::AbandonedHold => abandoned_hold(&h, &owner, &mut admitted).await?,
            Offer::RefusedHold => refused_hold(&h, &owner, &mut admitted, &ledger).await?,
            Offer::ExpiredDailyHold => expired_daily_hold(&h, &owner, &mut admitted).await?,
            Offer::RestoredHold | Offer::RestoredUnboundHold => {
                return Err("restore requires an unsealed recovery authority".into());
            }
        }
        let expected = match offer {
            Offer::XpBoost => vec![30, 45, 90, 180, 360, 30],
            Offer::PermanentSlot => vec![900, 900],
            Offer::Wildcards => [vec![10; 5], vec![50; 9]].concat(),
            Offer::LostDebit | Offer::UnavailableGrant | Offer::AbandonedHold => vec![10],
            Offer::RefusedHold => vec![50; 9],
            Offer::RestoredHold | Offer::RestoredUnboundHold => vec![10],
            Offer::ExpiredDailyHold => [vec![50; 9], vec![10; 4]].concat(),
        };
        assert_eq!(ledger.lock().map_err(debug)?.debits, expected);
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_store_hunting_pending")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(pending, 0);
        eprintln!("POSTGRES_EXECUTED Store hunting purchases and delivery owner");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn buy(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    seed: u8,
    offer: &str,
    price: u64,
    code: wire::StorePurchaseCode,
) -> TestResult<wire::StorePurchaseResult> {
    let result = purchase(owner, admitted, purchase_request(seed, offer, price)?).await?;
    assert_eq!(result.code, code, "offer {offer} at {price}");
    Ok(result)
}

async fn xp_boost(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use wire::StorePurchaseCode::{Accepted, OfferUnavailable};
    let request = wire::StoreCatalogueRequest {
        category: "boosts".into(),
        page_cursor: String::new(),
    };
    let inputs = [(
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        wire::encode_catalogue_request(&request).map_err(debug)?,
    )];
    let results = serve(owner, admitted, &inputs).await?;
    let (status, bytes) = results.first().ok_or("Store page")?;
    assert_eq!(*status, CommandStatus::Accepted);
    let page = wire::decode_catalogue_page(bytes).map_err(debug)?;
    assert_eq!(
        page.offers
            .iter()
            .map(|offer| (&*offer.offer_key, offer.price))
            .collect::<Vec<_>>(),
        vec![("store:xp-boost:1", 30)]
    );

    let query = wire::browse::Query {
        category: "boosts".into(),
        search: String::new(),
        sort: 0,
        cursor: String::new(),
    };
    let browsed = serve(
        owner,
        admitted,
        &[(wire::browse::COMMAND_TYPE, query.encode().map_err(debug)?)],
    )
    .await?;
    let (status, bytes) = browsed.first().ok_or("browse page")?;
    assert_eq!(*status, CommandStatus::Accepted);
    let page = wire::browse::Page::decode(bytes).map_err(debug)?;
    assert_eq!(page.products.len(), 1);
    assert_eq!(page.products[0].offer.price, 30);

    buy(
        owner,
        admitted,
        70,
        "store:xp-boost:2",
        45,
        OfferUnavailable,
    )
    .await?;
    let first = buy(owner, admitted, 71, "store:xp-boost:1", 30, Accepted).await?;
    assert_eq!(
        buy(owner, admitted, 71, "store:xp-boost:1", 30, Accepted).await?,
        first
    );
    buy(
        owner,
        admitted,
        72,
        "store:xp-boost:2",
        45,
        OfferUnavailable,
    )
    .await?;

    let started: i64 = sqlx::query_scalar(
        "SELECT started_at_ms FROM game_character_store_hunting_boosts WHERE ordinal=1",
    )
    .fetch_one(&h.pool)
    .await?;
    sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz LANGUAGE sql VOLATILE AS $$ SELECT clock_timestamp() + interval '3 hours' $$")
        .execute(&h.pool).await?;
    assert_eq!(boost_remaining(owner, admitted).await?, Some(3_600_000));
    for (seed, delta, stamina, spans) in [
        (201, 5_000, 840, &[][..]),
        (202, 35_000, 840, &[[30_000, 0]][..]),
        (203, 65_000, 839, &[[5_000, 0]][..]),
        (204, 95_000, 840, &[][..]),
    ] {
        let percent =
            settle_kill_with_activity(h, owner, admitted, (seed, started + delta, stamina, spans))
                .await?;
        assert_eq!(percent, if stamina < 840 { 0 } else { 50 });
    }
    assert_eq!(
        boost_remaining(owner, admitted).await?,
        Some(3_565_000),
        "crossing below fourteen hours still consumes the five seconds hunted before the kill"
    );
    settle_kill(h, owner, admitted, 204, started + 95_000, 840).await?;
    assert_eq!(boost_remaining(owner, admitted).await?, Some(3_565_000));

    sqlx::query("UPDATE game_character_store_hunting_boosts SET remaining_ms=1000")
        .execute(&h.pool)
        .await?;
    let expired = settle_kill_with_activity(
        h,
        owner,
        admitted,
        (206, started + 250_000, 840, &[[155_000, 75_000]]),
    )
    .await?;
    assert_eq!(
        expired, 0,
        "the older combat span exhausted the boost before hunting resumed"
    );
    assert_eq!(boost_remaining(owner, admitted).await?, None);
    let results = serve(owner, admitted, &inputs).await?;
    let (_, bytes) = results.first().ok_or("Store page")?;
    assert_eq!(
        wire::decode_catalogue_page(bytes).map_err(debug)?.offers[0].price,
        45
    );
    for (seed, tier, price) in [(73, 2, 45), (74, 3, 90), (75, 4, 180), (76, 5, 360)] {
        buy(
            owner,
            admitted,
            seed,
            &format!("store:xp-boost:{tier}"),
            price,
            Accepted,
        )
        .await?;
        sqlx::query("UPDATE game_character_store_hunting_boosts SET remaining_ms=0")
            .execute(&h.pool)
            .await?;
    }
    buy(
        owner,
        admitted,
        77,
        "store:xp-boost:5",
        360,
        OfferUnavailable,
    )
    .await?;
    super::daily_reward_fixtures::epoch(h, 2).await?;
    buy(owner, admitted, 78, "store:xp-boost:1", 30, Accepted).await?;
    Ok(())
}

async fn boost_remaining(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Option<u64>> {
    let results = serve(
        owner,
        admitted,
        &[(durations::COMMAND_TYPE_POTION_DURATIONS_QUERY, Vec::new())],
    )
    .await?;
    let (status, bytes) = results.first().ok_or("duration command")?;
    assert_eq!(*status, CommandStatus::Accepted);
    let reply = durations::PotionDurations::decode(bytes).ok_or("duration projection")?;
    Ok(reply
        .active
        .iter()
        .find(|entry| entry.source == 3)
        .map(|entry| entry.remaining_ms))
}

async fn settle_kill(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    seed: u8,
    at_ms: i64,
    stamina: u16,
) -> TestResult<i16> {
    settle_kill_with_activity(h, owner, admitted, (seed, at_ms, stamina, &[])).await
}

async fn settle_kill_with_activity(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    kill: (u8, i64, u16, &[[u64; 2]]),
) -> TestResult<i16> {
    let (seed, at_ms, stamina, spans) = kill;
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("kill fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err("kill requires a Channel".into());
    };
    let serial = u64::from(seed);
    let mut death = Vec::from(world_id.as_bytes().as_slice());
    death.extend_from_slice(channel_id.as_bytes());
    death.extend_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    death.extend_from_slice(&7_u32.to_be_bytes());
    death.extend_from_slice(&serial.to_be_bytes());
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression binding")?;
    let permit = owner.spell_lane_permit().await.ok_or("kill permit")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let scope = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await
    .map_err(debug)?;
    party_kill_xp::commit_in_transaction(
        &mut tx,
        &scope,
        owner.holder,
        PartyKillXpRequest {
            death_key: death.clone(),
            principal: fence.character_id,
            creature: "oteryn:creature.rat".into(),
            base_xp: 100,
            death_at_ms: serial,
            death_at_us: serial * 1000,
            died_at_unix_ms: u64::try_from(at_ms)?,
            shared: false,
            party_id: None,
            party_revision: None,
            members: vec![PartyKillXpMember {
                task_board_eligible: false,
                fence,
                premium: false,
                forge_dust: None,
                live_stamina: Some(stamina),
                hunt_spans_ms: spans.to_vec(),
                shared: false,
                party_id: None,
                party_revision: None,
                request: ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(seed)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(100),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
            }],
        },
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    slot.observe_endgame_progression();
    Ok(sqlx::query_scalar(
        "SELECT xp_boost_percent FROM game_party_kill_xp_members WHERE death_key=$1",
    )
    .bind(death)
    .fetch_one(&h.pool)
    .await?)
}

async fn permanent_slot(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use wire::StorePurchaseCode::{Accepted, OfferUnavailable};
    let key = "store:permanent-prey-slot";
    let first = buy(owner, admitted, 80, key, 900, Accepted).await?;
    assert_eq!(buy(owner, admitted, 80, key, 900, Accepted).await?, first);
    buy(owner, admitted, 81, key, 900, Accepted).await?;
    buy(owner, admitted, 82, key, 900, OfferUnavailable).await?;
    let targets: Vec<i16> = sqlx::query_scalar(
        "SELECT target FROM game_character_permanent_prey_slots ORDER BY target",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(targets, vec![2, 3]);
    assert_eq!(summary(owner, admitted).await?.permanent_prey_slots, 2);
    let (prey, permanent) = owner
        .prepare_admission_prey(*admitted, None)
        .await
        .map_err(debug)?
        .ok_or("admitted prey")?;
    assert_eq!(permanent, [false, true, true]);
    assert!(!prey.expansion_owned);
    // A purchased third slot must also be visible to the native kill modifier.
    sqlx::query(
        "INSERT INTO game_character_prey_slots(character_id,slot,state,creature_key,
             bonus_type,grade,bonus_percent,hunting_s_left,slot_epoch)
         SELECT character_id,3,'ACTIVE','oteryn:creature.rat','XP',10,40,7200,1
         FROM game_character_store_prey_slots WHERE target=3
         ON CONFLICT (character_id,slot) DO UPDATE SET state='ACTIVE',
             creature_key=excluded.creature_key,bonus_type='XP',grade=10,
             bonus_percent=40,hunting_s_left=7200",
    )
    .execute(&h.pool)
    .await?;
    settle_kill(h, owner, admitted, 205, 0, 840).await?;
    let slot: Option<i16> = sqlx::query_scalar("SELECT prey_slot FROM game_party_kill_xp_members")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(slot, Some(3));
    Ok(())
}

async fn wildcards(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use wire::StorePurchaseCode::{Accepted, OfferUnavailable};
    let first = buy(owner, admitted, 80, "store:prey-wildcards:1", 10, Accepted).await?;
    assert_eq!(
        buy(owner, admitted, 80, "store:prey-wildcards:1", 10, Accepted).await?,
        first
    );
    for seed in 81..85 {
        buy(
            owner,
            admitted,
            seed,
            "store:prey-wildcards:1",
            10,
            Accepted,
        )
        .await?;
    }
    for seed in 85..94 {
        buy(
            owner,
            admitted,
            seed,
            "store:prey-wildcards:5",
            50,
            Accepted,
        )
        .await?;
    }
    buy(
        owner,
        admitted,
        94,
        "store:prey-wildcards:1",
        10,
        OfferUnavailable,
    )
    .await?;
    let rows: (i64, i64) = sqlx::query_as(
        "SELECT count(*),sum(credited) FROM game_account_prey_wildcard_entries WHERE kind='STORE'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rows, (14, 50));
    assert_eq!(summary(owner, admitted).await?.prey_wildcards, 50);
    let (prey, _) = owner
        .prepare_admission_prey(*admitted, None)
        .await
        .map_err(debug)?
        .ok_or("admitted prey")?;
    assert_eq!(prey.wildcard_balance, 50);
    let daily_credit: i16 = sqlx::query_scalar(
        "SELECT game_prey_wildcard_daily_credit(account_id,world_id,balance,3::smallint)
         FROM game_account_prey_wildcards",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(daily_credit, 2);
    Ok(())
}

async fn summary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::cyclopedia_character::StoreSummary> {
    use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, TabView};
    let fence = admitted.item_fence.ok_or("Cyclopedia fence")?;
    let page = oteryn_protocol_oteryn::character_info::PageRequest::new(0, 1).ok_or("page")?;
    let TabView::StoreSummary(summary) = owner
        .root
        .read_cyclopedia_character_tab(
            owner.character,
            owner.holder,
            fence,
            fence.character_id,
            CharacterTab::StoreSummary,
            page,
            0,
        )
        .await
        .map_err(debug)?
    else {
        return Err("Store summary".into());
    };
    Ok(summary)
}

async fn fail_wildcard_grant(h: &Harness) -> TestResult {
    sqlx::raw_sql(
        "CREATE FUNCTION store_test_reject_wildcards() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected wildcard ledger failure'; END $$;
         CREATE TRIGGER store_test_reject_wildcards BEFORE INSERT ON game_account_prey_wildcard_entries
         FOR EACH ROW WHEN (NEW.kind = 'STORE') EXECUTE FUNCTION store_test_reject_wildcards()",
    ).execute(&h.pool).await?;
    Ok(())
}

async fn restore_paid_hold(
    h: &Harness,
    ledger: &Arc<Mutex<Ledger>>,
    predecessor: u64,
    issued_at: u64,
) -> TestResult {
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_store_hunting_pending")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        retained, 1,
        "the debit has no grant and still holds capacity"
    );
    let transition = h
        .recovery
        .begin_recovery(predecessor, id(181), issued_at)
        .map_err(debug)?;
    h.root
        .reconcile_character_recovery(&transition)
        .await
        .map_err(debug)?;
    let generation = transition.record().recovery_generation;
    drop(transition);
    let ready = {
        let mut platform = ledger.lock().map_err(debug)?;
        platform.recovery_generation = Some(generation.to_string());
        platform.advertise_deliveries = true;
        platform.reopened = true;
        platform.receipt_ready.clone()
    };
    let client = StoreClient::new(
        Box::new(FakePlatform(Arc::clone(ledger))),
        "oteryn-game/store-consumer".into(),
    );
    let worker =
        crate::gameplay_transport::run_retained_store_reconciliation(&h.root, client, generation);
    tokio::select! {
        received = tokio::time::timeout(std::time::Duration::from_secs(10), ready.notified()) => received?,
        () = worker => return Err("Store restore worker stopped".into()),
    }
    assert_eq!(ledger.lock().map_err(debug)?.restore_conflicts, ["DELIVER"]);
    let (pending, grants): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_store_hunting_pending),
                (SELECT count(*) FROM store_delivery_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        (pending, grants),
        (0, 0),
        "a restore conflict releases the paid hold"
    );
    eprintln!("POSTGRES_EXECUTED hunting hold released by Store restore worker");
    Ok(())
}

async fn heal_paid_purchase(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<Mutex<Ledger>>,
    lost_debit: bool,
) -> TestResult {
    {
        let mut platform = ledger.lock().map_err(debug)?;
        platform.advertise_deliveries = true;
        platform.lose_debit_answer_once = lost_debit;
    }
    let input = [purchase_request(80, "store:prey-wildcards:1", 10)?];
    if lost_debit {
        serve(owner, admitted, &input).await?;
    } else {
        let (output, stalled) = crate::durability::store::with_stalled_hunting_grant_commit(serve(
            owner, admitted, &input,
        ))
        .await;
        assert!(stalled, "the grant committed before its answer was lost");
        assert!(
            output?.is_empty(),
            "uncertain purchase must leave no terminal CommandResult"
        );
    }
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(grants, i64::from(!lost_debit));
    let ready = ledger.lock().map_err(debug)?.receipt_ready.clone();
    let client = StoreClient::new(
        Box::new(FakePlatform(Arc::clone(ledger))),
        "oteryn-game/store-consumer".into(),
    );
    let worker =
        crate::gameplay_transport::run_store_receipt_outbox(owner.root, owner.character, client);
    tokio::select! {
        received = tokio::time::timeout(std::time::Duration::from_secs(15), ready.notified()) => received?,
        () = worker => return Err("Store worker stopped".into()),
    }
    let balance: i16 = sqlx::query_scalar("SELECT balance FROM game_account_prey_wildcards")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        balance, 1,
        "paid purchase recovers without the buyer's command or session"
    );
    let (kind, payload) = &input[0];
    let (status, result) = crate::gameplay_transport::store::execute_command(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        *kind,
        payload,
    )
    .await
    .ok_or("exact confirmation could not read its recovered grant")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::decode_purchase_result(&result).map_err(debug)?.code,
        wire::StorePurchaseCode::Accepted
    );
    assert_eq!(ledger.lock().map_err(debug)?.debits, [10]);
    Ok(())
}

async fn reserve_unsent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    seed: u8,
    key: &str,
    price: u64,
) -> TestResult {
    use crate::durability::store::{PurchaseRequest, StoreHuntingPurchase};
    use oteryn_gameplay_economy::store as rules;

    let account = admitted.controller.ok_or("controller")?.account_id;
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("reservation fence")?;
    let revision = crate::gameplay_transport::store::active_catalogue_revision();
    let quote = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(id(seed)).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: fence.character_id,
        offer_key: key.into(),
        catalog_revision: revision.into(),
        price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let request = PurchaseRequest {
        purchase_request_id: id(seed),
        account_id: account,
        character_id: *fence.character_id.as_bytes(),
        binding: quote.binding_digest().to_vec(),
    };
    owner
        .root
        .record_purchase_request(request.clone())
        .await
        .map_err(debug)?;
    assert!(
        owner
            .root
            .reserve_store_hunting_purchase(
                owner.character,
                owner.holder,
                fence,
                StoreHuntingPurchase {
                    request,
                    offer_key: key.into(),
                    allow_new: true,
                    catalog_revision: revision.into(),
                    price: i64::try_from(price)?,
                },
                &crate::gameplay_transport::prey::Premium(None),
                *admitted.channel_id.as_bytes(),
            )
            .await
            .map_err(debug)?
            == Some(true)
    );
    Ok(())
}

async fn abandoned_hold(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    reserve_unsent(owner, admitted, 80, "store:prey-wildcards:1", 10).await?;
    sqlx::query(
        "UPDATE game_store_hunting_pending SET reserved_at=clock_timestamp()-interval '1 minute'",
    )
    .execute(&h.pool)
    .await?;
    buy(
        owner,
        admitted,
        81,
        "store:prey-wildcards:1",
        10,
        wire::StorePurchaseCode::Accepted,
    )
    .await?;
    assert_eq!(summary(owner, admitted).await?.prey_wildcards, 1);
    Ok(())
}

struct PremiumDaily;

impl crate::durability::daily_reward_delivery::DailyRewardPremiumGate for PremiumDaily {
    async fn current(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

async fn refused_hold(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<Mutex<Ledger>>,
) -> TestResult {
    let key = "store:prey-wildcards:5";
    for seed in 80..89 {
        buy(
            owner,
            admitted,
            seed,
            key,
            50,
            wire::StorePurchaseCode::Accepted,
        )
        .await?;
    }
    reserve_unsent(owner, admitted, 90, key, 50).await?;
    for epoch in 1..=30_u8 {
        claim_daily(h, owner, admitted, epoch).await?;
        if summary(owner, admitted).await?.prey_wildcards == 47 {
            let credits: i64 = sqlx::query_scalar("SELECT coalesce(sum(requested),0) FROM game_account_prey_wildcard_entries WHERE kind='DAILY_REWARD'")
                .fetch_one(&h.pool).await?;
            if credits >= 7 {
                break;
            }
        }
    }
    ledger.lock().map_err(debug)?.balance = 0;
    buy(
        owner,
        admitted,
        90,
        key,
        50,
        wire::StorePurchaseCode::InsufficientBalance,
    )
    .await?;
    assert_eq!(
        summary(owner, admitted).await?.prey_wildcards,
        52,
        "refusal restores the Daily Reward credits"
    );
    Ok(())
}

async fn claim_daily(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    epoch: u8,
) -> TestResult {
    use crate::durability::daily_reward_claim::DailyRewardClaimRequest;
    use crate::durability::daily_reward_delivery::DailyRewardDeliveryPlan;
    use crate::foundation::{CommandId, CommandRef};
    let fence = admitted.item_fence.ok_or("daily fence")?;
    let content =
        crate::content::daily_reward::DailyRewardContent::catalogue().ok_or("daily content")?;
    if epoch > 1 {
        super::daily_reward_fixtures::epoch(h, u64::from(epoch)).await?;
    }
    let command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(u64::from(epoch) + 1000).map_err(debug)?,
    );
    owner
        .root
        .commit_daily_reward_claim(
            owner.character,
            owner.holder,
            fence,
            DailyRewardClaimRequest {
                command,
                reset_epoch: u64::from(epoch),
                transaction_id: id(150 + epoch),
                request_binding: [epoch; 32],
            },
            DailyRewardDeliveryPlan {
                content_revision: content.revision.clone(),
                remote: false,
            },
            &PremiumDaily,
        )
        .await
        .map_err(debug)?;
    Ok(())
}

async fn expired_daily_hold(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use wire::StorePurchaseCode::Accepted;
    for seed in 80..89 {
        buy(
            owner,
            admitted,
            seed,
            "store:prey-wildcards:5",
            50,
            Accepted,
        )
        .await?;
    }
    for seed in 89..93 {
        buy(
            owner,
            admitted,
            seed,
            "store:prey-wildcards:1",
            10,
            Accepted,
        )
        .await?;
    }
    assert_eq!(summary(owner, admitted).await?.prey_wildcards, 49);
    reserve_unsent(owner, admitted, 93, "store:prey-wildcards:1", 10).await?;
    sqlx::query(
        "UPDATE game_store_hunting_pending SET reserved_at=clock_timestamp()-interval '1 minute'",
    )
    .execute(&h.pool)
    .await?;
    for epoch in 1..=6 {
        claim_daily(h, owner, admitted, epoch).await?;
    }
    assert_eq!(
        summary(owner, admitted).await?.prey_wildcards,
        52,
        "Daily Reward must see the full headroom before another checkout"
    );
    Ok(())
}

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

        match offer {
            Offer::XpBoost => xp_boost(&h, &owner, &mut admitted).await?,
            Offer::PermanentSlot => permanent_slot(&h, &owner, &mut admitted).await?,
            Offer::Wildcards => wildcards(&h, &owner, &mut admitted).await?,
        }
        let expected = match offer {
            Offer::XpBoost => vec![30, 45, 90, 180, 360, 30],
            Offer::PermanentSlot => vec![900, 900],
            Offer::Wildcards => [vec![10; 5], vec![50; 9]].concat(),
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
    for (seed, delta, stamina) in [
        (201, 5_000, 840),
        (202, 35_000, 840),
        (203, 65_000, 839),
        (204, 95_000, 840),
    ] {
        let percent = settle_kill(h, owner, admitted, seed, started + delta, stamina).await?;
        assert_eq!(percent, if stamina < 840 { 0 } else { 50 });
    }
    assert_eq!(boost_remaining(owner, admitted).await?, Some(3_570_000));
    settle_kill(h, owner, admitted, 204, started + 95_000, 840).await?;
    assert_eq!(boost_remaining(owner, admitted).await?, Some(3_570_000));

    sqlx::query("UPDATE game_character_store_hunting_boosts SET remaining_ms=0")
        .execute(&h.pool)
        .await?;
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
    sqlx::raw_sql(
        "CREATE FUNCTION store_test_reject_wildcards() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected wildcard ledger failure'; END $$;
         CREATE TRIGGER store_test_reject_wildcards BEFORE INSERT ON game_account_prey_wildcard_entries
         FOR EACH ROW WHEN (NEW.kind = 'STORE') EXECUTE FUNCTION store_test_reject_wildcards()",
    ).execute(&h.pool).await?;
    let commands = [purchase_request(80, "store:prey-wildcards:1", 10)?];
    let rejected = serve(owner, admitted, &commands).await?;
    assert_eq!(
        rejected.first().map(|(status, _)| *status),
        Some(CommandStatus::Rejected)
    );
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM store_delivery_receipts),
                (SELECT count(*) FROM game_account_prey_wildcard_entries)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        counts,
        (0, 0),
        "effect failure rolls back the Store receipt"
    );
    sqlx::query("DROP TRIGGER store_test_reject_wildcards ON game_account_prey_wildcard_entries")
        .execute(&h.pool)
        .await?;

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

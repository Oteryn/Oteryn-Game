use super::fixture::{Harness, debug, run};
use super::npcfix_m38_travel_guards_fixtures::NoPremium;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::npc_travel::{NpcTravelGate, TravelRefusal};

use crate::gameplay_transport::NpcTalkBook;

struct Premium;

impl NpcTravelGate for Premium {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

#[test]
fn lane_three_reference_routes_use_donor_quote_conditions_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("travel-lane-three", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "travel_lane_three", true, false)
                .await?;
            let result = check_source_quotes(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn check_source_quotes(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "current fence")?
        .ok_or("current fence")?;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    let route = |npc: &str, route: &str| {
        book.get(&format!("oteryn:npc.{npc}"))
            .and_then(|entry| entry.travel_routes.iter().find(|r| r.route.key == route))
            .cloned()
            .ok_or_else(|| format!("missing production route {npc}/{route}"))
    };
    let quote = |route| {
        h.root
            .quote_npc_travel(&authority, &h.node, fence, route, &NoPremium)
    };
    // Seed imported track snapshots, including an out-of-range Nargor value,
    // without constructing quest receipts unrelated to the quote under test.
    let seed_track = async |quest: &str, track: &str, value: i64| -> TestResult {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let changed = sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id, track_key, quest_key, value, committed_character_revision)
             SELECT character_id, $2, $3, $4, 2
             FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid
             ON CONFLICT(character_id, track_key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(track)
        .bind(quest)
        .bind(value)
        .execute(&mut *tx)
        .await?;
        assert_eq!(changed.rows_affected(), 1);
        tx.commit().await?;
        Ok(())
    };
    let premium_quote = |route| {
        h.root
            .quote_npc_travel(&authority, &h.node, fence, route, &Premium)
    };
    let postman_routes = [
        ("captain_bluebear", "carlin", (1, 2)),
        ("uzon", "edron", (2, 3)),
        ("captain_seahorse", "venore", (3, 4)),
        ("brodrosch", "cormaya", (4, 5)),
    ];
    for (npc, destination, passage) in postman_routes {
        let passenger_route = route(npc, destination)?;
        assert_eq!(passenger_route.postman_tour_passage(), Some(passage));
        assert_eq!(
            quote(passenger_route.clone()).await.map_err(debug)?,
            Ok(passenger_route.route.price),
            "{npc}/{destination} with no Postman stage"
        );
    }
    for (npc, destination, (before, after)) in postman_routes {
        let passenger_route = route(npc, destination)?;
        for value in [before, after] {
            seed_track(
                "oteryn:quest.the_postman_missions_quest",
                oteryn_gameplay_content::content::npc_catalogue::travel::POSTMAN_TOUR_TRACK,
                value,
            )
            .await?;
            assert_eq!(
                quote(passenger_route.clone()).await.map_err(debug)?,
                Ok(passenger_route.route.price),
                "{npc}/{destination} at Postman stage {value}"
            );
        }
    }
    for (npc, destinations) in [
        ("junkar_mines", ["kazordoon", "robsons_isle", "thais"]),
        ("junkar_robsons", ["kazordoon", "mines", "thais"]),
        ("junkar_thais", ["kazordoon", "mines", "robsons_isle"]),
    ] {
        for destination in destinations {
            assert_eq!(
                quote(route(npc, destination)?).await.map_err(debug)?,
                Err(TravelRefusal::UnavailableRoute),
                "{npc}/{destination} without active world changes"
            );
        }
    }
    seed_track(
        "oteryn:quest.the_postman_missions_quest",
        "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank",
        3,
    )
    .await?;
    let mut mismatches = Vec::new();
    for (npc, destination) in [
        ("melian", "marapur"),
        ("pino", "marapur"),
        ("tanyt", "marapur"),
    ] {
        let inbound = route(npc, destination)?;
        let actual = quote(inbound.clone()).await.map_err(debug)?;
        if actual != Err(TravelRefusal::PremiumRequired) {
            mismatches.push(format!("{npc}/{destination} for a free account: {actual:?}"));
        }
    }
    for (npc, destination) in [
        ("melian", "marapur"),
        ("pino", "marapur"),
        ("tanyt", "marapur"),
        ("kendra_thais", "nostalgia"),
    ] {
        let actual = premium_quote(route(npc, destination)?).await.map_err(debug)?;
        if actual != Err(TravelRefusal::UnavailableRoute) {
            mismatches.push(format!("{npc}/{destination} without its unlock: {actual:?}"));
        }
    }
    let nargor = route("sebastian", "nargor")?;
    let nargor_access =
        "oteryn:quest-progress/quest/u7_8/the_shattered_isles/access_to_nargor";
    assert!(!nargor.held, "the authored Shattered Isles route is live");
    let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    assert!(
        quests.catalogue()
            .transition("oteryn:quest-reward/shattered-isles/sabotage-start")
            .is_some_and(|operation| operation.effects.iter().any(|effect|
                effect.track == nargor_access
                    && effect.effect == oteryn_gameplay_quests::quest::QuestEffectKind::Set(1)
            )),
        "Raymond's live sabotage mission unlocks Sebastian's route"
    );
    assert_eq!(
        quote(nargor.clone()).await.map_err(debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    // Check the live route's exact storage predicate and the independent release hold.
    let mut held_nargor = nargor.clone();
    held_nargor.held = true;
    for value in [0, 1, 2] {
        seed_track(
            "oteryn:quest.the_shattered_isles_quest",
            nargor_access,
            value,
        )
        .await?;
        let expected = if value == 1 {
            Ok(50)
        } else {
            Err(TravelRefusal::UnavailableRoute)
        };
        let live = quote(nargor.clone()).await.map_err(debug)?;
        if live != expected {
            mismatches.push(format!(
                "Nargor at storage {value}: {live:?}, expected {expected:?}"
            ));
        }
        let held = quote(held_nargor.clone()).await.map_err(debug)?;
        if held != Err(TravelRefusal::UnavailableRoute) {
            mismatches.push(format!("held Nargor route at storage {value}: {held:?}"));
        }
    }
    for (npc, destination, fare) in [
        ("melian", "marapur", 70),
        ("pino", "marapur", 70),
        ("tanyt", "marapur", 70),
    ] {
        let mut candidate = route(npc, destination)?;
        candidate.held = false;
        let actual = premium_quote(candidate).await.map_err(debug)?;
        if actual != Ok(fare) {
            mismatches.push(format!(
                "{npc}/{destination}: {actual:?}, expected fare {fare}"
            ));
        }
    }
    // The Crystal helpers use StdModule.travel without an extra route-specific predicate.
    for (npc, destination, fare) in [
        ("captain_jack", "tibia", 0),
        ("ferryman_kamil", "meluna", 0),
        ("ferryman_kamil_meluna", "fibula", 0),
        ("harlow_vengoth", "yalahar", 0),
        ("hawkhurst", "passage", 400),
        ("hawkhurst_ingol", "passage", 400),
        ("tonar", "passage", 0),
    ] {
        let actual = quote(route(npc, destination)?).await.map_err(debug)?;
        if actual != Ok(fare) {
            mismatches.push(format!(
                "{npc}/{destination}: {actual:?}, expected fare {fare}"
            ));
        }
    }
    let kendra = route("kendra", "thais")?;
    assert_eq!(
        quote(kendra.clone()).await.map_err(debug)?,
        Err(TravelRefusal::PremiumRequired)
    );
    // The shared fixture has already earned the Grand Postman discount.
    let actual = premium_quote(kendra).await.map_err(debug)?;
    if actual != Ok(170) {
        mismatches.push(format!(
            "kendra/thais: {actual:?}, expected Grand Postman fare 170"
        ));
    }
    if !mismatches.is_empty() {
        return Err(mismatches.join("\n").into());
    }
    eprintln!("POSTGRES_EXECUTED Crystal ferry quotes and Kendra Premium refusal");
    Ok(())
}

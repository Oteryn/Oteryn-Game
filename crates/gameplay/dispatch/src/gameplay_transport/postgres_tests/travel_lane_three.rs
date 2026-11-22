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
    seed_track(
        "oteryn:quest.the_postman_missions_quest",
        "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank",
        3,
    )
    .await?;
    let mut mismatches = Vec::new();
    let premium_quote = |route| {
        h.root
            .quote_npc_travel(&authority, &h.node, fence, route, &Premium)
    };
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
        ("tonar", "passage"),
        ("kendra_thais", "nostalgia"),
    ] {
        let actual = premium_quote(route(npc, destination)?).await.map_err(debug)?;
        if actual != Err(TravelRefusal::UnavailableRoute) {
            mismatches.push(format!("{npc}/{destination} without its unlock: {actual:?}"));
        }
    }
    let nargor = route("sebastian", "nargor")?;
    if !nargor.held && oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?
        .program("oteryn:quest.the_shattered_isles_quest").is_none()
    {
        mismatches.push("Sebastian advertises Nargor before its quest program is live".into());
    }
    assert_eq!(
        quote(nargor.clone()).await.map_err(debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    // The release hold applies even to imported characters. Check the exact storage
    // predicate separately with the hold lifted in the quote fixture.
    let mut nargor_candidate = nargor.clone();
    nargor_candidate.held = false;
    for value in [0, 1, 2] {
        seed_track(
            "oteryn:quest.the_shattered_isles_quest",
            "oteryn:quest-progress/quest/u7_8/the_shattered_isles/access_to_nargor",
            value,
        )
        .await?;
        let expected = if value == 1 {
            Ok(50)
        } else {
            Err(TravelRefusal::UnavailableRoute)
        };
        let live = quote(nargor.clone()).await.map_err(debug)?;
        if live != Err(TravelRefusal::UnavailableRoute) {
            mismatches.push(format!("held Nargor route at storage {value}: {live:?}"));
        }
        let actual = quote(nargor_candidate.clone()).await.map_err(debug)?;
        if actual != expected {
            mismatches.push(format!(
                "Nargor at storage {value}: {actual:?}, expected {expected:?}"
            ));
        }
    }
    for (npc, destination, fare) in [
        ("melian", "marapur", 70),
        ("pino", "marapur", 70),
        ("tanyt", "marapur", 70),
        ("tonar", "passage", 0),
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
    if !mismatches.is_empty() {
        return Err(mismatches.join("\n").into());
    }
    Ok(())
}

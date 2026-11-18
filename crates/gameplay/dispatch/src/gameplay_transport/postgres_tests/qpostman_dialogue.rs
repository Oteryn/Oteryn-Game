use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug};
use super::npcfix_m38_travel_return::{commit, complete, request as passage};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::content::npc_catalogue::travel;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::{
    DurabilityError, item_mint_audit::Type2Transaction, npc_travel::NpcTravelGate,
};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn postman_briefing_and_completed_passages_advance_mission_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run_async(async move |_fixture_admin| {
        let h = Harness::create_with_admission_seed(admin, "postman_dialogue", true, false).await?;
        let result = exercise(&h).await;
        h.cleanup().await?;
        result
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED postman_briefing_and_completed_passages_advance_mission_postgres {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = super::fence(1)?.runtime_scope.world_id();
    let deps = Dependencies::with_content_and_capacity(
        crate::foundation::ChannelContentPin::test(world),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        4,
    )?;
    let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    owner.active_generation = Some(controller.active().ok_or("active generation")?);
    let mut admitted = deps.admitted;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    let admitted = &admitted;
    owner.admit_quest_session(admitted).await;
    let [kevin, captain, technomancer] = install(
        &owner,
        admitted,
        [
            "oteryn:npc.kevin",
            "oteryn:npc.captain_bluebear",
            "oteryn:npc.brodrosch",
        ],
    )
    .await?;
    let track = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";

    talk(&mut owner, admitted, kevin, 1, "hi").await?;
    assert!(
        !talk(&mut owner, admitted, kevin, 2, "mission")
            .await?
            .is_empty()
    );
    for command in 3..9 {
        assert!(
            !talk(&mut owner, admitted, kevin, command, "yes")
                .await?
                .is_empty()
        );
        assert_eq!(progress(&owner, admitted, &authority, track).await?, 0);
    }
    let quest_receipts = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
        )
        .bind("oteryn:quest.the_postman_missions_quest")
        .fetch_one(&h.pool)
    };
    assert_eq!(quest_receipts().await?, 0);
    assert!(
        !talk(&mut owner, admitted, kevin, 9, "yes")
            .await?
            .is_empty()
    );
    assert_eq!(progress(&owner, admitted, &authority, track).await?, 1);
    let session = admitted.game_session_id;
    for (command, carrier, destination, before, after) in [
        (10, "uzon", "edron", 1, 1),
        (11, "captain_bluebear", "carlin", 1, 2),
        (12, "captain_bluebear", "carlin", 2, 2),
        (13, "uzon", "edron", 2, 3),
        (14, "captain_seahorse", "venore", 3, 4),
        (15, "brodrosch", "cormaya", 4, 5),
    ] {
        let mut ride = passage(&owner, &deps, command, carrier).await?;
        ride.route.route.key = destination.into();
        ride.route.keyword = destination.into();
        let occurrence = ride.arrival.occurrence;
        commit(&owner, session, ride).await?;
        assert_eq!(progress(&owner, admitted, &authority, track).await?, before);
        complete(&owner, &deps, occurrence).await?;
        assert_eq!(progress(&owner, admitted, &authority, track).await?, after);
        sqlx::query(
                "UPDATE game_newhaven_travel_receipts SET arrived_at=arrived_at-interval '4 seconds' WHERE arrived_at IS NOT NULL",
            ).execute(&h.pool).await?;
    }
    assert_eq!(
        quest_receipts().await?,
        5,
        "briefing and four matching arrivals"
    );
    talk(&mut owner, admitted, kevin, 16, "bye").await?;
    assert_fares(h, &mut owner, &deps, [captain, technomancer]).await
}

async fn assert_fares(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    [captain, technomancer]: [EntityRef; 2],
) -> TestResult {
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let resolve = |npc: &str, destination: &str| -> TestResult<travel::TravelRoute> {
        let (_, definition) = model
            .npcs()
            .iter()
            .find(|(key, _)| key.key == npc)
            .ok_or("travel NPC definition")?;
        travel::resolve(&model, npc, &definition.services)
            .into_iter()
            .find(|route| route.route.key == destination)
            .ok_or_else(|| format!("missing route {npc}/{destination}").into())
    };
    let ship = resolve("oteryn:npc.captain_bluebear", "carlin")?;
    let carpet = resolve("oteryn:npc.uzon", "edron")?;
    let cormaya = resolve("oteryn:npc.brodrosch", "cormaya")?;
    assert!(cormaya.postman && !cormaya.held);
    assert_eq!(cormaya.route.price, 160);

    let mut command = 30;
    for (rank, mission06, ship_fare, carpet_fare, cormaya_fare) in [
        (2_i64, 13_i64, 110, 50, 160),
        (3, 12, 100, 60, 150),
        (5, 13, 100, 50, 150),
    ] {
        // Seed the later missions that belong to the separate quest-content lane.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (track, value) in [("rank", rank), ("mission06", mission06)] {
            sqlx::query(
                "INSERT INTO game_character_quest_tracks
                    (character_id,track_key,quest_key,value,committed_character_revision)
                 SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
                  WHERE character_id=encode($1,'hex')::uuid
                 ON CONFLICT (character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(format!(
                "oteryn:quest-progress/quest/u7_24/the_postman_missions/{track}"
            ))
            .bind("oteryn:quest.the_postman_missions_quest")
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        for (npc, route, expected) in [
            (captain, &ship, ship_fare),
            (technomancer, &cormaya, cormaya_fare),
        ] {
            talk(owner, &deps.admitted, npc, command, "hi").await?;
            let reply = talk(owner, &deps.admitted, npc, command + 1, &route.keyword).await?;
            assert!(
                reply
                    .iter()
                    .any(|line| line.contains(&format!("for {expected} gold"))),
                "rank {rank}, mission 6 at {mission06}, route {}: {reply:?}",
                route.service,
            );
            talk(owner, &deps.admitted, npc, command + 2, "bye").await?;
            command += 3;
        }
        assert_eq!(
            owner
                .root
                .quote_npc_travel(
                    owner.character,
                    owner.holder,
                    fence,
                    carpet.clone(),
                    &Premium,
                )
                .await
                .map_err(debug)?,
            Ok(carpet_fare),
            "rank {rank}, mission 6 at {mission06}",
        );
    }
    Ok(())
}

struct Premium;

impl NpcTravelGate for Premium {
    async fn travel_permitted(
        &self,
        _tx: &mut Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        Ok(true)
    }
}

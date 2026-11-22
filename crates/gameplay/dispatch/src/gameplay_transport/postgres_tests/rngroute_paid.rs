//! The fixture admits the Crystal routes on a small map through production NPC_TALK.
use super::fixture::{Harness, debug, id, run};
use super::npc_paid_travel_fixtures::{talk, travel_world};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NativeEntryStart, NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn rngroute_plate_armor_paid_travel_places_the_receipted_ghostship_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Route {
        npc: "captain_fearless",
        keyword: "darashia",
        fare: 60,
        normal: (33289, 32481, 6),
        alternate: (33324, 32173, 6),
        confirmation: 12,
    })
}

#[test]
fn rngroute_shattered_isles_paid_travel_places_the_receipted_shipwreck_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Route {
        npc: "jack_fate",
        keyword: "thais",
        fare: 180,
        normal: (32310, 32210, 6),
        alternate: (32161, 32558, 6),
        confirmation: 11,
    })
}

struct Route {
    npc: &'static str,
    keyword: &'static str,
    fare: u64,
    normal: (i32, i32, i16),
    alternate: (i32, i32, i16),
    // The fixed fixture session and Character produce face one at this command ordinal.
    confirmation: u64,
}

fn scenario(route: Route) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("rngroute-paid", move || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "rngroute_paid", true, false).await?;
            let result = exercise(&h, route).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness, route: Route) -> TestResult {
    prepare_current_session(h).await?;
    seed_draw_root(h, 93).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let mut points = Vec::new();
    for (x, y, floor) in [route.normal, route.alternate] {
        if !matches!((x, y, floor), (32310, 32210, 6) | (32161, 32558, 6)) {
            points.push((u16::try_from(x)?, u16::try_from(y)?, -i8::try_from(floor)?));
        }
    }
    let bundle = travel_world(world, &points)?;
    let start = bundle.start();
    let base = pin.qualified_room().clone();
    let cells = bundle
        .movement_cells(base.movement_cells())
        .map_err(debug)?;
    let room = base
        .bundle_view(
            cells.clone(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            NativeEntryStart {
                x: start.x.into(),
                y: start.y.into(),
                floor: 6,
            },
        )
        .ok_or("bundle room")?;
    let mut deps = Dependencies::with_content_and_capacity(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (start.x.into(), start.y.into(), 6),
        ),
        base,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(controller.active().ok_or("active catalogue")?);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    let npc_key = format!("oteryn:npc.{}", route.npc);
    let mut entry = book.get(&npc_key).ok_or("captain entry")?.clone();
    let mut sailing = entry
        .travel_routes
        .first()
        .ok_or("captain route template")?
        .clone();
    sailing.service = format!("oteryn:service.travel.{}", route.npc);
    sailing.keyword = route.keyword.into();
    sailing.route.key = route.keyword.into();
    sailing.route.price = route.fare;
    // The fixture uses ordinary access so the test isolates the route outcome.
    sailing.route.premium = false;
    sailing.held = false;
    sailing.route.required_quest_tracks.clear();
    sailing.quest_gate = None;
    sailing.yalahar_gate = false;
    sailing.yalahar_town_gate = None;
    (
        sailing.route.destination.x,
        sailing.route.destination.y,
        sailing.route.destination.floor,
    ) = route.normal;
    entry.travel_routes = vec![sailing];
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(book.with(&npc_key, entry)));
    let session = deps.admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc = {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let placed = runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: npc_key,
                        placements: vec![NpcPlacement {
                            x: start.x + 1,
                            y: start.y,
                            floor: start.floor,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let actor = placed.actors.first().ok_or("captain actor")?;
        EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        }
    };
    super::item_look_fixtures::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i3031", 100),
            ("oteryn:item.tibia.i3031", 100),
        ],
    )
    .await?;
    assert!(owner.ensure_source_map_initialized().await);
    talk(&owner, &deps, npc, 1, "hi").await?;
    talk(&owner, &deps, npc, 2, route.keyword).await?;
    let result = talk(&owner, &deps, npc, route.confirmation, "yes").await?;
    assert!(result.inventory_changed);
    let destination = route.alternate;
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    assert_eq!((position.x, position.y, position.floor), destination);
    let (roll, selected, arrived): (i16, Vec<i32>, bool) = sqlx::query_as(
        "SELECT random_draw, random_destination, arrived_at IS NOT NULL FROM game_newhaven_travel_receipts",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        roll, 1,
        "the fixture ordinal must select the alternate destination"
    );
    assert_eq!(
        selected,
        [destination.0, destination.1, i32::from(destination.2)]
    );
    assert!(arrived);
    Ok(())
}

#[test]
fn rngroute_travel_commit_replays_the_pending_destination_and_keeps_one_receipt() -> TestResult {
    use super::npcfix_m38_travel_guards_fixtures::NoPremium;
    use super::npcfix_m38_travel_return_fixtures::request;

    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("rngroute-replay", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "rngroute_replay", true, false).await?;
            let result: TestResult = async {
                prepare_current_session(&h).await?;
                seed_draw_root(&h, 93).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
                let owner = deps.owner(&h, &authority);
                let fence = owner.current_quest_fence(deps.admitted.game_session_id).await
                    .map_err(|()| "fence")?.ok_or("live fence")?;
                let mut request = request(&owner, &deps, 11, "jack_fate").await?;
                request.route.route.key = "thais".into();
                request.route.keyword = "thais".into();
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let first = slot.commit_npc_travel(
                    owner.root, owner.character, owner.holder, fence, request.clone(), &NoPremium,
                ).await.map_err(debug)?.map_err(debug)?.ok_or("first pending arrival")?;
                assert_eq!((first.destination.x, first.destination.y, first.destination.floor), (32161, 32558, 6));
                // With a different root the same ordinal would select another face.
                seed_draw_root(&h, 7).await?;
                let replay = slot.commit_npc_travel(
                    owner.root, owner.character, owner.holder, fence, request, &NoPremium,
                ).await.map_err(debug)?.map_err(debug)?;
                assert_eq!(replay, Some(first));
                let counts: (i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_newhaven_travel_receipts), (SELECT count(*) FROM game_character_pending_arrivals)",
                ).fetch_one(&h.pool).await?;
                assert_eq!(counts, (1, 1));
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    })
}

async fn seed_draw_root(h: &Harness, seed: u8) -> TestResult {
    // Let the same test reach placement on the base, where the draw root does not exist.
    let installed: bool =
        sqlx::query_scalar("SELECT to_regclass('game_receipt_random_root') IS NOT NULL")
            .fetch_one(&h.pool)
            .await?;
    if installed {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        let changed =
            sqlx::query("UPDATE game_receipt_random_root SET secret = $1 WHERE singleton")
                .bind(vec![seed; 32])
                .execute(&mut *tx)
                .await?;
        assert_eq!(changed.rows_affected(), 1);
        tx.commit().await?;
    }
    Ok(())
}

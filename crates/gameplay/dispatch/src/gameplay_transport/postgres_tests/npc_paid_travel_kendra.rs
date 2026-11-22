//! Kendra's live route hold and Crystal fare fixture through the durable NPC_TALK owner.
use super::fixture::{Harness, debug, id, run};
use super::npc_paid_travel_fixtures::talk;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const NPC: &str = "oteryn:npc.kendra";
const ROUTE: &str = "oteryn:service.travel.kendra/thais";
const GOLD: &str = "oteryn:item.tibia.i3031";
// Crystal 201189f8: kendra_thais.lua (Nostalgia landing) and kendra.lua:65–70 (Thais).
const NOSTALGIA: (i32, i32, i16) = (33494, 31059, 7);
const THAIS: (i32, i32, i16) = (32259, 32262, 7);

#[derive(Clone, Copy)]
enum RoutePolicy {
    LiveCatalogue,
    CrystalPaidFixture,
}

#[test]
fn kendra_thais_anniversary_route_does_not_quote_or_charge_outside_the_event() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("kendra-held", move || {
        run(async move |_| scenario(admin, RoutePolicy::LiveCatalogue).await)
    })
}

#[test]
fn kendra_thais_live_passage_uses_postman_fare_and_exact_arrival() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("kendra-thais", move || {
        run(async move |_| scenario(admin, RoutePolicy::CrystalPaidFixture).await)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED kendra_thais_live_passage_uses_postman_fare_and_exact_arrival {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn scenario(admin: String, policy: RoutePolicy) -> TestResult {
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world_id, None);
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let table = kendra_placement()?;
    let world = super::qc_pinned_world::world(native)?;
    for (x, y, floor) in [NOSTALGIA, THAIS] {
        let tile = crate::map::overlay::TilePos {
            x: u16::try_from(x)?,
            y: u16::try_from(y)?,
            floor: -i8::try_from(floor)?,
        };
        assert!(world.enterable(tile), "pinned World landing {tile:?}");
    }

    let room = pin.qualified_room().clone();
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let deps = Dependencies::with_content(
        ChannelContentPin::from_activation(
            world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            NOSTALGIA,
        ),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let h = Harness::create_with_admission_seed(admin, "kendra_thais", true, false).await?;
    prepare_current_session(&h).await?;
    super::item_look_fixtures::seed_main_backpack(&h, &[(GOLD, 100), (GOLD, 100)]).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock.clone()),
        Some(activation.clone()),
        Some(h.root.clone()),
    );
    let free = SpellPremiumCoordinator::new(
        Arc::default(),
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.premium_coordinator = Some(&free);
    deps.initialize(&owner).await;

    let catalogue = NpcDataCatalogue::load(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let mut book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    if matches!(policy, RoutePolicy::CrystalPaidFixture) {
        // Keep the exact donor fare/arrival coverage in an explicitly test-only book.
        let mut entry = book.get(NPC).cloned().ok_or("Kendra talk entry")?;
        let route = entry
            .travel_routes
            .iter_mut()
            .find(|route| route.route.key == "thais")
            .ok_or("Crystal Thais route")?;
        route.held = false;
        book = book.with(NPC, entry);
    }
    let npc = {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let placed = runtime
            .place_npcs(&table, |key| book.get(key).is_some(), true)
            .map_err(debug)?;
        let actor = placed.actors.first().ok_or("live Kendra actor")?;
        EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        }
    };
    let session = deps.admitted.game_session_id;
    owner.states.chat.install_npc_talk(Arc::new(book));
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    owner.admit_premium(id(40), session);

    talk(&owner, &deps, npc, 1, "hi").await?;
    if matches!(policy, RoutePolicy::LiveCatalogue) {
        owner.premium_coordinator = Some(&premium);
        let refused = talk(&owner, &deps, npc, 2, "thais").await?;
        assert_eq!(
            refused.conversation.ok_or("route hold reply")?.lines,
            ["I'm sorry but I don't sail there."]
        );
        let confirmed = talk(&owner, &deps, npc, 3, "yes").await?;
        assert!(!confirmed.inventory_changed);
        assert_eq!(live_position(&deps).await?, NOSTALGIA);
        assert_eq!(carried_gold(&h).await?, 200);
        let effects: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_newhaven_travel_receipts),
                    (SELECT count(*) FROM game_character_pending_arrivals)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(effects, (0, 0));
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }

    let refused = talk(&owner, &deps, npc, 2, "thais").await?;
    assert!(
        refused
            .conversation
            .ok_or("Premium reply")?
            .lines
            .join(" ")
            .contains("premium account")
    );
    let refused = talk(&owner, &deps, npc, 3, "yes").await?;
    assert!(!refused.inventory_changed);
    assert_eq!(live_position(&deps).await?, NOSTALGIA);

    owner.premium_coordinator = Some(&premium);
    let base = talk(&owner, &deps, npc, 4, "thais").await?;
    assert_eq!(
        base.conversation.ok_or("base quote")?.lines,
        ["Do you seek a passage to thais for 180 gold?"]
    );
    seed_grand_postman_rank(&h).await?;
    owner.clock_origin -= Duration::from_secs(1);
    let discounted = talk(&owner, &deps, npc, 5, "thais").await?;
    assert_eq!(
        discounted.conversation.ok_or("Postman quote")?.lines,
        ["Do you seek a passage to thais for 170 gold?"]
    );

    let arrived = talk(&owner, &deps, npc, 6, "yes").await?;
    assert!(!arrived.unknown);
    assert!(arrived.inventory_changed);
    assert_eq!(live_position(&deps).await?, THAIS);
    let saved: (i32, i32, i16) = sqlx::query_as(
        "SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(saved, THAIS);
    let receipt: (String, i64, i64, bool, bool) = sqlx::query_as(
        "SELECT route, fare_gold_units, base_fare_gold_units, postman_discount_applied, arrived_at IS NOT NULL FROM game_newhaven_travel_receipts",
    ).fetch_one(&h.pool).await?;
    assert_eq!(receipt, (ROUTE.to_owned(), 170, 180, true, true));
    assert_eq!(carried_gold(&h).await?, 30);
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(pending, 0);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn carried_gold(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_container_entries e
         JOIN game_item_instances i USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(GOLD)
    .fetch_one(&h.pool)
    .await?)
}

fn kendra_placement() -> TestResult<NpcPlacementTable> {
    let (bytes, _) = super::qc_pinned_world::artifact()?;
    let frame = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(())).map_err(debug)?;
    let kendra = frame
        .npcs
        .npcs
        .into_iter()
        .find(|npc| npc.key == NPC)
        .ok_or("Kendra's Nostalgia placement absent from the pinned World")?;
    let [placement] = kendra.placements.as_slice() else {
        return Err("expected one Kendra placement".into());
    };
    // Crystal 201189f8: data-global/world/world-npc.xml:1138–1140.
    assert_eq!(
        (placement.x, placement.y, placement.floor),
        (33494, 31057, -7)
    );
    assert_eq!(
        placement.direction,
        oteryn_world_bundle::npc::Direction::North
    );
    Ok(NpcPlacementTable {
        npcs: vec![NpcPlacements {
            key: kendra.key,
            placements: vec![NpcPlacement {
                x: placement.x,
                y: placement.y,
                floor: placement.floor,
                direction: MovementFacing::North,
            }],
        }],
    })
}

async fn seed_grand_postman_rank(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    // Match the paid-travel matrix's saved quest progress at revision 2.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id,track_key,quest_key,value,committed_character_revision)
         VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/quest/u7_24/the_postman_missions/rank',
                'oteryn:quest.the_postman_missions_quest',3,2)",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn live_position(deps: &Dependencies) -> TestResult<(i32, i32, i16)> {
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    Ok((position.x, position.y, position.floor))
}

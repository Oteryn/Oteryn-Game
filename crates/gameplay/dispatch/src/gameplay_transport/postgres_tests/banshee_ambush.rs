//! Hidden Seal flame starts one durable ambush for a character in a shared room.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const FIRST: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/first_seal";
const DOOR: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/first_seal_door";
const TRIGGER: TilePos = TilePos {
    x: 32278,
    y: 31903,
    floor: -13,
};
const START: TilePos = TilePos {
    x: 32277,
    ..TRIGGER
};
const DESTINATION: TilePos = TilePos {
    x: 32266,
    y: 31849,
    floor: -15,
};
const GHOST_WEST: TilePos = TilePos {
    x: 32274,
    y: 31902,
    ..TRIGGER
};
const GHOST_EAST: TilePos = TilePos {
    x: 32276,
    y: 31902,
    ..TRIGGER
};
const SKELETON: TilePos = TilePos {
    x: 32276,
    y: 31904,
    ..TRIGGER
};
const WITNESS: TilePos = TilePos {
    x: TRIGGER.x + 1,
    ..TRIGGER
};

#[test]
fn banshee_hidden_flame_step_credits_and_spawns_once_with_an_occupied_room_postgres() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hidden Seal ambush scenario panicked")?
        .map_err(Into::into)
}

fn bindings() -> TestResult<Bindings> {
    use super::banshee_circuit::position;
    let callback = serde_json::json!({
        "key":"oteryn:quest-callback/banshee/hidden-flame",
        "quest":QUEST,
        "trigger":"step_in",
        "selector":{"position":position(TRIGGER),"unique":35013},
        "requires_world_admission":true,
        "reversible":false, "passages":[], "offerings":[],
        "return_on_failure":true,
        "storage":[
            {"track":FIRST,"minimum":-1,"maximum":0,"set":1},
            {"track":DOOR,"minimum":-1,"maximum":1,"set":1}
        ],
        "effects":{"teleport_effect":11,"encounter":{
            "phase":"personal_ambush",
            "minimum_level":1,
            "players":[{"from":position(TRIGGER),"to":position(DESTINATION)}],
            "monsters":[
                {"creature":"oteryn:creature.ghost","position":position(GHOST_EAST)},
                {"creature":"oteryn:creature.ghost","position":position(GHOST_WEST)},
                {"creature":"oteryn:creature.demon_skeleton","position":position(SKELETON)}
            ],
            "room_min":position(GHOST_WEST),
            "room_max":position(TilePos {x:WITNESS.x,..SKELETON})
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-1-first_seal_flame.lua:3-20"]
    });
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe_sha256,"sources":callback["sources"],
            "tracks":[
                {"key":FIRST,"initial":-1,"minimum":-1,"maximum":1},
                {"key":DOOR,"initial":-1,"minimum":-1,"maximum":1}
            ],
            "stages":[{"key":"hidden-flame","completes":false,"trigger":{"kind":"step_in","callback":callback}}]
        }]
    }))?)?)
}

fn scenario() -> TestResult {
    use super::appearance_change_postgres::observer_visibility_tests as observer;
    use super::banshee_circuit::{local, scene_world};
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "banshee_hidden_flame", true, false).await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let catalogue =
            bindings()?.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?;
        let world = scene_world(
            START,
            &[(1959, true)],
            &[
                (START, None),
                (TRIGGER, Some((1, Some(35013)))),
                (DESTINATION, None),
                (GHOST_WEST, None),
                (GHOST_EAST, None),
                (SKELETON, None),
                (WITNESS, None),
            ],
            &[],
        )?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let server = generation.identity().server_artifact_digest();
        let client = generation.identity().client_artifact_digest();
        let origin = local(START);
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            server,
            client,
            frame,
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(debug)?;
        let session = deps.admitted.game_session_id;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        let peer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
        let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(peer, local(WITNESS))
            .map_err(debug)?;
        let native = generation.native_gameplay().ok_or("native companions")?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(Arc::new(catalogue));

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server.as_slice())
            .bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
        deps.initialize(&owner).await;
        let actor = deps.actor;
        deps.initialize_player(&owner, peer, peer_session).await;
        for (actor, session) in [(actor, session), (peer, peer_session)] {
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(debug)?
                .ok_or("fence")?;
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut admitted = deps.admitted;
            admitted.game_session_id = session;
            admitted.runtime_actor = Some(actor);
            admitted.item_fence = Some(fence.item_fence());
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
        }

        let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
        let map = owner.bundle_map().ok_or("bundle map")?;
        for (actor, session, start, expected, cohorts) in [
            (actor, session, START, DESTINATION, 1),
            (actor, session, START, START, 1),
            (peer, peer_session, WITNESS, DESTINATION, 2),
            (peer, peer_session, WITNESS, WITNESS, 2),
        ] {
            let (origin, entered) = {
                let mut runtime = owner.runtime.lock().await;
                let mut origin = runtime.read_actor_position(actor).map_err(debug)?;
                let closed = crate::map::floor::BundleDoors(&map.facts);
                if origin.position() == local(DESTINATION) {
                    let proof = crate::map::floor::prepare_quest_callback_teleport(
                        &runtime,
                        owner.movement_cells(),
                        actor,
                        session,
                        origin,
                        local(start),
                        false,
                        &closed,
                    )
                    .map_err(debug)?;
                    origin = runtime.commit_source_step(proof).map_err(debug)?;
                }
                let direction = if start.x < TRIGGER.x {
                    crate::movement::CardinalStep::East
                } else {
                    crate::movement::CardinalStep::West
                };
                let proof = crate::map::floor::prepare_bundle_step(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    origin,
                    direction,
                    false,
                    &closed,
                )
                .map_err(debug)?;
                assert_eq!(proof.destination(), local(TRIGGER));
                let entered = runtime.commit_source_step(proof).map_err(debug)?;
                (origin, entered)
            };
            let after = owner
                .step_quest_callback(&permit, actor, session, origin, entered)
                .await;
            assert_eq!(after.position(), local(expected));
            let runtime = owner.runtime.lock().await;
            let mut positions = runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .map(|creature| {
                    runtime
                        .read_actor_position(creature)
                        .map(|entry| entry.position())
                        .map_err(debug)
                })
                .collect::<Result<Vec<_>, _>>()?;
            positions.sort_by_key(|at| (at.x, at.y, at.floor));
            let expected_spawns: Vec<_> = [GHOST_WEST, GHOST_EAST, SKELETON]
                .into_iter()
                .flat_map(|at| std::iter::repeat_n(local(at), cohorts))
                .collect();
            assert_eq!(positions, expected_spawns);
            if actor != peer {
                assert_eq!(
                    runtime.read_actor_position(peer).map_err(debug)?.position(),
                    local(WITNESS)
                );
            }
        }
        let progress: Vec<(String,i64,i64)> = sqlx::query_as(
            "SELECT track_key, value, count(*) FROM game_character_quest_tracks WHERE quest_key=$1 GROUP BY track_key,value ORDER BY track_key"
        ).bind(QUEST).fetch_all(&h.pool).await?;
        assert_eq!(
            progress,
            [(FIRST.to_owned(), 1, 2), (DOOR.to_owned(), 1, 2)]
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

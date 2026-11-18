//! Demonrage sequence and Warlock plate through the shared USE and StepIn owners.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, CharacterId, CharacterLease};
use crate::gameplay_transport::{
    ComposedFreshAdmission, StepDirection, StepDisposition, connection::UseCommand,
};
use crate::map::{
    boot::BundleCollisionIndexExt,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const THIRD: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/third_seal";
const DOOR: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/third_seal_door";
const WARLOCKS: &str =
    "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/third_seal_warlocks";
const START: TilePos = TilePos {
    x: 32219,
    y: 31844,
    floor: -15,
};
const FLAME: TilePos = TilePos {
    x: 32215,
    y: 31849,
    ..START
};
const DESTINATION: TilePos = TilePos {
    x: 32271,
    y: 31857,
    ..START
};
const PLATE: TilePos = TilePos {
    x: 32215,
    y: 31838,
    ..START
};
const EAST_PLATE: TilePos = TilePos {
    x: PLATE.x + 1,
    ..PLATE
};
const SWITCHES: [TilePos; 5] = [
    TilePos {
        x: 32220,
        y: 31843,
        ..START
    },
    TilePos {
        x: 32220,
        y: 31845,
        ..START
    },
    TilePos {
        x: 32220,
        y: 31844,
        ..START
    },
    TilePos {
        x: 32220,
        y: 31842,
        ..START
    },
    TilePos {
        x: 32220,
        y: 31846,
        ..START
    },
];
// Crystal data-global/lib/quests/the_queen_of_the_banshees.lua:33-80.
const SWITCH_CUES: [&[(u16, u16)]; 5] = [
    &[
        (32217, 31845),
        (32218, 31845),
        (32219, 31845),
        (32220, 31845),
        (32217, 31843),
        (32218, 31842),
        (32219, 31841),
    ],
    &[
        (32217, 31844),
        (32218, 31844),
        (32219, 31843),
        (32220, 31845),
        (32219, 31845),
    ],
    &[
        (32217, 31842),
        (32219, 31843),
        (32219, 31845),
        (32218, 31844),
        (32217, 31844),
        (32217, 31845),
    ],
    &[
        (32217, 31845),
        (32218, 31846),
        (32218, 31844),
        (32219, 31845),
        (32220, 31846),
    ],
    &[
        (32219, 31841),
        (32219, 31842),
        (32219, 31846),
        (32217, 31843),
        (32217, 31844),
        (32217, 31845),
        (32218, 31843),
        (32218, 31845),
    ],
];
const BASINS: [TilePos; 3] = [
    TilePos {
        x: 32214,
        y: 31850,
        ..START
    },
    TilePos {
        x: 32215,
        y: 31850,
        ..START
    },
    TilePos {
        x: 32216,
        y: 31850,
        ..START
    },
];
const SPAWNS: [TilePos; 2] = [
    TilePos {
        x: 32215,
        y: 31835,
        ..START
    },
    TilePos {
        x: 32215,
        y: 31840,
        ..START
    },
];

#[test]
fn banshee_demonrage_use_sequence_resets_and_step_spawns_warlocks_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Demonrage scenario panicked")?
        .map_err(Into::into)
}

fn bindings() -> TestResult<Bindings> {
    use super::banshee_circuit::position;
    let mut stages = Vec::new();
    for (index, at) in SWITCHES.into_iter().enumerate() {
        let requirements: Vec<_> = SWITCHES
            .into_iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .map(|(other, other_at)| {
                serde_json::json!({
                    "position":position(other_at),"appearance":if other < index {2772} else {2773}
                })
            })
            .collect();
        let last = index + 1 == SWITCHES.len();
        let passages: Vec<_> = BASINS
            .into_iter()
            .filter(|_| last)
            .map(|basin| {
                serde_json::json!({
                    "position":position(basin),"appearance":2114,"replacement":2113,
                })
            })
            .collect();
        let switch_magic: Vec<_> = SWITCH_CUES[index]
            .iter()
            .map(|&(x, y)| serde_json::json!({"position":position(TilePos {x,y,floor:-14}),"effect":12}))
            .collect();
        let callback = serde_json::json!({
            "key":format!("oteryn:quest-callback/banshee/demonrage-switch-{index}"),
            "quest":QUEST,"trigger":"use","selector":{"position":position(at),"unique":30013+index},
            "requires_world_admission":true,"lever":[2773,2772],"reversible":false,
            "return_on_failure":false,"offerings":[],"passages":passages,
            "world_requirements":requirements,
            "storage":[{"track":THIRD,"minimum":-1,"maximum":0,"set":0,"read_only":true}],
            "effects":{"visual_passages":last,"requirement_failure_magic":12,
                "sequence_prime":"oteryn:switch-sequence/demonrage",
                "magic":{"position":position(at),"effect":13},
                "switch_magic":switch_magic},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-3-third_seal_lever.lua:3-39"]
        });
        stages.push(serde_json::json!({
            "key":format!("switch-{index}"),"completes":false,"trigger":{"kind":"map_use","callback":callback}
        }));
    }
    let requirements: Vec<_> = SWITCHES
        .into_iter()
        .map(|at| {
            serde_json::json!({
                "position":position(at),"appearance":2772,
            })
        })
        .collect();
    let passages: Vec<_> = SWITCHES.into_iter().map(|at| (at,2772,2773))
        .chain(BASINS.into_iter().map(|at| (at,2113,2114)))
        .map(|(at,from,to)| serde_json::json!({"position":position(at),"appearance":from,"replacement":to}))
        .collect();
    let flame = serde_json::json!({
        "key":"oteryn:quest-callback/banshee/demonrage-flame","quest":QUEST,
        "trigger":"step_in","selector":{"position":position(FLAME),"unique":35015},
        "requires_world_admission":true,"reversible":false,"return_on_failure":true,"offerings":[],
        "passages":passages,"world_requirements":requirements,"teleport":position(DESTINATION),
        "storage":[{"track":THIRD,"minimum":-1,"maximum":0,"set":1},
                   {"track":DOOR,"minimum":-1,"maximum":1,"set":1}],
        "effects":{"visual_passages":true,"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-3-third_seal_flame.lua:3-28"]
    });
    let plate = serde_json::json!({
        "key":"oteryn:quest-callback/banshee/demonrage-warlocks","quest":QUEST,
        "trigger":"step_in","selector":{"position":position(PLATE),"position_aliases":[position(EAST_PLATE)],"action":25023,"appearance":431},
        "requires_world_admission":true,"reversible":false,"return_on_failure":false,"offerings":[],"passages":[],
        "storage":[{"track":WARLOCKS,"minimum":-1,"maximum":0,"set":1}],
        "effects":{"magic":{"position":position(PLATE),"effect":14},"encounter":{
            "phase":"personal_ambush","minimum_level":1,"players":[],
            "monsters":[{"creature":"oteryn:creature.warlock","position":position(SPAWNS[0])},
                        {"creature":"oteryn:creature.warlock","position":position(SPAWNS[1])}],
            "room_min":position(SPAWNS[0]),"room_max":position(SPAWNS[1])
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-3-third_seal_warlock_tile.lua:3-14"]
    });
    for (key, callback) in [("flame", flame), ("warlocks", plate)] {
        stages.push(serde_json::json!({
            "key":key,"completes":false,"trigger":{"kind":"step_in","callback":callback}
        }));
    }
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let tracks: Vec<_> = [THIRD, DOOR, WARLOCKS]
        .into_iter()
        .map(|key| {
            serde_json::json!({
                "key":key,"initial":-1,"minimum":-1,"maximum":1
            })
        })
        .collect();
    Ok(Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe_sha256,
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-3-third_seal_lever.lua:3"],
            "tracks":tracks,"stages":stages
        }]
    }))?)?)
}

fn scenario() -> TestResult {
    use super::banshee_circuit::{local, scene_world};
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "banshee_demonrage", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let catalogue =
            bindings()?.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?;
        let approach = |at: TilePos| TilePos { x: at.x - 1, ..at };
        let mut tiles = vec![
            (approach(FLAME), None),
            (approach(PLATE), None),
            (FLAME, Some((4, Some(35015)))),
            (DESTINATION, None),
            (PLATE, Some((6, None))),
            (EAST_PLATE, Some((6, None))),
        ];
        tiles.extend(
            SWITCHES
                .into_iter()
                .enumerate()
                .map(|(index, at)| Ok((at, Some((1, Some(30013 + u16::try_from(index)?))))))
                .collect::<TestResult<Vec<_>>>()?,
        );
        tiles.extend(
            SWITCHES
                .into_iter()
                .map(|at| (TilePos { x: START.x, ..at }, None)),
        );
        tiles.extend(BASINS.into_iter().map(|at| (at, Some((3, None)))));
        tiles.extend(SPAWNS.into_iter().map(|at| (at, None)));
        let world = scene_world(
            START,
            &[
                (2773, true),
                (2772, true),
                (2114, false),
                (1959, true),
                (2113, false),
                (431, true),
            ],
            &tiles,
            &[(PLATE, 25023, None), (EAST_PLATE, 25023, None)],
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
        admitted.runtime_actor = Some(actor);
        admitted.item_fence = Some(fence.item_fence());
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;

        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err("bundle collision index".into());
        };
        let map = index.map();
        let item = |at: TilePos| -> TestResult<(u64, u16)> {
            let key = placement_key(at, 1).ok_or("object placement")?;
            let tile = map
                .overlay
                .base()
                .tile(at.x, at.y, at.floor)
                .ok_or("tile")?;
            let id = *tile.ids().get(1).ok_or("object")?;
            let entry = map.facts.base_entry(at, 1, id).ok_or("object facts")?;
            Ok((key, entry.appearance_id))
        };
        let move_actor =
            async |owner: &ComposedFreshAdmission<'_, '_, '_>, at: TilePos| -> TestResult<_> {
                let mut runtime = owner.runtime.lock().await;
                let origin = runtime.read_actor_position(actor).map_err(debug)?;
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    origin,
                    local(at),
                    false,
                    &closed,
                )
                .map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)
            };
        let step_onto =
            async |owner: &mut ComposedFreshAdmission<'_, '_, '_>, at| -> TestResult<_> {
                let origin = move_actor(owner, approach(at)).await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("advance step clock")?;
                let (outcome, _) = owner
                    .ordinary_paced_step(actor, session, StepDirection::East, Some(origin), None)
                    .await;
                assert_eq!(outcome.disposition, StepDisposition::Moved);
                let position = owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?;
                Ok(position.position())
            };
        for attempt in 0..=6 {
            let index = match attempt {
                0 => 0,
                1 => 1,
                _ => attempt - 2,
            };
            let at = SWITCHES[index];
            move_actor(&owner, TilePos { x: START.x, ..at }).await?;
            let (key, _) = item(at)?;
            let effect_cursor = map
                .facts
                .movement_effects_since(0)
                .last()
                .map_or(0, |cue| cue.0);
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("advance action clock")?;
            let outcome = owner
                .use_world_object(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: u64::try_from(attempt + 1)?,
                        item_fence: Some(fence.item_fence()),
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: map.facts.object_revision(key),
                    },
                )
                .await;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            assert_eq!(item(at)?.1, if attempt < 2 { 2773 } else { 2772 });
            let cues: Vec<_> = map
                .facts
                .movement_effects_since(effect_cursor)
                .into_iter()
                .map(|(_, at, effect)| (at, effect))
                .collect();
            match attempt {
                0 => assert!(cues.is_empty(), "priming emits no effect"),
                1 => assert_eq!(
                    cues,
                    [(at, 12)],
                    "out-of-order press emits only the refusal"
                ),
                _ => {
                    let expected: Vec<_> = std::iter::once((at, 13))
                        .chain(
                            SWITCH_CUES[index]
                                .iter()
                                .map(|&(x, y)| (TilePos { x, y, floor: -14 }, 12)),
                        )
                        .collect();
                    assert_eq!(cues, expected);
                }
            }
        }
        for basin in BASINS {
            assert_eq!(item(basin)?.1, 2113);
        }
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(before, 0, "shared switches only read personal eligibility");
        assert_eq!(step_onto(&mut owner, FLAME).await?, local(DESTINATION));
        for at in SWITCHES {
            assert_eq!(item(at)?.1, 2773);
        }
        for at in BASINS {
            assert_eq!(item(at)?.1, 2114);
        }

        let effect_cursor = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |cue| cue.0);
        for plate in [EAST_PLATE, PLATE] {
            assert_eq!(step_onto(&mut owner, plate).await?, local(plate));
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
            positions.sort_by_key(|position| position.y);
            assert_eq!(positions, SPAWNS.map(local));
        }
        let cues: Vec<_> = map
            .facts
            .movement_effects_since(effect_cursor)
            .into_iter()
            .map(|(_, at, effect)| (at, effect))
            .collect();
        assert_eq!(
            cues,
            [(EAST_PLATE, 14)],
            "only the first crossing emits a cue"
        );

        let tracks: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key")
            .bind(QUEST).fetch_all(&h.pool).await?;
        assert_eq!(
            tracks,
            [THIRD, DOOR, WARLOCKS].map(|key| (key.to_owned(), 1))
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

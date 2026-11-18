//! Plague entrance jointly consumes the two ground pearls and preserves quest progress.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, CharacterId, CharacterLease};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const SECOND: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/second_seal";
const TRIGGER: TilePos = TilePos {
    x: 32176,
    y: 31869,
    floor: -15,
};
const START: TilePos = TilePos {
    y: TRIGGER.y + 1,
    ..TRIGGER
};
const DESTINATION: TilePos = TilePos {
    y: TRIGGER.y - 6,
    ..TRIGGER
};
const EAST_TRIGGER: TilePos = TilePos {
    x: TRIGGER.x + 1,
    ..TRIGGER
};
const EAST_START: TilePos = TilePos {
    y: START.y,
    ..EAST_TRIGGER
};
const EAST_DESTINATION: TilePos = TilePos {
    y: DESTINATION.y,
    ..EAST_TRIGGER
};
const PEARL_POSITIONS: [TilePos; 2] = [
    TilePos {
        x: 32173,
        y: 31871,
        ..TRIGGER
    },
    TilePos {
        x: 32180,
        y: 31871,
        ..TRIGGER
    },
];

#[test]
fn banshee_plague_step_consumes_one_of_each_pearl_without_seal_progress_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Plague Seal scenario panicked")?
        .map_err(Into::into)
}

fn bindings() -> TestResult<Bindings> {
    use super::banshee_circuit::position;
    let callback = serde_json::json!({
        "key":"oteryn:quest-callback/banshee/plague-entrance",
        "quest":QUEST, "trigger":"step_in",
        "selector":{"action":35002,"appearance":1949},
        "requires_world_admission":true, "reversible":false, "passages":[],
        "return_on_failure":true, "teleport":position(DESTINATION),
        "storage":[{"track":SECOND,"minimum":-1,"maximum":0,"set":0,"preserve":true}],
        "offerings":[
            {"position":position(PEARL_POSITIONS[0]),"item":"oteryn:item.tibia.i3026","count":1},
            {"position":position(PEARL_POSITIONS[1]),"item":"oteryn:item.tibia.i3027","count":1}
        ],
        "effects":{"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-2-second_seal_pearl.lua:3-33"]
    });
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe_sha256,"sources":callback["sources"],
            "tracks":[{"key":SECOND,"initial":-1,"minimum":-1,"maximum":1}],
            "stages":[{"key":"plague-entrance","completes":false,"trigger":{"kind":"step_in","callback":callback}}]
        }]
    }))?)?)
}

fn scenario() -> TestResult {
    use super::banshee_circuit::{local, scene_world};
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "banshee_plague_pearls", false, false)
            .await?;
        prepare_current_session(&h).await?;
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
            &[(1949, true)],
            &[
                (START, None),
                (TRIGGER, Some((1, None))),
                (DESTINATION, None),
                (EAST_START, None),
                (EAST_TRIGGER, Some((1, None))),
                (EAST_DESTINATION, None),
                (PEARL_POSITIONS[0], None),
                (PEARL_POSITIONS[1], None),
            ],
            &[
                (TRIGGER, DESTINATION, 35002),
                (EAST_TRIGGER, EAST_DESTINATION, 35002),
            ],
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

        let map = owner.bundle_map().ok_or("bundle map")?;
        let portals = crate::gameplay_transport::quest_callbacks::CallbackPortals {
            inner: &crate::map::floor::BundleDoors(&map.facts),
            map: Some(map),
            callbacks: owner.callback_content(),
        };
        assert!(crate::map::floor::CellBlocker::suppresses_native_teleport(
            &portals,
            crate::content::LogicalCell {
                x: i32::from(TRIGGER.x),
                y: i32::from(TRIGGER.y),
                z: -i32::from(TRIGGER.floor),
            },
        ));
        let content = format!(
            "sha256:{}",
            server
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let pearl = seed_pearl(&h, 0, 3, &frame, map, &content).await?;
        let mut second = None;
        for (start, trigger, destination, remaining) in [
            (START, TRIGGER, START, 3),
            (START, TRIGGER, DESTINATION, 2),
            (EAST_START, EAST_TRIGGER, EAST_DESTINATION, 1),
        ] {
            if remaining == 2 {
                second = Some(seed_pearl(&h, 1, 3, &frame, map, &content).await?);
            }
            let origin = {
                let mut runtime = owner.runtime.lock().await;
                let mut origin = runtime.read_actor_position(actor).map_err(debug)?;
                let closed = crate::map::floor::BundleDoors(&map.facts);
                if origin.position() != local(start) {
                    let approach = crate::map::floor::prepare_quest_callback_teleport(
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
                    origin = runtime.commit_source_step(approach).map_err(debug)?;
                }
                let raw_portal = crate::map::floor::prepare_bundle_step(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    origin,
                    crate::movement::CardinalStep::North,
                    false,
                    &closed,
                )
                .map_err(debug)?;
                assert_eq!(
                    raw_portal.destination(),
                    local(TilePos {
                        y: DESTINATION.y,
                        ..trigger
                    })
                );
                let proof = crate::map::floor::prepare_bundle_step(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    origin,
                    crate::movement::CardinalStep::North,
                    false,
                    &portals,
                )
                .map_err(debug)?;
                assert_eq!(proof.destination(), local(trigger));
                origin
            };
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("advance step clock")?;
            let (outcome, _) = owner
                .ordinary_paced_step(
                    actor,
                    session,
                    crate::gameplay_transport::StepDirection::North,
                    Some(origin),
                    None,
                )
                .await;
            assert_eq!(
                outcome.disposition,
                crate::gameplay_transport::StepDisposition::Moved
            );
            let after = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            assert_eq!(after.position(), local(destination));
            assert_eq!(quantity(&h, &pearl).await?, remaining);
            if let Some(second) = &second {
                assert_eq!(quantity(&h, second).await?, remaining);
            }
        }
        let (track_count, state_count): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1), \
                    (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1)",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            (track_count, state_count),
            (0, 0),
            "the endpoint charges admission without writing seal progress"
        );
        let receipt_guards: Vec<bool> = sqlx::query_scalar(
            "SELECT guard_only FROM game_character_quest_receipts WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(receipt_guards, [true, true]);
        let consumption_counts: Vec<i32> = sqlx::query_scalar("SELECT quantity FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/banshee/plague-entrance' ORDER BY item_instance_id")
            .fetch_all(&h.pool).await?;
        assert_eq!(consumption_counts, [1, 1, 1, 1]);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=$1::uuid",
    )
    .bind(item)
    .fetch_one(&h.pool)
    .await?)
}

async fn seed_pearl(
    h: &Harness,
    offset: usize,
    quantity: i64,
    frame: &[u8; 32],
    map: &crate::map::boot::BundleMap,
    content: &str,
) -> TestResult<String> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let at = PEARL_POSITIONS[offset];
    let spatial =
        crate::durability::map_item_mint_audit::map_item_spatial_position(at.x, at.y, at.floor);
    let key = format!("oteryn:item.tibia.i{}", 3026 + offset);
    let item: String = sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,'definition-r1',$3,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
        .bind(id(42).as_slice()).bind(key).bind(quantity).fetch_one(&mut *tx).await?;
    let drop_ref = [
        crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
        id(79).as_slice(),
    ]
    .concat();
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,nextval('game_item_ground_stack_ordinal_seq'))")
        .bind(&item).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(spatial).bind(drop_ref)
        .bind(crate::map::overlay::map_revision(map.overlay.base())).bind(content).bind(frame.as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(item)
}

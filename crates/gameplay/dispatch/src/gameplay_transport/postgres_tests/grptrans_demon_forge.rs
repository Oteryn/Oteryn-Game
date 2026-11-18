//! Crystal Demon Forge entry through the native STEP owner and PostgreSQL quest tracks.
use super::fixture::{Harness, debug, id, run};
use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterLease, MovementLocalPosition, NodeId,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::time::Duration;

const START: MovementLocalPosition = MovementLocalPosition {
    x: 33191,
    y: 31691,
    floor: 14,
};
const FORGE: MovementLocalPosition = MovementLocalPosition {
    x: 33168,
    y: 31683,
    floor: 15,
};

#[test]
fn grptrans_demon_forge_step_accepts_any_exact_throne_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("grptrans-demon-forge", move || {
        run(async move |_| {
            let h =
                Harness::create_with_admission_seed(&admin, "grptrans_forge", false, false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active generation")?;
    let world = native_scene(
        active.native_gameplay().ok_or("native policies")?,
        vec![
            NativeCrop {
                bytes: include_bytes!(
                    "../../../../../../content/world/placements/region-z14-x129-y123.b3"
                ),
                region: (14, 129, 123),
                bounds: (33191, 31691, 33193, 31692),
                start: (33191, 31691),
                expected: vec![ExpectedTile {
                    position: (33192, 31691),
                    ids: vec![436, 1949],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: include_bytes!(
                    "../../../../../../content/world/placements/region-z15-x129-y123.b3"
                ),
                region: (15, 129, 123),
                bounds: (33168, 31683, 33169, 31684),
                start: (33168, 31683),
                expected: vec![ExpectedTile {
                    position: (33168, 31683),
                    ids: vec![4564],
                    counted_entry: None,
                }],
            },
        ],
        &[],
    )?;
    let cells = world
        .movement_cells(activation.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        activation.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (START.x, START.y, START.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, START)
        .map_err(debug)?;
    let fence = crate::gameplay_transport::item_fence_of(
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0,
    )
    .ok_or("current item fence")?;
    runtime
        .bind_attacker_lease(
            deps.actor,
            session,
            CharacterLease::new(fence.character_id, fence.character_lease_generation)
                .map_err(debug)?,
        )
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    {
        let runtime = deps.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                deps.actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    level: 100,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("live character facts")?;
    }
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
        SELECT character_id,'oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline','oteryn:quest.the_inquisition_quest',20,character_revision
        FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    for (throne, expected) in [(None, START), (Some(1_i64), START), (Some(10_i64), FORGE)] {
        if let Some(value) = throne {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                SELECT character_id,'oteryn:quest-progress/storage/41026','oteryn:quest.the_pits_of_inferno_quest',$2,character_revision
                FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
                ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
                .bind(id(41).as_slice()).bind(value).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("clock")?;
        let before = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        let (result, _) = owner
            .ordinary_paced_step(deps.actor, session, StepDirection::East, Some(before), None)
            .await;
        assert_eq!(result.disposition, StepDisposition::Moved);
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            expected,
            "Demon Forge destination with Pumin storage {throne:?}",
        );
    }
    Ok(())
}

//! Four source boss exits teleport without a quest or seasonal prerequisite.
use super::fixture::{Harness, debug, id, run};
use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
use super::threatened_dreams::barriers::relocate;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use std::sync::Arc;
// Exact TeleportUnique source positions, z8 lowered to runtime floor-8.
const TILES: [(u16, u16, i8); 4] = [
    (32013, 32467, -8),
    (32076, 32457, -8),
    (32006, 32395, -8),
    (32077, 32404, -8),
];
const DESTINATIONS: [(u16, u16, i8); 4] = [
    (33234, 32278, -12),
    (33162, 31320, -5),
    (33002, 31595, -11),
    (32278, 31367, -4),
];
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
fn north((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y: y - 1, floor }
}
#[test]
fn first_dragon_four_boss_exits_are_unconditional_native_teleports() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("FIRST_DRAGON_EXITS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("FIRST_DRAGON_EXITS POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("FIRST_DRAGON_EXITS POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"exit scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)
}

fn native_exit_world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    const SOURCE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z08-x125-y126.b3"
    ));
    const LANDINGS: [&[u8]; 4] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z12-x129-y126.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z05-x129-y122.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z11-x128-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z04-x126-y122.b3"
        )),
    ];
    let mut crops = Vec::new();
    for (i, (x, y, _)) in TILES.into_iter().enumerate() {
        crops.push(NativeCrop {
            bytes: SOURCE,
            region: (8, 125, 126),
            bounds: (x, y - 2, x + 1, y + 1),
            start: (x, y - 1),
            expected: vec![ExpectedTile {
                position: (x, y),
                ids: vec![[4408, 231, 11925, 799][i], [8649, 8646, 8648, 8647][i]],
                counted_entry: None,
            }],
        });
    }
    for (i, (x, y, floor)) in DESTINATIONS.into_iter().enumerate() {
        crops.push(NativeCrop {
            bytes: LANDINGS[i],
            region: (floor.unsigned_abs(), x / 256, y / 256),
            bounds: (x, y, x + 1, y + 1),
            start: (x, y),
            expected: Vec::new(),
        });
    }
    native_scene(native, crops, &[])
}

fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "first_dragon_exits", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("FIRST_DRAGON_EXITS POSTGRES_CASE_STARTED server_version={version}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let world = native_exit_world(
            generation
                .native_gameplay()
                .ok_or("native exit generation")?,
        )?;
        let first = TILES[0];
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = north(first);
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, super::quest_world_callbacks::local(start))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(catalogue);
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::item_look::seed_main_backpack(&h, &[]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
        let mut completed = 0;
        let mut command = 1;
        for (i, at) in TILES[..4].iter().copied().enumerate() {
            for _ in 0..2 {
                step(&mut owner, &mut admitted, at, command).await?;
                command += 1;
                let actual = owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                    .map_err(debug)?
                    .position();
                if actual == super::quest_world_callbacks::local(position(DESTINATIONS[i])) {
                    completed += 1;
                }
            }
        }
        let ordinary = north(TILES[0]);
        step(
            &mut owner,
            &mut admitted,
            (ordinary.x, ordinary.y, ordinary.floor),
            command,
        )
        .await?;
        let actual = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        assert_eq!(
            actual,
            super::quest_world_callbacks::local(ordinary),
            "ordinary source tile must not teleport"
        );
        eprintln!(
            "FIRST_DRAGON_EXITS POSTGRES_EVIDENCE real_source_crops=true exits=4 repeated_successes={completed} ordinary_tile_stays=true"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 8,
            "four literal source exit UIDs teleport on every visit without quest gates"
        );
        Ok(())
    })
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: (u16, u16, i8),
    command: u64,
) -> TestResult {
    relocate(owner, admitted, north(at)).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .item_fence(),
    );
    let _payload = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::South,
        ),
    )
    .await?;
    Ok(())
}

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle_compiler::{bundle, compile, source};

const PORTAL: (u16, u16) = (34041, 32330);
const START: (u16, u16) = (34040, 32330);

#[test]
fn doomforge_step_does_not_teleport_without_a_hard_win() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx70-doomforge", || {
        run(async |admin| {
            super::reset();
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let world = compiled_world(native)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let start = crate::content::NativeEntryStart {
                x: i32::from(START.0),
                y: i32::from(START.1),
                floor: 14,
            };
            let room = activated
                .qualified_room()
                .bundle_view(cells.clone(), frame, world.map_revision_digest(), start)
                .ok_or("Doomforge bundle room")?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (start.x, start.y, start.floor),
            );
            let h =
                Harness::create_with_admission_seed(admin, "qcx70_doomforge", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
            )
            .map_err(debug)?;
            let session = deps.admitted.game_session_id;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: start.x,
                        y: start.y,
                        floor: start.floor,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let destination = {
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                owner.movement_cells = (&cells).into();
                owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
                let fence = owner
                    .current_quest_fence(session)
                    .await
                    .map_err(|()| "quest fence")?
                    .ok_or("quest fence")?;
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
                let (step, _) = owner.paced_step(actor, session, StepDirection::East).await;
                assert_eq!(step.disposition, StepDisposition::Moved);
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position()
            };
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            assert_eq!(
                (destination.x, destination.y, destination.floor),
                (i32::from(PORTAL.0), i32::from(PORTAL.1), 14),
                "the actor must stay in the arena without a hard win",
            );
            Ok(())
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx70-doomforge duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn compiled_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    // Compile actual retained regions so provisional decorations receive the
    // same treatment as in the pinned non-production world.
    let project = source::project_regions(&root, &[(132, 126), (133, 126)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let mut pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: vec![],
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: START.0,
                y: START.1,
                floor: -14,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

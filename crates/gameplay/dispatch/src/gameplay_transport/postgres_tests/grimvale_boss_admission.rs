//! The actual source portals through ordinary STEP and Character PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

#[test]
fn curse_spreads_step_admits_bloodback_and_spends_cooldown_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("curse-spreads-step", || scenario(false))
}

#[test]
fn ancient_feud_step_enforces_level_and_spends_cooldown_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("ancient-feud-step", || scenario(true))
}

fn source_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    trial: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTeleport, ExpectedTile, NativeCrop, native_scene};
    let crops = if trial {
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z12-x129-y125.b3"
            )),
            region: (12, 129, 125),
            bounds: (33120, 32238, 33165, 32261),
            start: (33123, 32240),
            expected: vec![ExpectedTile {
                position: (33123, 32239),
                ids: vec![34106],
                counted_entry: None,
            }],
        }]
    } else {
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y124.b3"
                )),
                region: (8, 129, 124),
                bounds: (33165, 31976, 33169, 31980),
                start: (33167, 31978),
                expected: vec![ExpectedTile {
                    position: (33167, 31977),
                    ids: vec![1020, 998, 25049],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y125.b3"
                )),
                region: (8, 129, 125),
                bounds: (33174, 32007, 33192, 32021),
                start: (33182, 32012),
                expected: vec![],
            },
        ]
    };
    let bloodback_exit = [ExpectedTeleport {
        position: (33180, 32011, 8),
        ordinal: 2,
        destination: (33167, 31978, 8),
    }];
    native_scene(native, crops, if trial { &[] } else { &bloodback_exit })
}

fn scenario(trial: bool) -> TestResult {
    let (x, y, floor, boss, landing) = if trial {
        (33123, 32240, 12, "yirkas_blue_scales", (33154, 32246))
    } else {
        (33167, 31978, 8, "bloodback", (33182, 32012))
    };
    let origin = MovementLocalPosition { x, y, floor };
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "grimvale_boss_step", false, false).await?;
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
        let native = generation.native_gameplay().ok_or("native content")?;
        let world = source_world(native, trial)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        deps.combat_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: origin.x,
                        y: origin.y,
                        floor: origin.floor,
                    },
                )
                .ok_or("qualified Grimvale bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
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
            8,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        super::grizzly_boss::world::activate(
            &h,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
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
        assert!(owner.track_restored(
            admitted.game_session_id,
            deps.actor,
            &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
            fence,
        ));

        let north = encode_step_intent(StepDirection::North);
        if trial {
            super::quest_world_callbacks::wire(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                north.clone(),
            )
            .await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            super::qengine_outfit_stories::earn_level(&owner, &admitted, &authority, 250).await?;
        }
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            north.clone(),
        )
        .await?;
        let runtime = deps.runtime.lock().await;
        let arrived = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            arrived,
            MovementLocalPosition {
                x: landing.0,
                y: landing.1,
                floor
            }
        );
        assert!(
            runtime
                .positioned_melee_census()
                .map_err(debug)?
                .iter()
                .any(|(_, at, session)| {
                    let spawn = if trial {
                        (33154, 32252)
                    } else {
                        (33184, 32016)
                    };
                    session.is_none()
                        && at.position()
                            == MovementLocalPosition {
                                x: spawn.0,
                                y: spawn.1,
                                floor,
                            }
                })
        );
        drop(runtime);
        let cooldown: i64 = sqlx::query_scalar(
            "SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1",
        )
        .bind(format!("oteryn:boss.{boss}"))
        .fetch_one(&h.pool)
        .await?;
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        assert!((35_950_000..=36_000_000).contains(&(cooldown - now)));
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(601)).await;
        owner
            .boss_spawn_pass(&mut Default::default())
            .await
            .map_err(debug)?;
        tokio::time::resume();
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            origin
        );
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            3,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            north,
        )
        .await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            origin
        );
        let after: i64 = sqlx::query_scalar(
            "SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1",
        )
        .bind(format!("oteryn:boss.{boss}"))
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(after, cooldown);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

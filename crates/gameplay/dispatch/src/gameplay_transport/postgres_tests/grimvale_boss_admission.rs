//! The actual source portals through ordinary STEP and Character PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{ChannelRuntimeV1, CharacterLease, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome},
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[allow(
    dead_code,
    reason = "reuse native multi-region cropping without mounting unrelated pickup cases"
)]
#[path = "l6_world_pickups/world.rs"]
mod native_crop;

#[test]
fn curse_spreads_step_admits_bloodback_and_spends_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("curse-spreads-step", move || {
        scenario(admin, false)
    })
}

#[test]
fn ancient_feud_step_enforces_level_and_spends_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("ancient-feud-step", move || scenario(admin, true))
}

fn source_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    trial: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use native_crop::{ExpectedTile, NativeCrop, native_scene};
    const TRIAL: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z12-x129-y125.b3"
    ));
    const CURSE_ENTRANCE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z08-x129-y124.b3"
    ));
    const CURSE_ARENA: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z08-x129-y125.b3"
    ));
    let crops = if trial {
        vec![
            NativeCrop {
                bytes: TRIAL,
                region: (12, 129, 125),
                bounds: (33123, 32239, 33124, 32241),
                start: (33123, 32239),
                expected: vec![ExpectedTile {
                    position: (33123, 32239),
                    ids: vec![34106],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: TRIAL,
                region: (12, 129, 125),
                bounds: (33154, 32246, 33155, 32247),
                start: (33154, 32246),
                expected: vec![],
            },
            NativeCrop {
                bytes: TRIAL,
                region: (12, 129, 125),
                bounds: (33154, 32252, 33155, 32253),
                start: (33154, 32252),
                expected: vec![],
            },
        ]
    } else {
        vec![
            NativeCrop {
                bytes: CURSE_ENTRANCE,
                region: (8, 129, 124),
                bounds: (33167, 31977, 33169, 31979),
                start: (33168, 31977),
                expected: vec![ExpectedTile {
                    position: (33167, 31977),
                    ids: vec![1020, 998, 25049],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: CURSE_ARENA,
                region: (8, 129, 125),
                bounds: (33182, 32012, 33183, 32013),
                start: (33182, 32012),
                expected: vec![],
            },
            NativeCrop {
                bytes: CURSE_ARENA,
                region: (8, 129, 125),
                bounds: (33184, 32016, 33185, 32017),
                start: (33184, 32016),
                expected: vec![],
            },
        ]
    };
    native_scene(native, crops, &[])
}

fn scenario(admin: String, trial: bool) -> TestResult {
    let (x, y, floor, boss, landing) = if trial {
        (33123, 32240, 12, "yirkas_blue_scales", (33154, 32246))
    } else {
        (33168, 31977, 8, "bloodback", (33182, 32012))
    };
    let origin = MovementLocalPosition { x, y, floor };
    let (entry_direction, exit) = if trial {
        (StepDirection::North, origin)
    } else {
        (
            StepDirection::West,
            MovementLocalPosition {
                x: 33167,
                y: 31978,
                floor,
            },
        )
    };
    run(async |_| {
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
        owner.reset_epoch = 1;
        owner.movement_cells = (&cells).into();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice())
        .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        // Use the fixture's active World pin with the real current reset relation.
        sqlx::query(
            "INSERT INTO game_world_reset_epochs
             (world_id, reset_epoch, target_digest, map_revision, server_digest, client_digest,
              frame_digest, calendar, calendar_revision, activated_at, reset_month)
             VALUES (encode($1,'hex')::uuid, 1, $2, $3, $4, $5, $6,
                     'TIBIA_SERVER_SAVE_BERLIN_06', 1, '2031-06-13 12:00:00+00', '2031-06')",
        )
        .bind(admitted.world_id.as_bytes().as_slice())
        .bind(world.map_revision_digest().as_slice())
        .bind(world.map_revision())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(activated.frame_binding().digest().as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES (encode($1,'hex')::uuid, 1)")
            .bind(admitted.world_id.as_bytes().as_slice())
            .execute(&h.pool)
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

        let spawn = if trial {
            (33154, 32252)
        } else {
            (33184, 32016)
        };
        let spawn = MovementLocalPosition {
            x: spawn.0,
            y: spawn.1,
            floor,
        };
        let occupant = if trial {
            None
        } else {
            let mut runtime = deps.runtime.lock().await;
            let session = GameSessionId::decode(&id(51)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let player = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(player, spawn)
                .map_err(debug)?;
            Some(player)
        };
        step(&mut owner, &admitted, entry_direction).await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            origin,
            "an occupied Curse Spreads room or under-level Ancient Feud trial must turn the player back"
        );
        let cooldowns: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_boss_cooldowns")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(cooldowns, 0);
        if let Some(occupant) = occupant {
            deps.runtime
                .lock()
                .await
                .remove_test_actor(occupant)
                .map_err(debug)?;
        } else {
            earn_level(&owner, &admitted, &authority, 250).await?;
        }
        step(&mut owner, &admitted, entry_direction).await?;
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
        let spawned = runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find(|(_, at, session)| session.is_none() && at.position() == spawn)
            .ok_or("no boss spawned at the Crystal cell")?
            .0;
        let creature = runtime.companion_snapshot(spawned).map_err(debug)?;
        assert_eq!(
            creature.state.policy.definition_key,
            format!("oteryn:creature.{boss}")
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
            exit
        );
        step(&mut owner, &admitted, StepDirection::North).await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            exit
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

async fn earn_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    carrying_level: u32,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let target = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == carrying_level)
        .ok_or("carrying-level threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            authority,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(105))
                    .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(
                    target
                        .checked_sub(current.total_experience.get())
                        .ok_or("XP delta")?,
                ),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("paced step clock")?;
    super::maintain_ready(owner.root).await?;
    let (result, _) = owner
        .ordinary_paced_step(
            admitted.runtime_actor.ok_or("admitted runtime actor")?,
            admitted.game_session_id,
            direction,
            None,
            None,
        )
        .await;
    assert_eq!(result.disposition, StepDisposition::Moved);
    Ok(())
}

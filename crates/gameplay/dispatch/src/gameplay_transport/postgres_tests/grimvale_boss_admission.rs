//! The actual source portals through ordinary STEP and Character PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, fresh_fixture, prepare_current_session, reset};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{ChannelRuntimeV1, CharacterLease, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome},
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[path = "appearance_change_postgres/observer_admission_fixture.rs"]
mod observer_admission_fixture;

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

/// Reconstruct only the cells exercised by STEP and arena relocation, using the
/// same grounds and portal IDs as the pinned Crystal placements.
fn source_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    trial: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    use oteryn_world_bundle_compiler::compile::{KeyResolver, Resolution};
    use std::collections::BTreeMap;
    let (floor, start, trigger, exit, landing, spawn, entry_ground, arena_ground, portal) = if trial
    {
        (
            -12,
            (33123, 32240),
            (33123, 32239),
            (33123, 32240),
            (33154, 32246),
            (33154, 32252),
            923,
            10160,
            34106,
        )
    } else {
        (
            -8,
            (33168, 31977),
            (33167, 31977),
            (33167, 31978),
            (33182, 32012),
            (33184, 32016),
            1020,
            1020,
            25049,
        )
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let (registry, _) = oteryn_world_bundle_compiler::source::registry(
        &root,
        &oteryn_world_bundle_compiler::source::Project {
            index,
            regions: Vec::new(),
            families: Default::default(),
        },
    )?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    manifest.palette.clear();
    let mut by_id = BTreeMap::new();
    for id in [entry_ground, arena_ground, portal] {
        if by_id.contains_key(&id) {
            continue;
        }
        let key = format!("oteryn:item.tibia.i{id}");
        let Resolution::Resolved(family, id_ref) = registry.resolve(&key) else {
            return Err(format!("unresolved Grimvale fixture definition {key}").into());
        };
        by_id.insert(id, u32::try_from(manifest.palette.len())?);
        manifest.palette.push(bundle::PaletteEntry {
            terrain: registry.terrain(&key)?,
            key,
            family,
            id: id_ref,
        });
    }
    let mut cells = BTreeMap::from([
        (start, vec![entry_ground]),
        (exit, vec![entry_ground]),
        (landing, vec![arena_ground]),
        (spawn, vec![arena_ground]),
    ]);
    if trial {
        cells.insert(trigger, vec![portal]);
    } else {
        cells.insert(trigger, vec![entry_ground, portal]);
    }
    let mut sectors = BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for ((x, y), ids) in &cells {
        sectors
            .entry((*x / 32, *y / 32))
            .or_default()
            .push(sector::Tile {
                x: *x,
                y: *y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: ids
                    .iter()
                    .map(|id| sector::Item {
                        palette: by_id[id],
                        depth: 0,
                        attrs: Default::default(),
                    })
                    .collect(),
            });
    }
    manifest.world = bundle::Extent {
        min_x: cells
            .keys()
            .map(|(x, _)| *x)
            .min()
            .ok_or("room minimum x")?,
        min_y: cells
            .keys()
            .map(|(_, y)| *y)
            .min()
            .ok_or("room minimum y")?,
        max_x: cells
            .keys()
            .map(|(x, _)| *x)
            .max()
            .ok_or("room maximum x")?
            + 1,
        max_y: cells
            .keys()
            .map(|(_, y)| *y)
            .max()
            .ok_or("room maximum y")?
            + 1,
        floors: vec![floor],
    };
    let mut sectors = sectors
        .into_iter()
        .map(|((sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let map = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&map),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: start.0,
                y: start.1,
                floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            let mut definition = crate::map::boot::native_item_definition(Some(native), key)?;
            // The pinned WorldObject catalogue identifies i25049 as a
            // nonblocking teleport. The sparse fixture has no native destination.
            if key == "oteryn:item.tibia.i25049" {
                definition.solid = Some(false);
            }
            Some(definition)
        },
    )
    .map_err(debug)
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
        let landing = MovementLocalPosition {
            x: landing.0,
            y: landing.1,
            floor,
        };
        let occupant = if trial {
            None
        } else {
            let second = second_player(&owner, &deps, &h, exit).await?;
            step(&mut owner, &second, StepDirection::North).await?;
            assert_eq!(
                position(&owner, &second).await?,
                landing,
                "the first entrant owns the solo room"
            );
            Some(second)
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
        let cooldowns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid"
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(cooldowns, 0);
        if let Some(occupant) = occupant {
            let second_cooldowns: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key='oteryn:boss.bloodback'"
            ).bind(id(101).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(second_cooldowns, 1);
            expire_room(&owner).await?;
            assert_eq!(position(&owner, &occupant).await?, exit);
            assert_eq!(position(&owner, &admitted).await?, origin);
        } else {
            earn_level(&owner, &admitted, &authority, 250).await?;
        }
        step(&mut owner, &admitted, entry_direction).await?;
        let runtime = deps.runtime.lock().await;
        let arrived = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(arrived, landing);
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
            "SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1 AND character_id=encode($2,'hex')::uuid",
        )
        .bind(format!("oteryn:boss.{boss}"))
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        assert!((35_950_000..=36_000_000).contains(&(cooldown - now)));
        expire_room(&owner).await?;
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
            "SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1 AND character_id=encode($2,'hex')::uuid",
        )
        .bind(format!("oteryn:boss.{boss}"))
        .bind(id(41).as_slice())
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

async fn second_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
    origin: MovementLocalPosition,
) -> TestResult<AdmittedSession> {
    h.seed_pvp_target().await?;
    observer_admission_fixture::admit(h).await?;
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "second fence")?
        .ok_or("second current fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: deps.admitted.world_id,
        channel_id: deps.admitted.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])
                .map_err(debug)?,
            account_id: id(100),
        }),
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize_player(owner, actor, session).await;
    assert!(owner.track_restored(
        session,
        actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    Ok(admitted)
}

async fn expire_room(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(601)).await;
    let pass = owner.boss_spawn_pass(&mut Default::default()).await;
    tokio::time::resume();
    pass.map_err(debug)?;
    Ok(())
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("runtime actor")?)
        .map_err(debug)?
        .position())
}

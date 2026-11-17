//! Credited Mage deaths, spectator ejection and damage-dealt reset on PostgreSQL.
use super::fixture::{Harness, debug, id};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::world_event::changes::counters::{self, CounterKind};
use crate::foundation::{
    ChannelContentPin, ChannelId, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    OwnerDamageCommand, ScopeOwnershipGeneration, WorldId,
};
use crate::monster_owner_cycle::respawn_job::MonsterRespawnJob;
use oteryn_gameplay_entities::carrier::WorldSpawnProjection;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn worldchg_mage_tower_credited_energized_death_resurrects_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| scenario(admin).await)
}

async fn scenario(admin: String) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, "mage_tower_resurrection", false, false).await?;
    prepare_current_session(&h).await?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let map = mage_tower_room(world, channel)?;
    let baseline = pin.qualified_room();
    let cells = map
        .movement_cells(baseline.movement_cells())
        .map_err(debug)?;
    let room = super::rust_lottery::bundle_action_room(baseline, &map, world, &cells)?;
    let mut deps = Dependencies::with_content_and_capacity(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (33143, 31529, 2),
        ),
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        16,
    )?;
    deps.combat_room = Some(room);
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    let mut admitted = deps.admitted;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("current gameplay fence")?;
    admitted.item_fence = Some(fence.item_fence());
    let character =
        crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
    let lease = crate::foundation::CharacterLease::new(character, fence.character_lease_generation)
        .map_err(debug)?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(deps.actor, admitted.game_session_id, lease)
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native creature catalogue")?;

    let epoch = 1;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))")
        .bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let day = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("active day")?;
    let kind = CounterKind::MagesTower;
    let source = counters::bindings()?
        .spawn_sources
        .iter()
        .find(|source| source.kind == kind)
        .ok_or("Mage's Tower spawn source")?;
    let key = &source.point.creature;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        runtime.use_classic_creature_deaths().map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let point = &source.point;
        let at = &point.cell;
        let policy = runtime.companion_policy(key).map_err(debug)?;
        let (health, blockable) = (policy.maximum_health, policy.blocked_by_nearby_players);
        runtime
            .begin_activation_spawn_sources(vec![(
                vec![(
                    (at.x.into(), at.y.into(), at.floor.into()),
                    point.respawn_ms,
                    key.clone(),
                    health.try_into()?,
                    blockable,
                )],
                5_000,
            )])
            .map_err(debug)?;
        let activated = runtime.realize_activation_spawn_window(0).map_err(debug)?;
        assert_eq!((activated.realized, activated.remaining), (1, 0));
        runtime
            .project_world_spawn_points(|creature, _| {
                (creature == key).then(|| WorldSpawnProjection {
                    change_key: kind.key().into(),
                    generation: day.counter_generations[kind.index()],
                    creature: Some((key.clone(), health, blockable)),
                })
            })
            .map_err(debug)?;
        runtime
            .install_world_change_cycles(
                epoch,
                CounterKind::ALL
                    .into_iter()
                    .map(|kind| (kind.key().into(), day.counter_generations[kind.index()]))
                    .collect(),
            )
            .map_err(debug)?;
        assert_eq!(runtime.binding().scope_generation(), generation);
        runtime.spawn_point_creatures()[0]
            .2
            .ok_or("Energized Mage")?
    };
    super::engworldchg_journals::kill_actor(&owner, &admitted, target, 9001).await?;
    let credited: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_change_credit_receipts WHERE world_id=encode($1,'hex')::uuid AND source_kind=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(credited, 1);
    let now = owner.owner_now().get();
    let (mut job, replacement) = {
        let mut runtime = owner.runtime.lock().await;
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        let mut job = MonsterRespawnJob::new(&runtime).map_err(debug)?;
        job.pass(&mut runtime, now).map_err(debug)?;
        let replacement = runtime.spawn_point_creatures()[0]
            .2
            .ok_or("immediate resurrection")?;
        assert_ne!(replacement, target);
        assert!(runtime.matches_live_creature_identity(replacement, key.as_bytes()));
        (job, replacement)
    };

    let unix_seconds: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let current = crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
        &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
            root: h.root.clone(),
            node: h.node.clone(),
            recovery: &h.recovery,
        },
        60,
        63,
        70,
        unix_seconds,
    )
    .await?;
    let spectator = {
        let mut runtime = owner.runtime.lock().await;
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 33142,
                    y: 31529,
                    floor: 2,
                },
            )
            .map_err(debug)?;
        actor
    };
    for (character, count) in [
        (character, 250_i64),
        (current.current_character_lease().character_id(), 249),
    ] {
        sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::bigint)")
            .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(character.as_bytes().as_slice()).bind(count)
            .execute(&h.pool).await?;
    }

    // Seed prior Yielothax deaths, then cross the existing transition through the same
    // native kill caller. The Energized form must stay gone as the World enters Raging.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(state,'{counts,0}','2000') WHERE world_id=encode($1,'hex')::uuid AND change_key=$2")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).execute(&h.pool).await?;
    super::engworldchg_journals::kill_actor(&owner, &admitted, replacement, 9002).await?;
    let day = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("active day")?;
    assert_eq!(day.counter_stages[kind.index()], 2);
    let raging = {
        let mut runtime = owner.runtime.lock().await;
        let now = owner.owner_now().get().max(now + 50_000);
        job.pass(&mut runtime, now).map_err(debug)?;
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position()
                .floor,
            2
        );
        assert_eq!(
            runtime
                .read_actor_position(spectator)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 33142,
                y: 31529,
                floor: 3,
            }
        );

        let raging = "oteryn:creature.raging_mage";
        let policy = runtime.companion_policy(raging).map_err(debug)?;
        let health = policy.maximum_health;
        let blockable = policy.blocked_by_nearby_players;
        runtime
            .project_world_spawn_points(|creature, _| {
                (creature == key).then(|| WorldSpawnProjection {
                    change_key: kind.key().into(),
                    generation: day.counter_generations[kind.index()],
                    creature: Some((raging.into(), health, blockable)),
                })
            })
            .map_err(debug)?;
        for time in [now + 50_000, now + 4_250_000] {
            runtime
                .drive_spawn_respawns(time, |_, _| false)
                .map_err(debug)?;
        }
        runtime.spawn_point_creatures()[0].2.ok_or("Raging Mage")?
    };
    exercise_personal_reset(&h, &owner, &admitted, raging).await?;
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn exercise_personal_reset(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    let key = CounterKind::MagesTower.key();
    let queue = owner.states.attack.lock().await.kills().clone();
    for (command, before, after) in [(9003, 249_i64, 249), (9004, 250, 0), (9005, 251, 251)] {
        sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::bigint) ON CONFLICT(world_id,change_key,character_id) DO UPDATE SET counter=EXCLUDED.counter")
            .bind(admitted.world_id.as_bytes().as_slice()).bind(key).bind(character.as_bytes().as_slice())
            .bind(before).execute(&h.pool).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            let hit = runtime
                .borrow_exact_actor_commit_at_us(owner.owner_now().get())
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                    0,
                    OwnerDamageCommand {
                        target: b"oteryn:creature.raging_mage",
                        occurrence: session.as_bytes(),
                        binding: b"mage-tower-personal-qualification",
                        damage: 1,
                    },
                )
                .map_err(debug)?;
            assert!(hit.applied && hit.health_before > hit.health_after && hit.health_after > 0);
            queue.boss_damage_dealt(&runtime, target, actor, session);
        }
        for retry in 0..8 {
            if owner.drain_native_quest_hits().await {
                break;
            }
            assert!(retry < 7, "committed Raging Mage hit should settle");
            h.root.maintain_ready_once().await.map_err(debug)?;
        }
        let count: String = sqlx::query_scalar("SELECT counter::text FROM game_world_change_personal_counters WHERE world_id=encode($1,'hex')::uuid AND change_key=$2 AND character_id=encode($3,'hex')::uuid")
            .bind(admitted.world_id.as_bytes().as_slice()).bind(key).bind(character.as_bytes().as_slice())
            .fetch_one(&h.pool).await?;
        assert_eq!(count.parse::<i64>()?, after);
    }
    Ok(())
}

fn mage_tower_room(
    world: WorldId,
    channel: ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut source = bundle::read(&bytes)?;
    source.manifest.palette.truncate(1);
    source.manifest.world = bundle::Extent {
        min_x: 33142,
        min_y: 31529,
        max_x: 33145,
        max_y: 31530,
        floors: vec![-3, -2],
    };
    let tile = |x| sector::Tile {
        x,
        y: 31529,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }],
    };
    let mut sectors = vec![
        bundle::Sector {
            floor: -3,
            sx: 33142 / 32,
            sy: 31529 / 32,
            tiles: vec![tile(33142)],
        },
        bundle::Sector {
            floor: -2,
            sx: 33142 / 32,
            sy: 31529 / 32,
            tiles: (33142..=33144).map(tile).collect(),
        },
    ];
    sectors.sort_by_key(bundle::order);
    let bytes =
        oteryn_world_bundle::writer::write(&source.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 33143,
                y: 31529,
                floor: -2,
            },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(debug)
}

#[tokio::test]
async fn worldchg_mage_tower_portal_waits_until_deadline_and_restarts_at_save() {
    use oteryn_gameplay_world_change::{counter::mage_tower_portal_open_at_ms, daily::roll_calendar};
    use tokio::time::{Duration, Instant, advance, pause};

    pause();
    let world = id(42);
    let mut day = roll_calendar(&world, 1, 6, 13);
    let activation = Instant::now();
    let delay = mage_tower_portal_open_at_ms(&world, 1, 0);
    let now_ms = || u64::try_from(activation.elapsed().as_millis()).expect("test clock fits u64");
    day.project_mage_tower_portal(&world, 0, now_ms());
    assert!(!day.mage_tower_portal_open());
    advance(Duration::from_millis(delay - 1)).await;
    day.project_mage_tower_portal(&world, 0, now_ms());
    assert!(!day.mage_tower_portal_open());
    advance(Duration::from_millis(1)).await;
    day.project_mage_tower_portal(&world, 0, now_ms());
    assert!(day.mage_tower_portal_open());

    day.counter_stages[CounterKind::MagesTower.index()] = 4;
    assert!(!day.mage_tower_portal_open());
    let mut next_day = roll_calendar(&world, 2, 6, 14);
    next_day.project_mage_tower_portal(&world, now_ms(), now_ms());
    assert!(!next_day.mage_tower_portal_open());
}

#[test]
fn worldchg_mage_tower_portal_delay_is_bounded_and_epoch_stable() {
    use oteryn_gameplay_world_change::counter::mage_tower_portal_open_at_ms;

    let world = id(42);
    let delays = (1..=128)
        .map(|epoch| {
            let delay = mage_tower_portal_open_at_ms(&world, epoch, 0);
            assert!((600_000..=10_800_000).contains(&delay));
            assert_eq!(mage_tower_portal_open_at_ms(&world, epoch, 700), delay + 700);
            delay
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(delays.len() > 100);
}

#[test]
fn worldchg_mage_tower_portal_projects_active_save_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "mage_tower_portal_save", false, false)
            .await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        for (epoch, age, open) in [(1, 0, false), (2, 10_801, true), (3, 0, false)] {
            sqlx::query(
                "INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
                 VALUES(encode($1,'hex')::uuid,$2::bigint,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,
                        clock_timestamp() - make_interval(secs => $4),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))",
            )
            .bind(world.as_bytes().as_slice())
            .bind(epoch)
            .bind([7u8; 32].as_slice())
            .bind(f64::from(age))
            .execute(&h.pool)
            .await?;
            sqlx::query(
                "INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::bigint)
                 ON CONFLICT (world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch",
            )
            .bind(world.as_bytes().as_slice())
            .bind(epoch)
            .execute(&h.pool)
            .await?;
            for _ in 0..2 {
                let day = h.root.world_changes(world, u64::try_from(epoch)?).await?.ok_or("active day")?;
                assert_eq!(day.mage_tower_portal_open(), open, "save {epoch}");
                assert_eq!(day.counter_stages[CounterKind::MagesTower.index()], 1);
            }
        }
        Ok(())
    })
}

//! A credited Mage's Tower death returns its Energized actor on the first respawn drive.
use super::fixture::{Harness, debug, id};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::world_event::changes::counters::{self, CounterKind};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, OwnerDamageCommand, ScopeOwnershipGeneration, WorldId,
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
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        16,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    let mut admitted = deps.admitted;
    owner.admit_progression_session(&mut admitted).await;
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
    exercise_personal_reset(&h, &owner, &admitted, raging).await
}

async fn exercise_personal_reset(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    let character = id(41);
    let key = CounterKind::MagesTower.key();
    let queue = owner.states.attack.lock().await.kills().clone();
    for (command, before, after) in [(9003, 249_i64, 249), (9004, 250, 0), (9005, 251, 251)] {
        sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::bigint) ON CONFLICT(world_id,change_key,character_id) DO UPDATE SET counter=EXCLUDED.counter")
            .bind(admitted.world_id.as_bytes().as_slice()).bind(key).bind(character.as_slice())
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
            .bind(admitted.world_id.as_bytes().as_slice()).bind(key).bind(character.as_slice())
            .fetch_one(&h.pool).await?;
        assert_eq!(count.parse::<i64>()?, after);
    }
    Ok(())
}

//! Real native lethal projection -> existing kill queue -> fenced Quest receipt.
use super::*;
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_change_kills as rules;
use std::sync::Arc;

#[test]
fn wvar_native_kills_postgres_counter_replay_epoch_and_achievement() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("WVAR_NATIVE_KILLS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "native world kill PG thread")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let start = std::time::Instant::now();
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "wvar_native_kills", true, false).await?;
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
        let generation = controller.active().ok_or("native generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let world = deps.admitted.world_id;
        let epoch = (1..100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|day| day.variant("down_the_drain").is_some())
            })
            .ok_or("water epoch")?;
        seed_epoch(
            &h,
            world,
            epoch,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let position = oteryn_gameplay_world_change::variants::definitions()?
            .iter()
            .find(|d| d.key == "down_the_drain")
            .and_then(|d| d.variants.first())
            .and_then(|v| v.spawns.first())
            .ok_or("water source spawn")?
            .position;
        let origin = MovementLocalPosition {
            x: i32::from(position.0),
            y: i32::from(position.1),
            floor: i16::from(position.2),
        };
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            [8; 32],
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
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
        deps.runtime = Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.reset_epoch = epoch;
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        activate(
            &h,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        deps.initialize(&owner).await;
        owner.refresh_quest_session(session, true).await;
        let mut first = None;
        for index in 0..50_u64 {
            let (target, captured) = capture(&owner, deps.actor, session, index + 500).await?;
            first.get_or_insert(captured);
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            owner
                .runtime
                .lock()
                .await
                .remove_encounter_actor(target)
                .map_err(debug)?;
        }
        assert_eq!(counter(&h).await?, 50);
        let rows:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2").bind(id(41).as_slice()).bind(rules::DOWN_THE_DRAIN.transition).fetch_one(&h.pool).await?;
        assert_eq!(rows, 50);
        let context = first.ok_or("captured first native kill")?;
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let replay = slot
            .commit_world_change_kill(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                &permit,
                context,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay,
            crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        drop(slot);
        assert_eq!(counter(&h).await?, 50);
        let achievement: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
        )
        .bind(rules::DOWN_THE_DRAIN.achievement)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(achievement, 1);
        let (target, stale) = capture(&owner, deps.actor, session, 600).await?;
        seed_epoch(
            &h,
            world,
            epoch + 1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(matches!(slot.commit_world_change_kill(owner.root,owner.character,owner.holder,fence,Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),&permit,stale).await,Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
        drop(slot);
        drop(permit);
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(target)
            .map_err(debug)?;
        assert_eq!(counter(&h).await?, 50);
        eprintln!(
            "WVAR_NATIVE_KILLS POSTGRES_CASE_COMPLETED real-native-death,queue,50credits,replay,stale-epoch,achievement duration={:?}",
            start.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn counter(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)").bind(id(41).as_slice()).bind(rules::DOWN_THE_DRAIN.track).fetch_one(&h.pool).await?)
}

async fn activate(h: &Harness, server: [u8; 32], client: [u8; 32], frame: [u8; 32]) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn seed_epoch(
    h: &Harness,
    world: crate::foundation::WorldId,
    epoch: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01') ON CONFLICT DO NOTHING").bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(server.as_slice()).bind((epoch*86400)as f64).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch").bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    crate::durability::world_event::changes::record_reset_fixture(&mut tx, world, epoch).await?;
    tx.commit().await?;
    Ok(())
}
async fn capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    command: u64,
) -> TestResult<(
    ExactActorRef,
    crate::durability::quest_state::world_change_kills::NativeWorldChangeKill,
)> {
    owner.refresh_quest_session(session, false).await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target;
    {
        let mut runtime = owner.runtime.lock().await;
        let position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let key = rules::DOWN_THE_DRAIN.creature;
        let policy = runtime.companion_policy(key).map_err(debug)?;
        target = runtime
            .admit_encounter_actor(
                key,
                policy.maximum_health,
                MovementLocalPosition {
                    x: position.x + 1,
                    ..position
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, key)
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"wvar-native-kill",
                    damage: policy.maximum_health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
    }
    let lease = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
        queue.take(session, lease.generation())
    else {
        return Err("native queue entry".into());
    };
    let context = taken
        .entry()
        .and_then(|e| e.world_change_kill.clone())
        .ok_or("frozen world kill")?;
    drop(taken);
    Ok((target, context))
}

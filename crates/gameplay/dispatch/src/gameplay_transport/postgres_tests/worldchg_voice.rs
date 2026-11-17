use super::*;
use crate::durability::world_event::changes::counters::{CounterKind, CounterState};
use crate::foundation::{
    ChannelId, CharacterId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};

#[path = "world_event/reset_fixture.rs"]
mod event_fixture;

#[test]
fn worldchg_voice_mage_qualification_death_and_empty_restoration() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("worldchg-voice", move || {
        scenario(admin.clone(), 24, 1)?;
        scenario(admin, 25, 4)
    })
}

fn scenario(admin: String, gobbles: i64, expected_stage: u8) -> TestResult {
    run(async move |_| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "worldchg_voice", false, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let character = CharacterId::decode(&id(41)).map_err(debug)?;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let epoch = (1..=64)
            .find(|epoch| {
                crate::durability::world_event::changes::roll(world, *epoch, 0)
                    .is_ok_and(|day| day.their_masters_voice)
            })
            .ok_or("active voice epoch")?;
        event_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, epoch)
            .await?;
        event_fixture::admit(&h, epoch + 1).await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let deadline = now + 60_000;
        let kind = CounterKind::TheirMastersVoice;
        let mut state = CounterState::new(kind, epoch, false);
        state.stage = 3;
        state.counts = [100, 25, (1 << 25) - 1];
        state.deadline_ms = Some(u64::try_from(deadline)?);
        sqlx::query(
            "UPDATE game_world_change_counters SET state=$1 WHERE change_key='their_masters_voice'",
        )
        .bind(serde_json::to_value(state)?)
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter,period_deadline_ms) VALUES(encode($1,'hex')::uuid,'their_masters_voice',encode($2,'hex')::uuid,$3::bigint,$4)")
            .bind(world.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(gobbles).bind(deadline).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_terrain_tool_transforms(world_id,channel_id,placement,reset_epoch,map_revision,content_generation,state_item_id,object_revision,decay_states,decay_delays,decay_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,'map-1',$5,12065,1,ARRAY[12059]::bigint[],ARRAY[60000]::bigint[],$6)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
            .bind(placement()?).bind(i64::try_from(epoch)?).bind([7u8; 32].as_slice())
            .bind(deadline).execute(&h.pool).await?;
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("active change")?;
        assert!(day.their_masters_voice);
        let generation = ScopeOwnershipGeneration::new(epoch + 1).map_err(debug)?;
        let mut deps = Dependencies::new()?;
        deps.combat_room = Some(pin.qualified_room().clone());
        planned_world_reset::reset_fixture::set_runtime_with_capacity(
            &mut deps,
            world,
            channel,
            &pin,
            epoch + 1,
            4,
        )?;
        let session = deps.admitted.game_session_id;
        let target = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    crate::foundation::CharacterLease::new(character, 2).map_err(debug)?,
                )
                .map_err(debug)?;
            let native = controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("native")?;
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            let health = runtime
                .companion_policy(MAGE)
                .map_err(debug)?
                .maximum_health;
            let start = pin.qualified_room().entry_start();
            let cells = vec![(start.x + 1, start.y, start.floor)];
            assert_eq!(
                runtime
                    .realize_activation_spawns(vec![(
                        cells,
                        60_000,
                        5_000,
                        MAGE.into(),
                        u64::try_from(health)?
                    )])
                    .map_err(debug)?,
                1
            );
            let points = runtime.spawn_point_creatures();
            let [(point, _, Some(target))] = points.as_slice() else {
                return Err("Mad Mage fixture spawns".into());
            };
            runtime
                .bind_world_spawn_source(
                    point.source,
                    "their_masters_voice.mage",
                    day.counter_generations[kind.index()],
                )
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
            *target
        };
        let mut item_fence = fence(1)?.item_fence();
        item_fence.character_lease_generation = 2;
        item_fence.scope_ownership_generation = generation;
        deps.admitted.item_fence = Some(item_fence);
        deps.admitted.runtime_actor = Some(deps.actor);
        deps.admitted.first_entry = FirstEntryOutcome::Positioned;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        owner.reset_epoch = epoch;
        deps.initialize(&owner).await;
        owner.admit_progression_session(&deps.admitted).await;

        kill(&owner, character, session, deps.actor, target, 20000).await?;
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("voice day after death")?;
        assert_eq!(day.counter_stages[kind.index()], expected_stage);
        let terrain: (i64, i64, Option<i64>) = sqlx::query_as(
            "SELECT state_item_id,object_revision,decay_at_unix_ms FROM game_terrain_tool_transforms",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(terrain, (12059, 2, None));
        if expected_stage == 1 {
            return Ok(());
        }
        sqlx::query("UPDATE game_terrain_tool_transforms SET state_item_id=12065,decay_step=0,decay_at_unix_ms=$1")
            .bind(deadline).execute(&h.pool).await?;
        let mut players = Vec::new();
        {
            let mut runtime = owner.runtime.lock().await;
            for (tag, x) in [(51, 33312), (52, 33313)] {
                let visitor = GameSessionId::decode(&id(tag)).map_err(debug)?;
                let reservation = runtime.reserve_fresh_session(visitor).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(
                        actor,
                        crate::foundation::MovementLocalPosition {
                            x,
                            y: 31872,
                            floor: 9,
                        },
                    )
                    .map_err(debug)?;
                players.push((visitor, actor));
            }
        }
        for expected in [12065, 12065, 12059] {
            let mut runtime = &*owner.runtime;
            assert!(
                h.root
                    .project_world_change_counters(
                        &authority,
                        &h.node,
                        scope,
                        generation,
                        epoch,
                        &mut runtime,
                        |runtime, _| {
                            let runtime = runtime
                                .try_lock()
                                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                            Ok(!runtime.committed_player_positions().iter().any(|at| {
                                oteryn_gameplay_world::terrain_tools::voice_dungeon_contains(
                                    at.x, at.y, at.floor,
                                )
                            }))
                        },
                    )
                    .await?
            );
            let appearance: i64 =
                sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(appearance, expected);
            if let Some((visitor, actor)) = players.pop() {
                owner
                    .runtime
                    .lock()
                    .await
                    .remove_terminal_session(visitor, actor)
                    .map_err(debug)?;
            }
        }
        Ok(())
    })
}

#[test]
fn worldchg_voice_server_save_retires_unexpired_dungeon_progress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("worldchg-voice-save", move || {
        save_scenario(admin)
    })
}

fn save_scenario(admin: String) -> TestResult {
    run(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "worldchg_voice_save", false, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let (_, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Seed an in-progress encounter after the first save publishes, then let the
        // next real server save retire it before the one-hour deadline.
        sqlx::query("CREATE FUNCTION seed_voice_progress() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.reset_epoch=1 THEN UPDATE game_world_change_counters SET state=state || jsonb_build_object('counts',jsonb_build_array(25,0,0),'deadline_ms',floor(extract(epoch FROM clock_timestamp())*1000)::bigint+3600000) WHERE world_id=NEW.world_id AND change_key='their_masters_voice'; END IF; RETURN NEW; END $$")
            .execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER seed_voice_progress AFTER INSERT ON game_world_change_days FOR EACH ROW EXECUTE FUNCTION seed_voice_progress()")
            .execute(&h.pool).await?;
        event_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, 2)
            .await?;
        let value: serde_json::Value = sqlx::query_scalar(
            "SELECT state FROM game_world_change_counters WHERE change_key='their_masters_voice'",
        )
        .fetch_one(&h.pool)
        .await?;
        let state: CounterState = serde_json::from_value(value)?;
        assert_eq!(
            (state.epoch, state.stage, state.counts, state.deadline_ms),
            (2, 1, [0; 3], None)
        );
        assert_eq!(state.credit_generation, 2);
        Ok(())
    })
}

fn placement() -> TestResult<Vec<u8>> {
    let mut placement = vec![7; 32];
    let key = oteryn_world_bundle::bundle::placement_key(-9, 33312, 31872, 0)
        .ok_or("fungus placement")?;
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}

const MAGE: &str = "oteryn:creature.mad_mage";

async fn kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    character: CharacterId,
    session: GameSessionId,
    actor: ExactActorRef,
    target: ExactActorRef,
    command: u64,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    use crate::gameplay_transport::kill_reward::{self, DrainOutcome, KillSink};
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let observed = owner.owner_now();
    let now = observed.get() / 1_000;
    let queue = owner.states.attack.lock().await.kills().clone();
    {
        let mut runtime = owner.runtime.lock().await;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: MAGE.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"their-masters-voice",
                    damage: health,
                },
            )
            .map_err(debug)?;
        kill_reward::record_creature_kill(
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
                character,
                lease_generation: 2,
                session,
                actor,
            },
            now,
            Some(observed),
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    Ok(())
}

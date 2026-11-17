//! Background arbitration must let a bounded multi-pass position write finish.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::DurabilityRoot;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn background_position_batch_completes_both_passes_with_foreground_demand_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-position-operation", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "background_position_operation",
                true,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            track(&owner, &dependencies)?;
            let foreground = h.root.foreground_demand()?;
            let until = tokio::time::Instant::now() + crate::durability::ROOT_RECOVERY_WINDOW;
            assert_eq!(
                DurabilityRoot::with_background_holder(owner.write_due_last_positions(true)).await,
                1,
                "foreground demand refuses a fresh background operation"
            );
            tokio::time::sleep_until(until).await;
            assert_eq!(
                DurabilityRoot::with_background_holder(owner.write_due_last_positions(true)).await,
                0,
                "one fairness grant must cover the fence read and position write"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_last_positions")
                    .fetch_one(&h.pool)
                    .await?,
                1
            );
            assert!(
                matches!(
                    h.root.try_issue_background_semantic_pass(),
                    Err(crate::durability::DurabilityError::RootUnavailable)
                ),
                "a completed operation must release its fairness grant"
            );
            drop(foreground);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn background_quest_sweep_yields_between_sessions_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-quest-sweep", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "background_quest_sweep", false, false)
                    .await?;
            prepare_current_session(&h).await?;
            let mut sessions = Vec::new();
            for offset in 0..3 {
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                )
                .fetch_one(&h.pool)
                .await?;
                let current =
                    crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
                        &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                            root: h.root.clone(),
                            node: h.node.clone(),
                            recovery: &h.recovery,
                        },
                        60 + offset,
                        70 + offset,
                        100 + offset,
                        now,
                    )
                    .await?;
                sessions.push(current.commit().game_session_id());
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = super::fence(1)?.runtime_scope.world_id();
            let deps = Dependencies::with_content_and_capacity(
                crate::content::channel_content_pin::ChannelContentPin::test(world),
                crate::content::qualify_native_entry_room(world).map_err(debug)?,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                4,
            )?;
            let owner = deps.owner(&h, &authority);
            // Three genuine durable admissions mirrored by the native owner.
            // Each successful refresh updates its entry; a refused one retains it.
            for session in sessions {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                runtime.commit_fresh_session(reservation).map_err(debug)?;
                drop(runtime);
                owner.states.quest_sessions.lock().map_err(debug)?.insert(
                    session,
                    crate::gameplay_transport::quest_admission::QuestSession {
                        map_marks_world: None,
                        map_marks: Vec::new(),
                        copy: None,
                        experience_deferred: false,
                        version: 0,
                        retry_at: Some(std::time::Instant::now()),
                    },
                );
            }
            h.root.maintain_ready_once().await?;
            let foreground = h.root.foreground_demand()?;
            let _waiter = h.root.holder_waiter().ok_or("foreground waiter")?;
            tokio::time::sleep(crate::durability::ROOT_RECOVERY_WINDOW).await;
            DurabilityRoot::with_background_holder(owner.refresh_due_quest_sessions()).await;
            let completed = owner
                .states
                .quest_sessions
                .lock()
                .map_err(debug)?
                .values()
                .filter(|entry| entry.version == 1)
                .count();
            assert_eq!(
                completed, 1,
                "a fairness grant must end after one session, not the population"
            );
            // The returned holder belongs to foreground before any further background unit.
            h.root
                .try_issue_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
            drop(foreground);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn background_expired_quest_timer_executes_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-expired-quest-timer", || {
        super::quest_world_callbacks::scenario(
            "l6-tomb-switch-treasure-background-expired-12121",
            oteryn_gameplay_quests::quest::world_callbacks::Position { x: 7, y: 1, floor: -7 },
            false,
        )
    })
}

#[test]
fn background_empty_grizzly_ordinary_boss_respawns_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-ordinary-boss", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "background_ordinary_boss", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = super::fence(1)?.runtime_scope.world_id();
            let (controller, activation) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
            let active = controller.active().ok_or("active")?;
            // Creature policies and the runtime pin must name the same compiled artifact.
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                world, activation.activation_sequence(), active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(), activation.frame_binding().digest(),
                activation.qualified_room().map_revision_digest(), (2, 0, 7),
            );
            let deps = Dependencies::with_content(pin, crate::content::qualify_native_entry_room(world).map_err(debug)?, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
            let owner = deps.owner(&h, &authority);
            active.native_gameplay().ok_or("native")?
                .install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
            let creature = "oteryn:creature.rat".to_owned();
            owner.runtime.lock().await.configure_boss_spawns(vec![crate::foundation::BossSpawn {
                key: "background-boss".into(), creature,
                position: crate::foundation::MovementLocalPosition { x: 1, y: 11, floor: 7 },
                policy: oteryn_gameplay_content::content::BossPolicy::Respawn { min_s: 1, max_s: 1 },
                actor: None, cycle: 0, pending_claim: false,
            }]).map_err(debug)?;
            // A post-death respawn is already at cycle 1. Claiming it changes DUE
            // to ALIVE without advancing the cycle; only death advances it.
            sqlx::query("INSERT INTO game_boss_spawn_clocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'background-boss',1,1,0,0,NULL)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_grizzly_boss_entries WHERE state='ACTIVE'").fetch_one(&h.pool).await?, 0, "Grizzly discovery is empty");
            h.root.maintain_ready_once().await?;
            let _foreground = h.root.foreground_demand()?;
            let _waiter = h.root.holder_waiter().ok_or("foreground waiter")?;
            tokio::time::sleep(crate::durability::ROOT_RECOVERY_WINDOW).await;
            DurabilityRoot::with_background_holder(owner.boss_spawn_pass(&mut std::collections::BTreeMap::new())).await.map_err(debug)?;
            assert!(owner.runtime.lock().await.boss_spawns()[0].actor.is_some(), "ordinary boss was realized");
            let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_spawn_clocks WHERE spawn_key='background-boss' AND cycle>0").fetch_one(&h.pool).await?;
            assert_eq!(rows, 1, "the real boss clock changed");
            let generation = owner.runtime.lock().await.binding().scope_generation();
            let claimed: bool = sqlx::query_scalar("SELECT cycle=1 AND state=2 AND due_at IS NULL AND owner_generation IS NOT DISTINCT FROM $1::text::numeric FROM game_boss_spawn_clocks WHERE spawn_key='background-boss'")
                .bind(generation.get().to_string()).fetch_one(&h.pool).await?;
            assert!(claimed, "the due clock was claimed by the current owner without advancing its cycle");
            eprintln!("POSTGRES_CASE_EXECUTED background_empty_grizzly_ordinary_boss");
            drop(_waiter); drop(_foreground); drop(owner); drop(authority); drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn background_timed_sessions_rotate_within_bounded_grants_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("background-timed-rotation", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "background_timed_rotation", true, false).await?;
            prepare_current_session(&h).await?;
            let mut currents = Vec::new();
            for offset in 0..3 {
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
                currents.push(crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
                    &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                        root: h.root.clone(), node: h.node.clone(), recovery: &h.recovery,
                    }, 60 + offset, 70 + offset, 100 + offset, now,
                ).await?);
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = super::fence(1)?.runtime_scope.world_id();
            let (controller, activation) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
            let active = controller.active().ok_or("active")?;
            let native = active.native_gameplay().ok_or("native")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                world, activation.activation_sequence(), active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(), activation.frame_binding().digest(),
                activation.qualified_room().map_revision_digest(), (2, 0, 7),
            );
            let deps = Dependencies::with_content_and_capacity(pin, crate::content::qualify_native_entry_room(world).map_err(debug)?, crate::spell::cast::v1_spell_book().map_err(debug)?, 4)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
                .bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(activation.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let mut actors = Vec::new();
            for (index, current) in currents.iter().enumerate() {
                let session = current.commit().game_session_id();
                let actor = {
                    let mut runtime = owner.runtime.lock().await;
                    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime.bind_attacker_lease(actor, session, current.current_character_lease()).map_err(debug)?;
                    runtime.initialize_first_entry_position(actor).map_err(debug)?;
                    actor
                };
                deps.initialize_player(&owner, actor, session).await;
                let item = super::id(120 + u8::try_from(index)?);
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i6529',$3,1,1,encode($1,'hex')::uuid)")
                    .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(native.item_index().ok_or("index")?.revision("oteryn:item.tibia.i6529").ok_or("boots revision")?).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                    .bind(current.current_character_lease().character_id().as_bytes().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,4,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(current.current_character_lease().character_id().as_bytes().as_slice()).bind(item.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                // Establish each clock at zero before making all three due.
                assert!(owner.checkpoint_timed_equipment(actor, session, false).await);
                actors.push((actor, session));
            }
            assert_eq!(actors.len(), 3);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_timed_state_writes").fetch_one(&h.pool).await?, 0, "all three budgets start untouched");
            owner.clock_origin -= std::time::Duration::from_millis(60_001);
            h.root.maintain_ready_once().await?;
            let foreground = h.root.foreground_demand()?;
            let waiter = h.root.holder_waiter().ok_or("foreground waiter")?;
            // Two scope units plus equipment/quest for three sessions. Each grant
            // may consume only one unit, so eight grants suffice for all equipment.
            for _ in 0..8 {
                tokio::time::sleep(crate::durability::ROOT_RECOVERY_WINDOW).await;
                DurabilityRoot::with_background_holder(owner.tick_timed_items()).await;
                h.root.try_issue_semantic_pass()?.run(|_, _| Box::pin(async { Ok(()) })).await?;
            }
            let progressed: i64 = sqlx::query_scalar("SELECT count(DISTINCT item_instance_id) FROM game_item_timed_state_writes").fetch_one(&h.pool).await?;
            assert_eq!(progressed, 3, "all three sessions consumed a real timed budget");
            eprintln!("POSTGRES_CASE_EXECUTED background_timed_sessions_rotation rows={progressed}");
            drop(waiter); drop(foreground); drop(owner); drop(authority); drop(seal);
            h.cleanup().await
        })
    })
}

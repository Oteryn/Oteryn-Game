//! Background arbitration must let a bounded multi-pass position write finish.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::DurabilityRoot;

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

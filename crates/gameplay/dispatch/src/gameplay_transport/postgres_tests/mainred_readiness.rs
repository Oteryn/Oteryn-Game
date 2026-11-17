//! Physical holder contention/retirement at the real projection and progression callers.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::character_progression_binding::{
    ProgressionBinding, ProgressionDurable, ProgressionRounds, bind_character_progression,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::{Duration, Instant};

#[test]
fn mainred_resume_projections_wait_for_real_holder_postgres() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "mainred_projection_ready", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let mut admitted = deps.admitted;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[86, 117])
                .ok_or("registered recovery capabilities")?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        assert!(admitted.item_fence.is_some());
        for house in [false, true] {
            h.root.request_ready();
            assert!(h.root.maintain_ready_once().await?);
            let mut holder = h.root.try_acquire_ready().map_err(debug)?;
            assert!(!h.root.is_ready());
            let mut reading = Box::pin(async {
                if house {
                    owner
                        .project_house_transfer(admitted)
                        .await
                        .map(|v| v.is_none())
                } else {
                    owner
                        .project_activity_scope(admitted)
                        .await
                        .map(|v| v.is_none())
                }
            });
            assert!(
                poll_fn(|cx| Poll::Ready(reading.as_mut().poll(cx)))
                    .await
                    .is_pending()
            );
            tokio::time::sleep(Duration::from_millis(150)).await;
            let _ = holder
                .return_to_pool_observed_until(Instant::now() + crate::durability::DB_PASS_DEADLINE)
                .await;
            // No repoll of the projection between the observed return and the
            // scheduled owner's attempt to retake the same idle holder.
            assert!(h.root.is_ready());
            for _ in 0..16 {
                assert!(matches!(
                    h.root.try_issue_background_semantic_pass(),
                    Err(crate::durability::DurabilityError::RootUnavailable)
                ));
            }
            let projection = reading.await;
            assert_eq!(
                projection,
                Ok(true),
                "house={house}: a busy holder is not missing scope truth"
            );
            h.root
                .try_issue_background_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
        }
        eprintln!(
            "MAINRED_READINESS_POSTGRES projections=activity,house actual_holder_return=observed"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn mainred_progression_reopens_retired_holder_postgres() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "mainred_progression_ready", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let content =
            crate::content::character_progression_content::CharacterProgressionContent::decode(
                &crate::content::character_progression_content::test_support::section_bytes(),
            )
            .map_err(debug)?;
        h.root.request_ready();
        assert!(h.root.maintain_ready_once().await?);
        let holder = h.root.try_acquire_ready().map_err(debug)?;
        holder.close().await?;
        assert!(!h.root.is_ready());
        let binding = Box::pin(bind_character_progression(
            ProgressionDurable {
                root: &h.root,
                character: &authority,
                holder: &h.node,
            },
            |profile, ruleset, revision| content.policy_for(profile, ruleset, revision).ok(),
            || owner.current_quest_fence(deps.admitted.game_session_id),
            ProgressionRounds {
                attempts: 3,
                backoff: Duration::from_millis(5),
            },
        ))
        .await;
        assert!(
            matches!(binding, ProgressionBinding::Bound(_)),
            "{binding:?}"
        );
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_progression_state")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1, "the real initializer must commit exactly one row");
        eprintln!(
            "MAINRED_READINESS_POSTGRES progression=bound retired_holder=replaced initialization_rows=1"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn mainred_admission_retry_replenishes_retired_holder_postgres() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "mainred_admission_ready", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let session =
            crate::foundation::GameSessionId::decode(&super::fixture::id(50)).map_err(debug)?;
        h.root.await_ready().await?;
        let holder = h.root.try_acquire_ready().map_err(debug)?;
        holder.close().await?;
        assert!(!h.root.is_ready());
        let store =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
        let retry = crate::gameplay_transport::AdmissionRetryBudget::for_root(&session, &h.root);
        let (current, _) = retry
            .retry_unavailable(async || {
                store
                    .current_session_at(session)
                    .await
                    .map_err(crate::gameplay_transport::activity_recovery::projection_refusal)
            })
            .await
            .map_err(debug)?;
        assert_eq!(
            current.session_state(),
            crate::foundation::GameSessionState::Active
        );
        eprintln!(
            "MAINRED_READINESS_POSTGRES admission_retry=ready retired_holder=replenished current_session=active"
        );
        h.cleanup().await
    })
}

#[test]
fn mainred_fresh_house_probe_replenishes_retired_holder_postgres() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "mainred_fresh_house_ready", false, false)
                .await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let attempt = crate::gameplay_transport::FreshAdmissionAttempt {
            character_id: oteryn_protocol_oteryn::CharacterId::decode(&super::fixture::id(41))
                .map_err(debug)?,
            admission_material: b"not-read-after-commit",
            game_session_id: deps.admitted.game_session_id,
            transport: deps
                .admitted
                .controller
                .ok_or("fixture controller")?
                .transport,
            supported_capabilities: &[],
        };
        h.root.await_ready().await?;
        let holder = h.root.try_acquire_ready().map_err(debug)?;
        holder.close().await?;
        assert!(!h.root.is_ready());
        let restored = owner
            .restore_fresh_house(attempt, deps.actor, super::fixture::id(40), &request)
            .await;
        assert!(
            matches!(restored, Ok(None)),
            "a retired holder cannot refuse an ordinary committed admission: {restored:?}"
        );
        eprintln!("MAINRED_READINESS_POSTGRES fresh_house_probe=absent retired_holder=replenished");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

//! A failed admission reset cannot publish a current Wheel allocation.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use std::sync::Arc;

#[test]
fn wheel_admission_missing_progression_defers_allocation() -> TestResult {
    scenario(false)
}

#[test]
fn wheel_admission_busy_root_retries_on_owner_cadence() -> TestResult {
    scenario(true)
}

fn scenario(initialized: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "wheel_admission_deferred",
                    initialized,
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
                let deps = Dependencies::new()?;
                let mut owner = deps.owner(&h, &authority);
                let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
                    .map_err(debug)?;
                owner.wheel_ruleset = Some(Arc::new(
                    crate::durability::character_wheel::WheelRuleset::from_catalogue(
                        data.revision(),
                        data.wheel(),
                    )
                    .map_err(debug)?,
                ));
                let session = deps.admitted.game_session_id;
                // Install the quest retry entry before occupying the root's sole holder.
                owner.refresh_quest_session(session, true).await;
                let held = if initialized {
                    h.root.maintain_ready_once().await?;
                    Some(h.root.try_issue_semantic_pass()?)
                } else {
                    None
                };
                owner.admit_quest_session(&deps.admitted).await;
                assert!(
                    owner
                        .states
                        .wheel_sessions
                        .lock()
                        .map_err(debug)?
                        .get(&session)
                        .is_some_and(Option::is_none),
                    "failed reset must defer Wheel instead of publishing an allocation",
                );
                assert!(
                    owner
                        .states
                        .quest_sessions
                        .lock()
                        .map_err(debug)?
                        .get(&session)
                        .is_some_and(|entry| entry.retry_at.is_some()),
                    "deferred Wheel admission must use the existing owner retry cadence"
                );
                if let Some(held) = held {
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    owner
                        .states
                        .quest_sessions
                        .lock()
                        .map_err(debug)?
                        .get_mut(&session)
                        .ok_or("quest retry entry")?
                        .retry_at = Some(std::time::Instant::now());
                    owner.refresh_due_quest_sessions().await;
                    assert!(
                        owner
                            .states
                            .wheel_sessions
                            .lock()
                            .map_err(debug)?
                            .get(&session)
                            .is_some_and(Option::is_some),
                        "Wheel admission must recover on cadence after the root is ready"
                    );
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "wheel admission test thread panicked")??;
    Ok(())
}

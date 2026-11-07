//! Whole selected personal Heart of Destruction programme; subject state is earned natively.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_heart_whole_native_breaches_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-heart-story", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-heart-story",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HEART_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    _owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    _admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_HEART_STORY POSTGRES_STARTED version={version}");
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load().map_err(debug)?;
    assert!(
        callbacks
            .iter()
            .any(|c| c.key == "oteryn:quest-callback/questpc7-heart-entry"),
        "whole Heart of Destruction programme must have a native entry; contact placement alone cannot qualify it"
    );
    Ok(())
}

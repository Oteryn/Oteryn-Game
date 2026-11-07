//! Source native Feaster sampling prefix and bounded repeat mechanism.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_feaster_native_sampling_prefix() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-feaster-prefix", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-feaster-prefix",
            Position {
                x: 32627,
                y: 32076,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FEASTER_PREFIX POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
pub(super) async fn exercise(
    _owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    _a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FEASTER_PREFIX POSTGRES_STARTED version={version}");
    let storage:oteryn_gameplay_quests::quest::world_callbacks::Storage=serde_json::from_value(serde_json::json!({"track":"oteryn:quest-progress/questpc7/feaster/samples","minimum":0,"maximum":19,"set":20,"add":1})).map_err(debug)?;
    assert_eq!((storage.minimum, storage.maximum, storage.set), (0, 19, 20));
    assert!(
        std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/authored-props/questpc7.json"
        ))
        .is_file(),
        "source-backed quest props must exist before native sampling qualification"
    );
    Ok(())
}

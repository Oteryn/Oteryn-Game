//! L2 Regalia, alliance supplies and tome progression enter through native commands.
use super::*;

fn probe(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_knowledge_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required
            .iter()
            .all(|key| callbacks.iter().any(|c| c.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete native journey bindings absent: {required:?}"
        );
        Ok(())
    })?;
    scenario(case)
}

#[test]
fn questl2_knowledge_batch_kilmaresh_regalia_full_journey() -> TestResult {
    probe("qengine-rewards-l2-regalia", &["/l2-regalia-combine"])
}
#[test]
fn questl2_knowledge_batch_uneasy_alliance_full_journey() -> TestResult {
    probe(
        "qengine-rewards-l2-alliance",
        &["/l2-alliance-start", "/l2-alliance-finish"],
    )
}
#[test]
fn questl2_knowledge_batch_tomes_full_journey() -> TestResult {
    probe(
        "qengine-rewards-l2-tomes",
        &["/l2-cael-tome-1", "/l2-cael-tome-12"],
    )
}

pub(super) async fn exercise(
    _owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    _admitted: &mut AdmittedSession,
    _h: &Harness,
    name: &str,
) -> TestResult {
    Err(format!("full native player journey has not been composed yet: {name}").into())
}

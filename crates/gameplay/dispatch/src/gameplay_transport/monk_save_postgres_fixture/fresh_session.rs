use super::fresh_session_fixtures::prepare_after_runtime_observation;
use super::*;

#[test]
fn quest_world_callbacks_stag_fixture_refreshes_security_after_runtime_publication() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        let harness = Harness::create_with_admission_seed(
            admin,
            "stag_delayed_runtime_publication",
            false,
            false,
        )
        .await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&harness.pool)
                .await?;
        // Runtime observations have no five-second Platform evidence deadline.
        // Simulate publication work taking six seconds without a sleep or a
        // relaxed deadline: security and signing must be observed afterwards.
        let request = prepare_after_runtime_observation(&harness, now - 6).await?;
        let binding = request.binding();
        assert!(binding.security.provenance.source_observed_at >= now);
        assert_eq!(
            binding.accepted_deadline,
            binding.security.provenance.source_observed_at + 5
        );
        let runtime = binding
            .expected_guards
            .iter()
            .find(|row| matches!(row.key,
                crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)))
            .ok_or("runtime observation")?;
        assert_eq!(runtime.source.source_observed_at, now - 6);
        harness.cleanup().await
    })
}

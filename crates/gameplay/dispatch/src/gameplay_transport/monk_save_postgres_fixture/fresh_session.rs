//! The existing fresh-session publication helper, shared without rebuilding its receipt.
use super::*;

pub(super) async fn prepare_current_session(
    harness: &Harness,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    prepare_after_runtime_observation(harness, now).await
}

async fn prepare_after_runtime_observation(
    harness: &Harness,
    now: i64,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    // The new harness starts with no admission claims/session/history.
    // Complete owning evidence is published and committed through existing APIs.
    let mut runtime_source = fresh_fixture::fresh::Source::new(now)?;
    runtime_source.rows.retain(|row| matches!(row.key,
        crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let runtime_publication =
        AdmissionAuthorityPublicationV1::prepare(&runtime_source, now).map_err(debug)?;
    harness
        .root
        .publish_runtime_readiness(&harness.node, &runtime_publication)
        .await
        .map_err(debug)?;
    // Runtime publication may block under concurrent PostgreSQL load. Observe
    // Platform security/signing only afterwards: their existing five-second
    // decision deadline must cover admission, not runtime setup work.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let mut source = fresh_fixture::fresh::Source::new(now)?;
    let runtime = source
        .rows
        .iter_mut()
        .find(|row| matches!(row.key,
            crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)))
        .ok_or("runtime observation")?;
    // Keep the exact published runtime observation; do not re-age evidence.
    *runtime = runtime_publication
        .changes()
        .first()
        .ok_or("runtime publication")?
        .clone();
    let mut admission_source = fresh_fixture::fresh::Source::new(now)?;
    admission_source.rows.retain(|row| !matches!(row.key,
        crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&admission_source, now).map_err(debug)?;
    let guards = AdmissionGuardStore::from_root(harness.root.clone());
    guards
        .publish(&publication)
        .await
        .map_err(|e| format!("fresh fixture publication: {e:?}"))?;
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let request = source.request()?;
    let outcome = store
        .commit(&request)
        .await
        .map_err(|e| format!("fresh fixture commit: {e:?}"))?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "fresh fixture outcome={outcome:?}"
    );
    let (current, _) = store
        .current_session_at(GameSessionId::decode(&id(50)).map_err(debug)?)
        .await?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(current.current_character_lease().generation(), 2);
    Ok(request)
}

#[test]
fn quest_world_callbacks_stag_fixture_refreshes_security_after_runtime_publication() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
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

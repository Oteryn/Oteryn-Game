//! The existing fresh-session publication helper, shared without rebuilding its receipt.
use super::*;

pub(super) async fn prepare_current_session(
    harness: &Harness,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    // The new harness starts with no admission claims/session/history.
    // Complete owning evidence is published and committed through existing APIs.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let source = fresh_fixture::fresh::Source::new(now)?;
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
    assert!(matches!(
        store
            .commit(&request)
            .await
            .map_err(|e| format!("fresh fixture commit: {e:?}"))?,
        FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
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

//! The existing fresh-session publication helper, shared without rebuilding its receipt.
use super::*;

pub(super) async fn prepare_current_session(
    harness: &Harness,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    prepare_current_session_for_account(harness, id(40)).await
}

pub(super) async fn prepare_current_session_for_account(
    harness: &Harness,
    account: [u8; 16],
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    prepare_after_runtime_observation_for_account(harness, now, account).await
}

#[cfg(oteryn_pg_shard_primary)]
pub(super) async fn prepare_after_runtime_observation(
    harness: &Harness,
    now: i64,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    prepare_after_runtime_observation_for_account(harness, now, id(40)).await
}

pub(super) async fn commit_current_session(
    harness: &Harness,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    commit_after_runtime_observation_for_account(harness, now, id(40)).await
}

async fn prepare_after_runtime_observation_for_account(
    harness: &Harness,
    now: i64,
    account: [u8; 16],
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    let request = commit_after_runtime_observation_for_account(harness, now, account).await?;
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let (current, _) = store
        .current_session_at(request.binding().candidate_session)
        .await?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(current.current_character_lease().generation(), 2);
    Ok(request)
}

async fn commit_after_runtime_observation_for_account(
    harness: &Harness,
    now: i64,
    account: [u8; 16],
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
    let mut source = fresh_fixture::fresh::Source::appearance_observer(
        now,
        crate::gameplay_transport::canonical_uuid(&account),
        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
    )?;
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
    let request = if account == id(40) {
        source.request()?
    } else {
        source.request_with_nonce(
            GameSessionId::decode(&id(50)).map_err(debug)?,
            crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
            Some((
                &source.current.account_id,
                &crate::gameplay_transport::canonical_uuid(&id(41)),
            )),
            now,
            None,
        )?
    };
    source.rows.retain(|row| !matches!(row.key,
        crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    let guards = AdmissionGuardStore::from_root(harness.root.clone());
    guards
        .publish(&publication)
        .await
        .map_err(|e| format!("fresh fixture publication: {e:?}"))?;
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let outcome = store
        .commit(&request)
        .await
        .map_err(|e| format!("fresh fixture commit: {e:?}"))?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "fresh fixture outcome={outcome:?}"
    );
    Ok(request)
}

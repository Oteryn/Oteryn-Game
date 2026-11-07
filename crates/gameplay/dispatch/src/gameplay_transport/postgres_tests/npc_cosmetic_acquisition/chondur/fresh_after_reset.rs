//! Reuses the existing signed-source fresh-admission fixture, never inserts a session row.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::monk_save::postgres_tests::fresh_fixture;
pub(super) async fn fresh_after_reset(
    h: &Harness,
    character: [u8; 16],
    account: [u8; 16],
    successor: [u8; 16],
    generation: u64,
    previous_lease: u64,
) -> TestResult<(
    GameSessionId,
    crate::foundation::fresh_admission_durability::FreshAdmissionOperationV1,
)> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityGuardStateV1 as State,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    };
    crate::gameplay_transport::monk_save::postgres_tests::planned_world_reset::reset_fixture::publish_ready(h, generation).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let template = fresh_fixture::fresh::Source::new(now)?;
    let current = crate::foundation::fnd04_verifier::FreshCurrentEvidence {
        account_id: crate::gameplay_transport::canonical_uuid(&account),
        character_id: crate::foundation::CharacterId::decode(&character).map_err(debug)?,
        scope_ownership_generation: generation,
        ..template.current
    };
    let mut source = fresh_fixture::fresh::Source::with_current(now, current.clone())?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current source publications")?;
    let mut refresh = fresh_fixture::fresh::Source::with_current(now, current)?;
    refresh.rows = source
        .rows
        .iter()
        .filter(|row| matches!(row.key, Key::Account { .. } | Key::SigningTrust { .. }))
        .cloned()
        .collect();
    for row in &mut refresh.rows {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision += 1;
        row.source.source_revision += 1;
        row.source.decision_identity = format!("full-world-source-{}", row.source.source_revision);
        row.source.source_observed_at = now;
        if let State::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision = row.source.source_revision;
            security.provenance.accepted_source_revision = row.source.source_revision;
            security.provenance.decision_identity = row.source.decision_identity.clone();
            security.provenance.accepted_decision_identity = row.source.decision_identity.clone();
            security.provenance.source_observed_at = now;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&refresh, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("refreshed current publications")?;
    source.current.character_lease_generation = previous_lease;
    let mut nonce = [0_u8; 32];
    nonce[..16].copy_from_slice(&successor);
    nonce[16..24].copy_from_slice(&generation.to_be_bytes());
    nonce[24..].copy_from_slice(&previous_lease.to_be_bytes());
    let next = source.request_with_nonce(
        GameSessionId::decode(&successor).map_err(debug)?,
        crate::foundation::AuthenticatedTransportRefV1::decode(&successor).map_err(debug)?,
        Some((
            &source.current.account_id,
            &crate::gameplay_transport::canonical_uuid(&character),
        )),
        now,
        Some(nonce),
    )?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&next).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    // A replayed commit of the same request admits no third lease.
    let _ = store.commit(&next).await;
    let next_session = GameSessionId::decode(&successor).map_err(debug)?;
    assert_eq!(
        store
            .current_session(next_session)
            .await?
            .current_character_lease()
            .generation(),
        previous_lease + 1
    );
    Ok((next_session, next.operation().clone()))
}

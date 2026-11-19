use super::TestResult;

use super::fixture::{Harness, debug, id};

use crate::durability::admission_authority_guards::AdmissionGuardStore;

use crate::durability::fresh_admission::FreshAdmissionStore;

use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1,
};

use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;

use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, GameSessionId,
};

/// Publish and admit an already-created Character through the same verified
/// Fresh owner. Native progression tests create its history through bootstrap.
pub(super) async fn admit_created_member(
    h: &Harness,
    evidence: &crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    member: u8,
    character: [u8; 16],
) -> TestResult<GameSessionId> {
    let (account,character_text):(String,String)=sqlx::query_as("SELECT account_id::text,character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&h.pool).await?;
    // A published Account guard does not supply the producer-owned security
    // floor required by the existing scope-transfer writer. Refresh each new
    // account through the same authenticated source used for the first member.
    evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::appearance_observer(
        now,
        account,
        CharacterId::decode(&character).map_err(debug)?,
    )?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let keys = source
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.key,
                AdmissionAuthorityGuardKeyV1::Runtime(_)
                    | AdmissionAuthorityGuardKeyV1::SigningTrust { .. }
            )
        })
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    for prior in guards.load(&keys).await.map_err(debug)? {
        let prior = prior.ok_or("current global fixture publication")?;
        let row = source
            .rows
            .iter_mut()
            .find(|row| row.key == prior.key)
            .ok_or("global fixture key")?;
        if matches!(row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { .. }) {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: prior.publication_revision,
            };
            row.publication_revision = prior
                .publication_revision
                .checked_add(1)
                .ok_or("publication revision")?;
            row.source.source_revision = prior
                .source
                .source_revision
                .checked_add(1)
                .ok_or("source revision")?;
            row.source.decision_identity =
                format!("platform-observation-{}", row.source.source_revision);
        } else {
            *row = prior;
        }
    }
    for row in &mut source.rows {
        if matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
        ) {
            row.source.authority =
                crate::durability::fresh_admission_composition::COMPOSITION_SOURCE_AUTHORITY.into();
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                security.provenance.source_authority = "platform-test".into();
            }
        }
    }
    let rows = source.rows.clone();
    source
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    guards
        .publish(&AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    source.rows = rows;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let session = GameSessionId::decode(&id(140 + member)).map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[150 + member; 16]).map_err(debug)?;
    let request = source.activity_room_request(
        session,
        transport,
        &character_text,
        [170 + member; 32],
        now,
    )?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    // These direct fixtures have no node maintenance loop. Reconcile the same
    // immutable admission request after a holder replacement; never reseed or
    // manufacture a committed membership after an unavailable write.
    super::maintain_ready(&h.root).await?;
    let mut result = store.commit(&request).await;
    for _ in 0..3 {
        if result.is_ok() || !h.root.has_ready_demand() {
            break;
        }
        super::maintain_ready(&h.root).await?;
        result = store.commit(&request).await;
    }
    let outcome = result.map_err(|error| format!("Fresh member {member} commit: {error:?}"))?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "real pager source admission: {outcome:?}"
    );
    Ok(session)
}

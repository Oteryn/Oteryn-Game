//! Small adapters around the single existing fresh-admission fixture.
use super::fixture::debug;
use super::{Dependencies, TestResult};
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::FirstEntryOutcome};

pub(super) async fn admit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    request: &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
) -> TestResult {
    let (outcome, _, _) = Box::pin(owner.initialize_first_entry(
        request,
        dependencies.admitted.game_session_id,
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
        dependencies.actor,
    ))
    .await;
    assert!(
        matches!(
            outcome,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{outcome:?}"
    );
    Ok(())
}

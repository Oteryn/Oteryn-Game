//! Read the actual committed state through the existing bounded busy-root retry.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, debug};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::{AdmissionRetryBudget, connection::AdmissionRefusal};

pub(super) fn classify(error: crate::durability::DurabilityError) -> AdmissionRefusal {
    match error {
        crate::durability::DurabilityError::RootUnavailable => AdmissionRefusal::Unavailable,
        _ => AdmissionRefusal::Rejected,
    }
}

pub(super) async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    track: &str,
) -> TestResult<i64> {
    let fence = admitted.item_fence.ok_or("proven admitted item fence")?;
    let state = AdmissionRetryBudget::for_command(owner.root)
        .retry_unavailable(async || {
            owner.root.read_character_quest_state(authority, fence.character_id)
                .await.map_err(|error| match error {
                    crate::durability::character_progression::CharacterProgressionError::Unavailable(error) => classify(error),
                    _ => AdmissionRefusal::Rejected,
                })
        })
        .await.map_err(debug)?;
    Ok(state.tracks().get(track).copied().unwrap_or(0))
}

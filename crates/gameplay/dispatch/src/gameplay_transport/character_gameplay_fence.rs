//! Current session and Character fence, shared by native gameplay consumers.
use super::*;
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::character_progression::CurrentCharacterGameplayFence;

pub(super) async fn current(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    session_id: GameSessionId,
    reconnectable: bool,
) -> Option<CurrentCharacterGameplayFence> {
    current_read(admission, session_id, reconnectable)
        .await
        .ok()
        .flatten()
}

/// Keep storage availability distinct from a refused fence for bounded read retries.
pub(super) async fn current_read(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    session_id: GameSessionId,
    reconnectable: bool,
) -> Result<Option<CurrentCharacterGameplayFence>, DurabilityError> {
    let (session, _) = FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(session_id)
        .await?;
    if session.session_state() != GameSessionState::Active
        && !(reconnectable && session.session_state() == GameSessionState::Reconnectable)
    {
        return Ok(None);
    }
    let Ok(character) = CharacterId::from_bytes(*session.commit().character_id().as_bytes()) else {
        return Ok(None);
    };
    let record = match admission
        .root
        .read_current_character(admission.character, character)
        .await
    {
        Ok(record) => record,
        Err(CharacterAuthorityError::Unavailable(error)) => return Err(error),
        Err(
            CharacterAuthorityError::Rejected
            | CharacterAuthorityError::Conflict
            | CharacterAuthorityError::NameUnavailable,
        ) => return Ok(None),
    };
    Ok(Some(CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: session_id,
        connection_generation: session.current_connection_generation(),
        character_lease_generation: session.current_character_lease().generation(),
        runtime_scope: session.current_runtime_scope(),
        scope_ownership_generation: session.current_scope_generation(),
        expected_character_revision: record.revision,
    }))
}

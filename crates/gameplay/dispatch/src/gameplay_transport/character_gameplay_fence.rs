//! Current session and Character fence, shared by native gameplay consumers.
use super::*;
use crate::domain::CharacterId;
use crate::durability::character_progression::CurrentCharacterGameplayFence;

pub(super) async fn current(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    session_id: GameSessionId,
    reconnectable: bool,
) -> Option<CurrentCharacterGameplayFence> {
    current_once(admission, session_id, reconnectable)
        .await
        .ok()
        .flatten()
}

/// One durable read attempt with non-blocking holder acquisition. Callers retain a busy
/// error until their runtime guards are dropped, then retry without weakening fences.
pub(super) async fn current_once(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    session_id: GameSessionId,
    reconnectable: bool,
) -> Result<Option<CurrentCharacterGameplayFence>, crate::durability::DurabilityError> {
    let (session, record) = match admission
        .root
        .read_session_character(admission.character, session_id)
        .await
    {
        Ok(read) => read,
        Err(crate::durability::character_authority::CharacterAuthorityError::Unavailable(
            error,
        )) => {
            return Err(error);
        }
        Err(_) => return Ok(None),
    };
    if session.session_state() != GameSessionState::Active
        && !(reconnectable && session.session_state() == GameSessionState::Reconnectable)
    {
        return Ok(None);
    }
    let Ok(character) = CharacterId::from_bytes(*session.commit().character_id().as_bytes()) else {
        return Ok(None);
    };
    let Some(record) = record else {
        return Ok(None);
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

//! Current session and Character fence, shared by native gameplay consumers.
use super::*;
use crate::domain::CharacterId;
use crate::durability::character_progression::CurrentCharacterGameplayFence;

pub(super) async fn current(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    session_id: GameSessionId,
    reconnectable: bool,
) -> Option<CurrentCharacterGameplayFence> {
    let (session, _) = FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(session_id)
        .await
        .ok()?;
    if session.session_state() != GameSessionState::Active
        && !(reconnectable && session.session_state() == GameSessionState::Reconnectable)
    {
        return None;
    }
    let character = CharacterId::from_bytes(*session.commit().character_id().as_bytes()).ok()?;
    let record = admission
        .root
        .read_current_character(admission.character, character)
        .await
        .ok()?;
    Some(CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: session_id,
        connection_generation: session.current_connection_generation(),
        character_lease_generation: session.current_character_lease().generation(),
        runtime_scope: session.current_runtime_scope(),
        scope_ownership_generation: session.current_scope_generation(),
        expected_character_revision: record.revision,
    })
}

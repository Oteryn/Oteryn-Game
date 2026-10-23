//! Atomic tutorial step reset inside the existing death receipt transaction.
use super::character_tutorial::{TutorialProgressOutcome, tutorial_progress_in_transaction};
use super::{DurabilityError, character_death, spell_familiar_group};
use crate::domain::CharacterId;
use oteryn_gameplay_quests::tutorial::{ISLET_TEMPLE, TutorialEvent, tutorial_transition};

pub(super) async fn reset_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    respawn: &[u8],
) -> Result<(), DurabilityError> {
    let expected = [
        ISLET_TEMPLE.0.to_be_bytes().as_slice(),
        ISLET_TEMPLE.1.to_be_bytes().as_slice(),
        ISLET_TEMPLE.2.to_be_bytes().as_slice(),
    ]
    .concat();
    if respawn != expected {
        return Ok(());
    }
    let TutorialProgressOutcome::Read(Some(stored)) =
        tutorial_progress_in_transaction(tx, character, None).await?
    else {
        return Ok(());
    };
    let build = character_death::load_death_build(tx, character).await?;
    if tutorial_transition(
        true,
        Some(stored.progress),
        TutorialEvent::Death,
        build.vocation(),
        false,
    )
    .is_none()
    {
        return Ok(());
    }
    let group = spell_familiar_group::read(tx, character)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    if let Some(next) = tutorial_transition(
        true,
        Some(stored.progress),
        TutorialEvent::Death,
        build.vocation(),
        group.access(),
    ) {
        tutorial_progress_in_transaction(tx, character, Some((stored.version, next.progress)))
            .await?;
    }
    Ok(())
}

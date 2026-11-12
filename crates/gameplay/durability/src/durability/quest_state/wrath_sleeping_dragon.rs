//! Crystal 201189f a_sleeping_dragon.lua:185-198: Wayfarer and the two
//! quest stores are one completion, never separate best-effort rewards.
use super::super::DurabilityError;
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestTrackChange, wrath_sleeping_dragon as quest};

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    if request.transition_key != quest::COMPLETE {
        return Ok(None);
    }
    if !matches!(request.cause, QuestCause::Command(_)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let key = "oteryn:achievement/wayfarer";
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if entry.retired {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:quest-wrath-dream".into(),
            event_id: Vec::new(),
        },
    }))
}

pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    changes: &[QuestTrackChange],
    achievement: Option<&AchievementGrantRequest>,
) -> Result<(), DurabilityError> {
    let Some(achievement) = achievement else {
        return Ok(());
    };
    // Validate the actual locked changes; a malformed catalogue cannot claim a
    // successful finale or create the reward without both source quest writes.
    if request.transition_key != quest::COMPLETE
        || changes.len() != 3
        || !changes
            .iter()
            .any(|c| c.track == quest::QUESTLINE && c.before == 27 && c.after == 28)
        || !changes
            .iter()
            .any(|c| c.track == quest::MISSION && c.after == 2)
        || !changes
            .iter()
            .any(|c| c.track == quest::POTION && c.before == 1 && c.after == 1)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut grant = achievement.clone();
    // The finale is once per character; account achievement ownership additionally
    // deduplicates another character completing it on the same account.
    grant.source.event_id = fence.character_id.as_bytes().to_vec();
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

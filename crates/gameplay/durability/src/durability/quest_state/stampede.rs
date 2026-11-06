//! A single lethal occurrence writes the counter and its threshold achievement.
use super::super::DurabilityError;
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestTrackChange, stampede as quest};

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    if request.transition_key != quest::KILL {
        return Ok(None);
    }
    if !matches!(request.cause, QuestCause::CreatureDeath(_)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(quest::ACHIEVEMENT)
        .filter(|entry| !entry.retired)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(AchievementGrantRequest {
        achievement_key: quest::ACHIEVEMENT.into(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:quest-stampede".into(),
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
    let [change] = changes else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if request.transition_key != quest::KILL
        || change.track != quest::TRACK
        || !(0..quest::THRESHOLD).contains(&change.before)
        || change.after != change.before + 1
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if change.after != quest::THRESHOLD {
        return Ok(());
    }
    let QuestCause::CreatureDeath(occurrence) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let mut grant = achievement.clone();
    grant.source.event_id = occurrence.as_bytes().to_vec();
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

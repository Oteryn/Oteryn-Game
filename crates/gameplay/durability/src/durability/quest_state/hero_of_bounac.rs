//! Kesar's post-siege report grants the title and Lionheart in its existing quest transaction.
//! Crystal 201189f data-global/npc/kesar_the_younger_day.lua:241-255.
use super::super::DurabilityError;
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestTrackChange, hero_of_bounac as quest};

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    if request.transition_key != quest::COMPLETE {
        return Ok(None);
    }
    if !matches!(request.cause, QuestCause::Command(_)) {
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
            kind: "oteryn:quest-kesar-final-report".into(),
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
    if request.transition_key != quest::COMPLETE
        || changes.len() != 2
        || !changes
            .iter()
            .any(|change| change.track == quest::MISSION && change.before >= 5 && change.after == 5)
        || !changes
            .iter()
            .any(|change| change.track == quest::DOOR && change.after == 1)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut grant = achievement.clone();
    grant.source.event_id = fence.character_id.as_bytes().to_vec();
    let character = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, character, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms) VALUES(encode($1,'hex')::uuid,$2,floor(extract(epoch FROM statement_timestamp())*1000)::bigint) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(oteryn_protocol_oteryn::character_info::CharacterTitle::HeroOfBounac.id())).execute(&mut **tx).await?;
    Ok(())
}

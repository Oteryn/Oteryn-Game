//! The qualified Tirecz death earns Champion of Chazorai even outside questline 25.
use super::super::DurabilityError;
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::{QuestCause, QuestTransitionRequest};

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    use oteryn_gameplay_quests::quest::the_new_frontier::TIRECZ_VICTORY;
    if request.transition_key != TIRECZ_VICTORY {
        return Ok(None);
    }
    let QuestCause::CreatureDeath(occurrence) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let key = "oteryn:achievement/champion_of_chazorai";
    let catalogue = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = catalogue
        .get(key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let catalogue = if entry.retired {
        AchievementCatalogueLookup::Retired
    } else {
        AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        }
    };
    Ok(Some(AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue,
        source: AchievementSourceEvent {
            kind: "oteryn:quest-tirecz-death".into(),
            event_id: occurrence.as_bytes().to_vec(),
        },
    }))
}

pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    achievement: &AchievementGrantRequest,
) -> Result<(), DurabilityError> {
    let mut grant = achievement.clone();
    grant
        .source
        .event_id
        .extend_from_slice(fence.character_id.as_bytes());
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

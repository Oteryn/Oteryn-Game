//! Talented Dancer is earned in the same fenced transaction as the final dance step.
//!
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:55-60;
//! data/scripts/lib/register_achievements.lua:60.

use super::DurabilityError;
use super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::character_progression::CurrentCharacterGameplayFence;
use super::quest_state::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{
    QuestTrackChange, unnatural_selection,
    world_callbacks::{Callback, NativeRule},
};

const ACHIEVEMENT: &str = "oteryn:achievement/talented_dancer";

fn is_dance(callback: &Callback) -> bool {
    callback.quest == unnatural_selection::QUEST
        && matches!(
            callback.native_rule.as_ref(),
            Some(NativeRule::UnnaturalDance { .. })
        )
}

/// Resolve the existing compatible catalogue before opening the Character transaction.
pub(super) fn prepare(
    callback: Option<&Callback>,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    if !callback.is_some_and(is_dance) {
        return Ok(None);
    }
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(ACHIEVEMENT)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let catalogue = if entry.retired {
        AchievementCatalogueLookup::Retired
    } else {
        AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        }
    };
    Ok(Some(AchievementGrantRequest {
        achievement_key: ACHIEVEMENT.into(),
        catalogue,
        source: AchievementSourceEvent {
            kind: "oteryn:quest-unnatural-dance".into(),
            event_id: Vec::new(),
        },
    }))
}

fn completed(changes: &[QuestTrackChange]) -> bool {
    changes.len() == 3
        && changes.iter().any(|change| {
            change.track == unnatural_selection::DANCE_STATUS_TRACK
                && change.before == 16
                && change.after == 17
        })
        && changes
            .iter()
            .any(|change| change.track == unnatural_selection::MISSION03_TRACK && change.after == 3)
        && changes
            .iter()
            .any(|change| change.track == unnatural_selection::QUESTLINE_TRACK && change.after == 7)
}

/// Called only after the existing quest receipt and changes are inserted under
/// the complete Character fence and root lock. An error rolls back that transaction.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    changes: &[QuestTrackChange],
    achievement: Option<&AchievementGrantRequest>,
) -> Result<(), DurabilityError> {
    if !is_dance(callback) || !completed(changes) {
        return Ok(());
    }
    if !matches!(request.cause, QuestCause::Step(_)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut grant = achievement
        .ok_or(DurabilityError::InvalidStoredState)?
        .clone();
    let (cause, ordinal) = request.cause.key();
    grant.source.event_id = fence.character_id.as_bytes().to_vec();
    grant.source.event_id.extend_from_slice(&cause);
    grant
        .source
        .event_id
        .extend_from_slice(&ordinal.to_be_bytes());
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn final_changes() -> Vec<QuestTrackChange> {
        vec![
            QuestTrackChange {
                track: unnatural_selection::DANCE_STATUS_TRACK.into(),
                before: 16,
                after: 17,
            },
            QuestTrackChange {
                track: unnatural_selection::MISSION03_TRACK.into(),
                before: 2,
                after: 3,
            },
            QuestTrackChange {
                track: unnatural_selection::QUESTLINE_TRACK.into(),
                before: 6,
                after: 7,
            },
        ]
    }

    #[test]
    fn only_the_exact_finished_state_earns_talented_dancer() {
        let mut changes = final_changes();
        assert!(completed(&changes));
        changes[0].after = 1;
        assert!(
            !completed(&changes),
            "a failed final step only resets the sequence"
        );
        changes[0].after = 17;
        changes[0].before = 15;
        assert!(
            !completed(&changes),
            "a skipped step cannot grant the achievement"
        );
        changes[0].before = 16;
        changes[1].after = 2;
        assert!(
            !completed(&changes),
            "the mission must be part of the same commit"
        );
        changes[1].after = 3;
        changes[2].after = 6;
        assert!(
            !completed(&changes),
            "the questline must be part of the same commit"
        );
        assert!(!completed(&final_changes()[..2]));
    }

    #[test]
    fn the_existing_catalogue_contains_the_exact_source_achievement() -> Result<(), String> {
        let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
            &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
        )
        .map_err(|error| format!("{error:?}"))?;
        let entry = entries.get(ACHIEVEMENT).ok_or("Talented Dancer missing")?;
        assert_eq!(entry.name, "Talented Dancer");
        assert!(!entry.retired);
        Ok(())
    }
}

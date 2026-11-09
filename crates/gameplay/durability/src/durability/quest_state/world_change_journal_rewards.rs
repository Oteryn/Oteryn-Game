//! Native goal achievements share the fenced quest completion transaction.
use super::super::DurabilityError;
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrackChange, native_kills};
use sha2::{Digest, Sha256};

const SOURCE: &str = "oteryn:world-change-journal-goal";

fn prepare_key(key: &str) -> Result<AchievementGrantRequest, DurabilityError> {
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue: if entry.retired {
            AchievementCatalogueLookup::Retired
        } else {
            AchievementCatalogueLookup::Earnable {
                revision: entry.revision.clone(),
            }
        },
        source: AchievementSourceEvent {
            kind: SOURCE.into(),
            event_id: Vec::new(),
        },
    })
}

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> Result<Option<AchievementGrantRequest>, DurabilityError> {
    let Some(key) =
        oteryn_gameplay_quests::quest::world_change_journals::counter_completion_achievement(
            &request.transition_key,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?
    else {
        return Ok(None);
    };
    if !matches!(request.cause, QuestCause::CreatureDeath(_)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    prepare_key(key).map(Some)
}

pub(super) fn bind(
    binding: &mut [u8; 33],
    grant: Option<&AchievementGrantRequest>,
) -> Result<(), DurabilityError> {
    let Some(grant) = grant else { return Ok(()) };
    let revision = match &grant.catalogue {
        AchievementCatalogueLookup::Earnable { revision } => Some(revision.as_str()),
        AchievementCatalogueLookup::Retired => None,
        AchievementCatalogueLookup::Absent => return Err(DurabilityError::InvalidStoredState),
    };
    let policy = serde_json::to_vec(&(&grant.achievement_key, revision))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let digest = Sha256::new()
        .chain_update(*binding)
        .chain_update(SOURCE.as_bytes())
        .chain_update(policy)
        .finalize();
    binding[1..].copy_from_slice(&digest);
    Ok(())
}

fn reaches_goal(counter: native_kills::CounterRef<'_>, changes: &[QuestTrackChange]) -> bool {
    counter.maximum > counter.minimum
        && counter.maximum.checked_sub(1).is_some_and(|before| {
            changes.iter().any(|change| {
                change.track == counter.track
                    && change.before == before
                    && change.after == counter.maximum
            })
        })
}

pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    catalogue: &QuestStateCatalogue,
    changes: &[QuestTrackChange],
    native: bool,
    achievement: Option<&AchievementGrantRequest>,
) -> Result<(), DurabilityError> {
    let Some(achievement) = achievement else {
        return Ok(());
    };
    if !native || !matches!(request.cause, QuestCause::CreatureDeath(_)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let transition = catalogue
        .transition(&request.transition_key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let counter = native_kills::counter_for(catalogue, &request.transition_key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if !transition.completes || counter.quest != transition.quest {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !reaches_goal(counter, changes) {
        return Ok(());
    }
    if !catalogue.completes_after(transition, changes) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let (cause, ordinal) = request.cause.key();
    let mut grant = achievement.clone();
    grant.source.event_id = Sha256::new()
        .chain_update(SOURCE.as_bytes())
        .chain_update(fence.character_id.as_bytes())
        .chain_update(request.transition_key.as_bytes())
        .chain_update(cause)
        .chain_update(ordinal.to_be_bytes())
        .finalize()
        .to_vec();
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter() -> native_kills::CounterRef<'static> {
        native_kills::CounterRef {
            death_area: None,
            encounter_source: None,
            transition: "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragon",
            creature: "oteryn:creature.dragon",
            quest: "oteryn:quest.authored.fire_from_the_earth_mini_world_change",
            track: "oteryn:quest-progress/world-change-journal/fire_from_the_earth/kills",
            gate: "oteryn:quest-progress/world-change-journal/fire_from_the_earth/enabled",
            gate_value: 1,
            minimum: 0,
            maximum: 50,
        }
    }

    #[test]
    fn only_the_fiftieth_actual_increment_can_earn_the_goal_achievement() {
        let counter = counter();
        for (before, after) in [(0, 1), (48, 49), (0, 50), (50, 50), (50, 51)] {
            assert!(!reaches_goal(
                counter,
                &[QuestTrackChange {
                    track: counter.track.into(),
                    before,
                    after
                }]
            ));
        }
        assert!(reaches_goal(
            counter,
            &[QuestTrackChange {
                track: counter.track.into(),
                before: 49,
                after: 50
            }]
        ));
        assert!(!reaches_goal(counter, &[]));
        assert!(!reaches_goal(
            counter,
            &[QuestTrackChange {
                track: counter.gate.into(),
                before: 49,
                after: 50
            }]
        ));
    }

    #[test]
    fn achievement_policy_is_catalogue_checked_and_binds_key_and_revision()
    -> Result<(), DurabilityError> {
        let grant = prepare_key("oteryn:achievement/fire_from_the_earth")?;
        assert!(matches!(
            grant.catalogue,
            AchievementCatalogueLookup::Earnable { .. }
        ));
        assert!(prepare_key("oteryn:achievement/not_in_the_world_catalogue").is_err());
        let mut original = [1; 33];
        bind(&mut original, None)?;
        assert_eq!(original, [1; 33]);
        let mut first = original;
        bind(&mut first, Some(&grant))?;
        assert_ne!(first, original);
        assert_eq!(first[0], original[0]);
        let mut revised = grant.clone();
        revised.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "other-revision".into(),
        };
        let mut second = original;
        bind(&mut second, Some(&revised))?;
        assert_ne!(first, second);
        revised.achievement_key = "oteryn:achievement/loyal_subject".into();
        let mut third = original;
        bind(&mut third, Some(&revised))?;
        assert_ne!(second, third);
        revised.catalogue = AchievementCatalogueLookup::Absent;
        assert!(bind(&mut original, Some(&revised)).is_err());
        Ok(())
    }
}

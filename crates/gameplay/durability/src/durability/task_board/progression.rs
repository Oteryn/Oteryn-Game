//! Weekly completions use the admitted progression oracle. Two completions from
//! one kill recalculate the second reward after the first reward's level-up.
use super::{TaskBoardCatalogue, TaskBoardView};
use crate::domain::progression::{
    CurrentProgressionSnapshot, FiniteProgressionPolicy, ProgressionOperation,
    calculate_progression,
};
use crate::durability::DurabilityError;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, FencedCharacterRoot, numeric_u64,
    state_matches_root, stored_context_matches,
};
use oteryn_gameplay_content::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS;
use oteryn_gameplay_quests::quest::task_board::full::{TaskBoardEffects, weekly_experience};
use oteryn_simulation_determinism::ExactI64;
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

type Snapshot = CurrentProgressionSnapshot<(), ()>;
type Policy = FiniteProgressionPolicy<String, CHARACTER_EXPERIENCE_TABLE_LEVELS>;

pub(super) async fn qualify(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    root: &FencedCharacterRoot,
    catalogue: &TaskBoardCatalogue,
    inactive_kill: bool,
) -> Result<PgRow, CharacterProgressionError> {
    let row = sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await
        .map_err(DurabilityError::from)?.ok_or(CharacterProgressionError::MissingProgressionState)?;
    if numeric_u64(&row, "character_revision")? != root.revision {
        return Err(CharacterProgressionError::CharacterRevisionMismatch);
    }
    if !state_matches_root(&row, root)
        || (!inactive_kill && catalogue.gameplay_content_revision != root.content_revision)
    {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    Ok(row)
}

/// The root lock also serializes quest-obligation insertion/consumption. Read in
/// the existing quest-copy order and project already owed XP without paying it.
pub(super) async fn project_pending(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    row: &PgRow,
    view: &mut TaskBoardView,
    catalogue: &TaskBoardCatalogue,
    inactive_kill: bool,
) -> Result<Snapshot, CharacterProgressionError> {
    let mut snapshot = Snapshot {
        level: view.level,
        total_experience: ExactI64::new(
            row.try_get("total_experience")
                .map_err(DurabilityError::from)?,
        ),
        skill_progression: (),
        magic_progression: (),
    };
    if inactive_kill {
        return Ok(snapshot);
    }
    let limit = oteryn_gameplay_quests::quest::QUESTGATE0_RL_10;
    let pending = sqlx::query_scalar::<_, i64>("SELECT amount FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid ORDER BY created_at,reward_occurrence_id LIMIT $2")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(i64::try_from(limit + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    if pending.len() > limit {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    if !pending.is_empty() {
        let policy = policy(row, catalogue)?;
        for amount in pending {
            award(&mut snapshot, amount, &policy)?;
        }
        view.level = snapshot.level;
    }
    Ok(snapshot)
}

pub(super) fn plan_experience(
    row: &PgRow,
    view: &mut TaskBoardView,
    catalogue: &TaskBoardCatalogue,
    mut snapshot: Snapshot,
    effects: &mut TaskBoardEffects,
) -> Result<(), CharacterProgressionError> {
    if effects.experience == 0 && effects.weekly_completions == 0 {
        return Ok(());
    }
    if effects.weekly_completions > 2 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let policy = policy(row, catalogue)?;
    if effects.weekly_completions > 0 {
        let difficulty = view
            .data
            .weekly
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?
            .difficulty;
        effects.experience = weekly_total(
            &mut snapshot,
            difficulty,
            effects.weekly_completions,
            &policy,
        )?;
    } else {
        award(
            &mut snapshot,
            i64::try_from(effects.experience).map_err(|_| DurabilityError::InvalidStoredState)?,
            &policy,
        )?;
    }
    // Only the reply projection changes here. The existing XP obligation owner
    // performs the eventual durable level/experience commit exactly once.
    view.level = snapshot.level;
    Ok(())
}

fn policy(
    row: &PgRow,
    catalogue: &TaskBoardCatalogue,
) -> Result<Policy, CharacterProgressionError> {
    let content = catalogue
        .progression
        .as_ref()
        .ok_or(CharacterProgressionError::ProgressionContextMismatch)?;
    let profile: String = row
        .try_get("profile_revision")
        .map_err(DurabilityError::from)?;
    let ruleset: String = row
        .try_get("ruleset_revision")
        .map_err(DurabilityError::from)?;
    let content_revision: String = row
        .try_get("content_revision")
        .map_err(DurabilityError::from)?;
    let policy = content
        .policy_for(&profile, &ruleset, &content_revision)
        .map_err(|_| CharacterProgressionError::ProgressionContextMismatch)?;
    if !stored_context_matches(
        row,
        &policy.context,
        &policy.policy_revision,
        &policy.reward_revision,
    ) {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    Ok(policy)
}

fn weekly_total<const N: usize>(
    snapshot: &mut Snapshot,
    difficulty: u8,
    completions: u8,
    policy: &FiniteProgressionPolicy<String, N>,
) -> Result<u64, CharacterProgressionError> {
    let mut total = 0_u64;
    for _ in 0..completions {
        let amount = weekly_experience(snapshot.level, difficulty)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        award(
            snapshot,
            i64::try_from(amount).map_err(|_| DurabilityError::InvalidStoredState)?,
            policy,
        )?;
        total = total
            .checked_add(amount)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(total)
}

fn award<const N: usize>(
    snapshot: &mut Snapshot,
    amount: i64,
    policy: &FiniteProgressionPolicy<String, N>,
) -> Result<(), CharacterProgressionError> {
    let operation = ProgressionOperation::AwardExperience {
        source_occurrence: (),
        reward_revision: policy.reward_revision.clone(),
        amount: ExactI64::new(amount),
    };
    let next = calculate_progression(
        snapshot,
        &policy.context,
        &policy.policy_revision,
        &operation,
        policy,
    )
    .map_err(CharacterProgressionError::Calculation)?;
    snapshot.level = next.level_after;
    snapshot.total_experience = next.experience_after;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::progression::{LevelThreshold, ProgressionRevisionContext};
    use oteryn_simulation_determinism::RoundingMode;

    #[test]
    fn simultaneous_completions_recalculate_after_the_first_level_up()
    -> Result<(), CharacterProgressionError> {
        let context = ProgressionRevisionContext {
            profile: "profile".to_owned(),
            ruleset: "ruleset".to_owned(),
            content: "content".to_owned(),
            simulation: "simulation".to_owned(),
            evidence: "evidence".to_owned(),
            declaration: "declaration".to_owned(),
        };
        let policy = FiniteProgressionPolicy {
            context,
            policy_revision: "policy".to_owned(),
            reward_revision: "reward".to_owned(),
            death_policy_revision: "death".to_owned(),
            declared_difference_revision: "declaration".to_owned(),
            thresholds: [
                LevelThreshold {
                    level: 10,
                    minimum_experience: ExactI64::new(9_300),
                },
                LevelThreshold {
                    level: 11,
                    minimum_experience: ExactI64::new(13_200),
                },
                LevelThreshold {
                    level: 12,
                    minimum_experience: ExactI64::new(17_600),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(22_200),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        };
        let snapshot = |total| Snapshot {
            level: 10,
            total_experience: ExactI64::new(total),
            skill_progression: (),
            magic_progression: (),
        };
        let mut crossing = snapshot(13_200 - 1_850);
        assert_eq!(weekly_total(&mut crossing, 0, 2, &policy)?, 1_850 + 2_300);
        assert_eq!(crossing.level, 11);
        let mut same_level = snapshot(9_300);
        assert_eq!(weekly_total(&mut same_level, 0, 2, &policy)?, 2 * 1_850);
        Ok(())
    }
}

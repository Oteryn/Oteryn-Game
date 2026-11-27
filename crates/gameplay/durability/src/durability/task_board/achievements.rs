//! Tiered and overall Task Board milestones from the captured TibiaWiki achievement facts.
//! Crystal 201189f ioweeklytasks.cpp:244-264,342-345 and iobountytasks.cpp:527-528
//! identify the completion outcomes. Weekly payouts and bounty claims do not count again.

use super::{QuestCause, state};
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::account_achievement::add_progress;
use oteryn_gameplay_quests::quest::task_board::full::{
    BountyStatus, TaskBoardData, TaskBoardEffects,
};
use sqlx::{Postgres, Transaction};

const SOURCE: &str = "oteryn:task-board";
const TASKMASTER: (&str, u32) = ("oteryn:achievement/taskmaster", 500);
const WEEKLY_KILLS: [(&str, u32); 4] = [
    ("oteryn:achievement/verminbane", 100),
    ("oteryn:achievement/monsterhunter", 200),
    TASKMASTER,
    TASKMASTER,
];
const WEEKLY: [(&str, u32); 2] = [
    ("oteryn:achievement/errand_runner", 100),
    ("oteryn:achievement/workhorse", 500),
];

pub(super) async fn award_completed_tasks(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    cause: QuestCause,
    before: &TaskBoardData,
    after: &TaskBoardData,
    effects: &TaskBoardEffects,
) -> Result<(), DurabilityError> {
    let killed = matches!(cause, QuestCause::CreatureDeath(_));
    let bounty_completed = killed
        && before.bounty.state == BountyStatus::Active
        && after.bounty.state == BountyStatus::Completed;
    if effects.weekly_completions == 0 && !bounty_completed {
        return Ok(());
    }
    let (_, cause_id, ordinal) = state::cause_key(cause);
    let mut occurrence = [0; 40];
    occurrence[..16].copy_from_slice(character.as_bytes());
    occurrence[16..32].copy_from_slice(&cause_id);
    occurrence[32..].copy_from_slice(&ordinal.to_be_bytes());

    if effects.weekly_completions > 0 {
        let difficulty = after
            .weekly
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?
            .difficulty;
        let &tier = WEEKLY_KILLS
            .get(usize::from(difficulty))
            .ok_or(DurabilityError::InvalidStoredState)?;
        for (key, total) in WEEKLY.into_iter().chain(killed.then_some(tier)) {
            for _ in 0..effects.weekly_completions {
                add_progress(tx, character, key, total, SOURCE, &occurrence).await?;
            }
        }
    }
    if bounty_completed {
        add_progress(
            tx,
            character,
            "oteryn:achievement/pest_control",
            10_000,
            SOURCE,
            &occurrence,
        )
        .await?;
    }
    Ok(())
}

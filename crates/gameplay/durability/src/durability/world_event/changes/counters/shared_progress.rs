//! Source-qualified quest receipts join the existing shared counter transaction.
use super::{CounterKind, Credit, CreditAuthority, CreditEvent, CreditOutcome};
use crate::durability::{DurabilityError, quest_state::QuestCause};
use crate::foundation::{CharacterId, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::{
    QuestTrackChange,
    shared_world_progress::{self, SharedAction},
};
use sqlx::{Postgres, Transaction};

/// Resolve the current source cycle under its counter lock. Call only after
/// current World/node authorization, with the Character root already locked.
/// The lock keeps the lifecycle stable through the eventual Quest receipt.
pub(crate) async fn first_in_hive_cycle(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    character: CharacterId,
) -> Result<Option<bool>, DurabilityError> {
    let generation: Option<String> = sqlx::query_scalar(
        "SELECT coalesce(c.state->>'occurrence_generation','1') FROM game_world_change_counters c JOIN game_world_reset_active a ON a.world_id=c.world_id AND a.reset_epoch=c.reset_epoch WHERE c.world_id=encode($1,'hex')::uuid AND c.change_key='hive_born' FOR UPDATE OF c",
    ).bind(scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(generation) = generation else {
        return Ok(None);
    };
    if generation
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .is_none()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let accepts: Vec<_> = (0..shared_world_progress::ACTIVE_TRACKS.len())
        .map(shared_world_progress::accept_key)
        .collect();
    let accepted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_counter_quest_scopes s JOIN game_character_event_quest_progress p USING(character_id,committed_character_revision) WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND s.change_key='hive_born' AND s.occurrence_generation=$3::text::numeric AND p.transition_key=ANY($4::text[]))",
    ).bind(character.as_bytes().as_slice()).bind(scope.world_id().as_bytes().as_slice()).bind(generation).bind(accepts).fetch_one(&mut **tx).await?;
    Ok(Some(!accepted))
}

fn cycle_acceptance_shape(task: u8, changes: &[QuestTrackChange]) -> bool {
    let selected = shared_world_progress::ACTIVE_TRACKS[usize::from(task)];
    changes
        .iter()
        .filter(|change| change.track == selected && change.after == 1)
        .count()
        == 1
        && changes.iter().enumerate().all(|(index, change)| {
            shared_world_progress::counter_progress_track(
                shared_world_progress::HIVE_QUEST,
                &change.track,
            ) && change.before >= 0
                && change.after == i64::from(change.track == selected)
                && !changes[..index]
                    .iter()
                    .any(|earlier| earlier.track == change.track)
        })
}

/// Call after the complete native quest/object/NPC proof and before any quest
/// receipt commits. The authority is independently resolved in this same Tx;
/// source-kind2 deduplicates this immutable session/command across all task keys.
#[allow(
    clippy::too_many_arguments,
    reason = "current counter authority and native receipt facts remain explicit"
)]
pub(crate) async fn credit_completed(
    tx: &mut Transaction<'_, Postgres>,
    authority: &CreditAuthority,
    scope: RuntimeScopeRefV1,
    character: CharacterId,
    epoch: u64,
    key: &str,
    cause: QuestCause,
    changes: &[QuestTrackChange],
) -> Result<CreditOutcome, DurabilityError> {
    if let Some(task) = shared_world_progress::accepted_task(key) {
        let first = first_in_hive_cycle(tx, scope, character).await?;
        let valid = match first {
            Some(true) => cycle_acceptance_shape(task, changes),
            Some(false) => {
                changes.len() == 1
                    && changes[0].track == shared_world_progress::ACTIVE_TRACKS[usize::from(task)]
                    && (changes[0].before, changes[0].after) == (0, 1)
            }
            None => false,
        };
        if !matches!(cause, QuestCause::Command(_)) || !valid {
            return Ok(CreditOutcome::Refused);
        }
        let Some(day) = super::super::load(tx, scope.world_id(), epoch).await? else {
            return Ok(CreditOutcome::Refused);
        };
        let stage = day.counter_stages[CounterKind::HiveBorn.index()];
        return Ok(
            if stage == 3 || (task < 4 && stage == 1) || (task >= 4 && stage == 2) {
                CreditOutcome::Applied {
                    phase_changed: false,
                }
            } else {
                CreditOutcome::Refused
            },
        );
    }
    let Some(action) = shared_world_progress::completed_action(key, changes) else {
        return Ok(CreditOutcome::Refused);
    };
    let command = match (action, cause) {
        (SharedAction::HiveTask(_), QuestCause::Command(command)) => command,
        (
            SharedAction::HeartDestroyed
            | SharedAction::SoulNetDestroyed
            | SharedAction::CoralPathCompleted
            | SharedAction::StairPathCompleted,
            QuestCause::Use(command),
        ) => command,
        _ => return Ok(CreditOutcome::Refused),
    };
    let Some(day) = super::super::load(tx, scope.world_id(), epoch).await? else {
        return Ok(CreditOutcome::Refused);
    };
    let (kind, credit) = match action {
        SharedAction::HiveTask(task) => (CounterKind::HiveBorn, Credit::Task(task)),
        SharedAction::HeartDestroyed => (CounterKind::Deeplings, Credit::HeartDestroyed),
        SharedAction::SoulNetDestroyed => (CounterKind::Deeplings, Credit::SoulNetDestroyed),
        SharedAction::CoralPathCompleted => (CounterKind::Deeplings, Credit::CoralPathCompleted),
        SharedAction::StairPathCompleted => (CounterKind::Deeplings, Credit::StairPathCompleted),
    };
    // Reject ineligible stage instead of consuming personal objective progress.
    let stage = day.counter_stages[kind.index()];
    let eligible = match action {
        SharedAction::HiveTask(task) => {
            if stage == 3 {
                true
            } else if task < 4 {
                stage == 1
            } else {
                stage == 2
            }
        }
        SharedAction::HeartDestroyed | SharedAction::SoulNetDestroyed => stage == 1,
        SharedAction::CoralPathCompleted | SharedAction::StairPathCompleted => stage == 2,
    };
    if !eligible {
        return Ok(CreditOutcome::Refused);
    }
    let mut occurrence = scope.world_id().as_bytes().to_vec();
    if matches!(action, SharedAction::HiveTask(_)) {
        occurrence.extend_from_slice(command.game_session_id().as_bytes());
        occurrence.extend_from_slice(&command.command_id().get().to_be_bytes());
    } else {
        // One world object can contribute only once in this save, including
        // across actor changes, restarts and a recreated Channel projection.
        let binding =
            shared_world_progress::binding(key).ok_or(DurabilityError::InvalidStoredState)?;
        occurrence.extend_from_slice(&epoch.to_be_bytes());
        occurrence.extend_from_slice(&binding.position.0.to_be_bytes());
        occurrence.extend_from_slice(&binding.position.1.to_be_bytes());
        occurrence.push(binding.position.2.unsigned_abs());
        occurrence.extend_from_slice(&binding.appearance.to_be_bytes());
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let occurred_at_ms = u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?;
    super::credit(
        tx,
        authority,
        &CreditEvent {
            source_kind: 2,
            occurrence,
            credits: vec![(kind, credit)],
            counter_generations: day.counter_generations,
            occurred_at_ms,
            character: Some(character),
        },
    )
    .await
}

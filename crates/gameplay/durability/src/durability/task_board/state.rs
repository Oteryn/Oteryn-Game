use super::*;
use crate::durability::DurabilityError;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::QuestTrackChange;
use oteryn_gameplay_quests::quest::task_board;
use sqlx::{Postgres, Row, Transaction};

pub(super) fn balances(tracks: &BTreeMap<String, i64>) -> TaskBoardBalances {
    TaskBoardBalances::from_tracks(tracks)
}

pub(super) fn update_tracks(view: &mut TaskBoardView) {
    view.balances.store_into_tracks(&mut view.tracks);
}

/// Called only after the Character root lock and current gameplay fence. The reset
/// period comes from the reset owner's committed epoch, never the process clock.
pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    allow_inactive_without_calendar: bool,
) -> Result<TaskBoardView, DurabilityError> {
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let period: Option<i64> =
        sqlx::query_scalar("SELECT game_task_board_period(encode($1,'hex')::uuid)")
            .bind(world.as_bytes().as_slice())
            .fetch_one(&mut **tx)
            .await?;
    let database_now =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).fetch_one(&mut **tx).await?;
    let row = sqlx::query("SELECT body,uuid_send(world_id) AS world FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character).fetch_optional(&mut **tx).await?;
    let data = if let Some(row) = row {
        if row.try_get::<Vec<u8>, _>("world")? != world.as_bytes() {
            return Err(DurabilityError::InvalidStoredState);
        }
        let json: serde_json::Value = row.try_get("body")?;
        serde_json::from_value(json).map_err(|_| DurabilityError::InvalidStoredState)?
    } else {
        TaskBoardData::default()
    };
    let period = match period {
        Some(period) => period,
        None if allow_inactive_without_calendar && inactive(&data) => 0,
        None => return Err(DurabilityError::InvalidStoredState),
    };
    let rows = sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key LIKE 'oteryn:taskboard/%' ORDER BY track_key LIMIT 65 FOR UPDATE")
        .bind(character).fetch_all(&mut **tx).await?;
    if rows.len() > 64 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut tracks = BTreeMap::new();
    for row in rows {
        tracks.insert(row.try_get("track_key")?, row.try_get("value")?);
    }
    let tokens: Option<i64> = sqlx::query_scalar("SELECT balance FROM game_character_bounty_reroll_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    Ok(TaskBoardView {
        balances: balances(&tracks),
        data,
        tracks,
        level: u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
        period,
        database_now,
        reroll_tokens: u64::try_from(tokens.unwrap_or(0))
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        delivery_available: BTreeMap::new(),
    })
}

pub(super) fn inactive(data: &TaskBoardData) -> bool {
    !data.weekly.as_ref().is_some_and(|weekly| !weekly.finished)
        && data.bounty.state
            != oteryn_gameplay_quests::quest::task_board::full::BountyStatus::Active
}

pub(super) fn cause_key(cause: QuestCause) -> (&'static str, [u8; 16], u64) {
    match cause {
        QuestCause::CreatureDeath(id) => ("creature_death", *id.as_bytes(), 0),
        QuestCause::Command(command) => (
            "command",
            *command.game_session_id().as_bytes(),
            command.command_id().get(),
        ),
        QuestCause::Use(command) => (
            "use",
            *command.game_session_id().as_bytes(),
            command.command_id().get(),
        ),
        QuestCause::Step(command) => (
            "step",
            *command.game_session_id().as_bytes(),
            command.command_id().get(),
        ),
        QuestCause::ClaimObligation(command) => (
            "claim_obligation",
            *command.game_session_id().as_bytes(),
            command.command_id().get(),
        ),
    }
}

pub(super) async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    binding: &[u8],
) -> Result<Option<TaskBoardCommit>, super::writer::Error> {
    let (kind, cause, ordinal) = cause_key(request.cause);
    let row = sqlx::query("SELECT cause_kind,request_binding,reply,refusal FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric")
        .bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice())
        .bind(ordinal.to_string()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let Some(row) = row else { return Ok(None) };
    if row
        .try_get::<String, _>("cause_kind")
        .map_err(DurabilityError::from)?
        != kind
        || row
            .try_get::<Vec<u8>, _>("request_binding")
            .map_err(DurabilityError::from)?
            != binding
    {
        return Err(super::writer::Error::ConflictingOccurrence);
    }
    let view = serde_json::from_value(row.try_get("reply").map_err(DurabilityError::from)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let refusal = serde_json::from_value(row.try_get("refusal").map_err(DurabilityError::from)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Some(TaskBoardCommit {
        view,
        refusal,
        transition: None,
        replayed: true,
    }))
}

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    binding: &[u8],
    outcome: &TaskBoardCommit,
    effects: Option<&oteryn_gameplay_quests::quest::task_board::full::TaskBoardEffects>,
) -> Result<(), DurabilityError> {
    let (kind, cause, ordinal) = cause_key(request.cause);
    let reply =
        serde_json::to_value(&outcome.view).map_err(|_| DurabilityError::InvalidStoredState)?;
    if serde_json::to_vec(&reply)
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .len()
        > MAX_TASK_BOARD_STATE_BYTES
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let refusal =
        serde_json::to_value(&outcome.refusal).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_task_board_receipts(character_id,world_id,cause_kind,cause_id,cause_ordinal,request_binding,reply,refusal,committed_character_revision,action,effects) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9::text::numeric,$10,$11)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(kind).bind(cause.as_slice()).bind(ordinal.to_string()).bind(binding)
        .bind(reply).bind(refusal)
        .bind(outcome.transition.as_ref().map(|t| t.committed_character_revision.get().to_string()))
        .bind(serde_json::to_value(&request.action).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(serde_json::to_value(effects).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?;
    Ok(())
}

/// Freeze the exact track additions before any custody participant runs.
pub(super) fn plan_changes(
    before: &TaskBoardView,
    view: &mut TaskBoardView,
) -> Result<Vec<QuestTrackChange>, DurabilityError> {
    let board_revision = before
        .tracks
        .get(BOARD_REVISION)
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    view.tracks
        .insert(BOARD_REVISION.to_owned(), board_revision);
    let mut changes = Vec::new();
    for key in [
        task_board::POINTS,
        task_board::BOUNTY_POINTS,
        SOULSEALS,
        task_board::TALISMAN_DAMAGE,
        task_board::TALISMAN_LIFE_LEECH,
        task_board::TALISMAN_LOOT,
        task_board::TALISMAN_BESTIARY,
        BOARD_REVISION,
    ] {
        let previous = before.tracks.get(key).copied().unwrap_or(0);
        let next = view.tracks.get(key).copied().unwrap_or(0);
        if previous != next {
            changes.push(QuestTrackChange {
                track: key.to_owned(),
                before: previous,
                after: next,
            });
        }
    }
    changes.sort_by(|left, right| left.track.cmp(&right.track));
    Ok(changes)
}

/// Share the existing quest owner's global state/track admission. In particular,
/// Task Board must preserve space for a derived monk-shrine count on admission.
pub(super) async fn fits_capacity(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    before: &TaskBoardView,
    changes: &[QuestTrackChange],
) -> Result<bool, DurabilityError> {
    use crate::durability::quest_state::capacity;
    let has_state: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_states \
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(task_board::QUEST)
    .fetch_one(&mut **tx)
    .await?;
    if !has_state && !capacity::new_state_fits(tx, fence.character_id).await? {
        return Ok(false);
    }
    let keys: Vec<_> = changes.iter().map(|change| change.track.as_str()).collect();
    let existing = keys
        .iter()
        .filter(|key| before.tracks.contains_key(**key))
        .count();
    capacity::tracks_fit(tx, fence.character_id, &keys, existing).await
}

/// Extend the existing quest receipt chain. Board generation does not create an
/// alternate Character revision or experience writer.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn advance(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    binding: &[u8],
    view: &TaskBoardView,
    changes: Vec<QuestTrackChange>,
    experience: u64,
) -> Result<CommittedQuestTransition, DurabilityError> {
    use crate::domain::CharacterRevision;
    use crate::durability::character_progression::ExperienceRewardOccurrence;
    use crate::durability::quest_state::QuestXpObligation;

    let character = fence.character_id.as_bytes().as_slice();
    let original = fence.expected_character_revision.get();
    let committed = original
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let previous = sqlx::query("SELECT pinned_content_revision,definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
        .bind(character).bind(task_board::QUEST).fetch_optional(&mut **tx).await?;
    let (pin, hash): (String, Vec<u8>) = if let Some(previous) = previous {
        (
            previous.try_get("pinned_content_revision")?,
            previous.try_get("definition_hash")?,
        )
    } else {
        let pin = sqlx::query_scalar("SELECT content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).fetch_one(&mut **tx).await?;
        // Use the same code-built quest definition as the boot catalogue. Its hash
        // covers this quest's declared tracks/transitions only, independently of
        // the other quests loaded at boot. Existing Characters retain their pin.
        let empty = oteryn_gameplay_quests::quest::QuestStateCatalogue::empty(&pin)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let catalogue =
            task_board::extend(&empty).map_err(|_| DurabilityError::InvalidStoredState)?;
        let hash = catalogue
            .definition_hash(task_board::QUEST)
            .ok_or(DurabilityError::InvalidStoredState)?;
        (pin, hash.to_vec())
    };
    if experience > 0 {
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).fetch_one(&mut **tx).await?;
        if pending >= oteryn_gameplay_quests::quest::QUESTGATE0_RL_10 as i64 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    for statement in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
    ] {
        if sqlx::query(statement)
            .bind(character)
            .bind(committed.to_string())
            .bind(original.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    // Commands that can consume owned items use the established quest USE receipt
    // family; the task-board receipt still binds the original command bytes.
    let quest_cause = match request.cause {
        QuestCause::Command(command) => QuestCause::Use(command),
        other => other,
    };
    let (kind, cause, ordinal) = cause_key(quest_cause);
    let transition_key = "oteryn:taskboard/weekly/advance";
    let keys: Vec<_> = changes.iter().map(|change| change.track.as_str()).collect();
    let values_before: Vec<_> = changes.iter().map(|change| change.before).collect();
    let values_after: Vec<_> = changes.iter().map(|change| change.after).collect();
    sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT s.character_id,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9,FALSE,$10::text[],$11::bigint[],$12::bigint[],$13::text::numeric,$14::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid")
        .bind(character).bind(kind).bind(cause.as_slice()).bind(ordinal.to_string()).bind(transition_key)
        .bind(task_board::QUEST).bind(binding).bind(&pin).bind(&hash).bind(&keys).bind(values_before).bind(values_after)
        .bind(original.to_string()).bind(committed.to_string()).execute(&mut **tx).await?;
    for change in &changes {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(character).bind(&change.track).bind(task_board::QUEST).bind(change.after).bind(committed.to_string())
            .execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,NULL,$5::text::numeric) ON CONFLICT(character_id,quest_key) DO UPDATE SET committed_character_revision=EXCLUDED.committed_character_revision")
        .bind(character).bind(task_board::QUEST).bind(&pin).bind(&hash).bind(committed.to_string()).execute(&mut **tx).await?;
    let xp = if experience > 0 {
        let amount = i64::try_from(experience).map_err(|_| DurabilityError::InvalidStoredState)?;
        let bytes: Vec<u8> = sqlx::query_scalar("INSERT INTO game_character_quest_xp_obligations(reward_occurrence_id,character_id,cause_id,cause_ordinal,transition_key,amount,pinned_content_revision,created_at) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,floor(extract(epoch FROM statement_timestamp())*1000)::bigint) RETURNING uuid_send(reward_occurrence_id)")
            .bind(character).bind(cause.as_slice()).bind(ordinal.to_string()).bind(transition_key).bind(amount).bind(&pin)
            .fetch_one(&mut **tx).await?;
        Some(QuestXpObligation {
            occurrence: ExperienceRewardOccurrence::from_bytes(
                bytes
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?,
            amount,
        })
    } else {
        None
    };
    let body = serde_json::to_value(&view.data).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_character_task_boards(character_id,world_id,body,committed_character_revision,cause_id,cause_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,encode($5,'hex')::uuid,$6::text::numeric) ON CONFLICT(character_id) DO UPDATE SET body=EXCLUDED.body,committed_character_revision=EXCLUDED.committed_character_revision,cause_id=EXCLUDED.cause_id,cause_ordinal=EXCLUDED.cause_ordinal")
        .bind(character).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(body)
        .bind(committed.to_string()).bind(cause.as_slice()).bind(ordinal.to_string()).execute(&mut **tx).await?;
    Ok(CommittedQuestTransition {
        character_id: fence.character_id,
        cause: quest_cause,
        transition_key: transition_key.to_owned(),
        quest_key: task_board::QUEST.to_owned(),
        original_character_revision: fence.expected_character_revision,
        committed_character_revision: CharacterRevision::new(committed)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        pinned_content_revision: pin,
        definition_hash: hash
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        completes: false,
        changes,
        experience: xp,
    })
}

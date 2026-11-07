//! Native Forge occurrence reservations and immutable terminal replies.
use super::*;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub(super) const RULESET: &str = "crystal-201189f-forge-v1";
// Crystal game.cpp:11018 and config.lua.dist:469: uiActionsDelay - 10.
const ACTION_DELAY_MS: i64 = 490;

#[derive(Debug, Clone)]
pub(super) struct Reservation {
    pub character: CharacterId,
    pub occurrence: [u8; 16],
    pub transaction: [u8; 16],
    pub event: [u8; 16],
    pub entry: [u8; 16],
    pub occurred_at: i64,
    pub content: [u8; 32],
    pub world: [u8; 16],
    pub channel: [u8; 16],
    pub simulation_revision: u32,
    pub ruleset: String,
    pub intent: Vec<u8>,
    pub resolved_items: Option<ForgeResolvedItems>,
    pub expected_dust_upgrade_count: Option<u16>,
    pub rolls: [u16; 3],
    pub exhausted: bool,
}

pub(super) async fn lock(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: [u8; 16],
) -> NativeResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:forge:' || encode($1::bytea,'hex') || ':' || encode($2::bytea,'hex'),0))")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice())
        .execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn read_reply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: [u8; 16],
    intent: &[u8],
) -> LimitResult<Option<Value>> {
    let row = sqlx::query("SELECT r.intent,p.reply FROM game_forge_reservations r JOIN game_forge_operations p USING(character_id,occurrence_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.occurrence_id=encode($2,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice())
        .fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    row.map(|row| {
        if row
            .try_get::<Vec<u8>, _>("intent")
            .map_err(DurabilityError::from)?
            != intent
        {
            return Err(ForgeDustLimitError::ConflictingOccurrence);
        }
        row.try_get("reply")
            .map_err(DurabilityError::from)
            .map_err(Into::into)
    })
    .transpose()
}

pub(super) async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: [u8; 16],
) -> NativeResult<Option<Reservation>> {
    let row = sqlx::query("SELECT transaction_id::text,event_id::text,entry_id::text,world_id::text,channel_id::text,occurred_at,content_digest,simulation_revision,ruleset_revision,intent,first_item_instance_id::text,second_item_instance_id::text,expected_dust_upgrade_count,success_roll,protection_roll,bonus_roll,exhausted FROM game_forge_reservations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(Reservation {
            character,
            occurrence,
            transaction: uuid_text(row.try_get("transaction_id")?)?,
            event: uuid_text(row.try_get("event_id")?)?,
            entry: uuid_text(row.try_get("entry_id")?)?,
            world: uuid_text(row.try_get("world_id")?)?,
            channel: uuid_text(row.try_get("channel_id")?)?,
            occurred_at: row.try_get("occurred_at")?,
            content: row
                .try_get::<Vec<u8>, _>("content_digest")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            simulation_revision: u32::try_from(row.try_get::<i64, _>("simulation_revision")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ruleset: row.try_get("ruleset_revision")?,
            intent: row.try_get("intent")?,
            resolved_items: match (
                row.try_get::<Option<String>, _>("first_item_instance_id")?,
                row.try_get::<Option<String>, _>("second_item_instance_id")?,
            ) {
                (Some(first), Some(second)) => Some(ForgeResolvedItems {
                    first: uuid_text(&first)?,
                    second: uuid_text(&second)?,
                }),
                (None, None) => None,
                _ => return Err(DurabilityError::InvalidStoredState),
            },
            expected_dust_upgrade_count: row
                .try_get::<Option<i16>, _>("expected_dust_upgrade_count")?
                .map(u16::try_from)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            rolls: [
                u16::try_from(row.try_get::<i16, _>("success_roll")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                u16::try_from(row.try_get::<i16, _>("protection_roll")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                u16::try_from(row.try_get::<i16, _>("bonus_roll")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ],
            exhausted: row.try_get("exhausted")?,
        })
    })
    .transpose()
}

pub(super) async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    request: Request,
    intent: Vec<u8>,
    resolved_items: Option<ForgeResolvedItems>,
    content: [u8; 32],
) -> NativeResult<Reservation> {
    let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS entry_id,floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS occurred_at")
        .fetch_one(&mut **tx).await?;
    let occurred_at: i64 = row.try_get("occurred_at")?;
    let exhausted = charge_cooldown(tx, fence.character_id, occurred_at).await?;
    let secret: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_forge_decision_root WHERE singleton")
            .fetch_one(&mut **tx)
            .await?;
    let seed: [u8; 32] = secret
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    // A learned outcome on another Character must not predict this Character's draw.
    use sha2::{Digest, Sha256};
    let mut contextual_seed = Sha256::new();
    contextual_seed.update(seed);
    contextual_seed.update(fence.character_id.as_bytes());
    let root = GameplayDecisionRoot::from_bytes(contextual_seed.finalize().into());
    let occurrence = DecisionOccurrenceId::from_bytes(request.occurrence);
    let draw = |index, bound| {
        deterministic_decision_u64(&root, occurrence, "forge_fusion", index)
            .map(|value| (value % bound) as u16)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let expected_dust_upgrade_count = if request.operation == Operation::DustLimit {
        Some(
            lock_limit_balance(tx, fence.character_id)
                .await?
                .map_or(ForgeDustBalance::default(), |(balance, _)| balance)
                .limit()
                .upgrade_count(),
        )
    } else {
        None
    };
    let mut reserved = Reservation {
        character: fence.character_id,
        occurrence: request.occurrence,
        transaction: uuid_text(row.try_get("transaction_id")?)?,
        event: uuid_text(row.try_get("event_id")?)?,
        entry: uuid_text(row.try_get("entry_id")?)?,
        occurred_at,
        content,
        world: *fence.runtime_scope.world_id().as_bytes(),
        channel: *scope_of(&fence)
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .1
            .as_bytes(),
        intent,
        resolved_items,
        expected_dust_upgrade_count,
        exhausted,
        ruleset: RULESET.into(),
        simulation_revision: oteryn_simulation_determinism::active_profile()
            .revision()
            .get(),
        rolls: [draw(0, 100)?, draw(1, 100)?, draw(2, 10_001)?],
    };
    let mut work_units_used = 0_i16;
    if request.operation == Operation::DustLimit
        && let Some(legacy) =
            load_dust_limit_reservation(tx, fence.character_id, request.occurrence).await?
    {
        reserved.transaction = legacy.transaction_id;
        reserved.event = legacy.event_id;
        reserved.entry = legacy.entry_id;
        reserved.occurred_at = legacy.occurred_at_unix_ms;
        reserved.content = legacy.content_digest;
        reserved.world = legacy.world_id;
        reserved.channel = legacy.channel_id;
        reserved.simulation_revision = legacy.simulation_profile_revision;
        reserved.expected_dust_upgrade_count = Some(legacy.request.expected_upgrade_count);
        // Adopting an earlier API's occurrence must not reset its DUR-03 retry budget.
        work_units_used = sqlx::query_scalar("SELECT work_units_used FROM game_forge_dust_limit_reservations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice())
            .fetch_one(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_forge_reservations(character_id,occurrence_id,transaction_id,event_id,entry_id,world_id,channel_id,intent,content_digest,ruleset_revision,simulation_revision,success_roll,protection_roll,bonus_roll,occurred_at,exhausted,first_item_instance_id,second_item_instance_id,work_units_used,expected_dust_upgrade_count) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,$13,$14,$15,$16,encode($17,'hex')::uuid,encode($18,'hex')::uuid,$19,$20)")
        .bind(reserved.character.as_bytes().as_slice()).bind(reserved.occurrence.as_slice())
        .bind(reserved.transaction.as_slice()).bind(reserved.event.as_slice()).bind(reserved.entry.as_slice())
        .bind(reserved.world.as_slice()).bind(reserved.channel.as_slice())
        .bind(&reserved.intent).bind(reserved.content.as_slice()).bind(&reserved.ruleset).bind(i64::from(reserved.simulation_revision))
        .bind(i16::try_from(reserved.rolls[0]).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(i16::try_from(reserved.rolls[1]).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(i16::try_from(reserved.rolls[2]).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(reserved.occurred_at).bind(reserved.exhausted)
        .bind(reserved.resolved_items.as_ref().map(|items|items.first.as_slice()))
        .bind(reserved.resolved_items.as_ref().map(|items|items.second.as_slice()))
        .bind(work_units_used)
        .bind(reserved.expected_dust_upgrade_count.map(i16::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?;
    Ok(reserved)
}

pub(super) async fn now(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> NativeResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}

/// The Character root is locked before this row, including a history-only request.
pub(super) async fn charge_cooldown(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    now: i64,
) -> NativeResult<bool> {
    let next: Option<i64> = sqlx::query_scalar("SELECT next_action_at FROM game_forge_cooldowns WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let exhausted = next.is_some_and(|next| next > now);
    if !exhausted {
        sqlx::query("INSERT INTO game_forge_cooldowns(character_id,next_action_at) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE SET next_action_at=EXCLUDED.next_action_at")
            .bind(character.as_bytes().as_slice()).bind(now.checked_add(ACTION_DELAY_MS).ok_or(DurabilityError::InvalidStoredState)?)
            .execute(&mut **tx).await?;
    }
    Ok(exhausted)
}

pub(super) async fn charge_attempt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reserved: &Reservation,
) -> NativeResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT game_forge_charge_attempt(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(reserved.character.as_bytes().as_slice())
    .bind(reserved.occurrence.as_slice())
    .fetch_one(&mut **tx)
    .await?)
}

pub(super) async fn check_command(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: &[u8],
) -> LimitResult<()> {
    let previous: Option<(Vec<u8>,Vec<u8>)> = sqlx::query_as("SELECT uuid_send(p.character_id),r.intent FROM game_forge_operations p JOIN game_forge_reservations r USING(character_id,occurrence_id) WHERE p.game_session_id=encode($1,'hex')::uuid AND p.command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    if previous.is_some_and(|(character, previous)| {
        character != fence.character_id.as_bytes() || previous != intent
    }) {
        return Err(ForgeDustLimitError::ConflictingOccurrence);
    }
    Ok(())
}

pub(super) async fn history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    limit: u8,
) -> NativeResult<Value> {
    let rows = sqlx::query("SELECT history_ordinal,reply FROM game_forge_history WHERE character_id=encode($1,'hex')::uuid ORDER BY history_ordinal DESC LIMIT $2")
        .bind(character.as_bytes().as_slice()).bind(i64::from(limit)).fetch_all(&mut **tx).await?;
    let entries = rows.into_iter().map(|row| {
        Ok(json!({"ordinal": row.try_get::<i64, _>("history_ordinal")?, "receipt": row.try_get::<Value, _>("reply")?}))
    }).collect::<NativeResult<Vec<_>>>()?;
    Ok(Value::Array(entries))
}

pub(super) async fn append_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reserved: &Reservation,
    reply: &Value,
) -> NativeResult<()> {
    sqlx::query("INSERT INTO game_forge_history(character_id,occurrence_id,history_ordinal,reply) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,COALESCE(max(history_ordinal),0)+1,$3 FROM game_forge_history WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,occurrence_id) DO NOTHING")
        .bind(reserved.character.as_bytes().as_slice()).bind(reserved.occurrence.as_slice()).bind(reply).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_forge_history WHERE character_id=encode($1,'hex')::uuid AND history_ordinal<(SELECT min(history_ordinal) FROM (SELECT history_ordinal FROM game_forge_history WHERE character_id=encode($1,'hex')::uuid ORDER BY history_ordinal DESC LIMIT 1000) retained)")
        .bind(reserved.character.as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}

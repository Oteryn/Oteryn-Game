//! A participant of the existing Quest transaction, never a separate value owner.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::world_contributions::{self, Kind};

pub(crate) async fn current_session(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
) -> Result<bool, DurabilityError> {
    if super::super::super::character_progression::assert_item_only_fence(
        tx,
        &fence.item_fence(),
        node,
    )
    .await?
    .is_err()
    {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(Into::into)
}

/// The Quest writer has already locked its receipt key and Character root. A refusal
/// must drop that transaction; the deferred debit proof requires its exact Quest receipt.
pub(crate) async fn contribute_quest(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
) -> Result<bool, DurabilityError> {
    let Some(recipe) = world_contributions::recipe(&request.transition_key) else {
        return Ok(true);
    };
    let (QuestCause::Command(command) | QuestCause::Use(command)) = request.cause else {
        return Ok(false);
    };
    if command.game_session_id() != fence.game_session_id
        || assert_gameplay_fence(tx, &fence, node).await?.is_err()
        || !current_session(tx, node, fence).await?
    {
        return Ok(false);
    }
    assert_recovery_fence(tx, recovery).await?;
    let world = fence.runtime_scope.world_id();
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid",
    ).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(epoch) = epoch else {
        return Ok(false);
    };
    let epoch = epoch
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if load(tx, world, epoch).await?.is_none() {
        return Ok(false);
    }
    let kind = match recipe.kind {
        Kind::Coal => ContributionKind::Coal,
        Kind::Medicine => ContributionKind::Medicine,
        Kind::Corpses => ContributionKind::Corpses,
        Kind::Votes => ContributionKind::Votes,
    };
    let binding: [u8;32] = Sha256::digest(serde_json::to_vec(&serde_json::json!({
        "version":1,"character":fence.character_id.as_bytes(),
        "session":command.game_session_id().as_bytes(),"command":command.command_id().get(),
        "world":world.as_bytes(),"epoch":epoch,"transition":request.transition_key,
        "resource":kind.key(),"amount":recipe.amount,"item":recipe.item_key,"points":recipe.points_track,
    })).map_err(|_| DurabilityError::InvalidStoredState)?).into();
    // Occurrence identity excludes the payload: changing a resource or amount on
    // the same command must conflict, rather than create another debit receipt.
    let occurrence = Sha256::new()
        .chain_update(b"oteryn.world_contribution.occurrence.v1\0")
        .chain_update(fence.character_id.as_bytes())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .finalize();
    let mut operation = [0; 16];
    operation.copy_from_slice(&occurrence[..16]);
    let previous: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT request_binding FROM game_world_change_debits WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await?;
    if let Some(previous) = previous {
        return Ok(previous == binding);
    }
    let (item, before, location, location_kind, transaction, points) = if let Some(key) =
        recipe.item_key
    {
        let row = sqlx::query(
            "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before,c.location,c.kind FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 AND i.quantity >= $4 AND c.kind IN ('container','nested','equipment') ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
        ).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
            .bind(key).bind(i64::from(recipe.amount)).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let item: String = row.try_get("item")?;
        let location_kind: String = row.try_get("kind")?;
        let location: serde_json::Value = row.try_get("location")?;
        let query = match location_kind.as_str() {
            "container" => {
                "SELECT to_jsonb(l) FROM game_item_container_entries l WHERE item_instance_id=$1::uuid FOR UPDATE"
            }
            "nested" => {
                "SELECT to_jsonb(l) FROM game_item_nested_entries l WHERE item_instance_id=$1::uuid FOR UPDATE"
            }
            "equipment" => {
                "SELECT to_jsonb(l) FROM game_character_equipment_slots l WHERE item_instance_id=$1::uuid FOR UPDATE"
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let locked: Option<serde_json::Value> = sqlx::query_scalar(query)
            .bind(&item)
            .fetch_optional(&mut **tx)
            .await?;
        if locked.as_ref() != Some(&location) {
            return Ok(false);
        }
        let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut **tx)
            .await?;
        (
            Some(item),
            Some(row.try_get::<serde_json::Value, _>("before")?),
            Some(location),
            Some(location_kind),
            Some(transaction),
            None,
        )
    } else if let Some(track) = recipe.points_track {
        let before: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).bind(track).fetch_optional(&mut **tx).await?;
        let Some(before) = before.filter(|v| *v >= i64::from(recipe.amount)) else {
            return Ok(false);
        };
        (None, None, None, None, None, Some(before))
    } else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let timed: Option<serde_json::Value> = if let Some(item) = &item {
        sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=$1::uuid FOR UPDATE")
            .bind(item).fetch_optional(&mut **tx).await?
    } else { None };
    if recipe.kind == Kind::Corpses {
        let Some(timer) = &timed else { return Ok(false); };
        let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx).await?;
        if !timer["charges"].is_null()
            || !timer["remaining_ms"].as_i64().is_some_and(|remaining| (1..=60_000).contains(&remaining))
            || (!timer["deadline_at"].is_null() && !timer["deadline_at"].as_i64().is_some_and(|deadline| deadline>now))
            || before.as_ref().and_then(|v|v["quantity"].as_i64()) != Some(1)
        { return Ok(false); }
    } else if timed.is_some() { return Ok(false); }
    sqlx::query("INSERT INTO game_world_change_debits(world_id,reset_epoch,operation_id,character_id,game_session_id,cause_ordinal,transition_key,request_binding,amount,item_instance_id,before_item,before_location,location_kind,transaction_id,points_track,before_points,before_timed) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10::uuid,$11,$12,$13,$14::uuid,$15,$16,$17)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(operation.as_slice())
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&request.transition_key).bind(binding.as_slice())
        .bind(i64::from(recipe.amount)).bind(&item).bind(&before).bind(&location).bind(&location_kind)
        .bind(&transaction).bind(recipe.points_track).bind(points).bind(timed).execute(&mut **tx).await?;
    if let Some(item) = &item {
        let changed = sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity >= $2")
            .bind(item).bind(i64::from(recipe.amount)).bind(&transaction).execute(&mut **tx).await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        if before.as_ref().and_then(|v| v["quantity"].as_i64()) == Some(i64::from(recipe.amount)) {
            let query = match location_kind.as_deref() {
                Some("container") => {
                    "DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid"
                }
                Some("nested") => {
                    "DELETE FROM game_item_nested_entries WHERE item_instance_id=$1::uuid"
                }
                Some("equipment") => {
                    "DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::uuid"
                }
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            if sqlx::query(query)
                .bind(item)
                .execute(&mut **tx)
                .await?
                .rows_affected()
                != 1
            {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    let applied = matches!(
        contribute(tx, recovery, world, epoch, operation, kind, recipe.amount).await?,
        ContributionWrite::Applied(_)
    );
    if applied && let Some(ballot) = oteryn_gameplay_quests::quest::rathleton::ballot_for_transition(&request.transition_key) {
        super::elections::vote(tx, world, epoch, operation, ballot).await?;
    }
    Ok(applied)
}

//! Bounded stack hand-ins keep the existing row guards and transaction deadline.
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, QueryBuilder, types::Uuid};

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &super::super::quest_state::QuestTransitionRequest,
    callback: &RewardCallback,
    carried_use: Option<&super::super::reward_claim_mint::QuestCarriedUse>,
    inputs: &[Input],
) -> Result<bool, DurabilityError> {
    if inputs.is_empty() {
        return Ok(true);
    }
    // Each stack executes the same parameterized custody guards, including the
    // deferred guards at COMMIT (0268_quest_callback_carried.sql:110). Avoid
    // repeatedly planning their alternative proof joins for the first five rows
    // and JIT-compiling bounded OLTP queries inside the semantic pass deadline.
    // Both settings are transaction-local; every custody/receipt guard still runs.
    sqlx::query("SET LOCAL plan_cache_mode = force_generic_plan")
        .execute(&mut **tx)
        .await?;
    sqlx::query("SET LOCAL jit = off")
        .execute(&mut **tx)
        .await?;
    let ids = inputs
        .iter()
        .map(|input| input.item.parse::<Uuid>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    // The same plain-item exclusions as the single-input path, read once for
    // the already locked set. A Royal Costume hand-in can contain 275 stacks.
    let special: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=ANY($1::uuid[])
         UNION ALL SELECT 1 FROM game_item_timed_states WHERE item_instance_id=ANY($1::uuid[])
         UNION ALL SELECT 1 FROM game_item_texts WHERE item_instance_id=ANY($1::uuid[])
         UNION ALL SELECT 1 FROM game_item_imbuements WHERE item_instance_id=ANY($1::uuid[])
         UNION ALL SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=ANY($1::uuid[])
         UNION ALL SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=ANY($1::uuid[]))",
    ).bind(&ids).fetch_one(&mut **tx).await?;
    if special {
        return Ok(false);
    }
    for kind in ["container", "nested", "backpack", "equipment"] {
        let selected: Vec<_> = inputs
            .iter()
            .zip(&ids)
            .filter(|(input, _)| input.kind == kind)
            .collect();
        if selected.is_empty() {
            continue;
        }
        let selected_ids: Vec<_> = selected.iter().map(|(_, id)| **id).collect();
        let (read, _) = location_queries(kind)?;
        // Both query text and replacement are closed repository-owned strings.
        let read = read.replace("$1::uuid", "ANY($1::uuid[])");
        let actual: Vec<serde_json::Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(read))
            .bind(&selected_ids)
            .fetch_all(&mut **tx)
            .await?;
        if actual.len() != selected.len()
            || selected
                .iter()
                .any(|(input, _)| !actual.contains(&input.location))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let transaction: Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&mut **tx)
        .await?;
    let command = match request.cause {
        super::super::quest_state::QuestCause::Command(command) => command,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let character = Uuid::from_bytes(*fence.character_id.as_bytes());
    let cause = Uuid::from_bytes(*command.game_session_id().as_bytes());
    let ordinal = command.command_id().get().to_string();
    let proofs = inputs.iter().map(|input| {
        let mut proof = serde_json::json!({"reward_callback":oteryn_gameplay_quests::quest::reward_callbacks::durable_plan(callback).map_err(|_| DurabilityError::InvalidStoredState)?,"before":input.before,"location":input.location,"count":input.count});
        if let Some(use_) = carried_use {
            proof["carried_use"] = serde_json::to_value(use_).map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        serde_json::to_vec(&proof).map_err(|_| DurabilityError::InvalidStoredState)
    }).collect::<Result<Vec<_>, _>>()?;
    // Admit every immutable proof before updating any stack. Existing BEFORE
    // and deferred row triggers still verify exact custody, quantities and receipt.
    // Twelve binds per receipt: two rows keep each statement below WP3's 32.
    for ((input_chunk, id_chunk), proof_chunk) in
        inputs.chunks(2).zip(ids.chunks(2)).zip(proofs.chunks(2))
    {
        let mut insert = QueryBuilder::<Postgres>::new(
            "INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,location_kind) ",
        );
        insert.push_values(
            input_chunk.iter().zip(id_chunk).zip(proof_chunk),
            |mut row, ((input, id), proof)| {
                row.push_bind(transaction)
                    .push_bind(*id)
                    .push_bind(character)
                    .push_bind(cause)
                    .push_bind(&ordinal)
                    .push_unseparated("::numeric")
                    .push_bind(&request.transition_key)
                    .push_bind(&input.before)
                    .push("NULL")
                    .push_bind(i32::from(input.count))
                    .push_bind(proof)
                    .push_bind(Sha256::digest(proof).to_vec())
                    .push_bind(&input.location)
                    .push_bind(&input.kind);
            },
        );
        if insert.build().execute(&mut **tx).await?.rows_affected() != input_chunk.len() as u64 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    // One transaction bind plus three per stack: ten rows use 31 parameters.
    for (input_chunk, id_chunk) in inputs.chunks(10).zip(ids.chunks(10)) {
        let mut update = QueryBuilder::<Postgres>::new(
            "UPDATE game_item_instances i SET quantity=i.quantity-v.used,lifecycle=CASE WHEN i.quantity=v.used THEN 2 ELSE 1 END,last_transaction_id=",
        );
        update.push_bind(transaction).push(" FROM (");
        update.push_values(input_chunk.iter().zip(id_chunk), |mut row, (input, id)| {
            row.push_bind(*id)
                .push_bind(i64::from(input.count))
                .push_bind(input.quantity);
        });
        update.push(") AS v(item,used,before_quantity) WHERE i.item_instance_id=v.item AND i.lifecycle=1 AND i.quantity=v.before_quantity");
        if update.build().execute(&mut **tx).await?.rows_affected() != input_chunk.len() as u64 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    for kind in ["container", "nested", "backpack", "equipment"] {
        let removed: Vec<_> = inputs
            .iter()
            .zip(&ids)
            .filter(|(input, _)| input.kind == kind && input.quantity == i64::from(input.count))
            .map(|(_, id)| *id)
            .collect();
        if removed.is_empty() {
            continue;
        }
        let (_, delete) = location_queries(kind)?;
        if kind == "backpack" {
            for id in removed {
                if sqlx::query(delete)
                    .bind(id)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
                    != 1
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
        } else {
            let delete = delete.replace("$1::uuid", "ANY($1::uuid[])");
            if sqlx::query(sqlx::AssertSqlSafe(delete))
                .bind(&removed)
                .execute(&mut **tx)
                .await?
                .rows_affected()
                != removed.len() as u64
            {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    Ok(true)
}

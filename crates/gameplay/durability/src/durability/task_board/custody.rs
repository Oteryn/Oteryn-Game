use super::super::DurabilityError;
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::item_mint_audit::Type2Transaction;
use super::super::spell_owner_commit::SpellLanePermit;
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::TaskBoardEffects;
use sqlx::Row;

/// Participant of the already fenced Task Board transaction. Every refusal is
/// rolled back by the caller before its stable refusal receipt is retained.
pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    permit: Option<&SpellLanePermit>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    catalogue: &TaskBoardCatalogue,
    effects: &TaskBoardEffects,
    mint_candidate: Option<&super::super::character_inbox::RewardClaimInboxMintCandidate>,
) -> Result<Option<TaskBoardRefusal>, DurabilityError> {
    if let TaskBoardAction::BuyShop { offer } = &request.action
        && oteryn_gameplay_quests::quest::task_board::full::SHOP_OFFERS
            .iter()
            .any(|item| item.id == *offer && item.requires_decoration_kit())
    {
        return Ok(Some(TaskBoardRefusal::UnavailableItem));
    }
    let item_effects = effects.gold_cost > 0
        || !effects.consume_items.is_empty()
        || !effects.grant_items.is_empty();
    if item_effects {
        let Some(permit) = permit else {
            return Err(DurabilityError::InvalidStoredState);
        };
        permit.check_scope(fence.runtime_scope)?;
    }
    if effects.gold_cost > 0
        && let Some(refusal) =
            super::fee::burn(tx, fence, request, catalogue, effects.gold_cost).await?
    {
        return Ok(Some(refusal));
    }
    let (_, cause, ordinal) = state::cause_key(request.cause);
    for input in &effects.consume_items {
        let Some(item) = catalogue.items.get(&input.item_id) else {
            return Ok(Some(TaskBoardRefusal::UnavailableItem));
        };
        let consumed: bool = sqlx::query_scalar("SELECT game_task_board_consume_item(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(cause.as_slice()).bind(ordinal.to_string()).bind(i32::from(input.item_id))
            .bind(&item.key).bind(&item.revision).bind(i64::from(input.amount))
            .fetch_one(&mut ***tx).await?;
        if !consumed {
            return Ok(Some(TaskBoardRefusal::InsufficientItems));
        }
    }
    let max_stash_sources = effects
        .consume_items
        .iter()
        .try_fold(0_u64, |total, item| {
            total
                .checked_add(u64::from(item.amount))
                .ok_or(DurabilityError::InvalidStoredState)
        })?;
    audit_stash_consumption(tx, fence, request, max_stash_sources).await?;
    if let Some(refusal) = super::mint::apply(tx, fence, request, effects, mint_candidate).await? {
        return Ok(Some(refusal));
    }
    apply_tokens(tx, fence, request, effects).await?;
    if effects.outfit.is_some() || effects.mount.is_some() || effects.promotion_points != 0 {
        return super::entitlements::apply(tx, fence, request, effects, catalogue).await;
    }
    Ok(None)
}

async fn audit_stash_consumption(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    maximum: u64,
) -> Result<(), DurabilityError> {
    let (_, cause, ordinal) = state::cause_key(request.cause);
    let rows=sqlx::query("SELECT transaction_id::text,entry_id::text,event_id::text,occurred_at,before_balance,amount FROM game_task_board_stash_sources WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric ORDER BY transaction_id LIMIT $4")
        .bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .bind(i64::try_from(maximum.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?).map_err(|_|DurabilityError::InvalidStoredState)?)
        .fetch_all(&mut ***tx).await?;
    if u64::try_from(rows.len()).map_err(|_| DurabilityError::InvalidStoredState)? > maximum {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in rows {
        let QuestCause::Command(command) = request.cause else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let before: serde_json::Value = row.try_get("before_balance")?;
        let text = |name: &str| {
            before[name]
                .as_str()
                .map(str::to_owned)
                .ok_or(DurabilityError::InvalidStoredState)
        };
        let definition = super::super::item_mint::TypedDefinitionRef {
            family: "Item".to_owned(),
            production_key: text("definition_key")?,
            revision_ref: text("definition_revision")?,
        };
        let transaction: String = row.try_get("transaction_id")?;
        let event: String = row.try_get("event_id")?;
        let occurred: i64 = row.try_get("occurred_at")?;
        let envelope = super::super::character_stash::audit::encode_task_board_consumption(
            fence.item_fence(),
            command,
            &request.payload_binding,
            &definition,
            &row.try_get::<String, _>("entry_id")?,
            u32::try_from(row.try_get::<i64, _>("amount")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            before["quantity"]
                .as_i64()
                .ok_or(DurabilityError::InvalidStoredState)?,
            &transaction,
            &event,
            occurred,
        )?;
        sqlx::query("INSERT INTO game_task_board_stash_audit_proofs(transaction_id,event_id,envelope_sha256) VALUES($1::uuid,$2::uuid,sha256($3))")
            .bind(&transaction).bind(&event).bind(&envelope).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_character_stash_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,world_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,9,2,'ECONOMY_LEDGER_RETENTION_V1',encode($3,'hex')::uuid,$4,$4+2592000000,$5,sha256($5),1)")
            .bind(&event).bind(&transaction).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(occurred).bind(envelope).execute(&mut ***tx).await?;
    }
    Ok(())
}

async fn apply_tokens(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    effects: &TaskBoardEffects,
) -> Result<(), DurabilityError> {
    let amount = i64::from(effects.reroll_tokens_granted) - i64::from(effects.reroll_tokens_spent);
    if amount == 0 {
        return Ok(());
    }
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let (_, cause, ordinal) = state::cause_key(request.cause);
    let previous = sqlx::query("SELECT balance,last_claim_id::text FROM game_character_bounty_reroll_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let (before, last): (i64, Option<String>) = match previous {
        Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
        None => (0, None),
    };
    let after = before
        .checked_add(amount)
        .filter(|value| *value >= 0)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let claim: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO game_task_board_token_sources(claim_id,character_id,world_id,cause_id,cause_ordinal,amount) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6)")
        .bind(&claim).bind(character).bind(world.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .bind(i16::try_from(amount).map_err(|_| DurabilityError::InvalidStoredState)?).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_character_bounty_reroll_token_entries(claim_id,character_id,world_id,previous_claim_id,amount,balance_before,balance_after) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::uuid,$5,$6,$7)")
        .bind(&claim).bind(character).bind(world.as_bytes().as_slice()).bind(last)
        .bind(i16::try_from(amount).map_err(|_| DurabilityError::InvalidStoredState)?).bind(before).bind(after).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_character_bounty_reroll_tokens(character_id,world_id,balance,last_claim_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::uuid) ON CONFLICT(character_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id")
        .bind(character).bind(world.as_bytes().as_slice()).bind(after).bind(&claim).execute(&mut ***tx).await?;
    Ok(())
}

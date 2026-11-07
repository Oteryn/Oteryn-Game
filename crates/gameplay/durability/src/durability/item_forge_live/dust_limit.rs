//! Keep the existing D1230 owner and audit shape in the common Forge transaction.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;

#[allow(clippy::too_many_lines)]
pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: Request,
    reserved: &storage::Reservation,
) -> NativeResult<Value> {
    let row = lock_limit_balance(tx, fence.character_id).await?;
    let before = row.map_or(ForgeDustBalance::default(), |(balance, _)| balance);
    let legacy = load_dust_limit_reservation(tx, fence.character_id, request.occurrence).await?;
    let expected = reserved
        .expected_dust_upgrade_count
        .ok_or(DurabilityError::InvalidStoredState)?;
    if legacy
        .as_ref()
        .is_some_and(|frozen| frozen.request.expected_upgrade_count != expected)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let plan = match plan_dust_limit(before, expected) {
        Ok(plan) => plan,
        Err(error) => {
            let reason = match error {
                ForgeDustLimitError::InsufficientDust => "INSUFFICIENT_DUST",
                ForgeDustLimitError::MaximumLimitReached => "MAXIMUM_LIMIT",
                ForgeDustLimitError::StageChanged => "STAGE_CHANGED",
                _ => "INVALID_INPUT",
            };
            let reply = actions::refusal(request, reason);
            actions::record(tx, fence, command, request, reserved, &reply, None).await?;
            return Ok(reply);
        }
    };
    let (_, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let frozen = legacy.unwrap_or(FrozenDustLimit {
        character_id: fence.character_id,
        request: ForgeDustLimitRequest {
            occurrence_id: request.occurrence,
            expected_upgrade_count: expected,
        },
        world_id: *fence.runtime_scope.world_id().as_bytes(),
        channel_id: *channel.as_bytes(),
        transaction_id: reserved.transaction,
        event_id: reserved.event,
        entry_id: reserved.entry,
        occurred_at_unix_ms: reserved.occurred_at,
        content_digest: reserved.content,
        ruleset_revision: DUST_LIMIT_RULESET_REVISION.into(),
        simulation_profile_revision: reserved.simulation_revision,
    });
    if load_dust_limit_reservation(tx, fence.character_id, request.occurrence)
        .await?
        .is_none()
    {
        insert_dust_limit_reservation(tx, &frozen).await?;
    }
    // Phase one already charged the common allowance and any existing legacy row.
    // A newly inserted native projection mirrors that charge without a second attempt.
    let (legacy_used, common_used): (i16, i16) = sqlx::query_as("SELECT l.work_units_used,r.work_units_used FROM game_forge_dust_limit_reservations l JOIN game_forge_reservations r USING(character_id,occurrence_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.occurrence_id=encode($2,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice())
        .fetch_one(&mut ***tx).await?;
    if common_used <= 0 || legacy_used > common_used {
        return Err(DurabilityError::InvalidStoredState);
    }
    for used in legacy_used..common_used {
        let mirrored = sqlx::query("UPDATE game_forge_dust_limit_reservations SET work_units_used=work_units_used+1 WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid AND work_units_used=$3")
            .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(used)
            .execute(&mut ***tx).await?;
        if mirrored.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let entry = ForgeDustEntry {
        entry_id: frozen.entry_id,
        previous_entry_id: row.and_then(|(_, previous)| previous),
        character_id: fence.character_id,
        kind: ForgeDustEntryKind::LimitRaise,
        cause_occurrence_id: request.occurrence,
        transaction_id: frozen.transaction_id,
        amount: plan.amount,
        lost: 0,
        before,
        after: plan.after,
    };
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let tuple = tx.tuple();
    let envelope = dust_audit::encode_forge_dust_limit_event(
        mint_audit::MintEventIdentity {
            event_id: frozen.event_id,
            transaction_id: frozen.transaction_id,
            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID,
        },
        dust_limit_message(
            &frozen,
            &fence,
            command,
            revision
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            &entry,
        ),
        tuple.tuple(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    apply_dust_limit(tx, &frozen, &entry, &envelope, tuple).await?;
    let receipt = load_dust_limit_receipt(tx, &frozen)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    let result = reply(&receipt);
    actions::record(tx, fence, command, request, reserved, &result, None).await?;
    // The common receipt is a terminal alias of the native limit receipt; the
    // legacy-history trigger also covers direct users of the established API.
    sqlx::query("UPDATE game_forge_operations SET applied=true,envelope_sha256=$3 WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(receipt.envelope_sha256.map(|hash|hash.to_vec()))
        .execute(&mut ***tx).await?;
    storage::append_history(tx, reserved, &result).await?;
    Ok(result)
}

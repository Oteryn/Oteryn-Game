//! P05 participates in the existing coin/change/bank fee owner.
use super::super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, burn_fee_in_transaction,
};
use super::super::item_mint_audit::Type2Transaction;
use super::super::quest_state::QuestCause;
use super::super::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use super::{TaskBoardCatalogue, TaskBoardCommand, TaskBoardRefusal};
use oteryn_gameplay_quests::quest::task_board::full::{TALISMAN_PURCHASE_GOLD, TaskBoardAction};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

/// This is only the rules' affordability snapshot. The existing fee owner locks
/// and validates every eligible input, activation, bank payer and change output.
pub(super) async fn available_gold(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    _catalogue: &TaskBoardCatalogue,
) -> Result<u64, DurabilityError> {
    let coins: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_container_slots s JOIN game_item_container_entries e ON e.parent_item_instance_id=s.item_instance_id AND e.character_id=s.character_id AND e.world_id=s.world_id JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.world_id=s.world_id AND i.lifecycle=1 AND i.definition_family='Item'")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .fetch_one(&mut **tx).await?;
    let bank = if super::super::bank::fee_payer_is_junior(fence.character_id) {
        0
    } else {
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(b.balance,0) FROM game_character_roots r LEFT JOIN game_account_bank_balances b ON b.account_id=r.account_id AND b.world_id=r.world_id WHERE r.character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?
    };
    u64::try_from(coins)
        .ok()
        .and_then(|coins| {
            u64::try_from(bank)
                .ok()
                .and_then(|bank| coins.checked_add(bank))
        })
        .ok_or(DurabilityError::InvalidStoredState)
}

fn identity(command: crate::foundation::CommandRef, purpose: &[u8]) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(b"oteryn:taskboard:fee:v1")
        .chain_update(purpose)
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

pub(super) async fn burn(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    _catalogue: &TaskBoardCatalogue,
    amount: u64,
) -> Result<Option<TaskBoardRefusal>, DurabilityError> {
    if amount == 0 {
        return Ok(None);
    }
    if request.action != TaskBoardAction::BuyTalisman || amount != TALISMAN_PURCHASE_GOLD {
        return Err(DurabilityError::InvalidStoredState);
    }
    let QuestCause::Command(command) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let Some(change) = request.fee_change.clone() else {
        return Ok(Some(TaskBoardRefusal::UnavailableItem));
    };
    let occurrence = identity(command, b"purchase");
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut ***tx)
            .await?;
    let fee = FeeBurnRequest {
        cause: FeeBurnCause::TaskBoardTalisman { occurrence },
        fee_gold_units: amount,
        transaction_id: identity(command, b"transaction"),
        event_id: identity(command, b"event"),
        occurred_at_unix_ms: now,
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        change,
    };
    match burn_fee_in_transaction(tx, &fence, &fee).await {
        Ok(FeeBurnOutcome::Burned(_)) => {}
        Err(FeeBurnError::InsufficientFunds) => {
            return Ok(Some(TaskBoardRefusal::InsufficientGold));
        }
        Err(FeeBurnError::ChangeDoesNotFit | FeeBurnError::CapacityExceeded) => {
            return Ok(Some(TaskBoardRefusal::CapacityExceeded));
        }
        Err(FeeBurnError::InvalidInput) => return Ok(Some(TaskBoardRefusal::UnavailableItem)),
        Err(FeeBurnError::Unavailable(error)) => return Err(error),
        Ok(FeeBurnOutcome::AlreadyBurned(_))
        | Err(FeeBurnError::CharacterMismatch | FeeBurnError::ConflictingOccurrence) => {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let next = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_task_board_fee_proofs(character_id,world_id,cause_id,cause_ordinal,fee_occurrence,fee_gold_units,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,$6,$7::text::numeric)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(occurrence.as_slice()).bind(amount as i64).bind(next.to_string()).execute(&mut ***tx).await?;
    Ok(None)
}

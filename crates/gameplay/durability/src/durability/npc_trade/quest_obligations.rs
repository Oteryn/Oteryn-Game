//! A successful trade captures an eligible continuation; the Quest owner drains it.
use super::*;
use oteryn_gameplay_quests::quest::trade_callbacks::{self, Side, TradeCallback};
use sha2::{Digest, Sha256};

pub(super) fn select(request: &NpcTradeRequest) -> Result<Option<TradeCallback>> {
    let side = match request.cause.side {
        NpcTradeSide::Buy => Side::Buy,
        NpcTradeSide::Sell => Side::Sell,
    };
    Ok(trade_callbacks::load()
        .map_err(|_| NpcTradeError::InvalidInput)?
        .into_iter()
        .find(|rule| {
            rule.npc == request.cause.npc_key
                && rule.side == side
                && rule.items.contains(&request.item.definition.production_key)
                && rule.unit_price == request.unit_price
                && rule.offer_count == request.offer_count
        }))
}
pub(super) fn bind(binding: Vec<u8>, rule: Option<&TradeCallback>) -> Result<Vec<u8>> {
    let Some(rule) = rule else {
        return Ok(binding);
    };
    Ok(Sha256::new()
        .chain_update(binding)
        .chain_update(b"\0npc-trade-quest-v1\0")
        .chain_update(rule.binding().map_err(|_| NpcTradeError::InvalidInput)?)
        .finalize()
        .to_vec())
}
#[derive(Debug)]
pub(super) enum Eligibility {
    NotApplicable,
    Ready(std::collections::BTreeMap<String, i64>),
    CapacityExceeded,
}
/// CapacityExceeded refuses the whole trade when an eligible obligation cannot fit.
/// Missing persisted milestones are ineligible; this never starts a quest from an absent row.
pub(super) async fn eligible(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    rule: Option<&TradeCallback>,
    units: u32,
) -> std::result::Result<Eligibility, DurabilityError> {
    let Some(rule) = rule else {
        return Ok(Eligibility::NotApplicable);
    };
    if units < rule.minimum_units {
        return Ok(Eligibility::NotApplicable);
    }
    let keys: Vec<_> = rule.storage.iter().map(|s| s.track.as_str()).collect();
    let rows = sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    let values: std::collections::BTreeMap<String, i64> = rows
        .iter()
        .map(|row| Ok((row.try_get("track_key")?, row.try_get("value")?)))
        .collect::<std::result::Result<_, sqlx::Error>>()?;
    if rule.storage.iter().any(|s| {
        values
            .get(&s.track)
            .is_none_or(|v| !(s.minimum..=s.maximum).contains(v))
    }) {
        return Ok(Eligibility::NotApplicable);
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_obligations WHERE character_id=encode($1,'hex')::uuid AND state IN ('PENDING','WAITING_MIGRATION')")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if count
        >= i64::try_from(oteryn_gameplay_quests::quest::QUESTSTATE0_RL_07)
            .map_err(|_| DurabilityError::InvalidStoredState)?
    {
        return Ok(Eligibility::CapacityExceeded);
    }
    Ok(Eligibility::Ready(values))
}
pub(super) async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    rule: &TradeCallback,
    snapshot: &std::collections::BTreeMap<String, i64>,
) -> std::result::Result<(), DurabilityError> {
    let binding = rule
        .binding()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_character_quest_obligations(claim_game_session_id,claim_command_id,character_id,transition_key,state,created_at,updated_at,source_kind,source_trade_transaction_id,source_binding,source_state) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,'PENDING',$5,$5,'npc_trade',encode($6,'hex')::uuid,$7,$8)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(&rule.key).bind(request.occurred_at_unix_ms)
        .bind(request.transaction_id.as_slice()).bind(binding.as_slice()).bind(serde_json::to_value(snapshot).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_trades_keep_their_original_request_binding() -> Result<()> {
        let before = vec![9; 32];
        assert_eq!(bind(before.clone(), None)?, before);
        let mut rule = trade_callbacks::load()
            .map_err(|_| NpcTradeError::InvalidInput)?
            .into_iter()
            .next()
            .ok_or(NpcTradeError::InvalidInput)?;
        let with_rule = bind(before.clone(), Some(&rule))?;
        assert_ne!(with_rule, before);
        rule.unit_price += 1;
        assert_ne!(bind(before, Some(&rule))?, with_rule);
        Ok(())
    }
}

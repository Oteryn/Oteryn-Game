//! Dirty-water catch admission joins the existing fenced craft transaction and mint owner.
use super::*;
use oteryn_gameplay_world_change::contribution::{LakeState, can_catch_shimmer};

pub(super) struct Catch {
    pub yield_item: Option<u16>,
    pub caught_at: Option<i64>,
}

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    gathering: &GatheringCraft,
) -> Result<Option<Catch>, DurabilityError> {
    if !(12561..=12563).contains(&gathering.mutation.before)
        || gathering.mutation.tool_key != oteryn_gameplay_items::fishing::FISHING_ROD
        || gathering.mutation.after != gathering.mutation.before - 3
        || gathering.attempt.after.map(u32::from) != Some(gathering.mutation.after)
        || !gathering.attempt.consumes_bait
        || !matches!(gathering.attempt.yield_item, None | Some(3111 | 12557))
        || gathering.mutation.decay != vec![(gathering.mutation.before, 1_200_000)]
    {
        return Ok(None);
    }
    let Ok(epoch) = u64::try_from(gathering.scope.reset_epoch) else {
        return Ok(None);
    };
    let world = fence.runtime_scope.world_id();
    let Some(shared) = super::super::world_event::changes::shared(tx, world, epoch).await? else {
        return Ok(None);
    };
    if shared.lake_state == LakeState::Clean {
        return Ok(None);
    }
    let mut catch = Catch {
        yield_item: gathering.attempt.yield_item,
        caught_at: None,
    };
    if catch.yield_item == Some(12557) {
        // assert_current_equipment_in_transaction already holds the Character root;
        // every session/channel for this Character therefore serializes this cooldown.
        let last: Option<i64> = sqlx::query_scalar(
            "SELECT last_catch_at FROM game_twisted_water_catches \
             WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_optional(&mut ***tx)
        .await?;
        let now: i64 =
            sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut ***tx)
                .await?;
        if can_catch_shimmer(now, last) {
            catch.caught_at = Some(now);
        } else {
            // Crystal fishing.lua:119-126: a restricted rare catch becomes fishbones.
            catch.yield_item = Some(3111);
        }
    }
    Ok(Some(catch))
}

pub(super) async fn record(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    transaction: &str,
    caught_at: i64,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_twisted_water_catches(character_id,world_id,last_catch_at,transaction_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::uuid) \
         ON CONFLICT(character_id) DO UPDATE SET last_catch_at=EXCLUDED.last_catch_at, \
         world_id=EXCLUDED.world_id, transaction_id=EXCLUDED.transaction_id",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(caught_at)
    .bind(transaction)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

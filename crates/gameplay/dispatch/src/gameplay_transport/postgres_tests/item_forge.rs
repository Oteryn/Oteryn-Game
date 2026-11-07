//! P18 command bytes → current composed owner → native rows, receipts and read-back.
use super::TestResult;
use super::fixture::{Harness, id};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority};
use oteryn_gameplay_items::forge::FORGE_SLIVER_KEY;
use oteryn_protocol_oteryn::{
    CommandStatus,
    item_forge::{Operation, Request},
};
use serde_json::Value;

#[path = "item_forge/legacy_budget.rs"]
mod legacy_budget;
#[path = "item_forge/materials_and_history.rs"]
mod materials_and_history;
#[path = "item_forge/pair_wire.rs"]
pub(super) mod pair_wire;
#[path = "item_forge/recovery.rs"]
mod recovery;
#[path = "item_forge/revision_and_guards.rs"]
mod revision_and_guards;
#[path = "item_forge/support.rs"]
pub(super) mod support;
use support::{Case, balances, cooldown_elapsed, exchange, gain_dust, request, seed_items};

#[test]
fn forge_postgres_dust_sliver_core_conversion_and_history_replay() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-conversions", || {
        support::scenario(Case::Conversions)
    })
}

#[test]
fn forge_postgres_dust_limit_native_receipt_history_and_cooldown() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-limit", || {
        support::scenario(Case::DustLimit)
    })
}

#[test]
fn forge_postgres_dust_limit_cap_refusal_is_retained_without_value_changes() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-cap", || support::scenario(Case::DustCap))
}

#[test]
fn forge_postgres_resource_refusals_remain_retained_after_refill() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-refusals", || {
        support::scenario(Case::Refusals)
    })
}

#[test]
fn forge_postgres_capability_payload_and_session_generation_are_fenced() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-fences", || support::scenario(Case::Fences))
}

#[test]
fn forge_postgres_concurrent_duplicate_occurrence_applies_exactly_once() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-concurrent", || {
        support::scenario(Case::Concurrent)
    })
}

#[test]
fn forge_postgres_conversion_with_full_backpack_refuses_without_spending() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-no-room", || {
        support::scenario(Case::NoRoom)
    })
}

async fn conversions(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(h, &[(80, FORGE_SLIVER_KEY, 47, 0)]).await?;
    let dust = request(Operation::DustToSlivers, 100);
    let (status, first) = exchange(owner, admitted, dust).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(first["applied"], true);
    assert_eq!(first["result"]["slivers_created"], 3);
    assert_eq!(balances(h).await?, (40, 100, 50, 0));
    assert_eq!(
        exchange(owner, admitted, dust).await?,
        (CommandStatus::Accepted, first.clone()),
        "replay is the retained result during cooldown"
    );
    assert_eq!(balances(h).await?, (40, 100, 50, 0));
    cooldown_elapsed(h).await?;
    let core = request(Operation::SliversToCores, 101);
    let (status, second) = exchange(owner, admitted, core).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(second["succeeded"], true);
    assert_eq!(second["result"]["slivers_spent"], 50);
    assert_eq!(second["result"]["cores_created"], 1);
    assert_eq!(balances(h).await?, (40, 100, 0, 1));
    assert_eq!(
        exchange(owner, admitted, core).await?,
        (CommandStatus::Accepted, second.clone())
    );
    cooldown_elapsed(h).await?;
    let (status, history) = exchange(owner, admitted, request(Operation::QueryHistory, 0)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let history = history.as_array().ok_or("history array")?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0]["receipt"], second);
    assert_eq!(history[1]["receipt"], first);
    assert!(serde_json::to_vec(history)?.len() <= 32_768);
    let operations:i64=sqlx::query_scalar("SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND applied").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(operations, 2, "replays add no mutation receipts");
    let native_spends:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_forge_dust_entries WHERE character_id=encode($1,'hex')::uuid AND kind='CONVERT'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(native_spends, 1, "reuse the P19 native dust ledger");
    assert_eq!(balances(h).await?, (40, 100, 0, 1));
    cooldown_elapsed(h).await?;
    let mut empty = request(Operation::QueryHistory, 0);
    empty.history_limit = 0;
    assert_eq!(
        exchange(owner, admitted, empty).await?,
        (CommandStatus::Accepted, serde_json::json!([]))
    );
    Ok(())
}

async fn dust_limit(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let action = request(Operation::DustLimit, 110);
    let (status, first) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        first,
        serde_json::json!({"operation":"dust_limit","succeeded":true,"result":{"dust":75,"dust_limit":120}})
    );
    assert_eq!(balances(h).await?, (75, 120, 0, 0));
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, first.clone())
    );
    let (started,next):(i64,i64)=sqlx::query_as("SELECT r.occurred_at,c.next_action_at FROM game_forge_reservations r JOIN game_forge_cooldowns c USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        next - started,
        490,
        "Crystal configured UI delay500 minus10"
    );
    let work: (i16, i16) = sqlx::query_as("SELECT p.work_units_used,l.work_units_used FROM game_forge_reservations p JOIN game_forge_dust_limit_reservations l USING(character_id,occurrence_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        work,
        (1, 1),
        "one successful attempt charges the shared retry budget once"
    );
    // Control the persisted clock precondition, without relying on the speed of this test host.
    sqlx::query("UPDATE game_forge_cooldowns SET next_action_at=9223372036854770000 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&h.pool).await?;
    let next_request = request(Operation::DustLimit, 111);
    let (status, refused) = exchange(owner, admitted, next_request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(refused["reason"], "COOLDOWN");
    assert_eq!(refused["applied"], false);
    cooldown_elapsed(h).await?;
    assert_eq!(
        exchange(owner, admitted, next_request).await?,
        (CommandStatus::Accepted, refused)
    );
    assert_eq!(balances(h).await?, (75, 120, 0, 0));
    let (status, history) = exchange(owner, admitted, request(Operation::QueryHistory, 0)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(history.as_array().ok_or("history")?.len(), 1);
    assert_eq!(history[0]["receipt"], first);
    let legacy:i64=sqlx::query_scalar("SELECT count(*) FROM game_forge_dust_limit_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        legacy, 1,
        "existing DustLimit receipts remain the native authority"
    );
    Ok(())
}

async fn dust_cap(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let before = balances(h).await?;
    let action = request(Operation::DustLimit, 115);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["reason"], "MAXIMUM_LIMIT");
    assert_eq!(result["applied"], false);
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, result)
    );
    assert_eq!(balances(h).await?, before);
    cooldown_elapsed(h).await?;
    assert_eq!(
        exchange(owner, admitted, request(Operation::QueryHistory, 0)).await?,
        (CommandStatus::Accepted, serde_json::json!([]))
    );
    Ok(())
}

async fn refusals(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let action = request(Operation::DustToSlivers, 120);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["reason"], "INSUFFICIENT_DUST");
    assert_eq!(balances(h).await?, (59, 100, 0, 0));
    gain_dust(h, *admitted, 1).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, result),
        "refilling resources does not rewrite the refusal"
    );
    assert_eq!(balances(h).await?, (60, 100, 0, 0));
    cooldown_elapsed(h).await?;
    let (status, converted) =
        exchange(owner, admitted, request(Operation::DustToSlivers, 121)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(converted["applied"], true);
    assert_eq!(balances(h).await?, (0, 100, 3, 0));
    cooldown_elapsed(h).await?;
    let core = request(Operation::SliversToCores, 122);
    let (status, result) = exchange(owner, admitted, core).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["reason"], "INSUFFICIENT_SLIVERS");
    seed_items(h, &[(80, FORGE_SLIVER_KEY, 47, 0)]).await?;
    assert_eq!(
        exchange(owner, admitted, core).await?,
        (CommandStatus::Accepted, result)
    );
    assert_eq!(balances(h).await?, (0, 100, 50, 0));
    cooldown_elapsed(h).await?;
    let (status, converted) =
        exchange(owner, admitted, request(Operation::SliversToCores, 123)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(converted["applied"], true);
    assert_eq!(balances(h).await?, (0, 100, 0, 1));
    let mut conflict = action;
    conflict.operation = Operation::SliversToCores;
    assert_eq!(
        exchange(owner, admitted, conflict).await?,
        (CommandStatus::Rejected, Value::Null)
    );
    let terminal: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let applied:i64=sqlx::query_scalar("SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND applied").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((terminal, applied), (4, 2));
    Ok(())
}

async fn fences(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let before = balances(h).await?;
    let selected = admitted.continuity.selected_capabilities;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 12, 14],
    )
    .ok_or("unselected forge")?;
    assert_eq!(
        exchange(owner, admitted, request(Operation::DustToSlivers, 130)).await?,
        (CommandStatus::Rejected, Value::Null)
    );
    admitted.continuity.selected_capabilities = selected;
    let mut malformed = request(Operation::DustToSlivers, 131)
        .encode()
        .ok_or("request")?;
    malformed[17] = 4;
    assert_eq!(
        support::exchange_raw(owner, admitted, &malformed).await?,
        (CommandStatus::Rejected, Vec::new())
    );
    assert_eq!(balances(h).await?, before);
    let mut stale = *admitted;
    stale.continuity.connection_generation += 1;
    assert_eq!(
        owner
            .item_forge(
                stale,
                admitted.continuity.next_command_id,
                request(Operation::DustToSlivers, 132),
                None,
            )
            .await,
        Some(Vec::new())
    );
    let unknown = Request {
        first_item: std::num::NonZeroU64::new(u64::MAX),
        second_item: std::num::NonZeroU64::new(u64::MAX - 1),
        ..request(Operation::Fusion, 134)
    };
    let (status, refusal) = exchange(owner, admitted, unknown).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(refusal["reason"], "INVALID_ITEM_HANDLE");
    assert_eq!(refusal["applied"], false);
    assert_eq!(
        exchange(owner, admitted, unknown).await?,
        (CommandStatus::Accepted, refusal)
    );
    assert_eq!(balances(h).await?, before);
    cooldown_elapsed(h).await?;
    let action = request(Operation::DustToSlivers, 133);
    let original = exchange(owner, admitted, action).await?;
    assert_eq!(original.0, CommandStatus::Accepted);
    let after = balances(h).await?;
    assert_eq!(after, (40, 100, 3, 0));
    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid").bind(admitted.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
    assert_eq!(
        owner
            .item_forge(*admitted, admitted.continuity.next_command_id, action, None)
            .await,
        Some(Vec::new()),
        "even a retained receipt is session-generation fenced"
    );
    assert_eq!(balances(h).await?, after);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts, 2,
        "malformed, unselected and stale commands create no receipt"
    );
    // The old generation is refused above. The newly current generation must
    // still replay the old occurrence, using its committed receipt.
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
    let current_fence =
        crate::gameplay_transport::item_fence_of(current).ok_or("current reconnect item fence")?;
    assert_eq!(
        current_fence.connection_generation.get(),
        admitted.continuity.connection_generation + 1
    );
    admitted.continuity.connection_generation = current_fence.connection_generation.get();
    admitted.item_fence = Some(current_fence);
    assert_eq!(exchange(owner, admitted, action).await?, original);
    assert_eq!(balances(h).await?, after);
    let after_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(after_receipts, receipts);
    Ok(())
}

async fn concurrent(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let first = request(Operation::DustToSlivers, 140);
    assert_eq!(
        exchange(owner, admitted, first).await?.0,
        CommandStatus::Accepted
    );
    gain_dust(h, *admitted, 60).await?;
    cooldown_elapsed(h).await?;
    let action = request(Operation::DustToSlivers, 141);
    let command = admitted.continuity.next_command_id;
    let (a, b) = support::together(
        available_reply(owner, *admitted, command, action),
        available_reply(owner, *admitted, command + 1, action),
    )
    .await;
    assert_eq!(
        a?, b?,
        "concurrent callers observe the same retained outcome"
    );
    assert_eq!(balances(h).await?, (40, 100, 6, 0));
    let receipt:i64=sqlx::query_scalar("SELECT count(*) FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid AND applied").bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipt, 1);
    let entries:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_forge_dust_entries WHERE character_id=encode($1,'hex')::uuid AND cause_occurrence_id=encode($2,'hex')::uuid").bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(entries, 1);
    Ok(())
}

async fn available_reply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    request: Request,
) -> TestResult<Value> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(bytes) = owner.item_forge(admitted, command, request, None).await {
                if bytes.is_empty() {
                    return Err("concurrent Forge call was rejected".into());
                }
                return Ok(serde_json::from_slice(&bytes)?);
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

async fn no_room(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let items: Vec<_> = (80..100)
        .map(|seed| (seed, "oteryn:item.tibia.i3031", 1, 0))
        .collect();
    seed_items(h, &items).await?;
    let before = balances(h).await?;
    let action = request(Operation::DustToSlivers, 150);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["reason"], "NO_ROOM");
    assert_eq!(result["applied"], false);
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, result)
    );
    assert_eq!(balances(h).await?, before);
    let entries:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(entries, 20);
    Ok(())
}

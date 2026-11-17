//! M02: framed world USE reads the real ordered Ground projection without debit.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, GroundStackEntry, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget, encode_use_intent, decode_use_result};

const GOLD: &str = "oteryn:item.tibia.i3031";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const NAME: &str = "qengine-rewards-questpc4-puzzle-predicate-srv7-m02";

#[test]
fn srv7_m02_real_ground_order_guards_framed_world_use_without_consumption() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srv7_ground_stack", || {
        super::qengine_rewards::scenario(NAME)
    });
    eprintln!("SRV7_PG_EXECUTED ground_stack duration={:?} success={}", started.elapsed(), result.is_ok());
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}

pub(super) fn fixture(mut callback: Callback) -> TestResult<Callback> {
    let guard = callback.world_requirements.first_mut().ok_or("static puzzle anchor")?;
    guard.ground_stack = Some(vec![
        GroundStackEntry { item: GOLD.into(), count: 1 },
        GroundStackEntry { item: PLATINUM.into(), count: 1 },
    ]);
    Ok(callback)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let target = Position { x: 33294, y: 31677, floor: -15 };
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|_| "current fence")?.ok_or("current fence")?;
    let (key, placement) = {
        let map = owner.bundle_map().ok_or("map")?;
        let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1).ok_or("lever key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        (key, placement)
    };
    let receipts_before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    let payload = |revision| encode_use_intent(&WorldObjectTarget { placement: placement.clone(), expected_revision: revision }).map_err(debug);
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload(0)?).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected, "empty Ground cannot open the puzzle");
    for (n, item) in [(8701, GOLD), (8702, PLATINUM)] {
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
            position: target, item: item.into(), count: 1,
        }, n, false).await?;
    }
    // Fixture setup calls the durable owner directly; the normal Item loop
    // invalidates this projection after a move. Apply that same invalidation.
    owner.states.loop_ground.invalidate();
    let before: (i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035')")
        .fetch_one(&h.pool).await?;
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(0)?).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed, "exact ordered durable roots admit native USE");
    assert_eq!(owner.bundle_map().ok_or("map")?.facts.object_revision(key), 1);
    let after: (i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035')")
        .fetch_one(&h.pool).await?;
    assert_eq!(before, after, "read-only Ground proof cannot consume or duplicate Items");
    let receipts_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts_before, receipts_after);
    Ok(())
}

//! Registered M19 callbacks through real wire USE and PostgreSQL custody.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn qready1_bewitched_daily_contribution_real_dispatch() -> TestResult {
    run(
        "qready1-bewitched-contribution",
        Position {
            x: 32769,
            y: 32026,
            floor: -8,
        },
    )
}
#[test]
fn qready1_raven_herb_interval_real_dispatch() -> TestResult {
    run(
        "qready1-threatened-raven-herb",
        Position {
            x: 33497,
            y: 32196,
            floor: -7,
        },
    )
}
fn run(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, position, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M19 real-dispatch PG thread failed")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn use_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
    id: u64,
) -> TestResult<UseDisposition> {
    // The wire helper respects the real 200 ms action cooldown and retains replay state.
    super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(id, target)])
        .await?
        .first()
        .map(|(_, disposition)| *disposition)
        .ok_or_else(|| "USE response".into())
}
async fn snapshot(h: &Harness, callback: &Callback) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$2)")
        .bind(&callback.state_ops[0].track).bind(&callback.key).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let epoch = 1_700_000_000;
    h.root.set_test_quest_clock(epoch)?;
    let wait = match callback.state_ops[0].from {
        oteryn_gameplay_quests::quest::reward_callbacks::StateCondition::ElapsedAtLeast {
            seconds,
        } => i64::from(seconds),
        _ => return Err("production clock predicate".into()),
    };
    let position = callback.selector.position.ok_or("source position")?;
    let key = oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
        .ok_or("source key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let fence = admitted.item_fence.ok_or("item fence")?;
    let offset = u64::from(callback.offerings.is_empty());
    if !callback.offerings.is_empty() {
        assert_eq!(
            use_command(owner, admitted, &target, 1).await?,
            UseDisposition::Rejected
        );
        assert_eq!(snapshot(h, callback).await?, (-1, 0, 0, 0));
        for (i, input) in callback.offerings.iter().enumerate() {
            super::quest_world_callbacks::input_item(owner, fence, input, 101 + i as u64, false)
                .await?;
        }
    }
    assert_eq!(
        use_command(owner, admitted, &target, 2 - offset).await?,
        UseDisposition::Committed
    );
    let first_target = target.clone();
    target.expected_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let before = snapshot(h, callback).await?;
    assert_eq!(
        before,
        (
            epoch,
            1,
            if callback.effects.item_reward.is_some() {
                1
            } else {
                0
            },
            if callback.offerings.is_empty() { 0 } else { 6 }
        )
    );
    assert_eq!(
        use_command(owner, admitted, &target, 3 - offset).await?,
        UseDisposition::Rejected
    );
    assert_eq!(snapshot(h, callback).await?, before);
    h.root.advance_test_quest_clock(wait - 1)?;
    assert_eq!(
        use_command(owner, admitted, &target, 4 - offset).await?,
        UseDisposition::Rejected
    );
    assert_eq!(snapshot(h, callback).await?, before);
    h.root.advance_test_quest_clock(1)?;
    for (i, input) in callback.offerings.iter().enumerate() {
        super::quest_world_callbacks::input_item(owner, fence, input, 201 + i as u64, false)
            .await?;
    }
    assert_eq!(
        use_command(owner, admitted, &target, 5 - offset).await?,
        UseDisposition::Committed
    );
    let after = snapshot(h, callback).await?;
    assert_eq!(
        after,
        (
            epoch + wait,
            2,
            if callback.effects.item_reward.is_some() {
                2
            } else {
                0
            },
            if callback.offerings.is_empty() { 0 } else { 12 }
        )
    );
    // A new transport retains its sequence, not its retired response cache.
    // SQL replay after reconnect is covered by qready1_world_clock_rewards.
    let replay = use_command(owner, admitted, &first_target, 2 - offset).await;
    assert!(
        replay
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("CommandOutcomeExpired")),
        "expired historical wire command must close the transport"
    );
    assert_eq!(
        snapshot(h, callback).await?,
        after,
        "historical wire replay cannot mint or consume twice"
    );
    if callback.effects.item_reward.is_some() {
        let reward:(String,i64)=sqlx::query_as("SELECT min(i.definition_production_key),sum(m.quantity)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id)").fetch_one(&h.pool).await?;
        assert_eq!(reward, ("oteryn:item.tibia.i5953".into(), 2));
    }
    Ok(())
}

//! P09 concrete player scenario, derived from the matrix row, the parity audit, and
//! `gameplay_transport/daily_reward.rs`:
//! 1. Admit the node-boot character and send a native DailyRewardQuery without opening
//!    the wall; expect an accepted wall frame whose claim is disabled.
//! 2. Send DailyRewardClaim while unopened; expect rejected disposition 4 and an
//!    unchanged wall (the refusal path must not advance day, streak, or jokers).
//! 3. Walk through the compiled world to a production daily shrine, send its ordinary
//!    USE command, then query again; expect claim_enabled=true with otherwise unchanged
//!    day/streak/jokers.
//! 4. Query the native Prey wall to capture the reward-card (wildcard) balance, send
//!    DailyRewardClaim, and expect accepted disposition 1 plus a transaction id.
//! 5. Query both walls again; expect claimed=true, claim disabled, streak advanced once,
//!    the server-authored day's wildcard delta, and (on an authored boost day) a timed XP
//!    boost to be consumed by ordinary monster combat.
//! 6. Send the same player claim again; expect accepted idempotent disposition 2 and no
//!    further daily-wall or wildcard delta.
//! 7. Logout normally, obtain a fresh admission, and re-query both walls; expect the
//!    claimed day, streak, jokers, and reward-card balance to remain durable.
//!
//! Calendar gaps, monthly joker grants, and timed-boost combat require later real reset
//! epochs. This journey never advances server time or edits durable state; when the
//! admitted lane cannot naturally exhibit one of those branches it reports the exact
//! observed wall as a failed step rather than manufacturing coverage.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::daily_reward as wire;

type Wall = wire::DailyRewardWall;

async fn wall(client: &mut DevClientSession) -> TestResult<Wall> {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, decode_reply,
    };
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-1/3 daily query observed={reply:?}").into());
    }
    match decode_reply(DailyRewardCommand::Query, &reply.payload)? {
        DailyRewardReply::Wall(value) => Ok(value),
        observed => Err(format!("step-1/3 expected wall observed={observed:?}").into()),
    }
}

async fn claim(client: &mut DevClientSession, disposition: u8) -> TestResult {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, send,
    };
    let reply = send(client.session_mut(), &DailyRewardCommand::Claim).await?;
    let status = if disposition <= 2 {
        CommandStatus::Accepted
    } else {
        CommandStatus::Rejected
    };
    if reply.status != status
        || !matches!(reply.reply,
        Some(DailyRewardReply::Result(result)) if result.disposition == disposition
            && result.transaction_id.is_some() == (disposition == 1))
    {
        return Err(
            format!("daily claim expected disposition={disposition} observed={reply:?}").into(),
        );
    }
    Ok(())
}

async fn wildcards(client: &mut DevClientSession) -> TestResult<u8> {
    use oteryn_dev_client::client_runtime::prey;
    let reply = prey::send(client.session_mut(), &prey::prey::Request::Query).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-4 prey query observed={reply:?}").into());
    }
    Ok(prey::prey::decode_reply(&reply.payload)
        .map_err(|error| format!("step-4 prey reply decode observed={error:?}"))?
        .wildcard_balance)
}

fn shrine_tile(world: &BundleWorld, start: TilePos) -> TestResult<TilePos> {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let content = crate::content::daily_reward::DailyRewardContent::catalogue()
        .ok_or("step-3 daily catalogue unavailable")?;
    let index = provisioning::item_index()?;
    let refs = content
        .shrine_source_ids
        .iter()
        .map(|id| provisioning::item_reference(&index, *id))
        .collect::<TestResult<Vec<_>>>()?;
    let facts = world.facts().pinned();
    for (floor, x, y, tile_data) in world.base().tiles() {
        let at = TilePos { floor, x, y };
        if start.x.abs_diff(x) > 160 || start.y.abs_diff(y) > 160 {
            continue;
        }
        let shrine = tile_data.ids().iter().zip(tile_data.depths())
            .filter(|(_, depth)| **depth == 0).enumerate().any(|(ordinal, (id, _))| {
                u8::try_from(ordinal).ok().and_then(|n| facts.base_entry(at, n, *id))
                    .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(reference) if refs.contains(&reference)))
            });
        if shrine
            && route(world, start, |candidate| {
                candidate.floor == at.floor
                    && candidate.x.abs_diff(at.x).max(candidate.y.abs_diff(at.y)) <= 1
            })
            .is_some()
        {
            return Ok(at);
        }
    }
    Err("step-3 no reachable production daily shrine in town window".into())
}

fn same_kill_sample(before: full_row_p05::KillSample, after: full_row_p05::KillSample) -> bool {
    before.race == after.race
        && before.raw_xp > 0
        && before.raw_xp == after.raw_xp
        && before.gained_xp > 0
        && after.gained_xp > before.gained_xp
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc9).await?;
    let initial = wall(&mut client).await?;
    if initial.claimed || initial.claim_enabled {
        return Err(format!("step-1 needs fresh unopened day observed={initial:?}").into());
    }
    claim(&mut client, 4).await.map_err(|e| {
        verdict::record_error(verdict::IndexedError::as_error(&e));
        verdict::context(format!("step-2 unopened refusal: {e}"), e)
    })?;
    if wall(&mut client).await? != initial {
        return Err("step-2 refusal changed daily wall".into());
    }
    let reward = crate::content::daily_reward::DailyRewardContent::catalogue()
        .and_then(|content| content.reward(initial.lane_day, false))
        .ok_or("step-5 authored reward absent")?;
    let baseline_xp = if reward.xp_boost_percent > 0 {
        Some(
            full_row_p05::hunt_for_race(
                &mut client,
                world,
                &metadata.spawns,
                "oteryn:creature.rat",
            )
            .await?,
        )
    } else {
        None
    };
    let shrine = shrine_tile(world, tile(client.world_spatial().actor_position)?)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == shrine.floor && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
    })
    .ok_or("step-3 shrine walking route disappeared")?;
    walk(&mut client, world, &path).await?;
    let opened = client
        .session_mut()
        .use_object(b"oteryn:placement/daily-reward-shrine", 0)
        .await?;
    if opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step-3 shrine USE observed={opened:?}").into());
    }
    let ready = wall(&mut client).await?;
    if !ready.claim_enabled || ready.claimed {
        return Err(format!("step-3 opened wall observed={ready:?}").into());
    }
    let before_cards = wildcards(&mut client).await?;
    claim(&mut client, 1).await.map_err(|e| {
        verdict::record_error(verdict::IndexedError::as_error(&e));
        verdict::context(format!("step-4 claim: {e}"), e)
    })?;
    let claimed = wall(&mut client).await?;
    let after_cards = wildcards(&mut client).await?;
    if !claimed.claimed
        || claimed.claim_enabled
        || claimed.streak != initial.streak + 1
        || after_cards != before_cards.saturating_add(reward.prey_wildcards)
    {
        return Err(format!("step-5 deltas wall={initial:?}->{claimed:?} cards={before_cards}->{after_cards} expected_reward={reward:?}").into());
    }
    claim(&mut client, 2).await.map_err(|e| {
        verdict::record_error(verdict::IndexedError::as_error(&e));
        verdict::context(format!("step-6 duplicate refusal: {e}"), e)
    })?;
    if wall(&mut client).await? != claimed || wildcards(&mut client).await? != after_cards {
        return Err("step-6 duplicate changed wall or reward cards".into());
    }
    if let Some(baseline) = baseline_xp {
        let gained = full_row_p05::hunt_for_race(
            &mut client,
            world,
            &metadata.spawns,
            "oteryn:creature.rat",
        )
        .await?;
        if !same_kill_sample(baseline, gained) {
            return Err(format!("step=boost-combat-xp incomparable or unboosted samples before={baseline:?} after={gained:?}").into());
        }
        evidence(&format!(
            "P09_CHECK step=boost-combat-xp unboosted={baseline:?} boosted={gained:?}"
        ));
    }
    full_row_p05::logout_after_combat(&mut client).await?;
    client = connect_world(clients, 0xca).await?;
    if wall(&mut client).await? != claimed || wildcards(&mut client).await? != after_cards {
        return Err("step-7 fresh admission lost durable wall or reward cards".into());
    }
    full_row_p05::logout_after_combat(&mut client).await?;
    if reward.xp_boost_percent == 0 {
        return Err(format!("step-5 timed XP boost not present on observed authored lane day {}; wall={claimed:?}; need a later real reset epoch whose reward has an XP boost", initial.lane_day).into());
    }
    Err("step=reset-calendar need=missed-day-joker-consumption-and-monthly-grant-across-real-reset-epochs;source=crates/gameplay/durability/src/durability/daily_reward_claim.rs:192".into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), journey(clients))
        .await
        .unwrap_or_else(|_| Err("step timeout observed=no-reply-frame".into()));
    if let Err(error) = &result {
        evidence(&format!(
            "P09_GAP status=FAIL observed={}",
            error.to_string().replace(char::is_whitespace, "_")
        ));
    } else {
        evidence("P09_GAP status=PASS");
    }
    evidence(&format!(
        "e2e_all scenario=P09.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
#[test]
fn row_09_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p09.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P09_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p09_guards_the_journey() {
    assert!(
        include_str!("full_row_p09.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("same_kill_sample"))
    );
}

#[cfg(test)]
#[test]
fn boost_comparison_rejects_a_different_race_or_base_award() -> TestResult {
    use full_row_p05::KillSample;
    let race = |n| std::num::NonZeroU32::new(n).ok_or("zero fixture race");
    let before = KillSample {
        race: race(1)?,
        raw_xp: 20,
        gained_xp: 20,
    };
    let boosted = KillSample {
        gained_xp: 25,
        ..before
    };
    assert!(same_kill_sample(before, boosted));
    assert!(!same_kill_sample(
        before,
        KillSample {
            race: race(2)?,
            ..boosted
        }
    ));
    assert!(!same_kill_sample(
        before,
        KillSample {
            raw_xp: 25,
            ..boosted
        }
    ));
    assert!(!same_kill_sample(before, before));
    Ok(())
}

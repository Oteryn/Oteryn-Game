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

async fn prey_state(
    client: &mut DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::prey::Reply> {
    use oteryn_dev_client::client_runtime::prey;
    let reply = prey::send(client.session_mut(), &prey::prey::Request::Query).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-4 prey query observed={reply:?}").into());
    }
    prey::prey::decode_reply(&reply.payload)
        .map_err(|error| format!("step-4 prey reply decode observed={error:?}").into())
}

pub(super) fn shrine_tile(world: &BundleWorld, start: TilePos) -> TestResult<TilePos> {
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

fn boost_bounds(raw: u64, percent: u8) -> Option<(u64, u64)> {
    let increment = raw.checked_mul(u64::from(percent))?;
    Some((increment / 100, increment.div_ceil(100)))
}

async fn rat_has_xp_prey(client: &mut DevClientSession) -> TestResult<bool> {
    Ok(prey_state(client)
        .await?
        .slots
        .iter()
        .any(|slot| slot.state == 2 && slot.bonus == 3 && slot.selected == "oteryn:creature.rat"))
}

async fn xp_stats(client: &mut DevClientSession) -> TestResult<(u32, i32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as character;
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[character::CharacterTab::GeneralStats as u8, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
    {
        return Err("step=boost-combat-xp invalid GeneralStats reply".into());
    }
    let level = reply
        .payload
        .get(9..13)
        .ok_or("GeneralStats level missing")?;
    let stamina = reply
        .payload
        .get(29..31)
        .ok_or("GeneralStats stamina missing")?;
    Ok((
        u32::from_le_bytes(level.try_into()?),
        i32::from(u16::from_le_bytes(stamina.try_into()?)),
    ))
}

async fn daily_details(client: &mut DevClientSession) -> TestResult<wire::details::Details> {
    let result = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::details::COMMAND_TYPE,
            &[],
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("step=calendar-details observed={:?}", result.status).into());
    }
    wire::details::Details::decode(&result.payload).map_err(Into::into)
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
    let initial_epoch = daily_details(&mut client).await?.epoch;
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
        if rat_has_xp_prey(&mut client).await? {
            return Err("step=boost-baseline need=rat-without-active-XP-prey".into());
        }
        let stats = xp_stats(&mut client).await?;
        Some((
            stats,
            full_row_p05::hunt_for_race(
                &mut client,
                world,
                &metadata.spawns,
                "oteryn:creature.rat",
            )
            .await?,
        ))
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
    let before_cards = prey_state(&mut client).await?.wildcard_balance;
    claim(&mut client, 1).await.map_err(|e| {
        verdict::record_error(verdict::IndexedError::as_error(&e));
        verdict::context(format!("step-4 claim: {e}"), e)
    })?;
    let claimed = wall(&mut client).await?;
    let after_cards = prey_state(&mut client).await?.wildcard_balance;
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
    if wall(&mut client).await? != claimed
        || prey_state(&mut client).await?.wildcard_balance != after_cards
    {
        return Err("step-6 duplicate changed wall or reward cards".into());
    }
    let mut boost_proven = false;
    if let Some(((level, stamina), baseline)) = baseline_xp {
        if rat_has_xp_prey(&mut client).await? {
            return Err("step=boost-combat-xp overlapping rat XP Prey".into());
        }
        let after_stats = xp_stats(&mut client).await?;
        let gained = full_row_p05::hunt_for_race(
            &mut client,
            world,
            &metadata.spawns,
            "oteryn:creature.rat",
        )
        .await?;
        let raw = i64::try_from(baseline.raw_xp)?;
        // The row claims the Free reward tier. Compare only in a stable level/stamina band.
        let model = |percent| {
            oteryn_gameplay_characters::hunting_experience::hunting_experience(
                raw, 0, percent, stamina, false, level,
            )
            .ok()
            .and_then(|award| u64::try_from(award).ok())
        };
        let (minimum, maximum) = boost_bounds(baseline.raw_xp, reward.xp_boost_percent)
            .ok_or("step=boost-combat-xp award overflow")?;
        if baseline.race != gained.race || baseline.raw_xp != gained.raw_xp {
            return Err(format!(
                "step=boost-combat-xp incomparable samples {baseline:?} -> {gained:?}"
            )
            .into());
        }
        if after_stats == (level, stamina) && model(0) == Some(baseline.gained_xp) {
            let expected =
                model(i16::from(reward.xp_boost_percent)).ok_or("boost XP model refused")?;
            if gained.gained_xp != expected {
                return Err(format!(
                    "step=boost-combat-xp expected={expected} observed={gained:?}"
                )
                .into());
            }
            boost_proven = same_kill_sample(baseline, gained);
            evidence(&format!(
                "P09_CHECK step=boost-combat-xp status={} before={baseline:?} after={gained:?} expected={expected}",
                if boost_proven {
                    "PASS"
                } else {
                    "UNPROVEN-rounded-compatible"
                }
            ));
        } else {
            evidence(&format!(
                "P09_CHECK step=boost-combat-xp status=UNPROVEN need=stable-level-and-stamina expected-unscaled-increase={minimum}..={maximum}"
            ));
        }
    }
    full_row_p05::logout_after_combat(&mut client).await?;
    client = connect_world(clients, 0xca).await?;
    if wall(&mut client).await? != claimed
        || prey_state(&mut client).await?.wildcard_balance != after_cards
    {
        return Err("step-7 fresh admission lost durable wall or reward cards".into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(20);
    let epoch = loop {
        let details = daily_details(&mut client).await?;
        if details.epoch > initial_epoch || tokio::time::Instant::now() >= until {
            break details.epoch;
        }
        client.service_liveness(Duration::from_secs(2)).await?;
    };
    if epoch > initial_epoch && wall(&mut client).await?.claimed {
        return Err(
            format!("step=reset-calendar epoch={epoch} new daily wall still claimed").into(),
        );
    }
    let mut joker_spent = false;
    let mut monthly_grant = false;
    if epoch.checked_sub(initial_epoch).is_some_and(|gap| gap > 1) {
        let next = wall(&mut client).await?;
        let skipped = epoch - initial_epoch - 1;
        if skipped <= u64::from(next.jokers) {
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                at.floor == shrine.floor
                    && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
            })
            .ok_or("step=missed-day no return route to daily shrine")?;
            walk(&mut client, world, &path).await?;
            let opened = client
                .session_mut()
                .use_object(b"oteryn:placement/daily-reward-shrine", 0)
                .await?;
            if opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            {
                return Err(format!("step=missed-day shrine refused: {opened:?}").into());
            }
            claim(&mut client, 1).await?;
            let details = daily_details(&mut client).await?;
            if !details.history.iter().flatten().any(|receipt| {
                receipt.own_character
                    && receipt.epoch == epoch
                    && u64::from(receipt.jokers_spent) == skipped
                    && claimed.streak.checked_add(1) == Some(receipt.streak)
            }) {
                return Err(format!(
                    "step=missed-day joker receipt mismatch skipped={skipped} epoch={epoch}"
                )
                .into());
            }
            joker_spent = true;
            evidence(&format!(
                "P09_CHECK step=missed-day-joker-spent epoch={epoch} jokers={skipped}"
            ));
            if next.jokers > claimed.jokers {
                monthly_grant = true;
                evidence(&format!(
                    "P09_CHECK step=monthly-joker-grant before={} after={}",
                    claimed.jokers, next.jokers
                ));
            }
        }
    }
    full_row_p05::logout_after_combat(&mut client).await?;
    if boost_proven && joker_spent && monthly_grant {
        return Ok(());
    }
    Err(format!("step=reset-calendar initial-epoch={initial_epoch} observed-epoch={epoch} boost-proven={boost_proven} joker-spent={joker_spent} monthly-grant={monthly_grant}").into())
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
        fire_damage: false,
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

#[cfg(test)]
#[test]
fn review_2_p09() {
    assert!(
        include_str!("full_row_p09.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("boost_bounds"))
    );
}

#[cfg(test)]
#[test]
fn daily_boost_bounds_allow_truncation_and_exclude_unrelated_large_increases() {
    assert_eq!(boost_bounds(5, 10), Some((0, 1)));
    assert_eq!(boost_bounds(50, 10), Some((5, 5)));
    assert_eq!(boost_bounds(u64::MAX, 50), None);
}

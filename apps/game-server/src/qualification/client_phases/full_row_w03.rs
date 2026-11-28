//! W03 player scenario, derived from the W03 audit/matrix row and the native
//! `boss_rewards`/`kill_reward` owners:
//!
//! 1. Through ordinary movement and combat, participate in and defeat an eligible boss; expect
//!    its death to produce a private reward-chest group with a seven-day expiry.
//! 2. Send BOSS_REWARD_OPEN and page the chest; expect the boss/death identity, expiry and item
//!    count supplied by the server, then BROWSE that death and assert its item rows.
//! 3. At a reachable authored reward chest, send COLLECT_ALL for that death; expect an accepted
//!    `Ok` reply, positive moved count, zero remaining count and matching inventory deltas.
//! 4. Repeat COLLECT_ALL for the same death; expect the refusal path (`Empty`) with zero moved and
//!    no inventory delta. An expired group must instead return `Expired` and mint nothing.
//! 5. Log out, obtain a fresh admission, and repeat OPEN/BROWSE; expect the collected death to be
//!    absent (and any uncollected, unexpired groups and inventory rewards to remain durable).
//!
//! The node journey never manufactures a death, reward, clock or item. It sends the native
//! commands even when the provisioned character has no boss entitlement, and reports that
//! observed server state rather than turning a source audit into a verdict.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{boss_encounter, boss_rewards};
use oteryn_protocol_oteryn::boss_rewards::{
    BossRewardStatus, BrowseRequest, CollectAllRequest,
};
use std::collections::BTreeMap;

const ACTIONS: &[&str] = &[
    "open-after-player-boss-kill",
    "browse-server-items-and-expiry",
    "collect-all-inventory-delta",
    "repeat-collect-refusal",
    "fresh-admission-durable-state",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("W03_CHECK step={action} status=PASS observed={observed:?}"));
        }
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W03_CHECK step={action} status=FAIL observed=not-proven"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("W03_OBSERVED status=FAIL detail={error}"));
            evidence(&format!("W03_GAP {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=W03.full status={} seconds={:.3} evidence=runtime-native-boss-reward-commands",
            if result.is_ok() && self.passed.len() == ACTIONS.len() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout need=reachable-eligible-boss-and-reward-chest".into()),
    });
}

pub(super) async fn open(client: &mut DevClientSession) -> TestResult<boss_encounter::RewardChest> {
    let reply = client
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN,
            &boss_encounter::encode_reward_chest_query(None)
                .map_err(|error| format!("reward OPEN encode: {error:?}"))?,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=open observed=status:{:?}", reply.status).into());
    }
    boss_encounter::decode_rewards(&reply.payload)
        .ok_or_else(|| "step=open observed=missing-or-malformed-reward-chest-frame".into())
}

fn earned_group<'a>(
    prior: &boss_encounter::RewardChest,
    opened: &'a boss_encounter::RewardChest,
) -> Option<&'a oteryn_protocol_oteryn::boss_encounter::BossRewardChestEntry> {
    opened.entries.iter().find(|group| {
        group.boss_key == super::full_row_w02::BOSS
            && group.minted_at_unix_ms >= prior.now
            && !prior.entries.iter().any(|old| old.death == group.death)
    })
}

pub(super) async fn approach_chest(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    const CHEST: &str = "oteryn:item.tibia.i19250";
    const OWNER: &str = "crates/gameplay/dispatch/src/gameplay_transport/boss_rewards/physical.rs:10";
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let mut placements = Vec::new();
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                item.depth == 0 && manifest.palette[item.palette as usize].key == CHEST
            }) {
                placements.push(TilePos { x: tile.x, y: tile.y, floor: sector.floor });
            }
        }
        Ok(())
    })?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        placements.iter().any(|chest| {
            at.floor == chest.floor && at.x.abs_diff(chest.x).max(at.y.abs_diff(chest.y)) <= 1
        })
    }).ok_or_else(|| format!("step=walk-to-reward-chest observed=no-route from={from:?} authored={} need={CHEST} source={OWNER}", placements.len()))?;
    walk(client, world, &path).await
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0xf8, 0).await?;
    let prior = open(&mut client).await?;
    super::full_row_w02::defeat_faceless_bane(&mut client, clients).await?;
    let before = open(&mut client).await?;
    let mut repeated_collection = None;
    let group = earned_group(&prior, &before).ok_or_else(|| format!(
        "step=open-after-player-boss-kill observed=no-new-Faceless-Bane-reward-group count={} need=personal-reward-for-this-defeat source=crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs",
        before.entries.len()
    ))?;
    let death = group.death;
    approach_chest(&mut client, clients).await?;
    {
        row.pass("open-after-player-boss-kill", (&group.boss_key, group.death));
        if group.expires_at_unix_ms.saturating_sub(group.minted_at_unix_ms)
            != boss_encounter::BOSS_REWARD_EXPIRY_MS
            || group.expires_at_unix_ms <= before.now
        {
            return Err(format!("step=expiry observed=invalid-group:{group:?}").into());
        }
        let browse = boss_rewards::send(
            client.session_mut(),
            &boss_rewards::BossRewardsCommand::Browse(BrowseRequest {
                death: group.death,
                after_ordinal: 0,
            }),
        )
        .await?;
        match browse.reply {
            Some(boss_rewards::BossRewardsReply::Browse(reply))
                if reply.status == BossRewardStatus::Ok
                    && reply.death == group.death
                    && reply.expires_at_unix_ms == group.expires_at_unix_ms
                    && !reply.rows.is_empty() =>
            {
                row.pass(
                    "browse-server-items-and-expiry",
                    (reply.rows.len(), reply.expires_at_unix_ms),
                );
            }
            observed => {
                return Err(format!(
                    "step=browse-server-items-and-expiry observed=status:{:?},reply:{observed:?}",
                    browse.status
                )
                .into());
            }
        }
    }

    // A concrete unknown death exercises the private-handle refusal through the same native
    // BROWSE route. This is sent even when OPEN proves that node boot has no earned group.
    let unknown = [0x57; 16];
    let refused = boss_rewards::send(
        client.session_mut(),
        &boss_rewards::BossRewardsCommand::Browse(BrowseRequest {
            death: unknown,
            after_ordinal: 0,
        }),
    )
    .await?;
    let refused_status = match refused.reply {
        Some(boss_rewards::BossRewardsReply::Browse(reply)) => reply.status,
        _ => {
            return Err(format!(
                "step=unknown-death-refusal observed=status:{:?},missing-browse-frame",
                refused.status
            )
            .into());
        }
    };
    if !matches!(
        refused_status,
        BossRewardStatus::OutOfReach | BossRewardStatus::Rejected | BossRewardStatus::Empty
    ) {
        return Err(format!(
            "step=unknown-death-refusal observed=reward-status:{refused_status:?}"
        )
        .into());
    }

    // COLLECT_ALL is also a real mechanic attempt, not a source-derived probe. With no nearby
    // physical chest the expected runtime observation is OutOfReach and no mint.
    let inventory_before = inventory_quantities(&client)?;
    let collect = boss_rewards::send(
        client.session_mut(),
        &boss_rewards::BossRewardsCommand::CollectAll(CollectAllRequest { death: Some(death) }),
    )
    .await?;
    let collect_reply = match collect.reply {
        Some(boss_rewards::BossRewardsReply::Collect(reply)) => reply,
        _ => {
            return Err(format!(
                "step=collect-all observed=status:{:?},missing-collect-frame",
                collect.status
            )
            .into());
        }
    };
    evidence(&format!(
        "W03_ATTEMPT command=COLLECT_ALL reply={collect_reply:?} groups={}",
        before.entries.len()
    ));

    if collect_reply.status == BossRewardStatus::Ok
        && collect_reply.moved > 0
        && collect_reply.remaining == 0
        && inventory_quantities(&client)? != inventory_before
    {
        row.pass("collect-all-inventory-delta", collect_reply);
        let inventory_after = inventory_quantities(&client)?;
        let repeated = boss_rewards::send(
            client.session_mut(),
            &boss_rewards::BossRewardsCommand::CollectAll(CollectAllRequest {
                death: Some(death),
            }),
        )
        .await?;
        match repeated.reply {
            Some(boss_rewards::BossRewardsReply::Collect(reply))
                if reply.status == BossRewardStatus::Empty
                    && reply.moved == 0
                    && reply.remaining == 0
                    && inventory_quantities(&client)? == inventory_after =>
            {
                row.pass("repeat-collect-refusal", reply);
                repeated_collection = Some(reply);
            }
            observed => {
                return Err(format!(
                    "step=repeat-collect-refusal observed=status:{:?},reply:{observed:?}",
                    repeated.status
                )
                .into());
            }
        }
    }

    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?}").into());
    }
    let mut fresh = connect_character(clients, 0xf9, 0).await?;
    let after = open(&mut fresh).await?;
    if repeated_collection.is_some() && !after.entries.iter().eq(before.entries.iter().filter(|group| group.death != death)) {
        return Err("step=fresh-admission observed=collected-death-retained-or-uncollected-group-changed".into());
    }
    if repeated_collection.is_none() && !same_chest(&before, &after) {
        return Err("step=fresh-admission observed=reward-chest-snapshot-changed-without-accepted-collection".into());
    }
    row.pass("fresh-admission-durable-state", after.entries.len());
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=fresh-logout observed={cleanup:?}").into());
    }

    if let Some(repeated) = repeated_collection {
        return Err(format!("step=expiry-refusal observed=death:{death:?},collect:{collect_reply:?},repeat:{repeated:?},fresh-server-now:{},expiry:{}; need=separate-uncollected-player-earned-death-past-seven-day-expiry source=crates/gameplay/dispatch/src/gameplay_transport/boss_rewards.rs:49", after.now, group.expires_at_unix_ms).into());
    }
    Err(format!(
        "step=collect-all-inventory-delta observed={collect_reply:?};need=ordinary-walkable-route-to-authored-reward-chest"
    )
    .into())
}

fn inventory_quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut quantities = BTreeMap::new();
    for entry in &client
        .inventory()
        .ok_or("step=inventory-delta observed=missing-inventory-frame")?
        .entries
    {
        *quantities
            .entry(entry.item_definition_ref.get())
            .or_default() += u64::from(entry.count.get());
    }
    Ok(quantities)
}

fn same_chest(
    left: &boss_encounter::RewardChest,
    right: &boss_encounter::RewardChest,
) -> bool {
    left.entries == right.entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_reward_must_be_new_and_belong_to_the_fought_boss() {
        let reward = oteryn_protocol_oteryn::boss_encounter::BossRewardChestEntry {
            death: v7(7, 1),
            boss_key: "oteryn:boss.faceless_bane".into(),
            minted_at_unix_ms: 9,
            expires_at_unix_ms: 9 + boss_encounter::BOSS_REWARD_EXPIRY_MS,
            item_count: 1,
        };
        let prior = boss_encounter::RewardChest { now: 10, entries: vec![reward.clone()] };
        let mut other = reward.clone();
        other.death = v7(7, 2);
        other.boss_key = "oteryn:boss.the_primal_menace".into();
        let mut opened = boss_encounter::RewardChest { now: 11, entries: vec![reward.clone(), other] };
        assert_eq!(earned_group(&prior, &opened), None);
        let mut new = reward;
        new.death = v7(7, 3);
        new.minted_at_unix_ms = 11;
        new.expires_at_unix_ms = 11 + boss_encounter::BOSS_REWARD_EXPIRY_MS;
        opened.entries.push(new.clone());
        assert_eq!(earned_group(&prior, &opened), Some(&new));
    }

    #[test]
    fn durable_comparison_ignores_only_server_observation_time() {
        let left = boss_encounter::RewardChest {
            now: 10,
            entries: Vec::new(),
        };
        let right = boss_encounter::RewardChest {
            now: 11,
            entries: Vec::new(),
        };
        assert!(same_chest(&left, &right));
    }
}

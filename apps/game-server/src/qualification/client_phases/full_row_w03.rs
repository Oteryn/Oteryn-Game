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
use oteryn_protocol_oteryn::boss_rewards::{BossRewardStatus, BrowseRequest, CollectAllRequest};
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
            evidence(&format!(
                "W03_CHECK step={action} status=PASS observed={observed:?}"
            ));
        }
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!(
                    "W03_CHECK step={action} status=FAIL observed=not-proven"
                ));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("W03_OBSERVED status=FAIL detail={error}"));
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
    let result = verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=journey observed=timeout need=reachable-eligible-boss-and-reward-chest".into(),
        ),
    });
}

async fn open_page(
    client: &mut DevClientSession,
    cursor: Option<[u8; 16]>,
) -> TestResult<boss_encounter::RewardChest> {
    let reply = client
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN,
            &boss_encounter::encode_reward_chest_query(cursor)
                .map_err(|error| format!("reward OPEN encode: {error:?}"))?,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=open observed=status:{:?}", reply.status).into());
    }
    boss_encounter::decode_rewards(&reply.payload)
        .ok_or_else(|| "step=open observed=missing-or-malformed-reward-chest-frame".into())
}

async fn open(client: &mut DevClientSession) -> TestResult<boss_encounter::RewardChest> {
    use oteryn_protocol_oteryn::boss_encounter::REWARD_CHEST_PAGE_SIZE;
    let mut chest = open_page(client, None).await?;
    let mut deaths: std::collections::BTreeSet<_> =
        chest.entries.iter().map(|group| group.death).collect();
    let mut page_len = chest.entries.len();
    for _ in 0..16 {
        if page_len < REWARD_CHEST_PAGE_SIZE {
            return Ok(chest);
        }
        let page = open_page(client, chest.entries.last().map(|group| group.death)).await?;
        page_len = page.entries.len();
        for group in page.entries {
            if !deaths.insert(group.death) {
                return Err("step=open observed=repeated-death-across-chest-pages".into());
            }
            chest.entries.push(group);
        }
    }
    Err("step=open observed=chest-pagination-budget-exhausted".into())
}

async fn approach_reward_chest(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let reward_chest = MapDefinition::Item(provisioning::item_reference(
        &provisioning::item_index()?,
        19250,
    )?);
    let from = tile(client.world_spatial().actor_position)?;
    let path = reach_route(client, world, from, |at| {
        (-1..=1).any(|dx| {
            (-1..=1).any(|dy| {
                let Some((x, y)) = at.x.checked_add_signed(dx).zip(at.y.checked_add_signed(dy))
                else {
                    return false;
                };
                let pos = TilePos {
                    x,
                    y,
                    floor: at.floor,
                };
                world.base().tile(x, y, at.floor).is_some_and(|tile| {
                    tile.ids()
                        .iter()
                        .zip(tile.depths())
                        .filter(|(_, depth)| **depth == 0)
                        .enumerate()
                        .any(|(ordinal, (id, _))| {
                            u8::try_from(ordinal)
                                .ok()
                                .and_then(|ordinal| world.facts().base_entry(pos, ordinal, *id))
                                .is_some_and(|entry| entry.definition == reward_chest)
                        })
                })
            })
        })
    })
    .await?
    .map_err(|failure| format!("step=reward-chest-walk observed=no-route {failure}"))?;
    walk(client, world, &path).await
}

fn player_reward<'a>(
    before: &boss_encounter::RewardChest,
    after: &'a boss_encounter::RewardChest,
    defeat: [u8; 16],
) -> Option<&'a oteryn_protocol_oteryn::boss_encounter::BossRewardChestEntry> {
    after.entries.iter().find(|group| {
        group.death == defeat
            && group.boss_key == full_row_w02::BOSS
            && group.minted_at_unix_ms >= before.now
            && !before
                .entries
                .iter()
                .any(|prior| prior.death == group.death)
    })
}

async fn observed_defeat(clients: &SeamClients<'_>, handoff: [u8; 16]) -> TestResult<[u8; 16]> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let defeat: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(a.defeat_id) FROM game_boss_room_activities a
         JOIN game_boss_room_participants p USING (admission)
         WHERE a.admission = encode($1,'hex')::uuid
           AND a.world_id = encode($2,'hex')::uuid AND a.boss_key = $3
           AND p.character_id = encode($4,'hex')::uuid AND p.returned_at IS NOT NULL
           AND a.defeat_id IS NOT NULL",
    )
    .bind(handoff.as_slice())
    .bind(clients.world.as_slice())
    .bind(full_row_w02::BOSS)
    .bind(clients.characters[1].as_slice())
    .fetch_optional(&mut database)
    .await?;
    defeat
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "step=boss-defeat observed=no-defeat-for-the-returned-encounter".into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0xf8, 1).await?;
    let initial = open(&mut client).await?;
    let handoff = full_row_w02::fight_boss(&mut client, clients, "W03").await?;
    let defeat = observed_defeat(clients, handoff).await?;
    let before = open(&mut client).await?;
    let earned = player_reward(&initial, &before, defeat);
    let mut collected = None;

    if let Some(group) = earned {
        row.pass(
            "open-after-player-boss-kill",
            (&group.boss_key, group.death),
        );
        if group
            .expires_at_unix_ms
            .saturating_sub(group.minted_at_unix_ms)
            != boss_encounter::BOSS_REWARD_EXPIRY_MS
            || group.expires_at_unix_ms <= before.now
        {
            return Err(format!("step=expiry observed=invalid-group:{group:?}").into());
        }
        approach_reward_chest(&mut client, clients).await?;
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
        &boss_rewards::BossRewardsCommand::CollectAll(CollectAllRequest {
            death: Some(earned.map_or(unknown, |group| group.death)),
        }),
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
        let death = earned
            .ok_or("step=collect-all observed=Ok-without-an-open-reward-group")?
            .death;
        let inventory_after = inventory_quantities(&client)?;
        let repeated = boss_rewards::send(
            client.session_mut(),
            &boss_rewards::BossRewardsCommand::CollectAll(CollectAllRequest { death: Some(death) }),
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
                collected = Some((death, inventory_after));
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
    let mut fresh = connect_character(clients, 0xf9, 1).await?;
    let after = open(&mut fresh).await?;
    if let Some((death, inventory)) = &collected {
        let remaining: Vec<_> = before
            .entries
            .iter()
            .filter(|group| group.death != *death && group.expires_at_unix_ms > after.now)
            .collect();
        if after.entries.iter().collect::<Vec<_>>() != remaining
            || inventory_quantities(&fresh)? != *inventory
        {
            return Err(
                "step=fresh-admission observed=reward-chest-or-inventory-diverged-after-collection"
                    .into(),
            );
        }
    } else if !same_chest(&before, &after) {
        return Err("step=fresh-admission observed=reward-chest-snapshot-changed-without-accepted-collection".into());
    }
    row.pass("fresh-admission-durable-state", after.entries.len());
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=fresh-logout observed={cleanup:?}").into());
    }

    if collected.is_some() {
        return Ok(());
    }
    if earned.is_none() {
        return Err(format!(
            "step=open-after-player-boss-kill observed=accepted-chest-without-new-boss-reward;collect-all={:?};unknown-browse={refused_status:?};need=reachable-eligible-boss-that-settles-personal-reward",
            collect_reply.status
        )
        .into());
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

#[cfg(test)]
mod review_regressions {
    use super::*;
    use oteryn_protocol_oteryn::boss_encounter::BossRewardChestEntry;

    #[test]
    fn chest_reward_must_be_new_and_for_the_boss_fought() {
        let earned = |id, key: &str| BossRewardChestEntry {
            death: [id; 16],
            boss_key: key.into(),
            minted_at_unix_ms: 101,
            expires_at_unix_ms: 1000,
            item_count: 1,
        };
        let old = earned(1, full_row_w02::BOSS);
        let before = boss_encounter::RewardChest {
            now: 100,
            entries: vec![old.clone()],
        };
        let mut after = boss_encounter::RewardChest {
            now: 102,
            entries: vec![old, earned(2, "oteryn:boss.some_other_boss")],
        };
        assert!(player_reward(&before, &after, [3; 16]).is_none());
        after.entries.push(earned(4, full_row_w02::BOSS));
        assert!(player_reward(&before, &after, [3; 16]).is_none());
        after.entries.push(earned(3, full_row_w02::BOSS));
        assert_eq!(
            player_reward(&before, &after, [3; 16]).map(|group| group.death),
            Some([3; 16])
        );
    }
}

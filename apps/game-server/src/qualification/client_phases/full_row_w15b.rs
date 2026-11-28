//! Qualify the fixed Battle Axe Quest chest from normal wire commands.
//!
//! Seed only the approach placement. Verify one Battle Axe and one durable claim
//! transition, an unchanged Global Quest Log list, and exact duplicate refusals
//! both before and after fresh admission.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Custody(BTreeMap<(u8, u32, u32), u64>);

pub(super) fn reward_increased(
    before: &Custody,
    after: &Custody,
    definition: u32,
    count: u32,
) -> bool {
    let quantity = |custody: &Custody| {
        custody
            .0
            .iter()
            .filter(|((_, key, _), _)| *key == definition)
            .map(|(_, quantity)| quantity)
            .sum::<u64>()
    };
    quantity(after).checked_sub(quantity(before)) == Some(u64::from(count))
        && before
            .0
            .iter()
            .filter(|((_, key, _), _)| *key != definition)
            .eq(after.0.iter().filter(|((_, key, _), _)| *key != definition))
}

pub(super) fn custody(client: &DevClientSession) -> TestResult<Custody> {
    let inventory = client
        .inventory()
        .ok_or("step-1: inventory snapshot missing")?;
    let mut held = BTreeMap::new();
    let mut record = |slot, item: &oteryn_protocol_oteryn::item_view::ItemEntry| {
        *held
            .entry((slot, item.item_definition_ref.get(), item.sub_type))
            .or_default() += u64::from(item.count.get());
    };
    if let Some(backpack) = &inventory.main_backpack {
        record(0, backpack);
    }
    for item in &inventory.entries {
        record(u8::MAX, item);
    }
    for entry in &inventory.equipment {
        record(entry.slot as u8, &entry.item);
    }
    Ok(Custody(held))
}

#[derive(Clone)]
struct Chest {
    at: TilePos,
    target: WorldObjectTarget,
    claim: String,
    transition: String,
    reward_definition: u32,
    reward_count: u32,
}

fn quest_chests(world: &BundleWorld) -> TestResult<Vec<Chest>> {
    let items = provisioning::item_index()?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item.key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse().ok())
                .is_some_and(|id| provisioning::item_reference(&items, id).is_ok())
        },
    )?;
    let mut chests = Vec::new();
    for claim in report.served {
        for placement in claim.placements {
            let Some(transition) = placement.placement.quest_transition else {
                continue;
            };
            if placement.reward_callback.is_some() {
                continue;
            }
            let [reward] = placement.placement.items.as_slice() else {
                continue;
            };
            let reward_id = reward
                .item
                .key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .ok_or("step-2: bound chest reward has no Tibia item key")?
                .parse()?;
            let reward_definition = provisioning::item_reference(&items, reward_id)?.get();
            let mut key = world.base().digest().to_vec();
            key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
            chests.push(Chest {
                at: TilePos {
                    x: placement.cell.x,
                    y: placement.cell.y,
                    floor: placement.cell.floor,
                },
                target: WorldObjectTarget {
                    placement: key,
                    expected_revision: 0,
                },
                claim: claim.key.as_str().to_owned(),
                transition,
                reward_definition,
                reward_count: reward.count,
            });
        }
    }
    chests.sort_by_key(|chest| (chest.at, chest.claim.clone()));
    Ok(chests)
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

pub(super) const BATTLE_AXE_CLAIM: &str = "oteryn:reward-claim.kv.battleaxe";
pub(super) const BATTLE_AXE_CHEST: TilePos = TilePos {
    x: 32305,
    y: 32254,
    floor: -9,
};
const BATTLE_AXE_APPROACH: TilePos = TilePos {
    x: 32306,
    y: 32254,
    floor: -9,
};

pub(super) async fn connect_at_battle_axe(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
    row: &str,
) -> TestResult<DevClientSession> {
    wait_character_released(clients, index).await?;
    use sqlx::Connection as _;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    sqlx::query(
        "INSERT INTO game_character_pending_arrivals
         (character_id, occurrence_id, world_id, tile_x, tile_y, tile_floor)
         VALUES (encode($1,'hex')::uuid, uuidv7(), encode($2,'hex')::uuid, $3, $4, $5)",
    )
    .bind(clients.characters[index].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(BATTLE_AXE_APPROACH.x))
    .bind(i32::from(BATTLE_AXE_APPROACH.y))
    .bind(i16::from(-BATTLE_AXE_APPROACH.floor))
    .execute(&mut database)
    .await?;
    character_prep::declare_seeded(row, "placement", "isolate the fixed Battle Axe chest claim");
    let client = connect_character(clients, tag, index).await?;
    let pending = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM game_character_pending_arrivals
         WHERE character_id=encode($1,'hex')::uuid)",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut database)
    .await?;
    if pending {
        return Err("fixture arrival was not consumed by admission".into());
    }
    let observed = tile(client.world_spatial().actor_position)?;
    if observed != BATTLE_AXE_APPROACH || !adjacent(observed, BATTLE_AXE_CHEST) {
        return Err(format!(
            "fixture placement expected={BATTLE_AXE_APPROACH:?} observed={observed:?}"
        )
        .into());
    }
    evidence(&format!("W15B_FIXTURE row={row} placement={observed:?}"));
    Ok(client)
}

async fn use_chest(client: &mut DevClientSession, chest: &Chest) -> TestResult<UseDisposition> {
    let reply = client
        .session_mut()
        .use_object(&chest.target.placement, chest.target.expected_revision)
        .await?;
    Ok(reply.disposition)
}

async fn fresh_admission(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let old = client.join_snapshot().game_session_id;
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-5: logout refused: {reply:?}").into());
    }
    let fresh = connect_character(clients, 0xb7, 1).await?;
    if fresh.join_snapshot().game_session_id == old {
        return Err("step-5: fresh admission reused the old game session".into());
    }
    Ok(fresh)
}

async fn quest_receipts(
    database: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    transition: &str,
) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT count(*), count(*) FILTER (WHERE transition_key=$2) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid")
        .bind(clients.characters[1].as_slice()).bind(transition).fetch_one(database).await?)
}

use oteryn_dev_client::client_runtime::quest_log as journal;

fn quest_state(client: &mut DevClientSession) -> TestResult<journal::QuestState> {
    let selected = client.selected_capabilities().to_vec();
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG);
    journal::QuestState::from_snapshot(&selected, snapshot)
        .map_err(|error| format!("quest snapshot reducer: {error:?}").into())
}

async fn quest_list(
    client: &mut DevClientSession,
    state: &mut journal::QuestState,
) -> TestResult<Vec<oteryn_protocol_oteryn::quest_log::QuestListEntry>> {
    let command = state
        .command(&journal::QuestLogQuery::List)
        .map_err(|error| format!("quest list command: {error:?}"))?;
    let result = journal::send(client.session_mut(), &command).await?;
    state
        .record_result(result.status, &result.payload)
        .map_err(|error| format!("quest list result: {error:?}"))?;
    if result.status != CommandStatus::Accepted {
        return Err("quest journal List was refused".into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    let log = journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("quest list codec: {error:?}"))?;
    if !matches!(log.view, Some(journal::QuestLogView::List(_))) {
        return Err("quest list reply did not contain a List view".into());
    }
    state
        .apply(&oteryn_dev_client::AppliedDelta {
            server_sequence: delta.server_sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: log,
        })
        .map_err(|error| format!("quest list reducer: {error:?}"))?;
    Ok(state.list.clone())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_at_battle_axe(clients, 0xb5, 1, "W15b.full").await?;
    let mut journal = quest_state(&mut client)?;
    let before_quests = quest_list(&mut client, &mut journal).await?;
    let before_items = custody(&client)?;

    let chest = quest_chests(world)?
        .into_iter()
        .find(|chest| chest.at == BATTLE_AXE_CHEST && chest.claim == BATTLE_AXE_CLAIM)
        .ok_or("step-2: exact Battle Axe chest is not served")?;
    let expected = provisioning::item_reference(&provisioning::item_index()?, 3266)?.get();
    if chest.reward_definition != expected || chest.reward_count != 1 {
        return Err("step-2: Battle Axe chest does not bind exactly one Battle Axe".into());
    }
    use sqlx::Connection as _;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let receipts_before = quest_receipts(&mut database, clients, &chest.transition).await?;
    if receipts_before.1 != 0 {
        return Err("step-3: Battle Axe quest transition was already claimed".into());
    }
    let first = use_chest(&mut client, &chest).await?;
    if first != UseDisposition::Committed {
        return Err(format!("step-3: first USE reply={first:?} claim={}", chest.claim).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let awarded = loop {
        let observed = custody(&client)?;
        if reward_increased(
            &before_items,
            &observed,
            chest.reward_definition,
            chest.reward_count,
        ) {
            break observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step-3: USE committed but exact reward delta missing claim={} definition={} count={}",
                chest.claim, chest.reward_definition, chest.reward_count
            ).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let advanced = quest_list(&mut client, &mut journal).await?;
    let receipts_after = quest_receipts(&mut database, clients, &chest.transition).await?;
    if receipts_after != (receipts_before.0 + 1, receipts_before.1 + 1) {
        return Err(format!("step-3: exact quest receipt missing or unrelated transition committed: expected={} before={receipts_before:?} after={receipts_after:?}", chest.transition).into());
    }
    // The Battle Axe Quest is absent from Global's displayed Quest Log list.
    // Its once-only progress is proved by the exact durable claim transition above.
    if advanced != before_quests {
        return Err(format!(
            "step-3: Battle Axe claim unexpectedly changed the Global Quest Log list claim={}",
            chest.claim
        )
        .into());
    }

    let second = use_chest(&mut client, &chest).await?;
    if second != UseDisposition::NothingToUse {
        return Err(format!(
            "step-4: duplicate USE reply={second:?} claim={}",
            chest.claim
        )
        .into());
    }
    if custody(&client)? != awarded || quest_list(&mut client, &mut journal).await? != advanced {
        return Err("step-4: refused duplicate changed inventory or quest state".into());
    }

    let mut fresh = fresh_admission(&mut client, clients).await?;
    let mut fresh_journal = quest_state(&mut fresh)?;
    if custody(&fresh)? != awarded || quest_list(&mut fresh, &mut fresh_journal).await? != advanced
    {
        return Err(
            "step-5: reward custody or quest advance did not survive fresh admission".into(),
        );
    }
    if tile(fresh.world_spatial().actor_position)? != BATTLE_AXE_APPROACH {
        return Err("step-5: relog did not restore the chest approach tile".into());
    }
    let after_relog = use_chest(&mut fresh, &chest).await?;
    if after_relog != UseDisposition::NothingToUse {
        return Err(format!("step-5: post-relog duplicate USE reply={after_relog:?}").into());
    }
    if custody(&fresh)? != awarded || quest_list(&mut fresh, &mut fresh_journal).await? != advanced
    {
        return Err("step-5: post-relog refusal changed durable inventory or quest state".into());
    }
    if quest_receipts(&mut database, clients, &chest.transition).await? != receipts_after {
        return Err(
            "step-5: duplicate USE or fresh admission changed durable quest receipts".into(),
        );
    }
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-6: final logout refused: {logout:?}").into());
    }
    Ok(format!("claim={} reward-and-quest-durable", chest.claim))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(detail)) => evidence(&format!(
            "e2e_all scenario=W15b.full status=PASS seconds={:.3} evidence={detail}",
            started.elapsed().as_secs_f64()
        )),
        result => {
            let gap = match result {
                Ok(Err(error)) => error.to_string(),
                _ => "step-timeout:180s".to_owned(),
            };
            evidence(&format!("W15B_GAP {gap}"));
            evidence(&format!(
                "e2e_all scenario=W15b.full status=FAIL seconds={:.3} evidence={}",
                started.elapsed().as_secs_f64(),
                gap.replace(char::is_whitespace, "_")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_regression_requires_the_chests_exact_reward() {
        let before = Custody(BTreeMap::from([((255, 3031, 0), 5)]));
        let unrelated = Custody(BTreeMap::from([((255, 3031, 0), 6)]));
        let reward = Custody(BTreeMap::from([((255, 3031, 0), 5), ((255, 3607, 0), 1)]));
        assert!(!reward_increased(&before, &unrelated, 3607, 1));
        assert!(!reward_increased(&before, &reward, 3607, 2));
        assert!(reward_increased(&before, &reward, 3607, 1));
        let lost = Custody(BTreeMap::from([((255, 3607, 0), 1)]));
        assert!(!reward_increased(&before, &lost, 3607, 1));
    }

    #[test]
    fn adjacency_is_same_floor_and_chebyshev_one() {
        let chest = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        assert!(adjacent(
            chest,
            TilePos {
                x: 9,
                y: 11,
                floor: -7
            }
        ));
        assert!(!adjacent(
            chest,
            TilePos {
                x: 8,
                y: 10,
                floor: -7
            }
        ));
        assert!(!adjacent(
            chest,
            TilePos {
                x: 10,
                y: 10,
                floor: -6
            }
        ));
    }
}

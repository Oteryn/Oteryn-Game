//! W15b — quest reward chests/claims, derived from the matrix, the Crystal reaudit, and
//! `gameplay_transport::bundle_chest` plus `interaction_chest_use`:
//!
//! 1. Admit the provisioned character and send `QuestLogQuery::List`; require the accepted reply
//!    and retain the quest list and native inventory snapshot.
//! 2. From the production bundle's served reward-claim bindings, choose a reachable chest whose
//!    claim advances a real quest, then send ordinary cardinal `STEP` commands until adjacent;
//!    require `Moved` replies and spatial deltas for every step.
//! 3. Send native `USE_INTENT` for that exact bundle placement; require `Committed`, a new reward
//!    item in the inventory state, and a changed quest-journal list proving start/advance.
//! 4. Send a second native `USE_INTENT` for the same chest; require `NothingToUse` (the durable
//!    once-claim refusal), with neither inventory nor quest state changed.
//! 5. Send `LOGOUT`, discard the connection, and freshly admit the same character. Re-query the
//!    journal and require both the reward custody and advanced quest state to equal step 3; walk
//!    back if needed and send `USE_INTENT` once more, requiring the same refusal and no delta.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Custody(BTreeMap<(u8, u32, u32), u64>);

fn custody(client: &DevClientSession) -> TestResult<Custody> {
    let inventory = client.inventory().ok_or("step-1: inventory snapshot missing")?;
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
            if placement.placement.quest_transition.is_none() {
                continue;
            }
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
            });
        }
    }
    chests.sort_by_key(|chest| (chest.at, chest.claim.clone()));
    Ok(chests)
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

fn select_route(world: &BundleWorld, from: TilePos) -> TestResult<(Chest, Vec<StepDirection>)> {
    for chest in quest_chests(world)? {
        if let Some(path) = route(world, from, |at| adjacent(at, chest.at)) {
            return Ok((chest, path));
        }
    }
    Err("step-2: production bundle has no reachable served quest-transition reward chest".into())
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
    let fresh = connect_character(clients, 0xb6, 1).await?;
    if fresh.join_snapshot().game_session_id == old {
        return Err("step-5: fresh admission reused the old game session".into());
    }
    Ok(fresh)
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
    let mut client = connect_character(clients, 0xb5, 1).await?;
    let before_quests = query_quest_journal(&mut client, false).await?;
    let before_items = custody(&client)?;

    let (chest, path) = select_route(world, tile(client.world_spatial().actor_position)?)?;
    walk(&mut client, world, &path)
        .await
        .map_err(|error| format!("step-2: walking to {} failed: {error}", chest.claim))?;
    let first = use_chest(&mut client, &chest).await?;
    if first != UseDisposition::Committed {
        return Err(format!("step-3: first USE reply={first:?} claim={}", chest.claim).into());
    }
    let awarded = custody(&client)?;
    if awarded == before_items {
        return Err(format!("step-3: USE committed but inventory delta missing claim={}", chest.claim).into());
    }
    let advanced = query_quest_journal(&mut client, false).await?;
    if advanced == before_quests {
        return Err(format!("step-3: USE committed but quest-list delta missing claim={}", chest.claim).into());
    }

    let second = use_chest(&mut client, &chest).await?;
    if second != UseDisposition::NothingToUse {
        return Err(format!("step-4: duplicate USE reply={second:?} claim={}", chest.claim).into());
    }
    if custody(&client)? != awarded || query_quest_journal(&mut client, false).await? != advanced {
        return Err("step-4: refused duplicate changed inventory or quest state".into());
    }

    let mut fresh = fresh_admission(&mut client, clients).await?;
    if custody(&fresh)? != awarded || query_quest_journal(&mut fresh, false).await? != advanced {
        return Err("step-5: reward custody or quest advance did not survive fresh admission".into());
    }
    let back = route(world, tile(fresh.world_spatial().actor_position)?, |at| {
        adjacent(at, chest.at)
    })
    .ok_or("step-5: no player route back to claimed chest")?;
    walk(&mut fresh, world, &back).await?;
    let after_relog = use_chest(&mut fresh, &chest).await?;
    if after_relog != UseDisposition::NothingToUse {
        return Err(format!("step-5: post-relog duplicate USE reply={after_relog:?}").into());
    }
    if custody(&fresh)? != awarded || query_quest_journal(&mut fresh, false).await? != advanced {
        return Err("step-5: post-relog refusal changed durable inventory or quest state".into());
    }
    Ok(format!("claim={} reward-and-quest-durable", chest.claim))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(detail)) => evidence(&format!(
            "e2e_all scenario=W15b.full status=PASS seconds={:.3} evidence={detail}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15b.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=W15b.full status=FAIL seconds={:.3} evidence=step-timeout:180s",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacency_is_same_floor_and_chebyshev_one() {
        let chest = TilePos { x: 10, y: 10, floor: -7 };
        assert!(adjacent(chest, TilePos { x: 9, y: 11, floor: -7 }));
        assert!(!adjacent(chest, TilePos { x: 8, y: 10, floor: -7 }));
        assert!(!adjacent(chest, TilePos { x: 10, y: 10, floor: -6 }));
    }
}

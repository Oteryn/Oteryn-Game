//! W15d — per-quest executable population, exercised through the production callback owner.
//!
//! Concrete player scenario (The Way of the Monk, Shrine of Guidance):
//! 1. Log in with the node-boot character, walk from its saved position to an adjacent reachable
//!    tile at (32454, 32292, 7), and identify the authored shrine from the served bundle.
//! 2. Send native USE for that shrine. Expect an accepted/committed reply plus quest-log state
//!    advancement; the callback requires level 6, questline 3, and guidance 0 and sets guidance 1.
//! 3. Send USE again with the server's current object revision. Expect a rejected reply and no
//!    quest-log advancement because this one-way per-quest callback is no longer executable.
//! 4. Log out, obtain a fresh Platform admission, query the quest log, and require the committed
//!    quest entry and its completion flag to equal the post-USE durable state.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const SHRINE: TilePos = TilePos {
    x: 32454,
    y: 32292,
    floor: -7,
};
const APPEARANCE: u16 = 50242;

struct Row {
    started: std::time::Instant,
    step: &'static str,
}

impl Row {
    fn fail(self, observed: impl std::fmt::Display) {
        evidence(&format!("W15D_FAIL step={} observed={observed}", self.step));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=FAIL seconds={:.3}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: "admission",
    };
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients)).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=PASS seconds={:.3}",
            row.started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => row.fail(error),
        Err(_) => row.fail("timeout waiting for server reply/delta"),
    }
}

async fn quest_list(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<Vec<journal::QuestListEntry>> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("missing quest-log snapshot")?
        .clone();
    let state = journal::QuestState::from_snapshot(client.selected_capabilities(), Some(&snapshot))
        .map_err(|error| format!("quest snapshot: {error:?}"))?;
    let command = state
        .command(&journal::QuestLogQuery::List)
        .map_err(|error| format!("quest list command: {error:?}"))?;
    let result = journal::send(client.session_mut(), &command).await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("quest list reply status={:?}", result.status).into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    let log = journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("quest list delta: {error:?}"))?;
    let journal::QuestLogView::List(entries) = log.view.ok_or("quest list delta has no view")?
    else {
        return Err("quest list delta is not a list".into());
    };
    Ok(entries)
}

fn shrine_target(world: &BundleWorld) -> TestResult<WorldObjectTarget> {
    let tile = world
        .base()
        .tile(SHRINE.x, SHRINE.y, SHRINE.floor)
        .ok_or("authored guidance shrine tile absent")?;
    let ordinal = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(SHRINE, ordinal, *id)
                .filter(|facts| facts.appearance_id == APPEARANCE)
                .map(|_| ordinal)
        })
        .ok_or("authored guidance shrine appearance absent")?;
    let key = crate::map::view::placement_key(SHRINE, ordinal).ok_or("shrine placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: world.facts().object_revision(key),
    })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xd4).await?;
    row.step = "walk-to-guidance-shrine";
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == SHRINE.floor
            && at.x.abs_diff(SHRINE.x).max(at.y.abs_diff(SHRINE.y)) <= 1
            && at != SHRINE
    })
    .ok_or("no legitimate walking route to guidance shrine")?;
    walk(&mut client, world, &path).await?;

    row.step = "first-use-commit";
    let target = shrine_target(world)?;
    let before = quest_list(&mut client).await?;
    let first = client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if first.status != CommandStatus::Accepted || first.disposition != UseDisposition::Committed {
        return Err(format!(
            "USE reply status={:?} disposition={:?}; server needs a player-reachable prerequisite path for questline=3,guidance=0,level>=6",
            first.status, first.disposition
        )
        .into());
    }
    let after = quest_list(&mut client).await?;
    if after == before {
        return Err("committed USE produced no quest-log delta".into());
    }

    row.step = "repeat-use-refusal";
    let second = client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if second.status != CommandStatus::Rejected || second.disposition != UseDisposition::Rejected {
        return Err(format!(
            "repeat USE status={:?} disposition={:?}",
            second.status, second.disposition
        )
        .into());
    }
    if quest_list(&mut client).await? != after {
        return Err("refused repeat USE changed quest-log state".into());
    }

    row.step = "logout-fresh-admission";
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused after callback".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xd5).await?;
    if quest_list(&mut fresh).await? != after {
        return Err("fresh admission lost committed quest-log state".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("fresh admission logout refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_guidance_shrine_has_no_invented_placement() -> TestResult {
        let world = row_helper_world()?;
        assert!(shrine_target(&world).is_err());
        Ok(())
    }
}

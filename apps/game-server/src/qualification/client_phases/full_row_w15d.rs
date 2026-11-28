//! W15d — the node-boot Shrine of Guidance control.
//!
//! The full Paradox Tower and Way of the Monk journeys are exercised in dispatch's
//! `w15dq_quest_journey_readmission` PostgreSQL cases. This walk-in control verifies
//! one shrine mission against the node's real Platform admissions.
//!
//! Concrete player scenario (The Way of the Monk, Shrine of Guidance):
//! 1. Log in with the node-boot character, walk from its saved position to an adjacent reachable
//!    tile at (32454, 32292, 7), and identify the authored shrine from the served bundle.
//! 2. Send native USE for that shrine. Expect an accepted/committed reply plus quest-log state
//!    advancement; the callback requires level 6, questline 3, and guidance 0 and sets guidance 1.
//! 3. Send USE again with the server's current object revision. Expect a rejected reply and no
//!    quest-log advancement because this one-way per-quest callback is no longer executable.
//! 4. Log out, obtain a fresh Platform admission, and require the same committed quest line.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_gameplay_quests::quest::{loader, log, monk_shrines};
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
        let observed = observed.to_string();
        evidence(&format!("W15D_GAP step={} observed={observed}", self.step));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=FAIL seconds={:.3} evidence=step-{}:{}",
            self.started.elapsed().as_secs_f64(),
            self.step,
            observed.replace(char::is_whitespace, "_")
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: "admission",
    };
    let result = verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=PASS seconds={:.3}",
            row.started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => row.fail(error),
        Err(_) => row.fail("timeout waiting for server reply/delta"),
    }
}

async fn quest_line(
    client: &mut oteryn_dev_client::DevClientSession,
    quest_index: u32,
) -> TestResult<journal::QuestLine> {
    let command = journal::encode_quest_log_query(&journal::QuestLogQuery::Quest { quest_index })
        .map_err(|error| format!("quest line command: {error:?}"))?;
    let result = journal::send(client.session_mut(), &command).await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("quest line reply status={:?}", result.status).into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    let log = journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("quest line delta: {error:?}"))?;
    match log.view {
        Some(journal::QuestLogView::Line(line)) if line.quest_index == quest_index => Ok(line),
        other => Err(format!("expected Monk quest line {quest_index}, got {other:?}").into()),
    }
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

fn shrine_revision(base: u64, published: Option<u64>) -> TestResult<u64> {
    published.filter(|revision| *revision > base).ok_or_else(|| format!("repeat-use needs a newer published shrine revision, base={base} observed={published:?}").into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let revision = pin["content_revision"]
        .as_str()
        .ok_or("World content revision")?;
    let lowered = loader::load_embedded_quest_state(revision)
        .map_err(|error| format!("quest state catalogue: {error:?}"))?;
    let journal = log::loader::load_embedded_quest_log(&lowered)?;
    let quest_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == monk_shrines::QUEST)
            .ok_or("Monk quest index")?,
    )?;
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
    let guidance_done = |line: &journal::QuestLine| {
        line.missions
            .iter()
            .any(|mission| mission.name == "Shrine of Guidance" && mission.done)
    };
    let before = quest_line(&mut client, quest_index).await?;
    if before.completed || guidance_done(&before) {
        return Err(
            "Guidance control requires an unfinished pilgrimage with an unused shrine".into(),
        );
    }
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
    let after = quest_line(&mut client, quest_index).await?;
    if after.completed || !guidance_done(&after) {
        return Err(
            "committed USE did not complete the Guidance mission within the pilgrimage".into(),
        );
    }

    row.step = "repeat-use-refusal";
    let observed_revision = client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == target.placement)
        .map(|entry| entry.revision);
    let revision = shrine_revision(target.expected_revision, observed_revision)?;
    let second = client
        .session_mut()
        .use_object(&target.placement, revision)
        .await?;
    if second.status != CommandStatus::Rejected || second.disposition != UseDisposition::Rejected {
        return Err(format!(
            "repeat USE status={:?} disposition={:?}",
            second.status, second.disposition
        )
        .into());
    }
    // Two queries have used this connection's one-second journal budget.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if quest_line(&mut client, quest_index).await? != after {
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
    if quest_line(&mut fresh, quest_index).await? != after {
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
    fn journey_regression_replays_with_the_published_shrine_revision() {
        assert!(shrine_revision(2, None).is_err());
        assert!(shrine_revision(2, Some(2)).is_err());
        assert_eq!(shrine_revision(2, Some(3)).ok(), Some(3));
    }
}

//! W15a full-row player scenario, derived from the W15a audit/matrix row and the
//! quest-log query/projection owner named there.
//!
//! 1. Admit the provisioned character and send `QuestLogQuery::List`; require an
//!    accepted command and a real domain-16 List delta (which may initially be empty).
//! 2. Walk through the compiled world to Mr Morris, then send the ordinary player
//!    NPC commands `hi`, `amulet`, and `yes`; require accepted replies and the
//!    conversation delta confirming that the first real mission started.
//! 3. Send `QuestLogQuery::List` and `QuestLogQuery::Quest` for the started quest;
//!    require matching domain deltas and retain its mission projection.
//! 4. Send `QuestLogQuery::Track`; require an accepted command and a delta whose
//!    tracked line is that concrete quest.
//! 5. Exercise the refusal path with a protocol-encoded query for an unlisted quest;
//!    require `Rejected` and an empty command reply rather than fabricated state.
//! 6. Send the ordinary `herbs` and `yes` NPC commands, then query list and line;
//!    require a newly listed mission or changed line, and track that projection.
//! 7. Logout, make a fresh admission, query list and the same line, and require the
//!    durable mission state observed in step 6 to remain. Tracking is session UI
//!    continuity, while quest progress is the durable state re-checked here.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as runtime;
use oteryn_protocol_oteryn::quest_log as wire;

const NPC: &str = "oteryn:npc.mr_morris";

struct Row {
    started: std::time::Instant,
    step: usize,
}

impl Row {
    fn new() -> Self {
        Self { started: std::time::Instant::now(), step: 0 }
    }

    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) {
        self.step = step;
        evidence(&format!("W15A_CHECK step={step} status=PASS observed={observed:?}"));
    }

    fn finish(self, result: TestResult) {
        let (status, detail) = match result {
            Ok(()) if self.step == 7 => ("PASS", None),
            Ok(()) => ("FAIL", Some(format!("step-{}:journey-ended-before-lifecycle", self.step + 1))),
            Err(error) => ("FAIL", Some(format!("step-{}:{error}", self.step + 1).replace(' ', "_"))),
        };
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15a.full status={status} seconds={:.3}{}",
            self.started.elapsed().as_secs_f64(),
            detail.map_or_else(String::new, |detail| format!(" evidence={detail}")),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("deadline:timeout-waiting-for-server-frame".into()),
    });
}

async fn send(
    client: &mut DevClientSession,
    query: &wire::QuestLogQuery,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    let payload = wire::encode_quest_log_query(query)
        .map_err(|error| format!("quest-query-encode:{error:?}"))?;
    Ok(client.session_mut().gated_command(
        runtime::CAPABILITY_QUEST_LOG_V1,
        runtime::COMMAND_TYPE_QUEST_LOG_QUERY,
        &payload,
    ).await?)
}

async fn query(
    client: &mut DevClientSession,
    query: &wire::QuestLogQuery,
) -> TestResult<wire::QuestLog> {
    let reply = send(client, query).await?;
    if reply.status != CommandStatus::Accepted || !reply.payload.is_empty() {
        return Err(format!("query={query:?}:reply={reply:?}").into());
    }
    let delta = client.session_mut()
        .read_gated_delta(runtime::STATE_DOMAIN_QUEST_LOG, 1).await?;
    wire::decode_quest_log(&delta.payload)
        .map_err(|error| format!("query={query:?}:delta-decode={error:?}").into())
}

async fn settle_rate(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1050);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    client.take_events();
    Ok(())
}

fn listed(log: &wire::QuestLog) -> TestResult<Vec<wire::QuestListEntry>> {
    match &log.view {
        Some(wire::QuestLogView::List(entries)) if !entries.is_empty() => Ok(entries.clone()),
        other => Err(format!("list-delta={other:?}").into()),
    }
}

fn line(log: &wire::QuestLog, index: u32) -> TestResult<wire::QuestLine> {
    match &log.view {
        Some(wire::QuestLogView::Line(line)) if line.quest_index == index => Ok(line.clone()),
        other => Err(format!("quest={index}:line-delta={other:?}").into()),
    }
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as npc;
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    }).map_err(|error| format!("npc={NPC}:text={text}:encode={error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client.session_mut().gated_command(
            npc::CAPABILITY_NPC_SERVICE_V1,
            npc::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        ).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("npc={NPC}:text={text}:reply={reply:?}").into());
        }
        let disposition = npc::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("npc={NPC}:text={text}:result={error:?}"))?;
        if disposition == npc::NpcIntentDisposition::RateLimited
            && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(50)).await?;
            continue;
        }
        if disposition != npc::NpcIntentDisposition::Ok {
            return Err(format!("npc={NPC}:text={text}:disposition={disposition:?}").into());
        }
        break reply;
    };
    loop {
        let delta = client.session_mut()
            .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1).await?;
        if delta.server_sequence <= reply.server_sequence { continue; }
        let conversation = npc::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("npc={NPC}:text={text}:conversation={error:?}"))?;
        if conversation.open.map(|(open, _)| open) == Some(actor) {
            return Ok(conversation.lines);
        }
        return Err(format!("npc={NPC}:text={text}:unexpected-conversation={conversation:?}").into());
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0xa1).await?;

    let initial = query(&mut client, &wire::QuestLogQuery::List).await?;
    let initial_list = match &initial.view {
        Some(wire::QuestLogView::List(entries)) => entries.clone(),
        other => return Err(format!("initial-list-delta={other:?}").into()),
    };
    row.pass(1, &initial_list);

    let npc = metadata.npcs.npcs.iter().find(|npc| npc.key == NPC)
        .ok_or_else(|| format!("npc-command-not-sent:content:{NPC}"))?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|place| place.floor == at.floor
            && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3)
    }).ok_or_else(|| format!("npc-command-not-sent:no-player-walking-route-to:{NPC}"))?;
    walk(&mut client, world, &path).await?;
    let actor = client.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Npc && npc.placements.iter().any(|place| {
            entry.position.x == i32::from(place.x) && entry.position.y == i32::from(place.y)
                && entry.position.floor == -i16::from(place.floor)
        })
    })).map(|entry| entry.entity).ok_or_else(|| format!("npc-command-not-sent:actor-not-visible:{NPC}"))?;
    talk(&mut client, actor, "hi").await?;
    talk(&mut client, actor, "amulet").await?;
    let accepted = talk(&mut client, actor, "yes").await?;
    if !accepted.iter().any(|text| text.contains("Dormovo")) {
        return Err(format!("npc={NPC}:yes:observed={accepted:?}:missing-mission-confirmation").into());
    }
    row.pass(2, &accepted);

    settle_rate(&mut client).await?;
    let started_list = listed(&query(&mut client, &wire::QuestLogQuery::List).await?)?;
    let started_index = started_list.iter().find(|entry| !initial_list.contains(entry))
        .or_else(|| started_list.first()).ok_or("mission-start-produced-empty-list")?.quest_index;
    settle_rate(&mut client).await?;
    let before = line(&query(&mut client, &wire::QuestLogQuery::Quest { quest_index: started_index }).await?, started_index)?;
    row.pass(3, &before);
    settle_rate(&mut client).await?;
    let tracked = query(&mut client, &wire::QuestLogQuery::Track { quest_indexes: vec![started_index] }).await?;
    if tracked.tracked.iter().all(|entry| entry.quest_index != started_index) {
        return Err(format!("track={started_index}:delta={tracked:?}").into());
    }
    row.pass(4, started_index);
    settle_rate(&mut client).await?;
    let invalid = started_list.iter().map(|entry| entry.quest_index).max().unwrap_or(0).saturating_add(1);
    let refusal = send(&mut client, &wire::QuestLogQuery::Quest { quest_index: invalid }).await?;
    if refusal.status != CommandStatus::Rejected || !refusal.payload.is_empty() {
        return Err(format!("invalid-quest={invalid}:reply={refusal:?}").into());
    }
    row.pass(5, &refusal);

    talk(&mut client, actor, "herbs").await?;
    let progressed = talk(&mut client, actor, "yes").await?;
    if !progressed.iter().any(|text| text.contains("Dawnfire")) {
        return Err(format!("npc={NPC}:herbs-yes:observed={progressed:?}:missing-mission-confirmation").into());
    }
    settle_rate(&mut client).await?;
    let changed_list = listed(&query(&mut client, &wire::QuestLogQuery::List).await?)?;
    let changed_index = changed_list.iter().find(|entry| !started_list.contains(entry))
        .map(|entry| entry.quest_index).unwrap_or(started_index);
    settle_rate(&mut client).await?;
    let changed = line(&query(&mut client, &wire::QuestLogQuery::Quest { quest_index: changed_index }).await?, changed_index)?;
    if changed_index == started_index && changed == before {
        return Err(format!("mission-command-accepted-but-missing-delta:before={before:?}:after={changed:?}").into());
    }
    settle_rate(&mut client).await?;
    let tracked = query(&mut client, &wire::QuestLogQuery::Track { quest_indexes: vec![changed_index] }).await?;
    if tracked.tracked.iter().all(|entry| entry.quest_index != changed_index) {
        return Err(format!("changed-track={changed_index}:delta={tracked:?}").into());
    }
    row.pass(6, &changed);

    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("logout:server-refused".into());
    }
    drop(client);
    let mut client = connect_world(clients, 0xa2).await?;
    let durable_list = listed(&query(&mut client, &wire::QuestLogQuery::List).await?)?;
    if durable_list.iter().all(|entry| entry.quest_index != changed_index) {
        return Err(format!("fresh-admission:list-missing-quest={changed_index}:observed={durable_list:?}").into());
    }
    settle_rate(&mut client).await?;
    let durable = line(&query(&mut client, &wire::QuestLogQuery::Quest { quest_index: changed_index }).await?, changed_index)?;
    if durable != changed {
        return Err(format!("fresh-admission:durable-mismatch:expected={changed:?}:observed={durable:?}").into());
    }
    row.pass(7, &durable);
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("final-logout:server-refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_requires_a_real_nonempty_list_view() {
        assert!(listed(&wire::QuestLog::default()).is_err());
        let log = wire::QuestLog {
            view: Some(wire::QuestLogView::List(vec![wire::QuestListEntry { quest_index: 7, completed: false }])),
            tracked: Vec::new(),
        };
        assert!(listed(&log).is_ok_and(|entries| entries[0].quest_index == 7));
    }
}

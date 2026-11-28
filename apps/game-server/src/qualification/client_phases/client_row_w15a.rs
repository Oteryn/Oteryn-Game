//! W15a client replay: after a real Sam hand-in, the player queries the quest journal (`List`, the first
//! listed `Quest` line and `Track`) and the raw `QuestLog` delta payloads the server sent are
//! written byte for byte, together with the snapshot the join delivered. The native quest log
//! reducer consumes these live frames in this row and the capture is also available to
//! `tests/e2e_w15a.rs`. The query for an unlisted quest is refused with an empty reply.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)
        .map_err(|error| format!("W15A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"))?;
    Ok(())
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn line_text(prefix: &str, line: &journal::QuestLine) -> String {
    let mut text = format!(
        "{prefix}={}\t{}\t{}\n",
        line.quest_index,
        u8::from(line.completed),
        line.missions.len()
    );
    for mission in &line.missions {
        text.push_str(&format!(
            "{prefix}.mission={}\t{}\t{}\t{}\n",
            mission.mission_index,
            u8::from(mission.done),
            escape(&mission.name),
            escape(&mission.text)
        ));
    }
    text
}

async fn settle_rate(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1050);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    client.take_events();
    Ok(())
}

struct RawDelta {
    server_sequence: u64,
    base_revision: u64,
    new_revision: u64,
    payload: Vec<u8>,
}

impl RawDelta {
    fn decode(&self) -> TestResult<oteryn_dev_client::AppliedDelta<journal::QuestLog>> {
        let value = journal::decode_quest_log(&self.payload)
            .map_err(|error| format!("quest delta decode: {error:?}"))?;
        Ok(oteryn_dev_client::AppliedDelta {
            server_sequence: self.server_sequence,
            base_revision: self.base_revision,
            new_revision: self.new_revision,
            value,
        })
    }
}

/// Sends one query and returns the raw delta (accepted, empty reply) the server published.
async fn query(
    client: &mut DevClientSession,
    query: &journal::QuestLogQuery,
) -> TestResult<RawDelta> {
    settle_rate(client).await?;
    let payload = journal::encode_quest_log_query(query)
        .map_err(|error| format!("quest query encode: {error:?}"))?;
    let reply = journal::send(client.session_mut(), &payload).await?;
    if reply.status != CommandStatus::Accepted || !reply.payload.is_empty() {
        return Err(format!("query={query:?} reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    if delta.server_sequence <= reply.server_sequence {
        return Err(format!("stale quest delta behind query {query:?}").into());
    }
    Ok(RawDelta {
        server_sequence: delta.server_sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        payload: delta.payload,
    })
}

const SAM: TilePos = TilePos {
    x: 32361,
    y: 32198,
    floor: -7,
};
const OLD_BACKPACK: u32 = 3244;
const SAM_MISSION_TEXT: &str = "Sam sends you to see Kroox in Kazordoon to get a special dwarven armor. Just tell him, his old buddy Sam is sending you.";

async fn logout(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("logout refused: {outcome:?}").into());
    }
    Ok(())
}

fn backpack_count(client: &DevClientSession) -> TestResult<u64> {
    let items = provisioning::item_index()?;
    let definition = provisioning::item_reference(&items, OLD_BACKPACK)?;
    let inventory = client.inventory().ok_or("Sam setup requires inventory")?;
    Ok(inventory
        .entries
        .iter()
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .chain(inventory.main_backpack.iter())
        .filter(|item| item.item_definition_ref == definition)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

async fn talk_to_sam(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as npc;
    settle_rate(client).await?;
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("Sam talk {text:?} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            npc::CAPABILITY_NPC_SERVICE_V1,
            npc::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || npc::decode_npc_intent_result(&reply.payload).ok() != Some(npc::NpcIntentDisposition::Ok)
    {
        return Err(format!("Sam talk {text:?} refused: {reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("Sam talk {text:?} conversation: {error:?}"))?;
    if delta.server_sequence <= reply.server_sequence
        || conversation.open.map(|(speaker, _)| speaker) != Some(actor)
    {
        return Err(format!("Sam talk {text:?} wrong conversation: {conversation:?}").into());
    }
    Ok(conversation.lines)
}

async fn prepare_journal(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    let mut client = connect_character(clients, 0x86, 0).await?;
    if backpack_count(&client)? != 0 {
        return Err("Sam setup character arrived with the old backpack".into());
    }
    let before = query(&mut client, &journal::QuestLogQuery::List)
        .await?
        .decode()?;
    if before.value.view != Some(journal::QuestLogView::List(Vec::new())) {
        return Err(format!(
            "Sam setup character arrived with quest history: {:?}",
            before.value
        )
        .into());
    }
    character_prep::grant_item(&client, OLD_BACKPACK, 1).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0x86, 0).await?;
    if backpack_count(&client)? != 1 {
        return Err("Sam setup character did not receive exactly one old backpack".into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    if !metadata.npcs.npcs.iter().any(|npc| {
        npc.key == "oteryn:npc.sam"
            && npc.placements.iter().any(|placement| {
                (placement.x, placement.y, placement.floor) == (SAM.x, SAM.y, SAM.floor)
            })
    }) {
        return Err("compiled world lacks the authored Sam placement".into());
    }
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut at = tile(client.world_spatial().actor_position)?;
    let path = route(world, at, |cell| {
        cell != SAM
            && cell.floor == SAM.floor
            && cell.x.abs_diff(SAM.x).max(cell.y.abs_diff(SAM.y)) <= 2
    })
    .ok_or("Sam setup has no walking route to the authored placement")?;
    for direction in path {
        let expected = destination(world, at, direction).ok_or("Sam setup route lost a step")?;
        let step = client.step(direction).await?;
        let observed = tile(client.world_spatial().actor_position)?;
        if (step.status, step.disposition, observed)
            != (CommandStatus::Accepted, StepDisposition::Moved, expected)
        {
            return Err(
                format!("Sam setup step {at:?} {direction:?}: {step:?} at {observed:?}").into(),
            );
        }
        at = observed;
        tokio::time::sleep(Duration::from_millis(u64::from(step.step_wait_ms))).await;
    }
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && entry
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok("Sam"))
                    && tile(entry.position).ok() == Some(SAM)
            })
        })
        .map(|entry| entry.entity)
        .ok_or("Sam is not visible at the authored placement")?;
    talk_to_sam(&mut client, actor, "hi").await?;
    let offer = talk_to_sam(&mut client, actor, "old backpack").await?;
    if !offer.iter().any(|text| text.contains("my old backpack")) {
        return Err(format!("Sam did not offer the backpack hand-in: {offer:?}").into());
    }
    let confirmation = talk_to_sam(&mut client, actor, "yes").await?;
    if !confirmation
        .iter()
        .any(|text| text.contains("Thank you for your help."))
    {
        return Err(format!("Sam did not confirm the backpack hand-in: {confirmation:?}").into());
    }
    let hand_in_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);

    let client = connect_character(clients, 0x86, 0).await?;
    if client.join_snapshot().game_session_id == hand_in_session || backpack_count(&client)? != 0 {
        return Err("fresh admission did not prove Sam durably consumed the old backpack".into());
    }
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let character = character_prep::dedicated_character(clients, "NODE_BOOT_W15A_ACCOUNT").await?;
    let clients = character.clients(clients);
    let mut client = prepare_journal(&clients).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let mut state =
        journal::QuestState::from_snapshot(client.selected_capabilities(), Some(&snapshot))
            .map_err(|error| format!("native quest snapshot replay: {error:?}"))?;
    if state.revision != Some(snapshot.revision) {
        return Err(format!("native quest snapshot revision mismatch: {state:?}").into());
    }
    let listed = query(&mut client, &journal::QuestLogQuery::List).await?;
    let listed_delta = listed.decode()?;
    let list = match listed_delta.value.view.as_ref() {
        Some(journal::QuestLogView::List(entries)) if !entries.is_empty() => entries,
        other => return Err(format!("step=list view={other:?}").into()),
    };
    state
        .apply(&listed_delta)
        .map_err(|error| format!("native quest list replay: {error:?}"))?;
    if state.list != *list
        || state.revision != Some(listed.new_revision)
        || list.len() != 1
        || list[0].completed
    {
        return Err(format!("durable Sam quest list mismatch: {state:?}").into());
    }
    let index = list[0].quest_index;
    let line_query = journal::QuestLogQuery::Quest { quest_index: index };
    state
        .command(&line_query)
        .map_err(|error| format!("native quest line command: {error:?}"))?;
    let lined = query(&mut client, &line_query).await?;
    let lined_delta = lined.decode()?;
    let line = match lined_delta.value.view.as_ref() {
        Some(journal::QuestLogView::Line(line)) if line.quest_index == index => line,
        other => return Err(format!("step=line index={index} view={other:?}").into()),
    };
    state
        .apply(&lined_delta)
        .map_err(|error| format!("native quest line replay: {error:?}"))?;
    if state.log.view != lined_delta.value.view
        || state.list[0].completed != line.completed
        || line.completed
        || !matches!(line.missions.as_slice(), [mission]
            if mission.name == "Dwarven Armor Quest" && !mission.done && mission.text == SAM_MISSION_TEXT)
    {
        return Err(format!("durable Sam quest line mismatch: {state:?}").into());
    }
    let tracked = query(
        &mut client,
        &journal::QuestLogQuery::Track {
            quest_indexes: vec![index],
        },
    )
    .await?;
    let tracked_delta = tracked.decode()?;
    let track = &tracked_delta.value;
    let tracked_line = track
        .tracked
        .iter()
        .find(|entry| entry.quest_index == index)
        .ok_or_else(|| format!("step=track index={index} tracked={track:?}"))?;
    state
        .apply(&tracked_delta)
        .map_err(|error| format!("native quest track replay: {error:?}"))?;
    if state.log.tracked.as_slice() != [line.clone()]
        || state.revision != Some(tracked.new_revision)
    {
        return Err(format!("native tracked quest mismatch: {state:?}").into());
    }
    // An unlisted quest is refused with an empty reply and no delta.
    settle_rate(&mut client).await?;
    let invalid = list
        .iter()
        .map(|entry| entry.quest_index)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let invalid_query = journal::QuestLogQuery::Quest {
        quest_index: invalid,
    };
    if state.command(&invalid_query) != Err(journal::QuestRefusal::UnknownQuest) {
        return Err("native quest log admitted an unlisted quest query".into());
    }
    let payload = journal::encode_quest_log_query(&invalid_query)
        .map_err(|error| format!("step=refusal encode={error:?}"))?;
    let refused = client
        .session_mut()
        .gated_command(
            journal::CAPABILITY_QUEST_LOG_V1,
            journal::COMMAND_TYPE_QUEST_LOG_QUERY,
            &payload,
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("step=refusal invalid={invalid} observed={refused:?}").into());
    }
    let before = state.clone();
    state
        .record_result(refused.status, &refused.payload)
        .map_err(|error| format!("native quest refusal replay: {error:?}"))?;
    if state.refusal != Some(journal::QuestRefusal::Rejected)
        || (&state.list, &state.log, state.revision) != (&before.list, &before.log, before.revision)
        || state.apply(&listed_delta) != Err(journal::QuestRefusal::RevisionMismatch)
        || (&state.list, &state.log, state.revision) != (&before.list, &before.log, before.revision)
    {
        return Err(format!(
            "native refusal or stale delta changed the quest projection: {state:?}"
        )
        .into());
    }
    let mut expected = format!(
        "snapshot_revision={}\nlist_base={}\nlist_new={}\nlist_sequence={}\nline_base={}\nline_new={}\nline_sequence={}\ntrack_base={}\ntrack_new={}\ntrack_sequence={}\n",
        snapshot.revision,
        listed.base_revision,
        listed.new_revision,
        listed.server_sequence,
        lined.base_revision,
        lined.new_revision,
        lined.server_sequence,
        tracked.base_revision,
        tracked.new_revision,
        tracked.server_sequence
    );
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    expected.push_str(&format!("session={session}\n"));
    for entry in list {
        expected.push_str(&format!(
            "list={}\t{}\n",
            entry.quest_index,
            u8::from(entry.completed)
        ));
    }
    expected.push_str(&line_text("line", line));
    expected.push_str(&line_text("tracked", tracked_line));
    expected.push_str(&format!(
        "invalid={invalid}\nrefused={:?}\n",
        refused.status
    ));
    capture("w15a-snapshot.bin", &snapshot.payload)?;
    capture("w15a-list.bin", &listed.payload)?;
    capture("w15a-line.bin", &lined.payload)?;
    capture("w15a-track.bin", &tracked.payload)?;
    capture("w15a-expected.txt", expected.as_bytes())?;
    logout(&mut client).await?;
    wait_character_released(&clients, 0).await?;
    Ok(format!(
        "session={session} quests={} line={index} missions={}",
        list.len(),
        line.missions.len()
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15a.client status=FAIL evidence=timeout"),
    }
}

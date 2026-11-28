//! G29: live quest tracker and filters, achievements, titles, defence and blessings.
use super::*;
use oteryn_dev_client::{DevClientSession, client_ui::quest_log as journal};
use oteryn_protocol_oteryn::{
    character_info::{CAPABILITY_CHARACTER_INFO_V1, COMMAND_TYPE_CHARACTER_INFO, CharacterTitle},
    cyclopedia_character::CharacterTab,
    npc_service as npc,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G29.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

use oteryn_dev_client::client_runtime::raw_feature;
use oteryn_dev_client::client_ui::cyclopedia_character as cyc;

async fn panel_call(
    client: &mut DevClientSession,
    panel: &mut cyc::CyclopediaCharacterPanel,
    intent: cyc::Intent,
) -> TestResult<bool> {
    let command = cyc::request(panel, intent).ok_or_else(|| {
        format!(
            "step=cyclopedia-request observed=local-refusal:{:?}",
            panel.feedback
        )
    })?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    Ok(reply.status == CommandStatus::Accepted && cyc::apply_result(panel, &reply))
}

fn require_cyclopedia(client: &DevClientSession) -> TestResult {
    if !client.selected_capabilities().contains(&199) {
        return Err("step=admit observed=capability-199-not-selected".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut entry = connect_world(clients, 0xd9).await?;
    let entry_session = entry.join_snapshot().game_session_id;
    let title = CharacterTitle::Cyclopscamper.id();
    character_prep::declare_seeded(
        "G29.full",
        "sorcerer-level-1-progression",
        "initialize progression for the baseline title read",
    );
    // This existing build profile initializes progression without awarding XP or items.
    request_fixture_control(
        uuid_text(entry_session.as_bytes()),
        "--profile",
        "f5-c18a".into(),
    )
    .await?;
    let baseline_level = full_e2ef7::player_level(&mut entry).await?;
    if baseline_level != 1 {
        return Err(format!("step=title-precondition observed=level-{baseline_level}").into());
    }
    let (before, _) = title_state(&mut entry).await?;
    if before.contains(&title) {
        return Err("step=title-precondition observed=already-earned".into());
    }
    character_prep::declare_seeded(
        "G29.full",
        "sorcerer-level-100",
        "leveling exceeds the row budget",
    );
    request_fixture_control(
        uuid_text(entry_session.as_bytes()),
        "--profile",
        "sorcerer".into(),
    )
    .await?;
    logout(&mut entry).await?;
    drop(entry);

    let mut client = connect_world(clients, 0xd9).await?;
    if client.join_snapshot().game_session_id == entry_session {
        return Err("step=seed observed=reused-session".into());
    }
    let level = full_e2ef7::player_level(&mut client).await?;
    if level != 100 {
        return Err(format!("step=seed observed=level-{level}").into());
    }
    let (earned, selected) = title_state(&mut client).await?;
    if !earned.contains(&title) || selected == title {
        return Err(format!("step=title-initial observed={earned:?}/{selected}").into());
    }
    let reply = title_command(&mut client, &[3, title]).await?;
    if reply.len() != 9 || reply.first() != Some(&3) || reply[1..] == [0; 8] {
        return Err("step=title-select observed=wrong-revision-reply".into());
    }
    if title_state(&mut client).await? != (earned.clone(), title) {
        return Err("step=title-select observed=wrong-title-state".into());
    }
    require_cyclopedia(&client)?;
    let mut panel = cyc::CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    if !panel_call(&mut client, &mut panel, cyc::Intent::Achievements).await?
        || !panel
            .achievement_profile
            .as_ref()
            .is_some_and(|(id, name)| *id == clients.characters[1] && !name.is_empty())
    {
        return Err("step=achievements observed=reply-not-applied".into());
    }
    if !panel_call(&mut client, &mut panel, cyc::Intent::Protection).await? {
        return Err("step=defence-stats observed=reply-not-applied".into());
    }
    let forecast = panel
        .protection
        .forecast
        .ok_or("step=protection observed=no-forecast")?;
    let level_number = u64::from(level);
    let expected_xp =
        50 * (level_number.pow(3) + 17 * level_number - 6 * level_number.pow(2) - 12) / 3;
    if forecast.level != level
        || forecast.experience != expected_xp
        || forecast.regular_blessings != 0
        || forecast.twist
    {
        return Err(format!("step=protection observed={forecast:?}").into());
    }
    if !panel_call(
        &mut client,
        &mut panel,
        cyc::Intent::SelectTab(CharacterTab::Defence),
    )
    .await?
    {
        return Err("step=defence observed=missing-projection".into());
    }
    let rows = cyc::combat_rows(&panel).len();
    if rows
        != oteryn_protocol_oteryn::cyclopedia_character::combat::CombatStat::for_tab(
            CharacterTab::Defence,
        )
        .len()
        || rows == 0
    {
        return Err(format!("step=defence observed=rows-{rows}").into());
    }
    let command =
        cyc::request_blessings(&mut panel).ok_or("step=blessings observed=local-refusal")?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    if reply.status != CommandStatus::Accepted
        || !cyc::apply_result(&mut panel, &reply)
        || panel.blessings.snapshot.is_none()
    {
        return Err("step=blessings observed=no-store-summary".into());
    }
    let owned = panel
        .blessings
        .snapshot
        .as_ref()
        .map_or(0, |store| store.blessings.len());
    if owned != 0 {
        return Err(format!("step=blessings expected=unblessed observed={owned}").into());
    }
    let before = quest_list(&mut client).await?;
    start_inquisition(clients, &mut client).await?;
    let after = quest_list(&mut client).await?;
    let added: Vec<_> = after
        .iter()
        .filter(|entry| {
            !before
                .iter()
                .any(|prior| prior.quest_index == entry.quest_index)
        })
        .collect();
    let [quest] = added.as_slice() else {
        return Err(format!("step=quest-start observed={added:?}").into());
    };
    let quest_index = quest.quest_index;
    let log = quest_query(
        &mut client,
        &journal::QuestLogQuery::Track {
            quest_indexes: vec![quest_index],
        },
    )
    .await?;
    let [line] = log.tracked.as_slice() else {
        return Err("step=quest-track observed=wrong-line-count".into());
    };
    if quest.completed
        || line.quest_index != quest_index
        || line.completed
        || !line
            .missions
            .iter()
            .any(|mission| mission.name.contains("Interrogation") && !mission.done)
    {
        return Err(format!("step=quest-track observed={line:?}").into());
    }
    let line = line.clone();
    let mut tracker = journal::QuestPanel::default();
    tracker.state.log = log;
    for filter in [
        journal::TrackerFilter::All,
        journal::TrackerFilter::Active,
        journal::TrackerFilter::Completed,
    ] {
        tracker.tracker.filter = filter;
        let expected = if filter == journal::TrackerFilter::Completed {
            &[][..]
        } else {
            std::slice::from_ref(&line)
        };
        if journal::tracker_lines(&tracker).into_iter().ne(expected) {
            return Err(format!("step=quest-filter filter={filter:?} observed=wrong-lines").into());
        }
    }

    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xd9).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("step=relog observed=reused-session".into());
    }
    if title_state(&mut fresh).await? != (earned, title) {
        return Err("step=relog observed=title-selection-changed".into());
    }
    let persisted = quest_query(&mut fresh, &journal::QuestLogQuery::Quest { quest_index }).await?;
    if persisted.view != Some(journal::QuestLogView::Line(line)) {
        return Err("step=relog observed=quest-progress-changed".into());
    }
    logout(&mut fresh).await?;
    Ok(format!(
        "quest={quest_index};selected-title={title};defence-rows={rows};blessings={owned}"
    ))
}

async fn title_command(client: &mut DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CHARACTER_INFO_V1,
            COMMAND_TYPE_CHARACTER_INFO,
            payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=title request={payload:?} observed={reply:?}").into());
    }
    Ok(reply.payload)
}

async fn title_state(client: &mut DevClientSession) -> TestResult<(Vec<u8>, u8)> {
    let reply = title_command(client, &[0, 0, 0, 1]).await?;
    if reply.first() != Some(&0) || reply.len() < 71 || reply.len() != 71 + usize::from(reply[69]) {
        return Err(format!("step=title-query observed=malformed-reply:{reply:?}").into());
    }
    Ok((reply[70..reply.len() - 1].to_vec(), reply[reply.len() - 1]))
}

async fn quest_query(
    client: &mut DevClientSession,
    query: &journal::QuestLogQuery,
) -> TestResult<journal::QuestLog> {
    let payload = journal::encode_quest_log_query(query)
        .map_err(|error| format!("step=quest-query encode={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            journal::CAPABILITY_QUEST_LOG_V1,
            journal::COMMAND_TYPE_QUEST_LOG_QUERY,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted || !reply.payload.is_empty() {
        return Err(format!("step=quest-query query={query:?} observed={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    if delta.server_sequence <= reply.server_sequence {
        return Err("step=quest-query observed=stale-delta".into());
    }
    journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("step=quest-query decode={error:?}").into())
}

async fn quest_list(client: &mut DevClientSession) -> TestResult<Vec<journal::QuestListEntry>> {
    match quest_query(client, &journal::QuestLogQuery::List)
        .await?
        .view
    {
        Some(journal::QuestLogView::List(entries)) => Ok(entries),
        other => Err(format!("step=quest-list observed={other:?}").into()),
    }
}

async fn start_inquisition(clients: &SeamClients<'_>, client: &mut DevClientSession) -> TestResult {
    let world = full_e2ef7::qualification_world(clients)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let [placement] = metadata
        .npcs
        .npcs
        .iter()
        .find(|record| record.key == "oteryn:npc.henricus")
        .ok_or("step=henricus observed=missing-from-bundle")?
        .placements
        .as_slice()
    else {
        return Err("step=henricus observed=ambiguous-placement".into());
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == placement.floor
            && u32::from(at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)))
                <= npc::TALK_RANGE_TILES
    })
    .ok_or("step=henricus observed=no-walking-route")?;
    walk_to_henricus(world, client, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Npc
                    && e.position.x == i32::from(placement.x)
                    && e.position.y == i32::from(placement.y)
                    && e.position.floor == -i16::from(placement.floor)
            })
        })
        .map(|e| e.entity)
        .ok_or("step=henricus observed=actor-not-visible")?;
    for (text, prefix) in [
        ("hi", ""),
        ("mission", "Do you want to join the inquisition?"),
        ("yes", "So be it. Now you are a member of the inquisition."),
        ("mission", "Let's see if you are worthy."),
    ] {
        let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        })
        .map_err(|error| format!("step=inquisition text={text} encode={error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                npc::CAPABILITY_NPC_SERVICE_V1,
                npc::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted
            || npc::decode_npc_intent_result(&reply.payload).ok()
                != Some(npc::NpcIntentDisposition::Ok)
        {
            return Err(format!("step=inquisition text={text} observed={reply:?}").into());
        }
        let delta = client
            .session_mut()
            .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        let conversation = npc::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("step=inquisition text={text} decode={error:?}"))?;
        if delta.server_sequence <= reply.server_sequence
            || conversation.open.map(|(open, _)| open) != Some(actor)
            || !conversation
                .lines
                .iter()
                .any(|line| line.starts_with(prefix))
        {
            return Err(format!("step=inquisition text={text} observed={conversation:?}").into());
        }
    }
    Ok(())
}

async fn walk_to_henricus(
    world: &BundleWorld,
    client: &mut DevClientSession,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = client.world_spatial().actor_position;
        let expected = destination(world, tile(before)?, direction)
            .ok_or("step=henricus observed=planned-landing-disappeared")?;
        let mut outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked
            && client.world_spatial().actor_position == before
        {
            let blocker = client.world_entities().and_then(|view| view.iter().find(|e| {
                matches!(e.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Npc | oteryn_dev_client::EntityKind::Player)
                    && tile(e.position).ok() == Some(expected)
            })).map(|e| e.entity).ok_or_else(|| {
                format!("BLOCKER-SERVER step=henricus blocked={expected:?} occupant=none frame={outcome:?}")
            })?;
            evidence(&format!(
                "G29 movement blocked tile={expected:?} actor={blocker:?} frame={outcome:?}"
            ));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            loop {
                let position = client
                    .world_entities()
                    .and_then(|view| view.iter().find(|e| e.entity == blocker))
                    .map(|e| e.position);
                if position.is_some_and(|position| tile(position).ok() != Some(expected)) {
                    evidence(&format!(
                        "G29 movement blocker-cleared actor={blocker:?} position={position:?}"
                    ));
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(
                        format!("step=henricus blocker={blocker:?} observed={position:?}").into(),
                    );
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
            outcome = client.step(direction).await?;
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || before == client.world_spatial().actor_position
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "step=henricus from={before:?} expected={expected:?} frame={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
    }
    Ok(())
}

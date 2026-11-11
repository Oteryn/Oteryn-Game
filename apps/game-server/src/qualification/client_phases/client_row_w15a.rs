//! W15a client replay capture: an ordinary player queries the quest journal (`List`, the first
//! listed `Quest` line and `Track`) and the raw `QuestLog` delta payloads the server sent are
//! written byte for byte, together with the snapshot the join delivered. The native quest log
//! reducer (`crates/client-runtime/src/features/0016-quest-log/state.rs`) consumes them in
//! `tests/e2e_w15a.rs`. The query for an unlisted quest is refused with an empty reply.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

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

/// Sends one query and returns the raw delta (accepted, empty reply) the server published.
async fn query(
    client: &mut DevClientSession,
    query: &journal::QuestLogQuery,
) -> TestResult<RawDelta> {
    settle_rate(client).await?;
    let payload = journal::encode_quest_log_query(query)
        .map_err(|error| format!("quest query encode: {error:?}"))?;
    let reply = journal::send(client.session_mut(), &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("query={query:?} reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    Ok(RawDelta {
        server_sequence: delta.server_sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        payload: delta.payload,
    })
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0x86).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let listed = query(&mut client, &journal::QuestLogQuery::List).await?;
    let list = match journal::decode_quest_log(&listed.payload)
        .map_err(|error| format!("step=list decode={error:?}"))?
        .view
    {
        Some(journal::QuestLogView::List(entries)) if !entries.is_empty() => entries,
        other => return Err(format!("step=list view={other:?}").into()),
    };
    let index = list[0].quest_index;
    let lined = query(&mut client, &journal::QuestLogQuery::Quest { quest_index: index }).await?;
    let line = match journal::decode_quest_log(&lined.payload)
        .map_err(|error| format!("step=line decode={error:?}"))?
        .view
    {
        Some(journal::QuestLogView::Line(line)) if line.quest_index == index => line,
        other => return Err(format!("step=line index={index} view={other:?}").into()),
    };
    let tracked = query(
        &mut client,
        &journal::QuestLogQuery::Track { quest_indexes: vec![index] },
    )
    .await?;
    let track = journal::decode_quest_log(&tracked.payload)
        .map_err(|error| format!("step=track decode={error:?}"))?;
    let tracked_line = track
        .tracked
        .iter()
        .find(|entry| entry.quest_index == index)
        .ok_or_else(|| format!("step=track index={index} tracked={track:?}"))?;
    // An unlisted quest is refused with an empty reply and no delta.
    settle_rate(&mut client).await?;
    let invalid = list.iter().map(|entry| entry.quest_index).max().unwrap_or(0).saturating_add(1);
    let payload = journal::encode_quest_log_query(&journal::QuestLogQuery::Quest { quest_index: invalid })
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
    for entry in &list {
        expected.push_str(&format!("list={}\t{}\n", entry.quest_index, u8::from(entry.completed)));
    }
    expected.push_str(&line_text("line", &line));
    expected.push_str(&line_text("tracked", tracked_line));
    expected.push_str(&format!("invalid={invalid}\nrefused={:?}\n", refused.status));
    capture("w15a-snapshot.bin", &snapshot.payload)?;
    capture("w15a-list.bin", &listed.payload)?;
    capture("w15a-line.bin", &lined.payload)?;
    capture("w15a-track.bin", &tracked.payload)?;
    capture("w15a-expected.txt", expected.as_bytes())?;

    Ok(format!("quests={} line={index} missions={}", list.len(), line.missions.len()))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_escapes_delimiters_without_losing_backslashes() {
        assert_eq!(escape(""), "");
        assert_eq!(escape("\\\n\t"), "\\\\\\n\\t");
    }
}

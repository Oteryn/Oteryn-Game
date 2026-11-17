// Raw server domain payloads for the client-runtime replay test; no fabricated quest state.
async fn query_quest_journal(
    client: &mut oteryn_dev_client::DevClientSession,
    capture: bool,
) -> TestResult<Vec<oteryn_protocol_oteryn::quest_log::QuestListEntry>> {
    use oteryn_dev_client::client_runtime::quest_log as journal;
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("real World has no quest journal snapshot")?
        .clone();
    let mut state =
        journal::QuestState::from_snapshot(client.selected_capabilities(), Some(&snapshot))
            .map_err(|error| format!("quest snapshot reducer: {error:?}"))?;
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
    if capture && let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?;
        let path = Path::new(&path).canonicalize()?;
        if !path.starts_with(root) {
            return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
        }
        std::fs::write(path.join("quest-snapshot.bin"), &snapshot.payload)?;
        std::fs::write(path.join("quest-list.bin"), &delta.payload)?;
        std::fs::write(
            path.join("quest-revisions.txt"),
            format!(
                "{} {} {} {}",
                snapshot.revision, delta.base_revision, delta.new_revision, delta.server_sequence
            ),
        )?;
        let expected = state
            .list
            .iter()
            .map(|entry| format!("{} {}\n", entry.quest_index, u8::from(entry.completed)))
            .collect::<String>();
        std::fs::write(path.join("quest-expected.txt"), expected)?;
    }
    Ok(state.list)
}

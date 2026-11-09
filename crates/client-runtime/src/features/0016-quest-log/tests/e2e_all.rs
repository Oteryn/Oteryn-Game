use super::*;

// Unit mode uses an explicit codec fixture. The e2e_all runner requires raw payloads
// captured from its node, including an independent expected list from before relogin.
#[test]
fn e2e_all_quest_journal_replay() -> Result<(), Box<dyn std::error::Error>> {
    let (snapshot_payload, list_payload, revisions, expected) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("quest-snapshot.bin"))?,
                std::fs::read(path.join("quest-list.bin"))?,
                std::fs::read_to_string(path.join("quest-revisions.txt"))?,
                std::fs::read_to_string(path.join("quest-expected.txt"))?,
            )
        } else {
            let log = QuestLog {
                map_marks: Vec::new(),
                view: Some(QuestLogView::List(vec![
                    QuestListEntry {
                        quest_index: 7,
                        completed: false,
                    },
                    QuestListEntry {
                        quest_index: 12,
                        completed: true,
                    },
                ])),
                tracked: vec![],
            };
            (
                encode_quest_log(&QuestLog::default()).map_err(|e| format!("{e:?}"))?,
                encode_quest_log(&log).map_err(|e| format!("{e:?}"))?,
                "4 4 5 19".into(),
                "7 0\n12 1\n".into(),
            )
        };
    let numbers = revisions
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [
        snapshot_revision,
        base_revision,
        new_revision,
        server_sequence,
    ] = numbers.as_slice()
    else {
        return Err("invalid quest capture revision metadata".into());
    };
    let snapshot = GatedSnapshot {
        domain_id: STATE_DOMAIN_QUEST_LOG,
        snapshot_type: SNAPSHOT_TYPE_QUEST_LOG_V1,
        revision: *snapshot_revision,
        payload: snapshot_payload,
    };
    let mut state = QuestState::from_snapshot(&[CAPABILITY_QUEST_LOG_V1], Some(&snapshot))
        .map_err(|e| format!("{e:?}"))?;
    state
        .apply(&AppliedDelta {
            server_sequence: *server_sequence,
            base_revision: *base_revision,
            new_revision: *new_revision,
            value: decode_quest_log(&list_payload).map_err(|e| format!("{e:?}"))?,
        })
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(state.log.view, Some(QuestLogView::List(_))));
    assert_eq!(state.revision, Some(*new_revision));
    let visible = state
        .list
        .iter()
        .map(|entry| format!("{} {}\n", entry.quest_index, u8::from(entry.completed)))
        .collect::<String>();
    assert_eq!(visible, expected);
    assert_eq!(state.refusal, None);
    Ok(())
}

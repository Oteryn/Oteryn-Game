use super::*;

fn state() -> QuestState {
    QuestState {
        selected: true,
        revision: Some(0),
        ..QuestState::default()
    }
}
fn delta(base: u64, value: QuestLog) -> AppliedDelta<QuestLog> {
    AppliedDelta {
        server_sequence: base + 1,
        base_revision: base,
        new_revision: base + 1,
        value,
    }
}
fn list() -> QuestLog {
    QuestLog {
        view: Some(QuestLogView::List(vec![
            QuestListEntry {
                quest_index: 1,
                completed: false,
            },
            QuestListEntry {
                quest_index: 2,
                completed: true,
            },
        ])),
        tracked: vec![],
    }
}
fn line() -> QuestLine {
    QuestLine {
        quest_index: 1,
        completed: false,
        missions: vec![QuestMission {
            mission_index: 0,
            name: "Find the key".into(),
            text: "Search the tower.".into(),
            done: false,
        }],
    }
}

#[test]
fn quest_snapshot_and_delta_replace_the_view_and_tracked_set() -> Result<(), QuestRefusal> {
    let snapshot = GatedSnapshot {
        domain_id: 16,
        snapshot_type: 1,
        revision: 7,
        payload: encode_quest_log(&list()).map_err(super::state::wire_refusal)?,
    };
    let mut state = QuestState::from_snapshot(&[16], Some(&snapshot))?;
    assert_eq!(state.revision, Some(7));
    state.apply(&delta(
        7,
        QuestLog {
            view: Some(QuestLogView::Line(line())),
            tracked: vec![line()],
        },
    ))?;
    assert_eq!(state.list.len(), 2);
    assert_eq!(state.log.tracked, vec![line()]);
    state.apply(&delta(8, QuestLog::default()))?;
    assert_eq!(state.log, QuestLog::default());
    let before = state.clone();
    assert_eq!(
        state.apply(&delta(0, list())),
        Err(QuestRefusal::RevisionMismatch)
    );
    assert_eq!(state, before);
    Ok(())
}

#[test]
fn quest_capability_gate_unknown_quest_and_refusals_are_typed() -> Result<(), QuestRefusal> {
    let mut disabled = QuestState::from_snapshot(&[], None)?;
    assert_eq!(
        disabled.command(&QuestLogQuery::List),
        Err(QuestRefusal::CapabilityNotSelected)
    );
    assert_eq!(
        disabled.apply(&delta(0, list())),
        Err(QuestRefusal::CapabilityNotSelected)
    );
    let mut state = state();
    assert_eq!(
        state.command(&QuestLogQuery::Quest { quest_index: 99 }),
        Err(QuestRefusal::UnknownQuest)
    );
    state.record_result(CommandStatus::Rejected, &[])?;
    assert_eq!(state.refusal, Some(QuestRefusal::Rejected));
    state.record_result(CommandStatus::DuplicateOutcomeExpired, &[])?;
    assert_eq!(state.refusal, Some(QuestRefusal::ReconciliationRequired));
    assert_eq!(
        state.record_result(CommandStatus::Accepted, &[1]),
        Err(QuestRefusal::Malformed)
    );
    assert_eq!(
        QuestState::from_snapshot(&[16], None),
        Err(QuestRefusal::Malformed)
    );
    assert!(
        QuestState::from_snapshot(
            &[16],
            Some(&GatedSnapshot {
                domain_id: 16,
                snapshot_type: 1,
                revision: 0,
                payload: vec![255]
            })
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn quest_command_22_matches_golden_protobuf_bytes() -> Result<(), QuestRefusal> {
    let mut state = state();
    state.apply(&delta(0, list()))?;
    for (query, golden) in [
        (QuestLogQuery::List, vec![0x0a, 0x00]),
        (
            QuestLogQuery::Quest { quest_index: 1 },
            vec![0x12, 0x02, 0x08, 0x01],
        ),
        (
            QuestLogQuery::Track {
                quest_indexes: vec![1, 2],
            },
            vec![0x1a, 0x04, 0x08, 0x01, 0x08, 0x02],
        ),
        (
            QuestLogQuery::Track {
                quest_indexes: vec![],
            },
            vec![0x1a, 0x00],
        ),
    ] {
        assert_eq!(state.command(&query)?, golden);
        assert_eq!(decode_quest_log_query(&golden), Ok(query));
    }
    assert_eq!(
        state.command(&QuestLogQuery::Track {
            quest_indexes: vec![1, 1]
        }),
        Err(QuestRefusal::Malformed)
    );
    Ok(())
}

#[test]
fn quest_line_completion_updates_cached_rows_and_bad_delta_is_atomic() -> Result<(), QuestRefusal> {
    let mut state = state();
    state.apply(&delta(0, list()))?;
    let mut done = line();
    done.completed = true;
    state.apply(&delta(
        1,
        QuestLog {
            view: Some(QuestLogView::Line(done)),
            tracked: vec![],
        },
    ))?;
    assert!(state.list[0].completed);
    let before = state.clone();
    let mut invalid = line();
    invalid.missions[0].name.clear();
    assert_eq!(
        state.apply(&delta(
            2,
            QuestLog {
                view: Some(QuestLogView::Line(invalid)),
                tracked: vec![]
            }
        )),
        Err(QuestRefusal::Malformed)
    );
    assert_eq!(state, before);
    Ok(())
}

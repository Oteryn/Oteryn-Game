use super::*;
#[test]
fn quest_replacement_resets_cached_list_and_revision_before_new_base_delta() -> Result<(), String> {
    let log = QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![QuestListEntry {
            quest_index: 1,
            completed: false,
        }])),
        tracked: vec![],
    };
    let snapshot = |revision, log: &QuestLog| -> Result<GatedSnapshot, String> {
        Ok(GatedSnapshot {
            domain_id: 16,
            snapshot_type: 1,
            revision,
            payload: encode_quest_log(log).map_err(|e| format!("{e:?}"))?,
        })
    };
    let (link, mut commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).quest.state =
        QuestState::from_snapshot(&[16], Some(&snapshot(1, &log)?))
            .map_err(|e| format!("{e:?}"))?;
    {
        let mut m = mailbox::lock(&outbox.0);
        m.quest.tracker.auto_add = true;
        m.quest.tracker.observe(&[]);
        m.quest.tracker.observe(&[QuestListEntry {
            quest_index: 1,
            completed: false,
        }]);
    }
    let pending = oteryn_client_ui::quest_log::tracker_query(
        &mut mailbox::lock(&outbox.0).quest,
        std::time::Instant::now(),
    )
    .ok_or("automatic tracking")?;
    link.quest_request(&pending);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_client_runtime::CommandStatus::Rejected,
        server_sequence: 1,
        payload: vec![],
    });
    outbox.push(vec![
        SessionEvent::QuestLog(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: log,
        }),
        SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 3,
            base_revision: 2,
            new_revision: 9,
            value: snapshot(9, &QuestLog::default())?,
        }),
        SessionEvent::QuestLog(AppliedDelta {
            server_sequence: 4,
            base_revision: 9,
            new_revision: 10,
            value: QuestLog::default(),
        }),
    ]);
    let panel = link.quest_panel();
    assert_eq!(panel.state.revision, Some(10));
    assert!(panel.state.list.is_empty());
    assert_eq!(panel.state.refusal, None);
    let now = std::time::Instant::now() + std::time::Duration::from_secs(2);
    assert_eq!(
        oteryn_client_ui::quest_log::tracker_query(&mut mailbox::lock(&outbox.0).quest, now),
        Some(oteryn_client_runtime::quest_log::QuestLogQuery::ListOnly)
    );
    link.quest_request(&oteryn_client_runtime::quest_log::QuestLogQuery::ListOnly);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 5,
        payload: encode_quest_log(&QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![
                QuestListEntry {
                    quest_index: 1,
                    completed: false,
                },
                QuestListEntry {
                    quest_index: 2,
                    completed: false,
                },
            ])),
            tracked: vec![],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    assert_eq!(
        oteryn_client_ui::quest_log::tracker_query(
            &mut mailbox::lock(&outbox.0).quest,
            now + std::time::Duration::from_secs(1)
        ),
        Some(oteryn_client_runtime::quest_log::QuestLogQuery::Track {
            quest_indexes: vec![1, 2]
        })
    );
    assert!(link.quest_panel().state.log.tracked.is_empty());
    Ok(())
}

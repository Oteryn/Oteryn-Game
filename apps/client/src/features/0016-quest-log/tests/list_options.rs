use super::*;
use oteryn_client_runtime::quest_log::{QuestLine, QuestLogQuery, decode_quest_log_query};
use oteryn_client_ui::quest_log::{QuestSort, quest_row_query, quest_rows};
#[test]
fn sorted_filtered_quest_selection_sends_the_displayed_server_id() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let visible = QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::Line(QuestLine {
            quest_index: 1,
            completed: false,
            missions: vec![],
        })),
        tracked: vec![],
    };
    {
        let mut m = mailbox::lock(&outbox.0);
        m.quest.state.selected = true;
        m.quest.state.revision = Some(42);
        m.quest.state.log = visible.clone();
    }
    link.quest_request(&QuestLogQuery::ListOnly);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 1,
        payload: encode_quest_log(&QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![
                QuestListEntry {
                    quest_index: 1,
                    completed: false,
                },
                QuestListEntry {
                    quest_index: 2,
                    completed: false,
                },
                QuestListEntry {
                    quest_index: 20,
                    completed: true,
                },
            ])),
            tracked: vec![],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    let mut panel = link.quest_panel();
    panel.list_sort = QuestSort::NumberDescending;
    panel.list_search = "quest 2".into();
    let shown: Vec<_> = quest_rows(&panel)
        .into_iter()
        .map(|q| q.quest_index)
        .collect();
    assert_eq!(shown, [20, 2]);
    let query = quest_row_query(&panel, &shown, 0).ok_or("displayed quest")?;
    link.quest_request(&query);
    let Ok(PlayCommand::QuestLog(payload)) = commands.try_recv() else {
        return Err("query".into());
    };
    assert_eq!(
        decode_quest_log_query(&payload).map_err(|e| format!("{e:?}"))?,
        QuestLogQuery::Quest { quest_index: 20 }
    );
    assert_eq!(
        link.quest_panel().state.log,
        visible,
        "local filtering/sorting must not move the confirmed cursor"
    );
    panel.state.list.retain(|q| q.quest_index != 20);
    assert!(
        quest_row_query(&panel, &shown, 0).is_none(),
        "a stale displayed row must not select its replacement"
    );
    Ok(())
}

#[test]
fn journal_search_and_sort_use_confirmed_mission_text_and_keep_server_cursor() -> Result<(), String>
{
    use oteryn_client_runtime::quest_log::QuestMission;
    let (link, mut commands, outbox) = play_channel();
    outbox.quest_initialize(QuestState {
        selected: true,
        revision: Some(0),
        ..Default::default()
    });
    let log = QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![
            QuestListEntry {
                quest_index: 1,
                completed: false,
            },
            QuestListEntry {
                quest_index: 2,
                completed: true,
            },
        ])),
        tracked: vec![QuestLine {
            quest_index: 1,
            completed: false,
            missions: vec![QuestMission {
                mission_index: 1,
                name: "The tower".into(),
                text: "Find the key".into(),
                done: false,
            }],
        }],
    };
    outbox.push(vec![oteryn_session::SessionEvent::QuestLog(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: log.clone(),
        },
    )]);
    link.quest_list_options(" KEY ", QuestSort::NumberDescending);
    let p = link.quest_panel();
    assert_eq!(
        quest_rows(&p)
            .iter()
            .map(|q| q.quest_index)
            .collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(p.state.log, log);
    link.quest_list_options("", QuestSort::CompletedFirst);
    let p = link.quest_panel();
    let painted = quest_rows(&p)
        .iter()
        .map(|q| q.quest_index)
        .collect::<Vec<_>>();
    assert_eq!(painted, [2, 1]);
    link.quest_request(&quest_row_query(&p, &painted, 0).ok_or("selected row")?);
    let Ok(PlayCommand::QuestLog(bytes)) = commands.try_recv() else {
        return Err("quest command".into());
    };
    assert_eq!(
        decode_quest_log_query(&bytes).map_err(|e| format!("{e:?}"))?,
        QuestLogQuery::Quest { quest_index: 2 }
    );
    assert_eq!(link.quest_panel().state.log, log);
    Ok(())
}

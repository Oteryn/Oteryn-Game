use super::*;
#[test]
fn quest_list_search_and_all_sort_modes_preserve_confirmed_state() -> Result<(), &'static str> {
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.state.list = vec![
        QuestListEntry {
            quest_index: 1,
            completed: false,
        },
        QuestListEntry {
            quest_index: 2,
            completed: true,
        },
        QuestListEntry {
            quest_index: 20,
            completed: false,
        },
    ];
    p.state.log.view = Some(QuestLogView::Line(QuestLine {
        quest_index: 1,
        completed: false,
        missions: vec![],
    }));
    p.mission = Some(7);
    let state = p.state.clone();
    for (sort, expected) in [
        (QuestSort::NumberAscending, vec![1, 2, 20]),
        (QuestSort::NumberDescending, vec![20, 2, 1]),
        (QuestSort::ActiveFirst, vec![1, 20, 2]),
        (QuestSort::CompletedFirst, vec![2, 1, 20]),
    ] {
        p.list_sort = sort;
        assert_eq!(
            quest_rows(&p)
                .iter()
                .map(|q| q.quest_index)
                .collect::<Vec<_>>(),
            expected
        );
    }
    p.list_search = " COMPLETED ".into();
    assert_eq!(
        quest_rows(&p)
            .iter()
            .map(|q| q.quest_index)
            .collect::<Vec<_>>(),
        [2]
    );
    p.list_search = "quest 2".into();
    assert_eq!(quest_rows(&p).len(), 2);
    p.list_search = "unavailable".into();
    assert!(quest_rows(&p).is_empty());
    assert_eq!(p.state, state);
    assert_eq!(p.mission, Some(7));
    Ok(())
}
#[test]
fn painted_row_identity_is_checked_against_the_latest_list() -> Result<(), &'static str> {
    let mut p = QuestPanel::default();
    p.state.selected = true;
    p.state.list = vec![QuestListEntry {
        quest_index: 2,
        completed: false,
    }];
    assert_eq!(
        quest_row_query(&p, &[20, 2], 1),
        Some(QuestLogQuery::Quest { quest_index: 2 })
    );
    assert!(quest_row_query(&p, &[20, 2], 0).is_none());
    assert!(quest_row_query(&p, &[20, 2], usize::MAX).is_none());
    p.state.selected = false;
    assert!(quest_row_query(&p, &[2], 0).is_none());
    Ok(())
}

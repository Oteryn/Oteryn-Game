use super::*;

#[test]
fn journal_clear_tracker_sends_empty_tracking_selection() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    let mut snapshot = quest_state();
    if let Some(QuestLogView::Line(line)) = snapshot.log.view.clone() {
        snapshot.log.tracked.push(line);
    }
    outbox.bevy_journal_quest_state(snapshot);
    host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::ClearTracker
        },
        &actions::Values::default()
    ));
    let PlayCommand::QuestLog(payload) = commands.try_recv()? else {
        return Err("missing tracker update".into());
    };
    assert_eq!(
        decode_quest_log_query(&payload).map_err(|_| "quest query")?,
        QuestLogQuery::Track {
            quest_indexes: vec![]
        }
    );
    Ok(())
}

#[test]
fn journal_pin_hide_restore_and_order_quests() -> Result {
    let (mut host, _, outbox) = host()?;
    let mut snapshot = quest_state();
    snapshot.list.push(QuestListEntry {
        quest_index: 2,
        completed: false,
    });
    outbox.bevy_journal_quest_state(snapshot);
    host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    for (kind, expected, hidden) in [
        (Kind::Pin, vec![73, 2], 0),
        (Kind::HideQuest, vec![2], 1),
        (Kind::ShowHidden, vec![73, 2], 1),
        (Kind::HideQuest, vec![73, 2], 0),
        (Kind::Pin, vec![2, 73], 0),
    ] {
        assert!(actions::perform(
            &mut host,
            &mut state,
            Action { key, kind },
            &actions::Values::default()
        ));
        let panel = host.play.as_ref().ok_or("play")?.link.quest_panel();
        let view = model::quests(&panel, 0, false);
        assert_eq!(
            view.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(model::RowId::Quest)
                .collect::<Vec<_>>()
        );
        assert!(view.status.contains(&format!("{hidden} hidden")));
    }
    Ok(())
}

#[test]
fn journal_sort_cycles_through_both_alphabetical_orders() -> Result {
    let (mut host, _, outbox) = host()?;
    let mut snapshot = quest_state();
    snapshot.list.push(QuestListEntry {
        quest_index: 100,
        completed: false,
    });
    outbox.bevy_journal_quest_state(snapshot);
    host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    for index in 1..=5 {
        assert!(actions::perform(
            &mut host,
            &mut state,
            Action {
                key,
                kind: Kind::Sort
            },
            &actions::Values::default()
        ));
        let panel = host.play.as_ref().ok_or("play")?.link.quest_panel();
        assert_eq!(panel.list_sort.selection(), index);
        if index >= 4 {
            let rows = model::quests(&panel, 0, false).rows;
            assert_eq!(
                rows[0].id,
                model::RowId::Quest(if index == 4 { 100 } else { 73 })
            );
        }
    }
    Ok(())
}

#[test]
fn independent_tracker_opens_the_clicked_mission_and_adds_a_window() -> Result {
    let (host, mut commands, outbox) = host()?;
    let mut snapshot = quest_state();
    let Some(QuestLogView::Line(line)) = snapshot.log.view.take() else {
        return Err("line".into());
    };
    snapshot.log.tracked.push(line);
    outbox.bevy_journal_quest_state(snapshot);
    host.play.as_ref().ok_or("play")?.link.open_quest_tracker();
    let _refresh = commands.try_recv()?;
    outbox.bevy_journal_quest_result(&result(vec![]));
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(host)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    tracker::install(&mut app);
    app.update();
    let click = |app: &mut App, kind: tracker::Control| -> Result {
        let entity = app
            .world_mut()
            .query::<(Entity, &tracker::ButtonAction)>()
            .iter(app.world())
            .find(|(_, a)| a.kind == kind)
            .ok_or("missing tracker control")?
            .0;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    click(&mut app, tracker::Control::Add)?;
    assert_eq!(
        app.world_mut()
            .query::<&tracker::Window>()
            .iter(app.world())
            .count(),
        2
    );
    click(&mut app, tracker::Control::Mission(73, 1))?;
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .quest_panel();
    assert!(panel.open && panel.tracker_open);
    assert_eq!(panel.mission, Some(1));
    assert_eq!(
        model::quests(&panel, 0, false).detail,
        "Find the guide\n\nReturn to the island guide."
    );
    Ok(())
}

#[test]
fn journal_cleanup_preserves_active_tracked_quests() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    let mut snapshot = quest_state();
    let Some(QuestLogView::Line(line)) = &snapshot.log.view else {
        return Err("line".into());
    };
    snapshot.log.tracked = vec![
        line.clone(),
        QuestLine {
            quest_index: 90,
            completed: true,
            missions: vec![],
        },
    ];
    outbox.bevy_journal_quest_state(snapshot);
    host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::CleanTracker
        },
        &default()
    ));
    let PlayCommand::QuestLog(payload) = commands.try_recv()? else {
        return Err("cleanup command".into());
    };
    assert_eq!(
        decode_quest_log_query(&payload).map_err(|_| "query")?,
        QuestLogQuery::Track {
            quest_indexes: vec![73]
        }
    );
    Ok(())
}

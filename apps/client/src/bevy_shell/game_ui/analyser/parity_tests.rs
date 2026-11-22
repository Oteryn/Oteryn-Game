#[test]
fn analyser_clear_all_preserves_measured_drops() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    tracker_item(&mut app, "202")?;
    click_caption(&mut app, "Track item")?;
    assert!(report(&mut app)?.contains("(tracked)"));
    click_caption(&mut app, "Remove all tracked items")?;
    let text = report(&mut app)?;
    assert!(text.contains("#202"));
    assert!(!text.contains("(tracked)"));
    Ok(())
}

#[test]
fn tracker_lists_the_confirmed_source_of_each_drop() -> TestResult {
    let (mut app, outbox) = app()?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 2,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::KillLoot {
            race: Some(81.try_into()?),
            corpse_items: vec![oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: 202.try_into()?,
                count: 1.try_into()?,
            }],
            gold: 0,
        }],
    })]);
    select(&mut app, Tool::LootTracker)?;
    assert!(report(&mut app)?.contains("#202 × 1 · Creature race #81"));
    Ok(())
}

#[test]
fn starting_a_hunt_restarts_each_personal_interval() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::Hunting)?;
    click_caption(&mut app, "Start new session")?;
    for (tool, field) in [
        (Tool::Hunting, "Kills: 0"),
        (Tool::Loot, "Valued loot: 0"),
        (Tool::Supplies, "Supplies used: 0"),
        (Tool::Experience, "Experience: 0 raw / 0 gained"),
    ] {
        select(&mut app, tool)?;
        assert!(report(&mut app)?.contains(field));
    }
    Ok(())
}

#[test]
fn drop_tracker_opens_item_cyclopedia() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    click_caption(&mut app, "Item Cyclopedia")?;
    let owner = app.world().non_send::<HostOwner>();
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .inspection_panel()
            .catalogue_open
    );
    Ok(())
}

#[test]
fn party_hunt_log_drives_split_expenses_and_exclusion() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::Party)?;
    let set_field = |app: &mut App, index, value: &str| -> TestResult {
        let (_, mut field) = app
            .world_mut()
            .query::<(&ConfigField, &mut bevy::text::EditableText)>()
            .iter_mut(app.world_mut())
            .find(|(field, _)| field.0 == index && field.1 == Tool::Party)
            .ok_or("party editor")?;
        *field = bevy::text::EditableText::new(value);
        Ok(())
    };
    set_field(
        &mut app,
        3,
        "Session data: From today\nAlice (Leader)\n Loot: 100\n Supplies: 0\n Balance: 100\n Damage: 20\n Healing: 0\nBob\n Loot: 0\n Supplies: 0\n Balance: 0\n Damage: 0\n Healing: 10",
    )?;
    click_caption(&mut app, "Calculate split")?;
    app.update();
    assert!(report(&mut app)?.contains("Alice pays Bob: 50 gold"));
    set_field(&mut app, 4, "Bob")?;
    set_field(&mut app, 5, "20")?;
    click_caption(&mut app, "Apply extra cost")?;
    app.update();
    assert!(report(&mut app)?.contains("Alice pays Bob: 60 gold"));
    click_caption(&mut app, "Exclude member")?;
    app.update();
    assert!(report(&mut app)?.contains("Already balanced"));
    Ok(())
}

#[test]
fn analyser_controls_select_report_sections() -> TestResult {
    for (tool, caption, heading) in [
        (Tool::Impact, "DPS gauge", "DPS target:"),
        (Tool::Impact, "DPS graph", "Damage per minute"),
        (Tool::Impact, "Outgoing damage types", "Damage types"),
        (Tool::Impact, "HPS gauge", "HPS target:"),
        (Tool::Impact, "HPS graph", "Healing per minute"),
        (Tool::Input, "Incoming damage graph", "Per minute"),
        (Tool::Input, "Incoming damage types", "Damage types"),
        (Tool::Input, "Incoming damage sources", "Sources"),
        (Tool::Loot, "Loot rate gauge", "Loot/h target:"),
        (Tool::Loot, "Loot rate graph", "Per minute"),
        (Tool::Supply, "Supply rate gauge", "Supply/h target:"),
        (Tool::Supply, "Supply rate graph", "Per minute"),
        (Tool::Xp, "XP rate gauge", "XP/h target:"),
        (Tool::Xp, "XP rate graph", "Per minute"),
        (Tool::Xp, "Raw XP", "Raw XP/h:"),
    ] {
        let (mut app, _outbox) = app()?;
        select(&mut app, tool)?;
        assert!(report(&mut app)?.contains(heading));
        click_caption(&mut app, caption)?;
        app.update();
        assert!(!report(&mut app)?.contains(heading), "{caption}");
        click_caption(&mut app, caption)?;
        app.update();
        assert!(report(&mut app)?.contains(heading));
    }
    Ok(())
}

#[test]
fn analyser_rate_targets_follow_the_edited_value() -> TestResult {
    for (tool, caption, heading) in [
        (Tool::Impact, "Set DPS target", "DPS target: 800"),
        (Tool::Impact, "Set HPS target", "HPS target: 800"),
        (Tool::Xp, "Set XP/h target", "XP/h target: 800"),
        (Tool::Loot, "Set loot/h target", "Loot/h target: 800"),
        (Tool::Supply, "Set supply/h target", "Supply/h target: 800"),
    ] {
        let (mut app, _outbox) = app()?;
        select(&mut app, tool)?;
        let (_, mut field) = app
            .world_mut()
            .query::<(&ConfigField, &mut bevy::text::EditableText)>()
            .iter_mut(app.world_mut())
            .find(|(f, _)| f.0 == 6)
            .ok_or("rate editor")?;
        *field = bevy::text::EditableText::new("800");
        click_caption(&mut app, caption)?;
        app.update();
        assert!(report(&mut app)?.contains(heading));
    }
    Ok(())
}

#[test]
fn hunting_exports_measured_text_and_json_to_chosen_files() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::Hunting)?;
    for (caption, file) in [
        ("Save hunt report", "hunt.txt"),
        ("Export hunt JSON", "hunt.json"),
    ] {
        let path = folder.path().join(file);
        let (_, mut field) = app
            .world_mut()
            .query::<(&ConfigField, &mut bevy::text::EditableText)>()
            .iter_mut(app.world_mut())
            .find(|(f, _)| f.0 == 7)
            .ok_or("destination")?;
        *field = bevy::text::EditableText::new(path.to_str().ok_or("path")?);
        click_caption(&mut app, caption)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !path.exists() {
            assert!(std::time::Instant::now() < deadline);
            app.update();
        }
        app.update();
        let saved = std::fs::read_to_string(path)?;
        if file.ends_with(".json") {
            let json: serde_json::Value = serde_json::from_str(&saved)?;
            assert_eq!(json["XPGain"], 150);
            assert_eq!(json["Loot"], 77);
        } else {
            assert!(saved.contains("Experience: 100 raw / 150 gained"));
        }
    }
    Ok(())
}

#[test]
fn report_copy_uses_the_activated_window() -> TestResult {
    #[derive(Resource, Default)]
    struct Captured(String);
    for (tool, expected) in [
        (Tool::Hunting, "Kills: 23"),
        (Tool::Input, "Damage received:"),
        (Tool::Party, "Party Hunt"),
    ] {
        let (mut app, _) = app()?;
        app.init_resource::<Captured>().add_observer(
            |event: On<CopyReport>, mut capture: ResMut<Captured>| {
                capture.0.clone_from(&event.0);
            },
        );
        select(&mut app, tool)?;
        click_caption(&mut app, "Copy report")?;
        assert!(app.world().resource::<Captured>().0.contains(expected));
    }
    Ok(())
}

#[test]
fn boss_cooldown_controls_filter_sort_and_open_the_selected_boss() -> TestResult {
    let (mut app, outbox) = app()?;
    outbox.bevy_bosstiary_initialize();
    select(&mut app, Tool::Bosses)?;
    outbox.bevy_bosstiary_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![
            8, 100, 18, 7, 10, 3, b'a', b':', b'a', 16, 125, 18, 7, 10, 3, b'b', b':', b'b', 16,
            105, 40, 5,
        ],
    });
    app.update();
    click_caption(&mut app, "Sort by name")?;
    app.update();
    let text = report(&mut app)?;
    assert!(text.find("a:a:").ok_or("a")? < text.find("b:b:").ok_or("b")?);
    click_caption(&mut app, "Sort by cooldown")?;
    app.update();
    let text = report(&mut app)?;
    assert!(text.find("b:b:").ok_or("b")? < text.find("a:a:").ok_or("a")?);
    tracker_item(&mut app, "B:B")?;
    click_caption(&mut app, "Search bosses")?;
    app.update();
    assert!(!report(&mut app)?.contains("a:a:"));
    click_caption(&mut app, "Bosstiary: b:b")?;
    let owner = app.world().non_send::<HostOwner>();
    assert!(owner.settings.preferences.panel_visible("bosstiary"));
    assert_eq!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .boss_encounter_panel()
            .browser_search,
        "b:b"
    );
    Ok(())
}

#[test]
fn drop_value_threshold_tracks_new_confirmed_currency_at_the_boundary() -> TestResult {
    use oteryn_client_runtime::analyser::{ItemInfo, encode_reply};
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    tracker_item(&mut app, "100")?;
    click_caption(&mut app, "Set auto-track value")?;
    let ids = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .analyser_panel()
        .catalogue_pending;
    let rows = ids
        .into_iter()
        .map(|item_type| ItemInfo {
            item_type,
            name: format!("Item {item_type}"),
            coin_worth: if item_type == 202 { 100 } else { 0 },
        })
        .collect::<Vec<_>>();
    outbox.bevy_analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_reply(&rows)?,
    });
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 3,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: 202.try_into()?,
                count: 1.try_into()?,
            }],
            gold: 100,
        }],
    })]);
    app.update();
    assert!(report(&mut app)?.contains("#202 Item 202 (tracked)"));
    Ok(())
}

#[test]
fn impact_retains_highs_across_a_reset_and_switches_rate_modes() -> TestResult {
    use oteryn_client_runtime::analyser::AnalyserImpact;
    let (mut app, outbox) = app()?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 2,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
            700.try_into()?,
        ))],
    })]);
    select(&mut app, Tool::Impact)?;
    assert!(report(&mut app)?.contains("All-time high HPS: 700"));
    click_caption(&mut app, "Reset session")?;
    assert!(report(&mut app)?.contains("All-time high HPS: 700"));
    click_caption(&mut app, "Reset all-time high")?;
    app.update();
    assert!(report(&mut app)?.contains("All-time high HPS: 0"));
    for tool in [Tool::Impact, Tool::Input] {
        if tool == Tool::Input {
            click_caption(&mut app, "Close")?;
            select(&mut app, tool)?;
        }
        let entity = app
            .world_mut()
            .query::<(Entity, &ConfigControl)>()
            .iter(app.world())
            .find_map(|(e, c)| (c.tool == tool && matches!(c.kind, ConfigKind::Mode)).then_some(e))
            .ok_or("rate mode control")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        app.update();
        assert!(report(&mut app)?.contains("Rate mode: session"));
    }
    Ok(())
}

#[test]
fn confirmed_party_handoff_populates_log_and_copies_transfers() -> TestResult {
    use oteryn_client_runtime::analyser::party::{Member, encode_reply};
    #[derive(Resource, Default)]
    struct Captured(String);
    let (mut app, outbox) = app()?;
    outbox.bevy_party_selected(true, true, "Leader");
    app.init_resource::<Captured>().add_observer(
        |event: On<CopyReport>, mut captured: ResMut<Captured>| captured.0.clone_from(&event.0),
    );
    select(&mut app, Tool::Party)?;
    let mut commands = app
        .world_mut()
        .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>>();
    let query = std::iter::from_fn(|| commands.try_recv().ok())
        .find_map(|command| match command {
            crate::combat_input::PlayCommand::Feature(cmd) if cmd.kind == 735 => Some(cmd),
            _ => None,
        })
        .ok_or("party request")?;
    outbox.bevy_party_accounting_result(
        &query,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_reply(&[
                Member {
                    handle: 1,
                    name: "Alice".into(),
                    available: true,
                    loot_value: 150,
                    ..default()
                },
                Member {
                    handle: 2,
                    name: "Bob".into(),
                    available: true,
                    supply_cost: 50,
                    ..default()
                },
            ])?,
        },
    );
    click_caption(&mut app, "Copy report")?;
    let copied = app.world().resource::<Captured>().0.clone();
    assert!(copied.contains("Alice\n\tLoot: 150"));
    click_caption(&mut app, "Send to Loot Splitter")?;
    let (_, field) = app
        .world_mut()
        .query::<(&ConfigField, &bevy::text::EditableText)>()
        .iter(app.world())
        .find(|(field, _)| field.0 == 3)
        .ok_or("log")?;
    assert_eq!(field.value().to_string(), copied);
    click_caption(&mut app, "Copy settlement instructions")?;
    assert_eq!(
        app.world().resource::<Captured>().0.trim(),
        "Alice pays Bob: 100 gold"
    );
    Ok(())
}

#[test]
fn impact_high_survives_reopening_the_character_profile() -> TestResult {
    use oteryn_client_runtime::analyser::AnalyserImpact;
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("client.json");
    {
        let (mut app, outbox) = app_at(Some(path.clone()))?;
        outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
            server_sequence: 2,
            base_revision: 2,
            new_revision: 3,
            value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
                90.try_into()?,
            ))],
        })]);
        let profile = app
            .world()
            .non_send::<HostOwner>()
            .dock_layout_path()
            .ok_or("profile")?
            .with_extension("analyser-highs.json");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !profile.exists() {
            assert!(std::time::Instant::now() < deadline);
            app.update();
        }
    }
    let (mut app, _) = app_at(Some(path))?;
    select(&mut app, Tool::Impact)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !report(&mut app)?.contains("All-time high HPS: 90") {
        assert!(std::time::Instant::now() < deadline);
        app.update();
    }
    assert!(report(&mut app)?.contains("All-time high HPS: 90"));
    Ok(())
}

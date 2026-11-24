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
        // The file is persisted before the export task reports ready. Waiting only for the
        // file let the next click land while the task was still held, and that click was
        // refused as "A hunt report is being saved".
        let feedback = |app: &App| {
            app.world().non_send::<HostOwner>().play.as_ref()
                .and_then(|play| play.link.analyser_panel().feedback)
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !path.exists() || feedback(&app) != Some("Hunt report saved") {
            if std::time::Instant::now() >= deadline {
                return Err(format!("export did not finish: {:?}", feedback(&app)).into());
            }
            app.update();
            std::thread::yield_now();
        }
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
        let feedback = |app: &App| {
            app.world().non_send::<HostOwner>().play.as_ref()
                .and_then(|play| play.link.analyser_panel().feedback)
        };
        let completed = |status| matches!(status, Some("Report copied" | "Clipboard unavailable"));
        assert!(!completed(feedback(&app)), "clipboard finished inside the input frame");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !completed(feedback(&app)) {
            if std::time::Instant::now() >= deadline {
                return Err("clipboard worker did not publish its result".into());
            }
            app.update();
            std::thread::yield_now();
        }
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
                    damage: 456,
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
    assert!(copied.contains("Damage: 456"));
    click_caption(&mut app, "Copy settlement instructions")?;
    assert_eq!(
        app.world().resource::<Captured>().0.trim(),
        "Alice pays Bob: 100 gold"
    );
    click_caption(&mut app, "Copy report")?;
    assert!(
        app.world()
            .resource::<Captured>()
            .0
            .contains("Bob\n\tLoot: 0")
    );
    assert_eq!(app.world().resource::<Captured>().0, copied);
    let (_, mut editor) = app
        .world_mut()
        .query::<(&ConfigField, &mut bevy::text::EditableText)>()
        .iter_mut(app.world_mut())
        .find(|(f, _)| f.0 == 3)
        .ok_or("log")?;
    *editor = bevy::text::EditableText::new("Session data: truncated");
    click_caption(&mut app, "Calculate split")?;
    app.update();
    assert!(!report(&mut app)?.contains("Alice pays Bob"));
    app.world_mut().resource_mut::<Captured>().0.clear();
    click_caption(&mut app, "Copy settlement instructions")?;
    assert!(app.world().resource::<Captured>().0.is_empty());
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

#[test]
fn complete_fifty_member_hunt_log_fits_the_party_editor() -> TestResult {
    use std::fmt::Write;
    let (mut app, _) = app()?;
    select(&mut app, Tool::Party)?;
    let mut log = String::from("Session data: Party Hunt\n");
    for index in 0..50 {
        writeln!(
            log,
            "Member{index:02}{}\n Loot: 1,000,000,000\n Supplies: 1,000,000,000\n Balance: 0\n Damage: 1,000,000,000\n Healing: 1,000,000,000",
            "a".repeat(55)
        )?;
    }
    assert!(log.len() > 8192);
    let (_, mut editor) = app
        .world_mut()
        .query::<(&ConfigField, &mut bevy::text::EditableText)>()
        .iter_mut(app.world_mut())
        .find(|(f, _)| f.0 == 3)
        .ok_or("log")?;
    assert!(
        editor
            .max_characters
            .is_none_or(|limit| limit >= log.chars().count())
    );
    *editor = bevy::text::EditableText::new(log);
    click_caption(&mut app, "Calculate split")?;
    app.update();
    assert!(report(&mut app)?.contains("Member49"));
    Ok(())
}

#[test]
fn xp_and_impact_keep_the_selected_report_language() -> TestResult {
    use oteryn_client_ui::localization::{Language, with_language};
    for (tool, title) in [
        (Tool::Xp, "Análisis de XP"),
        (Tool::Impact, "Análisis de impacto"),
    ] {
        let (mut app, _) = app()?;
        select(&mut app, tool)?;
        with_language(Language::Es, || app.update());
        assert!(report(&mut app)?.contains(title));
    }
    Ok(())
}

#[test]
fn loot_visibility_does_not_change_the_consolidated_hunt() -> TestResult {
    let (mut app, _) = app()?;
    select(&mut app, Tool::Loot)?;
    click_caption(&mut app, "Loot rate graph")?;
    app.update();
    assert!(!report(&mut app)?.contains("Per minute"));
    select(&mut app, Tool::Hunting)?;
    let text = report(&mut app)?;
    let loot = text
        .split("Loot Analyser")
        .nth(1)
        .ok_or("loot")?
        .split("Supply Analyser")
        .next()
        .ok_or("supply")?;
    assert!(loot.contains("Per minute"));
    assert!(!loot.contains("Loot/h target:"));
    Ok(())
}

#[test]
fn failed_high_reset_is_retried_even_without_another_impact() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (mut app, _) = app_at(Some(folder.path().join("client.json")))?;
    select(&mut app, Tool::Impact)?;
    let path = app
        .world()
        .non_send::<HostOwner>()
        .dock_layout_path()
        .ok_or("profile")?
        .with_extension("analyser-highs.json");
    std::fs::create_dir_all(path.parent().ok_or("profile directory")?)?;
    std::fs::write(&path, "malformed")?;
    click_caption(&mut app, "Reset all-time high")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
    while app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .analyser_panel()
        .feedback
        != Some("Could not read or save analyser highs")
    {
        assert!(
            std::time::Instant::now() < deadline,
            "writer failure not observed"
        );
        app.update();
    }
    std::fs::write(&path, "[88,99]")?;
    while std::fs::read_to_string(&path)? != "[0,0]" {
        assert!(
            std::time::Instant::now() < deadline,
            "reset never persisted after recovery"
        );
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}

#[test]
fn refused_analyser_catalogue_page_retries_the_same_observed_items() -> TestResult {
    use crate::combat_input::PlayCommand;
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    let take_query = |app: &mut App| {
        let mut commands = app
            .world_mut()
            .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>();
        std::iter::from_fn(|| commands.try_recv().ok()).find_map(|command| match command {
            PlayCommand::Feature(query) if query.kind == 734 => Some(query.payload),
            _ => None,
        })
    };
    let payload = take_query(&mut app).ok_or("first query")?;
    outbox.bevy_analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 3,
        status: oteryn_session::CommandStatus::Rejected,
        payload: vec![],
    });
    app.update();
    assert!(take_query(&mut app).is_none());
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(3),
    ));
    app.update();
    assert_eq!(take_query(&mut app), Some(payload));
    Ok(())
}

use super::*;
#[test]
fn toolbar_registry_sorted() {
    assert!(TOOLBAR.windows(2).all(|pair| pair[0].0 < pair[1].0));
}
#[test]
fn toolbar_opens_window() -> Result<(), Box<dyn std::error::Error>> {
    for label in [
        "Analysers",
        "Battle",
        "Chat",
        "Inspection",
        "Inventory",
        "Loot Containers",
        "Market",
        "Minimap",
        "Notices",
        "Options",
        "Party",
        "Quest Log",
        "Skills",
        "Spellbook",
        "VIP",
    ] {
        let mut owner = HostOwner::new(None)?;
        owner.settings.preferences.visible_panels.clear();
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_hud_skills_fixture()?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let entry = TOOLBAR
            .iter()
            .find(|entry| entry.0 == label)
            .ok_or("missing toolbar entry")?;
        let mut app = App::new();
        app.init_resource::<toolbar::Manager>()
            .insert_non_send(owner);
        let button = app
            .world_mut()
            .spawn((entry.1, Node::default()))
            .observe(activate)
            .id();
        app.world_mut().trigger(Activate { entity: button });
        let owner = app.world().non_send::<HostOwner>();
        match entry.1 {
            Control::Panel(id) => assert!(owner.settings.preferences.panel_visible(id)),
            Control::Options => assert!(owner.draft().is_some()),
            Control::Analysers => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.analyser_panel().open)
            ),
            Control::Skills => assert!(commands.try_recv().is_ok()),
            Control::Inspection => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.inspection_panel().open)
            ),
            Control::Loot => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.discovery_panel().loot.open)
            ),
            Control::Notices => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.notices_panel().open)
            ),
            Control::Market => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.market_panel().open)
            ),
            Control::Party => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.party_is_open())
            ),
            Control::Quests => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.quest_window_open())
            ),
            Control::Spells => assert!(
                owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.spellbook_window_open())
            ),
            _ => return Err("unexpected toolbar action".into()),
        }
    }
    Ok(())
}

#[test]
fn toolbar_covers_existing_keyboard_windows() {
    for label in [
        "Cyclopedia",
        "Daily Rewards",
        "Forge",
        "Highscores",
        "Imbuing",
        "NPC Trade",
        "Outfit",
        "Prey",
        "Report Bug",
        "Report Player",
        "Stash",
        "Store",
        "Task Board",
        "Weapon Proficiency",
        "Wheel",
    ] {
        assert!(
            TOOLBAR.iter().any(|entry| entry.0 == label),
            "missing {label}"
        );
    }
}

#[test]
fn toolbar_shortcuts_activate_existing_window_owners() -> Result<(), Box<dyn std::error::Error>> {
    for &(label, control) in TOOLBAR {
        let Control::Shortcut(id) = control else {
            continue;
        };
        let mut owner = HostOwner::new(None)?;
        let (link, _commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        // Capability admission is required; no report response payload is fabricated.
        if id == "report" {
            outbox.bevy_report_selected(true);
        }
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let mut app = App::new();
        app.init_resource::<toolbar::Manager>()
            .insert_non_send(owner);
        let button = app
            .world_mut()
            .spawn((control, Node::default()))
            .observe(activate)
            .id();
        app.world_mut().trigger(Activate { entity: button });
        if id == "map" || id == "houses" {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            assert!(if id == "map" {
                owner.take_satellite_request()
            } else {
                owner.take_house_request()
            });
            continue;
        }
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let open = match id {
            "bars" => !owner.settings.preferences.action_bar.visible,
            "cyclopedia" => link.cyclopedia_character_panel().open,
            "rewards" => link.daily_reward_panel().open,
            "forge" => link.item_forge_panel().open,
            "outfit" => link.appearance_panel().open,
            "prey" => link.prey_panel().open,
            "proficiency" => link.weapon_proficiency_is_open(),
            "wheel" => link.wheel_panel().open,
            "highscores" => owner.controls.highscores_generation == Some(owner.generation()),
            "imbuing" => link.imbue_panel().open,
            "stash" => link.stash_panel().open,
            "store" => link.store_panel().open,
            "tasks" => link.task_board_panel().open,
            "bug" => link.player_report_panel().bug.open,
            "report" => link.player_report_panel().open,
            // Without a selected NPC the keyboard owner must not invent one.
            "npc" => !link.npc_dialogue_is_visible(),
            _ => false,
        };
        assert!(open, "{label} did not reach its window owner");
    }
    Ok(())
}

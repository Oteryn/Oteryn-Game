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
        "Store Inbox",
        "VIP",
    ] {
        let mut owner = HostOwner::new(None)?;
        owner.settings.preferences.visible_panels.clear();
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_hud_skills_fixture()?;
        outbox.bevy_cyclopedia_initialize(true, false, false);
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let entry = TOOLBAR
            .iter()
            .find(|entry| entry.0 == label)
            .ok_or("missing toolbar entry")?;
        let mut app = App::new();
        app.insert_non_send(owner);
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
            Control::StoreInbox => {
                let play = owner.play.as_ref().ok_or("missing play")?;
                assert!(play.link.cyclopedia_character_panel().items.open);
                let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()?
                else {
                    return Err("expected remote inbox summary request".into());
                };
                assert_eq!((command.capability, command.kind), (199, 388));
                assert_eq!(command.payload, [130, 0, 0]);
            }
            _ => return Err("unexpected toolbar action".into()),
        }
    }
    Ok(())
}

#[test]
fn store_inbox_has_remote_toolbar_entry() {
    assert!(TOOLBAR.iter().any(|entry| entry.0 == "Store Inbox"));
}

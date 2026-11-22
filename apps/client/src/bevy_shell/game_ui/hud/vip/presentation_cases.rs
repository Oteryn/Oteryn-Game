#[test]
fn vip_default_hidden_sidebar_is_reachable_through_real_host_ctrl_p_and_region_projection()
-> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    app.add_systems(
        Update,
        crate::bevy_shell::game_ui::projection::project_regions,
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .use_shared_frontend();
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let vip = app
        .world()
        .resource::<crate::bevy_shell::game_ui::GameRegions>()
        .vip;
    assert_eq!(
        app.world().get::<Node>(vip).ok_or("VIP region")?.display,
        Display::None
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .vip_is_open()
    );
    // Global's ToggleVipWidget chord is Ctrl+P.
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(19)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        }],
        false,
    );
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .vip_is_open()
    );
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.update();
    assert_eq!(
        app.world().get::<Node>(vip).ok_or("VIP region")?.display,
        Display::Flex
    );
    assert_eq!(app.world_mut().query::<&Row>().iter(app.world()).count(), 2);
    click(&mut app, |action| matches!(action, Action::Add))?;
    assert!(app.world().resource::<State>().draft.is_some());
    Ok(())
}

#[test]
fn vip_close_hides_a_saved_visible_sidebar_and_preserves_a_pending_canonical_query() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    app.add_systems(
        Update,
        crate::bevy_shell::game_ui::projection::project_regions,
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .visible_panels
        .push("vip".into());
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let region = app
        .world()
        .resource::<crate::bevy_shell::game_ui::GameRegions>()
        .vip;
    assert_eq!(
        app.world().get::<Node>(region).ok_or("VIP region")?.display,
        Display::Flex
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .vip_request(Intent::Query(0));
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    click(&mut app, |action| matches!(action, Action::Close))?;
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("vip")
    );
    assert_eq!(
        app.world().get::<Node>(region).ok_or("VIP region")?.display,
        Display::None
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(!link.vip_is_open());
    assert!(link.vip_panel().pending);
    assert!(
        link.vip_panel()
            .view
            .as_ref()
            .is_some_and(|raw| raw == CONTACTS)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn vip_double_click_and_secondary_message_open_private_conversation() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, _, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    app.update();
    let row = app
        .world_mut()
        .query::<(Entity, &Row)>()
        .iter(app.world())
        .find_map(|(entity, row)| (row.entry == 73).then_some(entity))
        .ok_or("contact row")?;
    let click = |entity, button, count| PointerClick {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        ),
        button,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count,
    };
    app.world_mut()
        .trigger(click(row, PointerButton::Primary, 1));
    app.update();
    assert!(app.world().resource::<State>().draft.is_none());
    app.world_mut()
        .trigger(click(row, PointerButton::Primary, 2));
    app.update();
    assert!(app.world().resource::<State>().draft.is_none());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .current,
        Some(oteryn_client_ui::chat::ChannelTab::Conversation(
            "Alice".into()
        ))
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_console_tab(oteryn_client_ui::chat::ConsoleTab::All);
    app.world_mut()
        .trigger(click(row, PointerButton::Secondary, 1));
    app.update();
    let message = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "vip.message").then_some(entity))
        .ok_or("Message context action")?;
    app.world_mut().trigger(Activate { entity: message });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .current,
        Some(oteryn_client_ui::chat::ChannelTab::Conversation(
            "Alice".into()
        ))
    );
    Ok(())
}

#[test]
fn vip_rows_project_saved_icon_and_description() -> TestResult {
    let (mut app, _, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    app.update();
    let entries = app
        .world_mut()
        .query::<(&Name, &Text)>()
        .iter(app.world())
        .map(|(name, text)| (name.as_str().to_owned(), text.0.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(
        entries
            .get("vip.icon.73")
            .is_some_and(|icon| !icon.is_empty())
    );
    assert_eq!(
        entries.get("vip.description.73").map(String::as_str),
        Some("Friend")
    );
    let row = app
        .world_mut()
        .query::<(Entity, &Row)>()
        .iter(app.world())
        .find_map(|(entity, row)| (row.entry == 73).then_some(entity))
        .ok_or("Alice")?;
    app.world_mut()
        .entity_mut(row)
        .insert(bevy::picking::hover::Hovered(true));
    let tooltip = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "vip.description.73").then_some(entity))
        .ok_or("tooltip")?;
    for (enabled, delay, elapsed_ms, expected) in [
        (false, 0, 0, Display::None),
        (true, 1000, 0, Display::None),
        (true, 1000, 999, Display::None),
        (true, 1000, 1, Display::Flex),
        (true, 0, 0, Display::Flex),
    ] {
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(elapsed_ms),
        ));
        let prefs = &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences;
        prefs.tooltips_enabled = enabled;
        prefs.tooltip_delay_ms = delay;
        app.update();
        assert_eq!(
            app.world()
                .get::<Node>(tooltip)
                .ok_or("tooltip node")?
                .display,
            expected
        );
    }
    publish(
        &outbox,
        &CONTACTS.replace("Friend\"", "Hunting partner\""),
        5,
        VipPresenceStatus::Online,
    );
    app.update();
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Name, &Text)>()
            .iter(app.world())
            .any(
                |(name, text)| name.as_str() == "vip.description.73" && text.0 == "Hunting partner"
            )
    );
    Ok(())
}

#[test]
fn vip_presentation_controls_are_reachable_and_icons_are_visible() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Offline);
    app.update();
    for id in ["vip.sort", "vip.hide_offline"] {
        let entity = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, name)| (name.as_str() == id).then_some(entity))
            .ok_or("VIP presentation control missing")?;
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<List>>()
            .single(app.world())?;
        app.world_mut().trigger(secondary_click(list));
        app.update();
        app.world_mut().trigger(Activate { entity });
        app.update();
    }
    let alice = app
        .world_mut()
        .query::<(&Row, &Node)>()
        .iter(app.world())
        .find_map(|(row, node)| (row.entry == 73).then_some(node.display))
        .ok_or("Alice")?;
    assert_eq!(alice, Display::None);
    assert!(
        app.world_mut()
            .query::<(&Name, &ImageNode)>()
            .iter(app.world())
            .any(|(name, image)| name.as_str() == "vip.icon" && image.rect.is_some())
    );
    let icon = app
        .world_mut()
        .query::<(&presentation::Icon, &ImageNode)>()
        .iter(app.world())
        .find_map(|(icon, image)| (icon.0 == 73).then_some(image.clone()))
        .ok_or("Alice icon")?;
    assert_eq!(icon.rect, Some(Rect::new(24., 0., 36., 12.)));
    let images = app.world().resource::<Assets<Image>>();
    let rgba = images
        .get(&icon.image)
        .and_then(|image| image.data.as_ref())
        .ok_or("VIP artwork")?;
    assert!(
        (0..12).any(|y| (24..36).any(|x| rgba[(y * 240 + x) * 4 + 3] != 0)),
        "selected icon contains visible pixels"
    );
    Ok(())
}
fn row_order(app: &mut App) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let list = app
        .world_mut()
        .query_filtered::<&Children, With<List>>()
        .single(app.world())?;
    Ok(list
        .iter()
        .filter_map(|entity| app.world().get::<Row>(entity).map(|row| row.entry))
        .collect())
}
#[test]
fn vip_sorting_and_hide_offline_follow_live_presence_without_hiding_unknown() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(
        &outbox,
        &CONTACTS.replace("Alice", "Zelda"),
        4,
        VipPresenceStatus::Offline,
    );
    app.update();
    assert_eq!(
        row_order(&mut app)?,
        vec![91, 73],
        "unknown precedes confirmed offline"
    );
    click(&mut app, |action| matches!(action, Action::Sort))?;
    assert_eq!(row_order(&mut app)?, vec![73, 91], "icon 2 precedes icon 0");
    click(&mut app, |action| matches!(action, Action::Sort))?;
    assert_eq!(row_order(&mut app)?, vec![91, 73], "alphabetical names");
    click(&mut app, |action| matches!(action, Action::HideOffline))?;
    let visible = |app: &mut App, entry| {
        app.world_mut()
            .query::<(&Row, &Node)>()
            .iter(app.world())
            .any(|(row, node)| row.entry == entry && node.display != Display::None)
    };
    assert!(!visible(&mut app, 73));
    assert!(
        visible(&mut app, 91),
        "missing presence must not imply offline"
    );
    publish(
        &outbox,
        &CONTACTS.replace("Alice", "Zelda"),
        5,
        VipPresenceStatus::Online,
    );
    app.update();
    assert!(
        visible(&mut app, 73),
        "presence updates unhide online contacts"
    );
    Ok(())
}
#[test]
fn vip_context_message_remove_and_stale_actions_use_authoritative_owners() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    app.init_resource::<bevy::input_focus::InputFocus>()
        .add_plugins(bevy::ui_widgets::ButtonPlugin);
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    };
    let entity = control(&mut app, |action| matches!(action, Action::Edit(73)))?;
    app.world_mut()
        .trigger(bevy::picking::events::PointerPress {
            entity,
            count: 1,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Secondary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
    app.update();
    app.world_mut().trigger(PointerClick {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();

    assert!(
        app.world().resource::<State>().draft.is_none(),
        "secondary click must only open the context menu"
    );
    click(&mut app, |action| matches!(action, Action::Message(73)))?;
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .current,
        Some(oteryn_client_ui::chat::ChannelTab::Conversation(
            "Alice".into()
        ))
    );
    assert!(
        commands.try_recv().is_err(),
        "opening a conversation does not send a message"
    );
    click(&mut app, |action| matches!(action, Action::Context(73)))?;
    let stale = control(&mut app, |action| matches!(action, Action::Remove(73)))?;
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.world_mut().trigger(Activate { entity: stale });
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "old revision cannot remove a contact"
    );
    click(&mut app, |action| matches!(action, Action::Context(73)))?;
    click(&mut app, |action| matches!(action, Action::Remove(73)))?;
    let remove = request(&mut commands)?;
    assert_eq!(remove.operation, wire::Operation::Remove);
    assert_eq!(remove.number, 73);
    assert!(
        app.world_mut()
            .query::<&Row>()
            .iter(app.world())
            .any(|row| row.entry == 73),
        "removal waits for authoritative refresh"
    );
    Ok(())
}

#[test]
fn vip_sidebar_sends_incoming_consent_and_visibility_commands() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    for (revision, id, operation, number) in [
        (1, "vip.consent.accept", wire::Operation::Accept, 37),
        (2, "vip.consent.reject", wire::Operation::Reject, 37),
        (3, "vip.consent.revoke", wire::Operation::Revoke, 38),
        (4, "vip.presence.offline", wire::Operation::Status, 2),
        (5, "vip.presence.online", wire::Operation::Status, 1),
    ] {
        publish(&outbox, r#"{"entries":[],"groups":[],"shown":true,"consents":[{"handle":37,"name":"Bob","accepted":false},{"handle":38,"name":"Alice","accepted":true}]}"#, revision, VipPresenceStatus::Online);
        app.update();
        let entity = app.world_mut().query::<(Entity, &Name)>().iter(app.world())
            .find_map(|(entity, name)| (name.as_str() == id).then_some(entity))
            .ok_or(id)?;
        app.world_mut().trigger(Activate { entity });
        let command = request(&mut commands)?;
        assert_eq!((command.operation, command.number), (operation, number));
    }

    publish(&outbox, CONTACTS, 6, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Context(91)))?;
    let entity = app.world_mut().query::<(Entity, &Name)>().iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "vip.menu.withdraw").then_some(entity))
        .ok_or("withdraw control")?;
    app.world_mut().trigger(Activate { entity });
    let command = request(&mut commands)?;
    assert_eq!((command.operation, command.number), (wire::Operation::Withdraw, 91));
    Ok(())
}

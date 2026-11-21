#[test]
fn sent_chat_history_recall_restores_the_unsent_draft() -> TestResult {
    let (mut app, editor, mut commands) = app()?;
    super::super::set_text(
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .as_deref_mut()
            .ok_or("editor")?,
        "sent message",
    );
    app.update();
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(
        matches!(sent(&mut commands), Some(ChatIntent::Say { text, .. }) if text == "sent message")
    );
    super::super::set_text(
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .as_deref_mut()
            .ok_or("editor")?,
        "unsent draft",
    );
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp, None);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "sent message"
    );
    key(&mut app, KeyCode::ArrowDown, Key::ArrowDown, None);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "unsent draft"
    );
    app.world_mut().spawn((
        super::super::super::GameModal,
        GlobalZIndex(300),
        Node::default(),
    ));
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp, None);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "unsent draft"
    );
    Ok(())
}

#[test]
fn chat_transcript_preserves_retained_speech_colours() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    use oteryn_session::{
        ActorPosition, ChatLine, ChatRoom, ChatSpeaker, ChatSpeechMode, GuildChatLine,
    };
    let (mut app, _, _) = app()?;
    let (link, _, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        vec![
            ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: [1; 16],
                    generation: std::num::NonZeroU64::MIN,
                },
                speaker_name: "Alice".into(),
                mode: ChatSpeechMode::Say,
                text: "local words".into(),
                position: ActorPosition {
                    x: 100,
                    y: 100,
                    floor: 0,
                },
            },
            ChatLine::Private {
                speaker_name: "Bob".into(),
                text: "private words".into(),
            },
            ChatLine::Guild(GuildChatLine {
                guild_id: [1; 16],
                rank: 1,
                speaker_name: "Carol".into(),
                text: "guild words".into(),
            }),
            ChatLine::Room {
                room: ChatRoom::Help,
                speaker_name: "Dave".into(),
                text: "room words".into(),
            },
        ],
    );
    outbox.bevy_chat_guild_member();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    app.world_mut()
        .run_system_once(crate::bevy_shell::ui::setup)
        .map_err(|error| format!("{error:?}"))?;
    let root = app.world_mut().spawn(Node::default()).id();
    app.insert_resource(GameRegions {
        vitals: root,
        minimap: root,
        equipment: root,
        containers: root,
        battle: root,
        skills: root,
        vip: root,
        analyzers: root,
        chat: root,
        bars: root,
        world: root,
    });
    app.world_mut()
        .run_system_once(widgets::setup)
        .map_err(|error| format!("{error:?}"))?;
    app.add_systems(Update, project);
    app.update();
    for (message, colour) in [
        ("local words", Color::srgb_u8(255, 255, 0)),
        ("private words", Color::srgb_u8(95, 195, 195)),
        ("guild words", Color::srgb_u8(255, 165, 0)),
        ("room words", Color::srgb_u8(255, 255, 0)),
    ] {
        assert!(
            app.world_mut()
                .query::<(&Text, &TextColor)>()
                .iter(app.world())
                .any(|(text, color)| text.0.contains(message) && color.0 == colour),
            "{message} needs its retained speech colour"
        );
    }
    Ok(())
}

#[test]
fn sent_history_shift_arrows_restore_the_unsent_draft() -> TestResult {
    let (mut app, editor, mut commands) = app()?;
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(sent(&mut commands).is_some());
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .editor_mut()
        .set_text("unsent draft");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp, None);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "hell"
    );
    key(&mut app, KeyCode::ArrowDown, Key::ArrowDown, None);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "unsent draft"
    );
    Ok(())
}

#[test]
fn chat_console_uses_tibia_tab_and_input_geometry() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, _, _) = app()?;
    app.world_mut()
        .run_system_once(crate::bevy_shell::ui::setup)
        .map_err(|error| format!("{error:?}"))?;
    let root = app.world_mut().spawn(Node::default()).id();
    app.insert_resource(GameRegions {
        vitals: root,
        minimap: root,
        equipment: root,
        containers: root,
        battle: root,
        skills: root,
        vip: root,
        analyzers: root,
        chat: root,
        bars: root,
        world: root,
    });
    app.world_mut()
        .run_system_once(widgets::setup)
        .map_err(|error| format!("{error:?}"))?;
    let world = app.world_mut();
    let size = |node: &Node| (node.width, node.height);
    let mut controls = world.query::<(&Control, &Node)>();
    let tab_buttons: Vec<_> = controls
        .iter(world)
        .filter(|(control, _)| matches!(control.action, Action::PreviousTab | Action::NextTab))
        .map(|(_, node)| size(node))
        .collect();
    assert_eq!(tab_buttons, [(px(18.), px(18.)); 2]);
    let mut chat_mode = world.query_filtered::<&Node, With<ChatModeButton>>();
    assert_eq!(chat_mode.single(world).map(size)?, (px(64.), px(16.)));
    let mut rows = world.query_filtered::<(&Node, &Children), With<ComposeRow>>();
    let (row, children) = rows.single(world)?;
    assert_eq!(row.height, px(16.));
    assert_eq!(row.column_gap, px(2.));
    // Chat On/Off is the last control of the input row; Enter sends, so no Send button.
    let last = children.last().copied().ok_or("compose row")?;
    assert!(world.get::<ChatModeButton>(last).is_some());
    let mut texts = world.query::<&Text>();
    assert!(!texts.iter(world).any(|text| text.0 == "Send"));
    Ok(())
}
#[test]
fn say_mode_button_cycles_say_whisper_yell_with_art() {
    use oteryn_client_ui::chat::Mode;
    assert_eq!(widgets::next_mode(Mode::Say), Mode::Whisper);
    assert_eq!(widgets::next_mode(Mode::Whisper), Mode::Yell);
    assert_eq!(widgets::next_mode(Mode::Yell), Mode::Say);
    assert_eq!(
        widgets::mode_art(Mode::Yell),
        crate::bevy_shell::gameplay_dialogs::art::ArtSlot::ChatYell
    );
}
#[test]
fn tab_in_the_focused_input_cycles_channels_and_keeps_the_text() -> TestResult {
    use oteryn_client_ui::chat::ChannelTab;
    let (mut app, editor, _) = app()?;
    app.world_mut()
        .entity_mut(editor)
        .observe(widgets::tab_cycle);
    let tab = |app: &App| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .tab_index())
    };
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.chat_open_entry(&ChannelEntry::Local(ChannelTab::ServerLog));
        link.chat_console_tab(ConsoleTab::All);
    }
    let before = tab(&app)?;
    key(&mut app, KeyCode::Tab, Key::Tab, None);
    app.update();
    let after = tab(&app)?;
    assert_ne!(after, before);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(editor),
        "Tab must not move the focus out of the chat input"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "hell"
    );
    Ok(())
}
#[test]
fn empty_unfocused_input_shows_the_placeholder() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, editor, _) = app()?;
    let hint = app
        .world_mut()
        .spawn((widgets::Placeholder, Visibility::Inherited, Text::default()))
        .id();
    let visibility = |app: &mut App| -> Result<Visibility, Box<dyn std::error::Error>> {
        app.world_mut()
            .run_system_once(widgets::placeholder)
            .map_err(|error| format!("{error:?}"))?;
        Ok(*app.world().get::<Visibility>(hint).ok_or("hint")?)
    };
    assert_eq!(visibility(&mut app)?, Visibility::Hidden);
    app.world_mut().resource_mut::<InputFocus>().clear();
    assert_eq!(visibility(&mut app)?, Visibility::Hidden, "text is present");
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Delete);
    app.update();
    app.world_mut().resource_mut::<InputFocus>().clear();
    assert_eq!(visibility(&mut app)?, Visibility::Inherited);
    let caption = |app: &App| app.world().get::<Text>(hint).map(|text| text.0.clone());
    assert_eq!(caption(&app).as_deref(), Some("Type here to chat"));
    let set = |app: &App, mode| -> TestResult {
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_set_mode(mode);
        Ok(())
    };
    set(&app, oteryn_client_ui::chat::ComposerMode::Auto)?;
    assert_eq!(visibility(&mut app)?, Visibility::Inherited);
    assert_eq!(caption(&app).as_deref(), Some("Click here to chat"));
    set(&app, oteryn_client_ui::chat::ComposerMode::Off)?;
    assert_eq!(
        visibility(&mut app)?,
        Visibility::Hidden,
        "chat Off shows no hint"
    );
    Ok(())
}
#[test]
fn selected_tab_caption_is_white_and_the_others_grey() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, _, _) = app()?;
    let caption = |checked: bool, app: &mut App| {
        let caption = app
            .world_mut()
            .spawn((source::ButtonCaption, TextColor(Color::BLACK)))
            .id();
        let mut tab = app.world_mut().spawn(Control {
            action: Action::Tab(commands::Tab::Fixed(ConsoleTab::All)),
            generation: 0,
        });
        if checked {
            tab.insert(bevy::ui::Checked);
        }
        tab.add_child(caption);
        caption
    };
    let active = caption(true, &mut app);
    let inactive = caption(false, &mut app);
    app.world_mut()
        .run_system_once(widgets::tab_colors)
        .map_err(|error| format!("{error:?}"))?;
    let color = |entity| app.world().get::<TextColor>(entity).map(|color| color.0);
    assert_eq!(color(active), Some(widgets::ACTIVE_TAB));
    assert_eq!(color(inactive), Some(widgets::INACTIVE_TAB));
    Ok(())
}
#[test]
fn chat_auto_enter_sends_and_returns_the_keys_to_the_game() -> TestResult {
    let (mut app, _editor, mut commands) = app()?;
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.chat_set_mode(oteryn_client_ui::chat::ComposerMode::Auto);
        link.chat_auto_focus(true);
    }
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(matches!(sent(&mut commands), Some(ChatIntent::Say { text, .. }) if text == "hell"));
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    Ok(())
}
#[test]
fn chat_auto_click_outside_the_input_releases_the_keyboard() -> TestResult {
    use bevy::{ecs::system::RunSystemOnce, picking::hover::Hovered};
    let (mut app, editor, _) = app()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_set_mode(oteryn_client_ui::chat::ComposerMode::Auto);
    let area = app
        .world_mut()
        .spawn((widgets::InputArea, Hovered(true)))
        .id();
    let mut buttons = ButtonInput::<MouseButton>::default();
    buttons.press(MouseButton::Left);
    app.insert_resource(buttons);
    let release = |app: &mut App| -> TestResult {
        app.world_mut()
            .run_system_once(widgets::auto_release)
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    };
    release(&mut app)?;
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(editor),
        "a click into the input keeps it"
    );
    app.world_mut().entity_mut(area).insert(Hovered(false));
    release(&mut app)?;
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        None,
        "a click on the map gives the keys back"
    );
    Ok(())
}

#[test]
fn focused_tab_cycles_once_across_bevy_and_native_input() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode as NativeKey, Modifiers, NormalizedInputEvent};
    let (mut app, editor, _) = app()?;
    app.world_mut()
        .entity_mut(editor)
        .observe(widgets::tab_cycle);
    let tab_index = |owner: &HostOwner| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(owner.play.as_ref().ok_or("play")?.link.chat_panel().console.tab_index())
    };
    let before = {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.chat_open_conversation("Alice"));
        link.chat_console_tab(ConsoleTab::All);
        tab_index(owner)?
    };
    key(&mut app, KeyCode::Tab, Key::Tab, None);
    app.update();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    crate::bevy_shell::input::route_keyboard(
        &mut owner,
        NormalizedInputEvent::Key {
            code: NativeKey::new(43)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        },
        false,
        Some(true),
    );
    assert_eq!(tab_index(&owner)?, before + 1);
    Ok(())
}

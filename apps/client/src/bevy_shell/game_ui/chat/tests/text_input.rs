use super::super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputDispatchPlugin,
    text::{RemSize, TextPlugin},
    ui_widgets::{TextInput, TextInputPlugin},
    window::WindowPlugin,
};
use oteryn_session::{ChatIntent, ChatRoomSet};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    Entity,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
);
fn app() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        vec![],
    );
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let generation = owner.generation();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        InputDispatchPlugin,
        WindowPlugin::default(),
        TextPlugin,
        TextInputPlugin,
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<UiScale>()
    .init_resource::<RemSize>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .insert_non_send(owner)
    .insert_resource(ChatState {
        generation: Some(generation),
        ..default()
    })
    .add_systems(Update, keyboard)
    .add_systems(
        PostUpdate,
        submission::finish.after(bevy::text::EditableTextSystems),
    )
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    install_history(&mut app);
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/assets/FiraMono-subset.ttf"
            ))
            .to_vec(),
        ));
    let editor = app
        .world_mut()
        .spawn((
            Field::Message,
            TextInput,
            EditableText::new("hell"),
            source::font(font, false),
            Node::default(),
        ))
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::TextEnd(false));
    app.update();
    Ok((app, editor, commands))
}
fn key(app: &mut App, code: KeyCode, logical_key: Key, text: Option<&str>) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}
fn sent(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>) -> Option<ChatIntent> {
    std::iter::from_fn(|| commands.try_recv().ok()).find_map(|command| {
        if let PlayCommand::ChatPublic(intent) = command {
            Some(intent)
        } else {
            None
        }
    })
}
#[test]
fn last_character_and_enter_in_one_frame_send_the_completed_editor() -> TestResult {
    let (mut app, _editor, mut commands) = app()?;
    key(
        &mut app,
        KeyCode::KeyO,
        Key::Character("o".into()),
        Some("o"),
    );
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(matches!(sent(&mut commands), Some(ChatIntent::Say { text, .. }) if text == "hello"));
    assert_eq!(
        app.world()
            .resource::<ChatState>()
            .history
            .lines
            .back()
            .map(String::as_str),
        Some("hello")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn enter_waits_for_async_paste_without_sending_the_old_draft() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, editor, mut commands) = app()?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "pending clipboard must not send the previous draft"
    );
    *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("pasted message".to_owned()));
    app.update();
    assert!(
        matches!(sent(&mut commands), Some(ChatIntent::Say { text, .. }) if text == "pasted message")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn enter_waits_for_queued_editor_edits() -> TestResult {
    let (mut app, editor, mut commands) = app()?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("o".into()));
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert!(matches!(sent(&mut commands), Some(ChatIntent::Say { text, .. }) if text == "hello"));
    Ok(())
}
#[test]
fn recipient_enter_and_button_wait_for_the_completed_name() -> TestResult {
    for button in [false, true] {
        let (mut app, editor, _commands) = app()?;
        app.world_mut().entity_mut(editor).insert(Field::Recipient);
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_channel_dialog_open();
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .queue_edit(bevy::text::TextEdit::Insert("o".into()));
        if button {
            let generation = app.world().non_send::<HostOwner>().generation();
            let send = app
                .world_mut()
                .spawn(Control {
                    action: Action::Conversation,
                    generation,
                })
                .observe(widgets::activate)
                .id();
            app.world_mut()
                .trigger(bevy::ui_widgets::Activate { entity: send });
        } else {
            key(&mut app, KeyCode::Enter, Key::Enter, None);
        }
        app.update();
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
                "hello".into()
            ))
        );
    }
    Ok(())
}
#[test]
fn pending_send_cancels_when_destination_or_modal_scope_changes() -> TestResult {
    use std::sync::{Arc, Mutex};
    for scope in 0..3 {
        let (mut app, editor, mut commands) = app()?;
        let pending = Arc::new(Mutex::new(None));
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
            &pending,
        )));
        key(&mut app, KeyCode::Enter, Key::Enter, None);
        app.update();
        if scope == 1 {
            app.world_mut().spawn((
                super::super::super::GameModal,
                GlobalZIndex(200),
                Node::default(),
            ));
        } else if scope == 2 {
            app.world_mut().non_send_mut::<HostOwner>().close();
        } else {
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .chat_console_tab(ConsoleTab::Local);
        }
        *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("later text".into()));
        app.update();
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "a pending submit must not follow a different destination or modal"
        );
        assert!(app.world().resource::<ChatState>().pending.is_none());
    }
    Ok(())
}

#[test]
fn transcript_supports_read_only_selection_and_copy() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::text::TextReadWriteMode;
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
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Transcript>>()
        .single(app.world())?;
    assert_eq!(
        app.world().get::<TextReadWriteMode>(entity),
        Some(&TextReadWriteMode::ReadOnly)
    );
    assert!(app.world().get::<TextInput>(entity).is_some());
    assert!(app.world().get::<EditableText>(entity).is_some());
    assert_ne!(app.world().get::<Pickable>(entity), Some(&Pickable::IGNORE));
    Ok(())
}

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

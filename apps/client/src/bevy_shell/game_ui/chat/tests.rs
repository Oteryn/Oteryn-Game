use super::commands::{Tab, select, submit};
use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_ui::chat::{ChannelTab, ConsoleTab, Mode};
use oteryn_session::{ChatIntent, ChatRoom, ChatRoomSet, ChatSpeechMode};

#[test]
fn composer_sends_once_through_the_real_session_owner() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    assert!(submit(&link, 7, 7, Mode::Whisper, "hello"));
    let sent = std::iter::from_fn(|| commands.try_recv().ok())
        .find_map(|command| match command {
            PlayCommand::ChatPublic(intent) => Some(intent),
            _ => None,
        })
        .ok_or("no chat command")?;
    assert_eq!(
        sent,
        ChatIntent::Say {
            mode: ChatSpeechMode::Whisper,
            text: "hello".into()
        }
    );
    assert!(!submit(&link, 7, 7, Mode::Say, "duplicate"));
    assert!(commands.try_recv().is_err());
    assert!(
        link.chat_panel().lines.is_empty(),
        "outgoing text is never faked into server history"
    );
    Ok(())
}

#[test]
fn composer_rejects_old_session_and_preserves_rejected_draft() {
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    link.chat_text("current session draft".into());
    assert!(!submit(&link, 1, 2, Mode::Say, "old session"));
    assert_eq!(link.chat_panel().text, "current session draft");
    assert!(commands.try_recv().is_err());
    assert!(!submit(&link, 2, 2, Mode::Say, " "));
    assert_eq!(link.chat_panel().text, " ");
    assert!(link.chat_panel().feedback.is_some());
}

#[test]
fn selected_channel_controls_the_real_command_destination() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    let mut rooms = ChatRoomSet::default();
    rooms.insert(ChatRoom::Help);
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(&outbox, rooms, Vec::new());
    select(&link, &Tab::Channel(ChannelTab::Room(ChatRoom::Help)));
    assert!(submit(&link, 3, 3, Mode::Say, "help message"));
    let sent = std::iter::from_fn(|| commands.try_recv().ok())
        .find_map(|command| match command {
            PlayCommand::ChatPublic(intent) => Some(intent),
            _ => None,
        })
        .ok_or("no room message")?;
    assert_eq!(
        sent,
        ChatIntent::Room {
            room: ChatRoom::Help,
            text: "help message".into()
        }
    );
    select(&link, &Tab::Fixed(ConsoleTab::Local));
    assert_eq!(link.chat_panel().console.current, None);
    assert_eq!(link.chat_panel().console.tab, ConsoleTab::Local);
    Ok(())
}

#[test]
fn native_activate_sends_the_visible_editor_through_the_existing_owner()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{Action, ChatState, Control, Field, HostOwner, widgets};
    use bevy::{prelude::*, text::EditableText, ui_widgets::Activate};
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let generation = host.generation();
    let mut app = App::new();
    app.init_resource::<bevy::input_focus::InputFocus>()
        .insert_non_send(host)
        .insert_resource(ChatState {
            generation: Some(generation),
            ..Default::default()
        });
    let editor = app
        .world_mut()
        .spawn((Field::Message, EditableText::new("visible typed message")))
        .id();
    let send = app
        .world_mut()
        .spawn(Control {
            action: Action::Send,
            generation,
        })
        .observe(widgets::activate)
        .id();
    app.world_mut().trigger(Activate { entity: send });
    let sent = std::iter::from_fn(|| commands.try_recv().ok())
        .find_map(|command| match command {
            PlayCommand::ChatPublic(intent) => Some(intent),
            _ => None,
        })
        .ok_or("Activate did not reach session command")?;
    assert!(matches!(sent, ChatIntent::Say { text, .. } if text == "visible typed message"));
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        ""
    );
    assert_eq!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        Some(editor)
    );
    // Replace the session, so the old session's pending command cannot hide a missing fence.
    let (next_link, mut next_commands, next_outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &next_outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = next_link;
    super::set_text(
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .as_deref_mut()
            .ok_or("editor")?,
        "stale draft",
    );
    app.world_mut().resource_mut::<ChatState>().generation = Some(generation.saturating_add(1));
    app.world_mut().trigger(Activate { entity: send });
    assert!(
        next_commands.try_recv().is_err(),
        "stale visible controls must not send"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "stale draft"
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .text
            .is_empty()
    );
    Ok(())
}

#[test]
fn native_composer_mode_button_cycles_the_existing_console_owner()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{Action, ChatModeButton, ChatState, Control, Field, HostOwner, widgets};
    use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
    use oteryn_client_ui::chat::ComposerMode;
    let (link, _commands, _outbox) = play_channel();
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let generation = host.generation();
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .insert_non_send(host)
        .insert_resource(ChatState {
            generation: Some(generation),
            ..default()
        });
    let editor = app
        .world_mut()
        .spawn((Field::Message, EditableText::new("unsent draft")))
        .id();
    let button = app
        .world_mut()
        .spawn((
            ChatModeButton,
            Control {
                action: Action::ComposerMode,
                generation,
            },
        ))
        .observe(widgets::activate)
        .id();
    app.world_mut().trigger(Activate { entity: button });
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .mode,
        ComposerMode::Off
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().trigger(Activate { entity: button });
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .mode,
        ComposerMode::Temporary
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
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
fn foreground_game_modal_blocks_background_chat_activate() -> Result<(), Box<dyn std::error::Error>>
{
    use super::{Action, ChatState, Control, Field, HostOwner, widgets};
    use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let generation = host.generation();
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .insert_non_send(host)
        .insert_resource(ChatState {
            generation: Some(generation),
            ..default()
        });
    app.world_mut().spawn((
        crate::bevy_shell::game_ui::GameModal,
        GlobalZIndex(200),
        Node::default(),
    ));
    let editor = app
        .world_mut()
        .spawn((
            Field::Message,
            EditableText::new("must not send behind modal"),
        ))
        .id();
    let send = app
        .world_mut()
        .spawn(Control {
            action: Action::Send,
            generation,
        })
        .observe(widgets::activate)
        .id();
    app.world_mut().trigger(Activate { entity: send });
    assert!(commands.try_recv().is_err());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .pending
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "must not send behind modal"
    );
    Ok(())
}

#[test]
fn foreground_game_modal_blocks_enter_from_previous_composer_focus()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{ChatState, Field, HostOwner, keyboard};
    use bevy::{
        input::keyboard::{Key, KeyboardInput},
        input_focus::{FocusCause, InputFocus},
        prelude::*,
        text::EditableText,
    };
    let (link, mut commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let generation = host.generation();
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .add_message::<KeyboardInput>()
        .insert_non_send(host)
        .insert_resource(ChatState {
            generation: Some(generation),
            ..default()
        })
        .add_systems(Update, keyboard);
    let editor = app
        .world_mut()
        .spawn((Field::Message, EditableText::new("must not send on Enter")))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.world_mut().spawn((
        crate::bevy_shell::game_ui::GameModal,
        GlobalZIndex(200),
        Node::default(),
    ));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .pending
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .value(),
        "must not send on Enter"
    );
    Ok(())
}

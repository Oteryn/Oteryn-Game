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

/// Presses Enter once through the chat keyboard owner; Tibia Global has no Send button.
fn enter(app: &mut bevy::prelude::App) -> Result<(), Box<dyn std::error::Error>> {
    use bevy::{
        ecs::system::RunSystemOnce,
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        prelude::*,
    };
    if !app.world().contains_resource::<Messages<KeyboardInput>>() {
        app.add_message::<KeyboardInput>();
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.world_mut()
        .run_system_once(super::keyboard)
        .map_err(|error| format!("chat keyboard: {error:?}"))?;
    Ok(())
}

#[test]
fn enter_sends_the_visible_editor_through_the_existing_owner()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{ChatState, Field, HostOwner};
    use bevy::{prelude::*, text::EditableText};
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
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    enter(&mut app)?;
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
    enter(&mut app)?;
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
fn foreground_game_modal_blocks_background_chat_enter() -> Result<(), Box<dyn std::error::Error>> {
    use super::{ChatState, Field, HostOwner};
    use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};
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
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    enter(&mut app)?;
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

mod text_input;

#[test]
fn chat_input_sends_typing() -> Result<(), Box<dyn std::error::Error>> {
    use super::{ChatState, Field, HostOwner};
    use bevy::{
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
    crate::play::feature_0233_player_typing::protocol::bevy_typing_fixture(&outbox);
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let generation = host.generation();
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<InputFocus>()
        .insert_resource(ChatState {
            generation: Some(generation),
            ..default()
        })
        .insert_resource(crate::bevy_shell::ui::Skins {
            buttons: oteryn_client_ui_bevy::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        })
        .add_systems(Update, super::project);
    let editor = app
        .world_mut()
        .spawn((Field::Message, EditableText::new("")))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.update();
    assert!(commands.try_recv().is_err());
    for (text, expected) in [
        ("hello", Some(true)),
        ("hello again", None),
        ("", Some(false)),
    ] {
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .editor_mut()
            .set_text(text);
        app.update();
        if let Some(typing) = expected {
            assert!(
                matches!(commands.try_recv()?, PlayCommand::PlayerTyping(command) if command.typing == typing)
            );
        }
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn message_menu_mirrors_the_selected_tab_read_only_and_closes_it()
-> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use bevy::{
        asset::AssetPlugin,
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerButton, PointerId},
        },
        text::TextReadWriteMode,
        ui_widgets::Activate,
    };
    let (link, _, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        vec![oteryn_session::ChatLine::Private {
            speaker_name: "Sender".into(),
            text: "select this message".into(),
        }],
    );
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(owner)
        .init_resource::<InputFocus>()
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        );
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    install(&mut app);
    app.update();
    app.world_mut()
        .query_filtered::<&mut Node, With<super::super::GameRoot>>()
        .single_mut(app.world_mut())?
        .display = Display::Flex;
    let action = |app: &mut App, label: &str| -> Result<(), Box<dyn std::error::Error>> {
        let row = app
            .world_mut()
            .query::<(Entity, &EditableText, &TextReadWriteMode)>()
            .iter(app.world())
            .find_map(|(entity, text, mode)| {
                (text.value().to_string().contains("select this message")
                    && *mode == TextReadWriteMode::ReadOnly)
                    .then_some(entity)
            })
            .ok_or("selectable message row missing")?;
        app.world_mut().trigger(PointerClick {
            entity: row,
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
        let entity = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == label).then_some(parent.parent()))
            .ok_or("message menu action missing")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    let pane = |app: &mut App| -> Result<(Display, String), Box<dyn std::error::Error>> {
        let display = app
            .world_mut()
            .query_filtered::<&Node, With<MirrorPane>>()
            .single(app.world())?
            .display;
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<MirrorText>>()
            .single(app.world())?
            .0
            .clone();
        Ok((display, text))
    };
    assert_eq!(pane(&mut app)?.0, Display::None);
    action(&mut app, "Mirror This Tab")?;
    let (display, text) = pane(&mut app)?;
    assert_eq!(display, Display::Flex);
    assert!(text.contains("select this message"));
    {
        let owner = app.world().non_send::<HostOwner>();
        let panel = owner.play.as_ref().ok_or("play")?.link.chat_panel();
        assert_eq!(panel.console.mirror, Some(panel.console.tab));
        assert_eq!(
            oteryn_client_ui::chat::mirror_text(&panel).as_deref(),
            Some(text.as_str())
        );
    }
    action(&mut app, "Close Mirrored Tab")?;
    assert_eq!(pane(&mut app)?, (Display::None, String::new()));
    Ok(())
}

#[test]
fn transcript_rows_are_selectable_and_offer_sender_actions()
-> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use bevy::{
        asset::AssetPlugin,
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerButton, PointerId},
        },
        text::TextReadWriteMode,
        ui_widgets::Activate,
    };
    let (link, _, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        vec![oteryn_session::ChatLine::Private {
            speaker_name: "Sender".into(),
            text: "select this message".into(),
        }],
    );
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(owner)
        .init_resource::<InputFocus>()
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        );
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    install(&mut app);
    app.update();
    app.world_mut()
        .query_filtered::<&mut Node, With<super::super::GameRoot>>()
        .single_mut(app.world_mut())?
        .display = Display::Flex;
    let action = |app: &mut App, label: &str| -> Result<(), Box<dyn std::error::Error>> {
        let row = app
            .world_mut()
            .query::<(Entity, &EditableText, &TextReadWriteMode)>()
            .iter(app.world())
            .find_map(|(entity, text, mode)| {
                (text.value().to_string().contains("select this message")
                    && *mode == TextReadWriteMode::ReadOnly)
                    .then_some(entity)
            })
            .ok_or("selectable message row missing")?;
        app.world_mut().trigger(PointerClick {
            entity: row,
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
        let entity = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == label).then_some(parent.parent()))
            .ok_or("message menu action missing")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    action(&mut app, "Communication Lists")?;
    action(&mut app, "Message to Sender")?;
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
            "Sender".into()
        ))
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let lists = crate::play::FeatureForm::all()
        .iter()
        .map(|form| link.feature_form_view(*form))
        .find(|view| view.title == "Communication lists")
        .ok_or("communication lists form missing")?;
    assert!(lists.open);
    assert_eq!(lists.fields, vec!["Sender".to_owned()]);
    Ok(())
}

#[test]
fn speech_colors_survive_message_projection() -> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use bevy::asset::AssetPlugin;
    let panel = oteryn_client_ui::chat::ChatPanel {
        selected: true,
        open: true,
        lines: vec![
            oteryn_session::ChatLine::Private {
                speaker_name: "Sender".into(),
                text: "private-color".into(),
            },
            oteryn_session::ChatLine::Room {
                room: oteryn_session::ChatRoom::Help,
                speaker_name: "Helper".into(),
                text: "room-color".into(),
            },
        ],
        ..default()
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.add_systems(
        Startup,
        (move |mut commands: Commands, skins: Res<Skins>| {
            for row in widgets::transcript_rows(&panel) {
                widgets::message_row(&mut commands, &skins, 1, &row);
            }
        })
        .after(crate::bevy_shell::ui::setup),
    );
    app.update();
    let color = |app: &mut App, marker: &str| -> Result<Color, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(&EditableText, &TextColor)>()
            .iter(app.world())
            .find_map(|(text, color)| text.value().to_string().contains(marker).then_some(color.0))
            .ok_or_else(|| "message color missing".into())
    };
    let private = color(&mut app, "private-color")?;
    let room = color(&mut app, "room-color")?;
    assert_ne!(private, room);
    Ok(())
}

#[test]
fn console_font_size_option_sizes_the_transcript_rows() -> Result<(), Box<dyn std::error::Error>> {
    use super::*;
    use bevy::asset::AssetPlugin;
    let (link, _commands, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        Vec::new(),
    );
    let mut settings = crate::play::SettingsState::new(None);
    let mut preferences = settings.preferences.clone();
    preferences.console_font_px = 20;
    settings.apply(preferences);
    settings.sync_local_preferences(&link);
    let panel = link.chat_panel();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.add_systems(
        Startup,
        (move |mut commands: Commands, skins: Res<Skins>| {
            for row in widgets::transcript_rows(&panel) {
                widgets::message_row(&mut commands, &skins, 1, &row);
            }
        })
        .after(crate::bevy_shell::ui::setup),
    );
    app.update();
    let sizes = app
        .world_mut()
        .query_filtered::<&bevy::text::TextFont, With<Transcript>>()
        .iter(app.world())
        .map(|font| font.font_size)
        .collect::<Vec<_>>();
    assert!(!sizes.is_empty());
    assert!(
        sizes
            .iter()
            .all(|size| *size == bevy::text::FontSize::Px(20.))
    );
    Ok(())
}

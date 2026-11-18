//! The read-only mirror opened from the message menu keeps the exact tab it mirrors and its
//! own share of the console.
use super::*;
use crate::play::play_channel;
use bevy::{
    asset::AssetPlugin,
    picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    },
    text::TextReadWriteMode,
    ui_widgets::Activate,
};
use oteryn_session::{ChatLine, ChatRoom, ChatRoomSet};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn console_app(
    rooms: ChatRoomSet,
    lines: Vec<ChatLine>,
) -> Result<(App, crate::play::Outbox), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(&outbox, rooms, lines);
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
    app.update();
    Ok((app, outbox))
}
fn with_link(app: &mut App, action: impl FnOnce(&crate::play::PlayLink)) -> TestResult {
    {
        let owner = app.world().non_send::<HostOwner>();
        action(&owner.play.as_ref().ok_or("play")?.link);
    }
    app.update();
    app.update();
    Ok(())
}
/// Right-clicks the transcript row holding `message` and picks `label` from its menu.
fn menu(app: &mut App, message: &str, label: &str) -> TestResult {
    open_menu(app, message)?;
    choose(app, label)
}

fn open_menu(app: &mut App, message: &str) -> TestResult {
    let row = app
        .world_mut()
        .query::<(Entity, &EditableText, &TextReadWriteMode)>()
        .iter(app.world())
        .find_map(|(entity, text, mode)| {
            (text.value().to_string().contains(message) && *mode == TextReadWriteMode::ReadOnly)
                .then_some(entity)
        })
        .ok_or("message row missing")?;
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
    Ok(())
}

fn choose(app: &mut App, label: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == label).then_some(parent.parent()))
        .ok_or("message menu action missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    app.update();
    Ok(())
}
/// The mirror pane's display and text, and the original transcript's width.
fn panes(app: &mut App) -> Result<(Display, String, Val), Box<dyn std::error::Error>> {
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
    let width = app
        .world_mut()
        .query_filtered::<&Node, With<HistoryScroll>>()
        .single(app.world())?
        .width;
    Ok((display, text, width))
}

#[test]
fn mirrored_conversation_keeps_its_partner_shares_the_console_and_closes_with_it() -> TestResult {
    let (mut app, _) = console_app(
        ChatRoomSet::default(),
        vec![
            ChatLine::Private {
                speaker_name: "Alice".into(),
                text: "from alice".into(),
            },
            ChatLine::Private {
                speaker_name: "Bob".into(),
                text: "from bob".into(),
            },
        ],
    )?;
    assert_eq!(panes(&mut app)?.2, percent(100.));
    with_link(&mut app, |link| {
        link.chat_open_conversation("Alice");
    })?;
    menu(&mut app, "from alice", "Mirror This Tab")?;
    let (display, text, width) = panes(&mut app)?;
    assert_eq!(display, Display::Flex);
    assert!(text.contains("from alice"), "{text}");
    assert!(
        !text.contains("from bob"),
        "another conversation leaked: {text}"
    );
    // The original transcript gives the mirror its third instead of running under it.
    assert_eq!(width, percent(100. - 100. / 3.));

    // The original moves on; the mirror keeps the conversation.
    with_link(&mut app, |link| link.chat_console_tab(ConsoleTab::All))?;
    let (_, text, _) = panes(&mut app)?;
    assert!(
        text.contains("from alice") && !text.contains("from bob"),
        "{text}"
    );

    // Closing the conversation closes its mirror and gives the transcript its width back.
    with_link(&mut app, |link| {
        link.chat_open_conversation("Alice");
    })?;
    with_link(&mut app, |link| link.chat_close_current())?;
    assert_eq!(
        panes(&mut app)?,
        (Display::None, String::new(), percent(100.))
    );
    Ok(())
}

#[test]
fn mirrored_public_channel_shows_its_own_messages() -> TestResult {
    let mut rooms = ChatRoomSet::default();
    rooms.insert(ChatRoom::Help);
    let (mut app, _) = console_app(
        rooms,
        vec![
            ChatLine::Room {
                room: ChatRoom::Help,
                speaker_name: "Helper".into(),
                text: "help answer".into(),
            },
            ChatLine::Private {
                speaker_name: "Bob".into(),
                text: "from bob".into(),
            },
        ],
    )?;
    with_link(&mut app, |link| {
        super::commands::select(
            link,
            &super::commands::Tab::Channel(oteryn_client_ui::chat::ChannelTab::Room(
                ChatRoom::Help,
            )),
        );
    })?;
    menu(&mut app, "help answer", "Mirror This Tab")?;
    let (display, text, _) = panes(&mut app)?;
    assert_eq!(display, Display::Flex);
    assert!(
        text.contains("help answer"),
        "the channel's messages are missing: {text}"
    );
    assert!(
        !text.contains("from bob"),
        "private messages leaked: {text}"
    );
    Ok(())
}

#[test]
fn outgoing_message_menu_opens_recipient_and_copies_one_multiline_notice() -> TestResult {
    use crate::play::feature_0007_chat::protocol::edit_chat_fixture;
    use oteryn_client_ui::chat::{ChannelTab, NoticeClass};
    let line = ChatLine::Private {
        speaker_name: "Alice".into(),
        text: "outgoing hello".into(),
    };
    let (mut app, _) = console_app(ChatRoomSet::default(), vec![line])?;
    with_link(&mut app, |link| {
        edit_chat_fixture(link, |panel| {
            panel.recipients = vec![Some("Bob".into())];
            panel.console.display.time = false;
            panel
                .console
                .notices
                .push(NoticeClass::Loot, "gold\n[Loot] still gold", 0);
            panel.console.notices.push(NoticeClass::Loot, "silver", 1);
        })
    })?;
    open_menu(&mut app, "To Bob")?;
    with_link(&mut app, |link| {
        edit_chat_fixture(link, |panel| {
            panel.lines.clear();
            panel.recipients.clear();
        })
    })?;
    choose(&mut app, "Message to Bob")?;
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_panel()
            .console
            .current,
        Some(ChannelTab::Conversation("Bob".into()))
    );
    with_link(&mut app, |link| {
        edit_chat_fixture(link, |panel| {
            panel.console.current = Some(ChannelTab::Loot);
        })
    })?;
    capture_copy(&mut app);
    menu(&mut app, "still gold", "Copy Message")?;
    assert_eq!(
        app.world().resource::<Copied>().0,
        "[Loot] gold\n[Loot] still gold\n"
    );
    Ok(())
}

#[derive(Resource)]
struct Copied(String);

fn capture_copy(app: &mut App) {
    app.world_mut()
        .remove_resource::<bevy::clipboard::Clipboard>();
    app.insert_resource(Copied("before Copy Message".into()))
        .add_observer(
            |request: On<widgets::CopyText>, mut copied: ResMut<Copied>| {
                copied.0.clone_from(&request.0);
            },
        );
}

#[test]
fn copy_speech_after_history_evicts_its_source() -> TestResult {
    let speech = |index| ChatLine::Private {
        speaker_name: "Alice".into(),
        text: format!("message {index}."),
    };
    let (mut app, outbox) = console_app(ChatRoomSet::default(), (0..64).map(speech).collect())?;
    capture_copy(&mut app);
    let displayed = app
        .world_mut()
        .query::<&EditableText>()
        .iter(app.world())
        .map(|row| row.value().to_string())
        .find(|text| text.contains("message 0."))
        .ok_or("first speech row")?;
    open_menu(&mut app, "message 0.")?;
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        ChatRoomSet::default(),
        (1..=64).map(speech).collect(),
    );
    app.update();
    choose(&mut app, "Copy Message")?;
    assert_eq!(app.world().resource::<Copied>().0, displayed);
    assert!(app.world().resource::<ChatState>().menu.is_none());
    Ok(())
}

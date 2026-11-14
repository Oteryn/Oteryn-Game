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
fn send_button_waits_for_queued_editor_edits() -> TestResult {
    let (mut app, editor, mut commands) = app()?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("o".into()));
    let generation = app.world().non_send::<HostOwner>().generation();
    let send = app
        .world_mut()
        .spawn(Control {
            action: Action::Send,
            generation,
        })
        .observe(widgets::activate)
        .id();
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: send });
    assert!(commands.try_recv().is_err());
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

use super::*;
use bevy::{
    input::InputPlugin,
    input_focus::{FocusCause, InputDispatchPlugin},
    text::{RemSize, TextPlugin},
    ui_widgets::TextInputPlugin,
    window::WindowPlugin,
};
use std::sync::{Arc, Mutex};
type Fixture = (
    App,
    Entity,
    Entity,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn editor_app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (host, commands, outbox) = fixture()?;
    outbox.bevy_npc_dialogue_supported();
    host.play.as_ref().ok_or("play")?.link.npc_shop_close();
    reply(&outbox, &["Welcome. Ask for {trade}."]);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
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
    .insert_non_send(host)
    .add_systems(Startup, (crate::bevy_shell::ui::setup, test_font).chain())
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    super::super::super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let widgets = app
        .world()
        .resource::<State>()
        .widgets
        .ok_or("widgets")?
        .conversation;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(widgets.message, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<EditableText>(widgets.message)
        .ok_or("message")?
        .queue_edit(bevy::text::TextEdit::Insert("hell".into()));
    app.update();
    Ok((app, widgets.message, widgets.send, commands, outbox))
}
fn test_font(mut skins: ResMut<Skins>, mut fonts: ResMut<Assets<Font>>) {
    skins.font = fonts.add(Font::from_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/assets/FiraMono-subset.ttf"
        ))
        .to_vec(),
    ));
}
#[test]
fn same_frame_typing_and_send_uses_completed_message_and_actual_actor() -> Result {
    let (mut app, editor, send, mut commands, _) = editor_app()?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("message")?
        .queue_edit(bevy::text::TextEdit::Insert("o".into()));
    app.world_mut().trigger(Activate { entity: send });
    assert!(commands.try_recv().is_err());
    app.update();
    let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
        return Err("talk".into());
    };
    assert_eq!(intent.npc_actor, npc());
    assert_eq!(intent.text, "hello");
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("message")?
            .value()
            .to_string(),
        ""
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn pending_clipboard_enter_waits_then_sends_once_without_retargeting_actor() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, editor, _, mut commands, _) = editor_app()?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("message")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    let clipboard = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("message")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &clipboard,
    )));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(commands.try_recv().is_err());
    *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("hello from clipboard".into()));
    app.update();
    let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
        return Err("talk".into());
    };
    assert_eq!(intent.npc_actor, npc());
    assert_eq!(intent.text, "hello from clipboard");
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn pending_message_cancels_on_actor_reply_generation_modal_or_close_changes() -> Result {
    for scope in 0..5 {
        let (mut app, editor, send, mut commands, outbox) = editor_app()?;
        let clipboard = Arc::new(Mutex::new(None));
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("message")?
            .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
            &clipboard,
        )));
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(commands.try_recv().is_err());
        match scope {
            0 => {
                outbox.push(vec![SessionEvent::NpcConversation(AppliedDelta {
                    server_sequence: 2,
                    base_revision: 1,
                    new_revision: 2,
                    value: npc_service::NpcConversation {
                        open: Some((
                            EntityRef {
                                identity: [8; 16],
                                generation: 3,
                            },
                            npc_service::NpcConversationState::Open,
                        )),
                        lines: vec!["Another conversation.".into()],
                    },
                })]);
            }
            1 => reply(
                &outbox,
                &["A different question, with {trade} still available."],
            ),
            2 => app.world_mut().non_send_mut::<HostOwner>().close(),
            3 => {
                app.world_mut().spawn((
                    super::super::super::super::GameModal,
                    GlobalZIndex(180),
                    Node::default(),
                ));
            }
            _ => {
                let close = app
                    .world_mut()
                    .query::<(Entity, &Action)>()
                    .iter(app.world())
                    .find(|(_, a)| matches!(a.kind, Kind::Close))
                    .map(|(e, _)| e)
                    .ok_or("close")?;
                app.world_mut().trigger(Activate { entity: close });
            }
        }
        *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("yes".into()));
        app.update();
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "message must not follow changed scope {scope}"
        );
    }
    Ok(())
}
#[test]
fn old_keyword_button_cannot_answer_a_new_server_reply_or_higher_modal() -> Result {
    let (mut app, _, _, mut commands, outbox) = editor_app()?;
    let keyword = app
        .world_mut()
        .query::<(Entity, &conversation::Control)>()
        .iter(app.world())
        .find(|(_, c)| matches!(&c.kind,conversation::Kind::Keyword(word) if word=="trade"))
        .map(|(e, _)| e)
        .ok_or("keyword")?;
    reply(&outbox, &["Changed context: {trade}."]);
    app.world_mut().trigger(Activate { entity: keyword });
    assert!(commands.try_recv().is_err());
    app.update();
    let keyword = app
        .world_mut()
        .query::<(Entity, &conversation::Control)>()
        .iter(app.world())
        .find(|(_, c)| matches!(&c.kind,conversation::Kind::Keyword(word) if word=="trade"))
        .map(|(e, _)| e)
        .ok_or("keyword")?;
    app.world_mut().spawn((
        super::super::super::super::GameModal,
        GlobalZIndex(180),
        Node::default(),
    ));
    app.world_mut().trigger(Activate { entity: keyword });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[path = "conversation_dismissal_tests.rs"]
mod dismissal_tests;

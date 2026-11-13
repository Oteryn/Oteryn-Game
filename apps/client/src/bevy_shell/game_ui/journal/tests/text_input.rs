use super::super::*;
use bevy::{
    asset::AssetPlugin,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key as LogicalKey, KeyboardInput},
    },
    input_focus::InputDispatchPlugin,
    text::{RemSize, TextPlugin},
    ui_widgets::{TextInput, TextInputPlugin},
    window::WindowPlugin,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn app() -> Result<(App, Entity), Box<dyn std::error::Error>> {
    let (owner, _commands, outbox) = super::host()?;
    outbox.bevy_journal_quest_state(super::quest_state());
    owner.play.as_ref().ok_or("play")?.link.quest_set_open(true);
    let key = Key {
        generation: owner.generation(),
        tab: Tab::Quests,
    };
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
    .insert_resource(State {
        key: Some(key),
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
    let root = app
        .world_mut()
        .spawn((Overlay, GameModal, GlobalZIndex(150), Node::default()))
        .id();
    let editor = app
        .world_mut()
        .spawn((
            Field::Search,
            TextInput,
            EditableText::new("hell"),
            source::font(font, false),
            Node::default(),
        ))
        .id();
    app.world_mut().entity_mut(root).add_child(editor);
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::TextEnd(false));
    app.update();
    Ok((app, editor))
}
fn search(app: &App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .quest_panel()
        .list_search)
}
#[test]
fn journal_enter_searches_the_last_character_from_the_native_editor() -> TestResult {
    let (mut app, _) = app()?;
    for (key_code, logical_key, text) in [
        (
            KeyCode::KeyO,
            LogicalKey::Character("o".into()),
            Some("o".into()),
        ),
        (KeyCode::Enter, LogicalKey::Enter, None),
    ] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
    assert_eq!(search(&app)?, "hello");
    Ok(())
}
#[test]
fn journal_search_button_waits_for_async_paste() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, editor) = app()?;
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
    let key = app.world().resource::<State>().key.ok_or("key")?;
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<Overlay>>()
        .single(app.world())?;
    let button = app
        .world_mut()
        .spawn((
            Action {
                key,
                kind: Kind::Search,
            },
            Node::default(),
        ))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(root).add_child(button);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(
        search(&app)?,
        "",
        "unresolved paste must not submit the previous search"
    );
    *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("pasted quest".into()));
    app.update();
    assert_eq!(search(&app)?, "pasted quest");
    Ok(())
}
#[test]
fn deferred_journal_search_is_not_replayed_after_the_journal_closes() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, editor) = app()?;
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: LogicalKey::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .quest_set_open(false);
    app.update();
    assert!(
        app.world().resource::<State>().pending.is_none(),
        "closure cancels even while paste is unresolved"
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .quest_set_open(true);
    *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("never submit".into()));
    app.update();
    assert_eq!(search(&app)?, "");
    assert!(app.world().resource::<State>().pending.is_none());
    Ok(())
}

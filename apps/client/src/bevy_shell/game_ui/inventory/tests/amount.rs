use super::*;
use bevy::{
    asset::AssetPlugin,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    text::{EditableText, RemSize},
    ui_widgets::{TextInput, ValueChange},
};
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
    ItemEntry,
);
fn app() -> Result<Fixture, Box<dyn std::error::Error>> {
    app_with(false)
}
fn app_with(native: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    let item = entry(17, 9)?;
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: None,
            entries: vec![item],
            equipment: vec![],
        }),
        Some(Default::default()),
        true,
        true,
    );
    link.item_drag_begin(DragItem {
        equipped: false,
        handle: item.handle,
        count: item.count.get(),
    });
    link.item_drag_drop(Some(ItemMoveDestination::MainBackpack));
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<RemSize>()
    .init_resource::<UiScale>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .insert_non_send(host(link)?)
    .add_message::<KeyboardInput>()
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            crate::bevy_shell::game_ui::setup,
        )
            .chain(),
    )
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    if native {
        app.add_plugins((
            bevy::input::InputPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::window::WindowPlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui_widgets::TextInputPlugin,
        ))
        .init_resource::<bevy::picking::events::PointerState>()
        .add_message::<bevy::picking::events::PointerRelease>();
    }
    if native {
        // Native selection must shape actual glyphs; the product font remains external.
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/assets/FiraMono-subset.ttf"
        ));
        let font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(bytes.to_vec()));
        app.add_systems(
            Startup,
            (move |mut skins: ResMut<Skins>| {
                skins.font = font.clone();
            })
            .after(crate::bevy_shell::ui::setup)
            .before(crate::bevy_shell::game_ui::setup),
        );
    }
    crate::bevy_shell::game_ui::inventory::install(&mut app);
    app.update();
    Ok((app, commands, outbox, item))
}
fn field(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query_filtered::<Entity, With<TextInput>>()
        .single(app.world())
        .map_err(|_| "numeric amount input missing".into())
}
fn change(app: &mut App, field: Entity, value: &str) {
    app.world_mut().trigger(ValueChange {
        source: field,
        value: value.to_owned(),
        is_final: false,
    });
}
fn move_button(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, Control::Confirm(_)).then_some(entity))
        .ok_or("Move button".into())
}
fn key(app: &mut App, code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: if code == KeyCode::Escape {
            Key::Escape
        } else {
            Key::Enter
        },
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}
#[test]
fn numeric_value_change_reaches_retained_move_owner_only_after_confirm() -> TestResult {
    let (mut app, mut commands, _outbox, item) = app()?;
    let field = field(&mut app)?;
    change(&mut app, field, "7");
    assert!(commands.try_recv().is_err());
    app.update();
    assert_eq!(
        app.world()
            .entity(field)
            .get::<EditableText>()
            .ok_or("editor")?
            .value()
            .to_string(),
        "7"
    );
    let button = move_button(&mut app)?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(7)
        })
    );
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn invalid_amounts_and_removed_source_never_send_the_previous_valid_count() -> TestResult {
    let (mut app, mut commands, outbox, item) = app()?;
    let field = field(&mut app)?;
    let button = move_button(&mut app)?;
    for value in ["0", "10", "4294967296", "", "-1", "abc"] {
        change(&mut app, field, value);
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "invalid amount {value} sent a move"
        );
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == "Enter a whole amount from 1 to 9.")
        );
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .item_movement()
                .prompt
                .is_some()
        );
    }
    change(&mut app, field, "7");
    inventory(&outbox, vec![entry(item.handle.get(), 6)?]);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Enter a whole amount from 1 to 6.")
    );
    let mut transformed = entry(item.handle.get(), 9)?;
    transformed.item_definition_ref = 43.try_into()?;
    inventory(&outbox, vec![transformed]);
    change(&mut app, field, "5");
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(commands.try_recv().is_err());
    change(&mut app, field, "7");
    inventory(&outbox, vec![]);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn enter_is_owned_by_the_top_amount_modal_and_editor_identity_survives_refresh() -> TestResult {
    let (mut app, mut commands, outbox, item) = app()?;
    let field = field(&mut app)?;
    let styles = app
        .world()
        .entity(field)
        .get::<EditableText>()
        .ok_or("editor")?
        .editor()
        .get_styles()
        .inner()
        .clone();
    assert!(styles.len() > 3);
    change(&mut app, field, "5");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    let higher = app
        .world_mut()
        .spawn((
            crate::bevy_shell::game_ui::GameModal,
            GlobalZIndex(200),
            Node::default(),
        ))
        .id();
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Escape);
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .item_movement()
            .prompt
            .is_some()
    );
    app.world_mut().entity_mut(higher).despawn();
    inventory(&outbox, vec![entry(item.handle.get(), 7)?]);
    app.update();
    assert_eq!(self::field(&mut app)?, field);
    assert_eq!(
        app.world()
            .entity(field)
            .get::<EditableText>()
            .ok_or("editor")?
            .editor()
            .get_styles()
            .inner(),
        &styles
    );
    key(&mut app, KeyCode::NumpadEnter);
    app.update();
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(5)
        })
    );
    key(&mut app, KeyCode::Enter);
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn count_buttons_and_all_keep_the_editor_live_and_use_the_latest_stack_bound() -> TestResult {
    let (mut app, mut commands, outbox, item) = app()?;
    let field = field(&mut app)?;
    change(&mut app, field, "4");
    let controls: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .filter_map(|(entity, control)| match control {
            Control::Count(_, delta) => Some((entity, Some(*delta))),
            _ => None,
        })
        .collect();
    let increase = controls
        .iter()
        .find_map(|(entity, delta)| (*delta == Some(1)).then_some(*entity))
        .ok_or("increase")?;
    let decrease = controls
        .iter()
        .find_map(|(entity, delta)| (*delta == Some(-1)).then_some(*entity))
        .ok_or("decrease")?;
    let all = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "All").then_some(parent.parent()))
        .ok_or("All button")?;
    for entity in [increase, increase, decrease] {
        app.world_mut().trigger(Activate { entity });
    }
    app.update();
    assert_eq!(
        app.world()
            .entity(field)
            .get::<EditableText>()
            .ok_or("editor")?
            .value()
            .to_string(),
        "5"
    );
    inventory(&outbox, vec![entry(item.handle.get(), 7)?]);
    app.update();
    app.world_mut().trigger(Activate { entity: all });
    app.update();
    assert_eq!(self::field(&mut app)?, field);
    assert_eq!(
        app.world()
            .entity(field)
            .get::<EditableText>()
            .ok_or("editor")?
            .value()
            .to_string(),
        "7"
    );
    assert!(commands.try_recv().is_err());
    let button = move_button(&mut app)?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(7)
        })
    );
    Ok(())
}

#[test]
fn native_keyboard_typing_and_enter_in_one_frame_send_the_edited_amount() -> TestResult {
    let (mut app, mut commands, _outbox, item) = app_with(true)?;
    let field = field(&mut app)?;
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    let event = |code, logical, state, text| KeyboardInput {
        key_code: code,
        logical_key: logical,
        state,
        text,
        repeat: false,
        window,
    };
    app.world_mut().write_message(event(
        KeyCode::ControlLeft,
        Key::Control,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    app.world_mut().write_message(event(
        KeyCode::KeyA,
        Key::Character("a".into()),
        ButtonState::Pressed,
        Some("a".into()),
    ));
    app.update();
    app.world_mut().write_message(event(
        KeyCode::ControlLeft,
        Key::Control,
        ButtonState::Released,
        None,
    ));
    app.update();
    app.world_mut().write_message(event(
        KeyCode::Digit7,
        Key::Character("7".into()),
        ButtonState::Pressed,
        Some("7".into()),
    ));
    app.world_mut().write_message(event(
        KeyCode::Enter,
        Key::Enter,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    let editor = app
        .world()
        .entity(field)
        .get::<EditableText>()
        .ok_or("editor")?;
    let sent = commands.try_recv().map_err(|error| {
        format!(
            "numeric keyboard: {error}; editor={:?}; pendingedits={:?}",
            editor.value().to_string(),
            editor.pending_edits
        )
    })?;
    assert_eq!(
        sent,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(7)
        })
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn cancel_and_stale_field_callbacks_never_commit_the_numeric_draft() -> TestResult {
    let (mut app, mut commands, _outbox, _item) = app()?;
    let field = field(&mut app)?;
    change(&mut app, field, "5");
    let cancel = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(
                control,
                Control::Cancel {
                    prompt: Some(_),
                    ..
                }
            )
            .then_some(entity)
        })
        .ok_or("Cancel")?;
    app.world_mut().trigger(Activate { entity: cancel });
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .item_movement()
            .prompt
            .is_none()
    );
    change(&mut app, field, "1");
    key(&mut app, KeyCode::Enter);
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn enter_waits_for_the_upstream_async_clipboard_value_before_confirming() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, _outbox, item) = app_with(true)?;
    let field = field(&mut app)?;
    app.world_mut()
        .entity_mut(field)
        .get_mut::<EditableText>()
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .entity_mut(field)
        .get_mut::<EditableText>()
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    key(&mut app, KeyCode::Enter);
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "unresolved clipboard submitted the previous amount"
    );
    *pending
        .lock()
        .map_err(|_| "clipboard fixture lock poisoned")? = Some(Ok("7".to_owned()));
    app.update();
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(7)
        })
    );
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}

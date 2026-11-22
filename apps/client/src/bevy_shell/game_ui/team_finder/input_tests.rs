use super::tests::{
    Fixture, Result, advertised, applicants_page, click, command, fixture_with_layout,
    listing_editor, manage_team_with_layout, page, reply, shown,
};
use super::*;
use crate::play::Outbox;
use bevy::ui::{InteractionDisabled, UiTransform};
use oteryn_client_ui::team_finder::*;

fn browsing_reply_during_preedit() -> Result<(App, Entity)> {
    use bevy::window::Ime;
    let (mut app, _, outbox) = pending_browse()?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(name, bevy::input_focus::FocusCause::Pressed);
    complete_browse(&outbox)?;
    app.world_mut().write_message(Ime::Preedit {
        window: Entity::PLACEHOLDER,
        value: "り".into(),
        cursor: None,
    });
    app.update();
    Ok((app, name))
}

fn assert_idle_composition(app: &App, name: Entity) -> Result {
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    let editor = app.world().get::<EditableText>(name).ok_or("name editor")?;
    assert!(editor.is_composing());
    assert!(editor.pending_edits.is_empty());
    Ok(())
}

#[test]
fn regression_browse_reply_keeps_ime_commit_destination() -> Result {
    use bevy::window::Ime;

    let (mut app, name) = browsing_reply_during_preedit()?;
    assert_idle_composition(&app, name)?;
    assert_browse_projected(&mut app);
    app.update();
    assert_idle_composition(&app, name)?;
    app.world_mut().write_message(Ime::Commit {
        window: Entity::PLACEHOLDER,
        value: "李".into(),
    });
    app.update();
    app.update();
    click(&mut app, Kind::Filter(TeamFilter::default()))?;
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.discovery_panel();
    assert_eq!(panel.teams.filter.name, "李");
    Ok(())
}

#[test]
fn regression_close_survives_browse_reply_during_ime_composition() -> Result {
    let (mut app, name) = browsing_reply_during_preedit()?;
    app.update();
    assert_idle_composition(&app, name)?;
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Close).then_some(entity))
        .ok_or("close button")?;
    let location = press_button(&mut app, close)?;
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(close));
    for _ in 0..4 {
        app.update();
    }
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    Ok(())
}

#[test]
fn regression_selection_has_pickable_controls_in_the_same_frame() -> Result {
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    let previous_root = app.world().resource::<State>().root;
    click(&mut app, Kind::Select(1))?;
    app.update();
    assert!(shown(&mut app, "> Second"));
    assert_eq!(app.world().resource::<State>().root, previous_root);
    let (close, node, visibility) = app
        .world_mut()
        .query::<(Entity, &Control, &ComputedNode, &InheritedVisibility)>()
        .iter(app.world())
        .find_map(|(entity, control, node, visibility)| {
            (control.kind == Kind::Close).then_some((entity, node, visibility))
        })
        .ok_or("close button")?;
    assert!(
        node.size().cmpgt(Vec2::ZERO).all(),
        "close size: {:?}",
        node.size()
    );
    assert!(visibility.get(), "retained close button is hidden");

    let location = press_button(&mut app, close)?;
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn press_button(app: &mut App, button: Entity) -> Result<bevy::picking::pointer::Location> {
    use super::super::pointer_layout;
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        window::PrimaryWindow,
    };

    let primary = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(primary))
            .ok_or("primary target")?,
        position: pointer_layout::center(app, button)?,
    };
    let computed = app.world().get::<ComputedNode>(button).copied();
    let visible = app.world().get::<InheritedVisibility>(button).copied();
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
    }
    app.update();
    assert!(
        app.world().get::<bevy::ui::Pressed>(button).is_some(),
        "button {button} at {:?}, size {:?}, visibility {:?}, hover {:?}",
        location.position,
        computed.map(|node| node.size()),
        visible,
        app.world()
            .resource::<bevy::picking::hover::HoverMap>()
            .get(&PointerId::Mouse)
    );
    Ok(location)
}

fn release_button(app: &mut App, location: bevy::picking::pointer::Location) {
    use bevy::picking::pointer::{PointerAction, PointerButton, PointerId, PointerInput};

    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Release(PointerButton::Primary),
    ));
    app.update();
}

fn pending_browse() -> Result<Fixture> {
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    app.update();
    assert!(!shown(&mut app, "First"));
    Ok((app, rx, outbox))
}

fn complete_browse(outbox: &Outbox) -> Result {
    reply(
        outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    Ok(())
}

fn assert_browse_projected(app: &mut App) {
    assert!(shown(app, "> First"), "browse reply remained unprojected");
    assert!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .is_some_and(|(_, panel, _)| panel.pending.is_none())
    );
}

fn close_button(app: &mut App) -> Result<Entity> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Close).then_some(entity))
        .ok_or_else(|| "close button".into())
}

fn primary_window(app: &mut App) -> Result<Entity> {
    Ok(app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())?)
}

#[test]
fn regression_pending_browse_projects_during_mouse_press() -> Result {
    let (mut app, _, outbox) = pending_browse()?;
    // Close must remain under the pointer when the user has moved the dialog.
    let root = app.world().resource::<State>().root.ok_or("team modal")?;
    let mut dialogs = app.world_mut().query_filtered::<
        (&ChildOf, &mut UiTransform),
        With<crate::bevy_shell::window_drag::DraggableDialog>,
    >();
    let (_, mut transform) = dialogs
        .iter_mut(app.world_mut())
        .find(|(parent, _)| parent.parent() == root)
        .ok_or("team dialog")?;
    transform.translation = bevy::ui::Val2::px(160., 20.);
    app.update();
    let close = close_button(&mut app)?;
    let location = press_button(&mut app, close)?;
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    release_button(&mut app, location);
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn touch(app: &mut App, phase: bevy::input::touch::TouchPhase, position: Vec2) -> Result {
    use bevy::{input::touch::TouchInput, window::WindowEvent};
    let event = TouchInput {
        phase,
        position,
        window: primary_window(app)?,
        force: None,
        id: 1,
    };
    // Winit sends both the unified and typed messages.
    app.world_mut()
        .write_message(WindowEvent::TouchInput(event));
    app.world_mut().write_message(event);
    app.update();
    Ok(())
}

#[test]
fn regression_pending_browse_projects_during_touch_press() -> Result {
    use bevy::input::touch::TouchPhase;
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    touch(&mut app, TouchPhase::Ended, position)?;
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn regression_pending_browse_projects_after_touch_cancel() -> Result {
    use bevy::{input::touch::TouchPhase, picking::pointer::PointerId};
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    touch(&mut app, TouchPhase::Canceled, position)?;
    assert!(
        !app.world_mut()
            .query::<&PointerId>()
            .iter(app.world())
            .any(|id| *id == PointerId::Touch(1))
    );
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    let images = app
        .world()
        .get::<oteryn_client_ui_bevy::ButtonImages>(close)
        .ok_or("button images")?;
    assert_eq!(
        app.world()
            .get::<ImageNode>(close)
            .ok_or("button image")?
            .image,
        images.up
    );
    Ok(())
}

#[test]
fn regression_tab_switch_keeps_keyboard_edit() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, mut rx, outbox) = fixture_with_layout(true, true)?;
    let manage = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Team Finder: Manage Team").then_some(entity))
        .ok_or("manage tab")?;
    let location = press_button(&mut app, manage)?;
    let window = primary_window(&mut app)?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (*field == Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Tab,
        logical_key: Key::Tab,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("a".into()),
        state: ButtonState::Pressed,
        text: Some("a".into()),
        repeat: false,
        window,
    });
    release_button(&mut app, location);
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&applicants_page(None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    complete_browse(&outbox)?;
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(name)
            .ok_or("retained name editor")?
            .value(),
        "a"
    );
    Ok(())
}

#[test]
fn regression_pending_browse_projects_with_keyboard_edit() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, _, outbox) = pending_browse()?;
    let name = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name editor")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(name, bevy::input_focus::FocusCause::Pressed);
    let window = primary_window(&mut app)?;
    complete_browse(&outbox)?;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character("a".into()),
        state: ButtonState::Pressed,
        text: Some("a".into()),
        repeat: false,
        window,
    });
    app.update();
    assert_browse_projected(&mut app);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(name));
    assert_eq!(
        app.world()
            .get::<EditableText>(name)
            .ok_or("name editor")?
            .value()
            .to_string(),
        "a"
    );
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn regression_pending_browse_projects_after_window_focus_loss() -> Result {
    use bevy::{
        input::keyboard::KeyboardFocusLost,
        window::{WindowEvent, WindowFocused},
    };
    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    press_button(&mut app, close)?;
    let window = primary_window(&mut app)?;
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("window")?
        .focused = false;
    let event = WindowFocused {
        window,
        focused: false,
    };
    app.world_mut()
        .write_message(WindowEvent::WindowFocused(event.clone()));
    app.world_mut().write_message(event);
    app.world_mut().write_message(KeyboardFocusLost);
    app.update();
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    Ok(())
}

#[test]
fn regression_touch_cancel_after_browse_reply_releases_close() -> Result {
    use bevy::input::touch::TouchPhase;

    let (mut app, _, outbox) = pending_browse()?;
    let close = close_button(&mut app)?;
    let position = super::super::pointer_layout::center(&app, close)?;
    touch(&mut app, TouchPhase::Started, position)?;
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    complete_browse(&outbox)?;
    app.update();
    assert_browse_projected(&mut app);
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_some());
    touch(&mut app, TouchPhase::Canceled, position)?;
    for _ in 0..4 {
        app.update();
    }
    assert!(app.world().resource::<State>().root.is_some());
    assert!(app.world().get::<bevy::ui::Pressed>(close).is_none());
    let images = app
        .world()
        .get::<oteryn_client_ui_bevy::ButtonImages>(close)
        .ok_or("close images")?;
    assert_eq!(
        app.world()
            .get::<ImageNode>(close)
            .ok_or("close image")?
            .image,
        images.up
    );
    Ok(())
}

#[test]
fn regression_publish_press_survives_applicants_refresh() -> Result {
    let listing = Some(advertised(10));
    let (mut app, mut rx, outbox) = manage_team_with_layout(listing, true)?;
    click(&mut app, Kind::Applicants(false))?;
    assert_eq!(command(&mut rx)?.command_type, 444);
    app.update();
    let publish = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control.kind, Kind::Publish(_)).then_some(entity))
        .ok_or("Publish listing")?;
    let location = press_button(&mut app, publish)?;
    reply(
        &outbox,
        encode_result(&applicants_page(listing)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    assert!(app.world().get::<InteractionDisabled>(publish).is_none());
    release_button(&mut app, location);
    assert_eq!(command(&mut rx)?.command_type, 390);
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[derive(Resource, Default)]
struct CopySelection(Option<Option<String>>);

// Observe the selection at Bevy's clipboard boundary without a desktop clipboard.
fn record_copy_selection(
    mut copied: ResMut<CopySelection>,
    editors: Query<(&Field, &EditableText)>,
) {
    for (field, text) in &editors {
        if *field == Field::Listing(0) && text.pending_edits.contains(&bevy::text::TextEdit::Copy) {
            copied.0 = Some(text.editor().selected_text().map(str::to_owned));
        }
    }
}

#[test]
fn regression_copy_and_tab_use_selection_before_listing_reply_sync() -> Result {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        text::TextEdit,
    };

    let (mut app, mut rx, outbox) = manage_team_with_layout(Some(advertised(10)), true)?;
    app.init_resource::<CopySelection>().add_systems(
        PostUpdate,
        record_copy_selection.before(bevy::text::EditableTextSystems),
    );
    click(&mut app, Kind::Applicants(false))?;
    command(&mut rx)?;
    app.update();
    let editor = listing_editor(&mut app)?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Pressed);
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("level editor")?
        .queue_edit(TextEdit::SelectAll);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("level editor")?
            .editor()
            .selected_text(),
        Some("10")
    );
    reply(
        &outbox,
        encode_result(&applicants_page(Some(advertised(20))))
            .map_err(|error| format!("{error:?}"))?,
    );
    let window = primary_window(&mut app)?;
    for (key_code, logical_key) in [(KeyCode::Copy, Key::Copy), (KeyCode::Tab, Key::Tab)] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    }
    app.update();
    assert_ne!(app.world().resource::<InputFocus>().get(), Some(editor));
    assert_eq!(
        app.world().resource::<CopySelection>().0,
        Some(Some("10".into()))
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("level editor")?
            .value(),
        "20"
    );
    Ok(())
}

#[test]
fn regression_next_page_resets_scroll_after_same_page_refresh() -> Result {
    use bevy::{
        camera::RenderTarget,
        input::{mouse::MouseScrollUnit, touch::TouchPhase},
        picking::pointer::{Location, PointerAction, PointerId, PointerInput},
        ui::ScrollPosition,
        ui_widgets::ScrollArea,
    };

    let (mut app, mut rx, outbox) = pending_browse()?;
    let browse_page = |first| TeamFinderResult::Browse {
        listings: (first..first + 100)
            .map(|leader_handle| BrowseListing {
                leader_handle,
                name: format!("Team {leader_handle}"),
                listing: advertised(10),
                occupied_slots: 1,
                status: MemberStatus::None,
            })
            .collect(),
        next: Some(first + 99),
    };
    let encode = |first| encode_result(&browse_page(first)).map_err(|error| format!("{error:?}"));
    reply(&outbox, encode(1)?);
    app.update();
    let list = app
        .world_mut()
        .query_filtered::<(Entity, &ComputedNode), With<ScrollArea>>()
        .iter(app.world())
        .find_map(|(entity, node)| (node.size().y > 0.).then_some(entity))
        .ok_or("visible team list")?;
    let window = primary_window(&mut app)?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(window))
            .ok_or("window target")?,
        position: super::super::pointer_layout::center(&app, list)?,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Scroll {
            unit: MouseScrollUnit::Pixel,
            x: 0.,
            y: -500.,
            phase: TouchPhase::Moved,
        },
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
    }
    app.update();
    let offset = |app: &App| app.world().get::<ScrollPosition>(list).map(|value| value.y);
    assert_eq!(offset(&app), Some(500.));
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    app.update();
    reply(&outbox, encode(1)?);
    app.update();
    assert_eq!(offset(&app), Some(500.));
    let next = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Browse(true)).then_some(entity))
        .ok_or("Next page")?;
    let location = press_button(&mut app, next)?;
    release_button(&mut app, location);
    let query = command(&mut rx)?;
    assert_eq!(
        decode_query(&query.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: Some(100) }
    );
    app.update();
    reply(&outbox, encode(101)?);
    app.update();
    assert!(shown(&mut app, "> Team 101"));
    assert_eq!(offset(&app), Some(0.));
    Ok(())
}

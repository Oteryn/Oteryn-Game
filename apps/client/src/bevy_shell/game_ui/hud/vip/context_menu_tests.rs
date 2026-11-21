use super::tests::{CONTACTS, click, control, publish, secondary_click};
use super::*;
use oteryn_session::VipPresenceStatus;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn vip_rows_reserve_primary_for_messaging_and_keyboard_activation_opens_edit() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    };
    use bevy::{
        input::keyboard::{Key, KeyboardInput},
        input_focus::FocusedInput,
    };
    let (mut app, _, outbox) = super::super::tests::app()?;
    app.init_resource::<bevy::input_focus::InputFocus>()
        .add_plugins(bevy::ui_widgets::ButtonPlugin);
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let entity = control(&mut app, |a| matches!(a, Action::Edit(73)))?;
    app.world_mut().trigger(PointerClick {
        entity,
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
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    assert!(app.world().resource::<State>().draft.is_none());
    for key_code in [KeyCode::Enter, KeyCode::Space] {
        app.world_mut().trigger(FocusedInput::new(
            entity,
            KeyboardInput {
                key_code,
                logical_key: Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            },
            Entity::PLACEHOLDER,
        ));
        app.update();
        assert!(app.world().resource::<State>().draft.is_some());
        click(&mut app, |a| matches!(a, Action::Cancel(_)))?;
    }
    Ok(())
}

#[test]
fn vip_list_actions_open_from_the_right_click_menu_without_a_header() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let region = app
        .world()
        .resource::<crate::bevy_shell::game_ui::GameRegions>()
        .vip;
    let mut texts = Vec::new();
    let mut stack = vec![region];
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            texts.push(text.0.clone());
        }
        stack.extend(app.world().get::<Children>(entity).into_iter().flatten());
    }
    for header in ["VIP", "↻", "Groups"] {
        assert!(!texts.iter().any(|text| text == header), "{header}");
    }
    let menu = app
        .world_mut()
        .query_filtered::<Entity, With<view::ListMenu>>()
        .single(app.world())?;
    let display = |app: &App| app.world().get::<Node>(menu).map(|node| node.display);
    assert_eq!(display(&app), Some(Display::None));
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<List>>()
        .single(app.world())?;
    app.world_mut().trigger(secondary_click(list));
    app.update();
    assert_eq!(display(&app), Some(Display::Flex));
    let entries: Vec<String> = app
        .world()
        .get::<Children>(menu)
        .ok_or("menu entries")?
        .iter()
        .filter_map(|entry| app.world().get::<Name>(entry))
        .map(|name| name.as_str().to_owned())
        .collect();
    assert_eq!(
        entries,
        [
            "vip.add",
            "vip.groups.add",
            "vip.next",
            "vip.refresh",
            "vip.close",
            "vip.sort",
            "vip.hide_offline",
        ]
    );
    click(&mut app, |action| matches!(action, Action::Sort))?;
    assert_eq!(display(&app), Some(Display::None));
    Ok(())
}

#[test]
fn swapped_contact_and_list_context_clicks() -> TestResult {
    use bevy::picking::pointer::PointerButton;
    let (mut app, _, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .swap_mouse_buttons = true;
    app.update();
    app.update();
    let row = control(&mut app, |action| matches!(action, Action::Edit(73)))?;
    let mut click = secondary_click(row);
    click.count = 2;
    app.world_mut().trigger(click);
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
            "Alice".into()
        ))
    );
    assert!(
        app.world()
            .resource::<presentation::Preferences>()
            .context
            .is_none()
    );
    let mut click = secondary_click(row);
    click.button = PointerButton::Primary;
    app.world_mut().trigger(click);
    assert!(
        app.world()
            .resource::<presentation::Preferences>()
            .context
            .is_some()
    );
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<List>>()
        .single(app.world())?;
    let mut click = secondary_click(list);
    click.button = PointerButton::Primary;
    app.world_mut().trigger(click);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Node, With<view::ListMenu>>()
            .single(app.world())?
            .display,
        Display::Flex
    );
    Ok(())
}

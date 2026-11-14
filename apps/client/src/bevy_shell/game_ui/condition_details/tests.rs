use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayLink, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_ui::{
    battle_instinct_status::BattleInstinctStatus,
    condition_status::{ActiveCondition, ConditionStatus},
    rest_status::RestStatus,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn app() -> Result<Fixture, Box<dyn std::error::Error>> {
    app_with_details(true)
}
fn app_with_details(details: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.condition_details_test_selected();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link: link.condition_details_test_link(),
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    if details {
        install(&mut app);
    }
    app.update();
    Ok((app, link, commands, outbox))
}
fn request(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    target: Target,
) -> TestResult {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("not a feature query".into());
    };
    assert_eq!(
        (command.capability, command.kind),
        match target {
            Target::Conditions => (276, 492),
            Target::Rest => (277, 493),
            Target::BattleInstinct => (278, 494),
        }
    );
    assert!(command.payload.is_empty());
    Ok(())
}
fn publish(outbox: &Outbox, target: Target) -> TestResult {
    let payload = match target {
        Target::Conditions => ConditionStatus {
            conditions: vec![ActiveCondition {
                icon: 1,
                sub_id: 0,
                remaining_ms: 4001,
            }],
        }
        .encode(),
        Target::Rest => RestStatus {
            inside_resting_area: true,
            streak: 7,
            food_remaining_ms: 180001,
            health_multiplier: 2,
            mana_multiplier: 1,
            stamina: true,
            soul: false,
        }
        .encode(),
        Target::BattleInstinct => BattleInstinctStatus {
            active: true,
            melee: 17,
            shielding: 23,
        }
        .encode(),
    }
    .ok_or("encode")?;
    outbox.condition_details_test_reply(target, payload);
    Ok(())
}
fn visible(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn button(
    app: &mut App,
    target: Target,
    action: Action,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.target == target && control.action == action).then_some(entity)
        })
        .ok_or("button".into())
}
#[test]
fn condition_details_window_shows_server_data_only() -> TestResult {
    let (mut app, link, mut commands, outbox) = app()?;
    for target in Target::ALL {
        toggle(app.world().non_send::<HostOwner>(), target, false);
        request(&mut commands, target)?;
        app.update();
        assert!(!visible(&mut app).contains("Melee: +17"));
        publish(&outbox, target)?;
        app.update();
        assert!(visible(&mut app).contains(&link.condition_details_snapshot(target).text()));
        let close = button(&mut app, target, Action::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        app.update();
    }
    assert!(!visible(&mut app).contains("Poison: 5s"));
    Ok(())
}
#[test]
fn condition_details_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, link, mut commands, outbox) = app()?;
    for target in Target::ALL {
        toggle(app.world().non_send::<HostOwner>(), target, false);
        request(&mut commands, target)?;
        publish(&outbox, target)?;
        app.update();
        let confirmed = link.condition_details_snapshot(target).text();
        let refresh = button(&mut app, target, Action::Refresh)?;
        app.world_mut().trigger(Activate { entity: refresh });
        request(&mut commands, target)?;
        app.world_mut().trigger(Activate { entity: refresh });
        assert!(commands.try_recv().is_err());
        assert_eq!(link.condition_details_snapshot(target).text(), confirmed);
        publish(&outbox, target)?;
        app.update();
        let close = button(&mut app, target, Action::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        app.update();
    }
    // An old connection's reply cannot populate the replacement connection's view.
    let (replacement, _, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    replacement_outbox.condition_details_test_selected();
    publish(&outbox, Target::Conditions)?;
    assert!(
        replacement
            .condition_details_snapshot(Target::Conditions)
            .text()
            .contains("Loading")
    );
    Ok(())
}
#[test]
fn condition_details_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, link, mut commands, outbox) = app()?;
    let target = Target::Conditions;
    toggle(app.world().non_send::<HostOwner>(), target, false);
    request(&mut commands, target)?;
    publish(&outbox, target)?;
    app.update();
    let refresh = button(&mut app, target, Action::Refresh)?;
    let stale = link.condition_details_snapshot(target);
    outbox.condition_details_test_changed();
    assert!(!link.condition_details_dispatch_bound(&stale, Action::Refresh, false));
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    app.update();
    let refresh = button(&mut app, target, Action::Refresh)?;
    let before = link.condition_details_snapshot(target);
    assert!(!link.condition_details_dispatch_bound(&before, Action::Refresh, true));
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(200), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    assert!(link.condition_details_snapshot(target) == before);
    app.world_mut().entity_mut(higher).despawn();
    let (other, mut other_commands, other_outbox) = play_channel();
    other_outbox.condition_details_test_selected();
    assert!(!other.condition_details_dispatch_bound(&before, Action::Refresh, false));
    assert!(other_commands.try_recv().is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = other;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    outbox.condition_details_test_end();
    let ended = link.condition_details_snapshot(target);
    assert!(!link.condition_details_dispatch_bound(&ended, Action::Refresh, false));
    assert!(link.condition_details_snapshot(target) == ended);
    Ok(())
}
#[test]
fn condition_details_window_registered() -> TestResult {
    let (mut app, link, mut commands, _) = app()?;
    for (target, id) in [
        (Target::BattleInstinct, "shell.battle_instinct"),
        (Target::Conditions, "shell.conditions"),
        (Target::Rest, "shell.rest_food"),
    ] {
        let entity = app
            .world_mut()
            .query::<(Entity, &crate::bevy_shell::ui::Control)>()
            .iter(app.world())
            .find_map(|(entity, control)| (control.id() == id).then_some(entity))
            .ok_or("toolbar entry")?;
        app.world_mut().trigger(Activate { entity });
        request(&mut commands, target)?;
        app.update();
        assert!(link.condition_details_snapshot(target).open());
        app.world_mut().trigger(Activate { entity });
        app.update();
        assert!(!link.condition_details_snapshot(target).open());
        let event = normalized_shortcut(target, false, false)?;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(&[event], false);
        app.update();
        if target == Target::BattleInstinct {
            assert!(!link.condition_details_snapshot(target).open());
            continue;
        }
        assert!(link.condition_details_snapshot(target).open());
        app.world_mut().write_message(escape_key());
        app.update();
        assert!(!link.condition_details_snapshot(target).open());
    }
    Ok(())
}

#[test]
fn condition_details_refreshes_open_windows_and_stops_after_close() -> TestResult {
    let (mut app, link, mut commands, outbox) = app()?;
    for target in Target::ALL {
        toggle(app.world().non_send::<HostOwner>(), target, false);
        request(&mut commands, target)?;
        publish(&outbox, target)?;
        app.update();
        let refresh = button(&mut app, target, Action::Refresh)?;
        app.world_mut().trigger(Activate { entity: refresh });
        app.update();
        request(&mut commands, target)?;
        app.update();
        assert!(commands.try_recv().is_err());
        let close = button(&mut app, target, Action::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        publish(&outbox, target)?;
        app.update();
        assert!(!link.condition_details_snapshot(target).open());

        app.update();
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn condition_details_open_rejects_editing_phase_and_blocked_gameplay() -> TestResult {
    for target in Target::ALL {
        for fence in 0..5 {
            let (mut app, link, mut commands, outbox) = app()?;
            match fence {
                0 => {
                    let editor = app
                        .world_mut()
                        .spawn((Node::default(), bevy::text::EditableText::new("draft")))
                        .id();
                    app.world_mut()
                        .resource_mut::<InputFocus>()
                        .set(editor, bevy::input_focus::FocusCause::Navigated);
                }
                1 => {
                    app.world_mut().non_send_mut::<HostOwner>().phase =
                        crate::bevy_shell::owner::Phase::Offline
                }
                2 => outbox.condition_details_test_block_gameplay(),
                3 => outbox.condition_details_test_chat_context(),
                _ => {
                    app.world_mut().spawn((
                        bevy::window::PrimaryWindow,
                        bevy::window::Window {
                            focused: false,
                            ..default()
                        },
                    ));
                    app.update();
                }
            }
            let toolbar = app
                .world_mut()
                .query::<(Entity, &crate::bevy_shell::ui::Control)>()
                .iter(app.world())
                .find_map(|(entity, control)| (control.id() == target.id()).then_some(entity))
                .ok_or("toolbar")?;
            let before = link.condition_details_snapshot(target);
            app.world_mut().trigger(Activate { entity: toolbar });
            assert!(link.condition_details_snapshot(target) == before);
            assert!(commands.try_recv().is_err());
            let event = normalized_shortcut(target, false, false)?;
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .route_input(&[event], matches!(fence, 0 | 4));
            app.update();
            assert!(link.condition_details_snapshot(target) == before);
            assert!(commands.try_recv().is_err());
        }
    }
    Ok(())
}
#[test]
fn condition_details_refresh_is_manual_only() -> TestResult {
    let (mut app, _link, mut commands, outbox) = app()?;
    for target in Target::ALL {
        toggle(app.world().non_send::<HostOwner>(), target, false);
        request(&mut commands, target)?;
        publish(&outbox, target)?;
        app.update();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(2),
        ));
        app.update();
        assert!(commands.try_recv().is_err());
        let refresh = button(&mut app, target, Action::Refresh)?;
        app.world_mut().trigger(Activate { entity: refresh });
        request(&mut commands, target)?;
    }
    Ok(())
}

#[test]
fn condition_details_registrations_preserve_existing_order() -> TestResult {
    let (mut app, _link, _commands, _outbox) = app()?;
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(&crate::bevy_shell::ui::Control, &ChildOf)>()
        .iter(app.world())
        .find_map(|(control, parent)| (control.id() == "shell.sign_in").then_some(parent.parent()))
        .ok_or("toolbar")?;
    let children = app
        .world()
        .get::<Children>(toolbar)
        .ok_or("toolbar children")?;
    let ids = children
        .iter()
        .filter_map(|entity| {
            app.world()
                .get::<crate::bevy_shell::ui::Control>(entity)
                .map(|control| control.id())
        })
        .collect::<Vec<_>>();
    let existing = [
        "shell.sign_in",
        "shell.cancel_sign_in",
        "shell.options",
        "shell.market",
        "shell.party",
        "shell.quests",
        "shell.spells",
    ];
    assert_eq!(
        ids.iter()
            .copied()
            .filter(|id| existing.contains(id))
            .collect::<Vec<_>>(),
        existing
    );
    let ordered = [
        "shell.options",
        "shell.battle_instinct",
        "shell.conditions",
        "shell.market",
        "shell.party",
        "shell.quests",
        "shell.rest_food",
        "shell.spells",
    ];
    let positions = ordered
        .iter()
        .map(|id| {
            ids.iter()
                .position(|entry| entry == id)
                .ok_or("window entry")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    for (node, control) in app
        .world_mut()
        .query::<(&Node, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
    {
        if ordered.contains(&control.id()) && control.id() != "shell.options" {
            assert_eq!(node.display, Display::None);
        }
    }
    Ok(())
}

#[test]
fn condition_details_focus_guard_does_not_register_headless_windows() -> TestResult {
    let (app, _link, _commands, _outbox) = app_with_details(false)?;
    assert!(
        app.world()
            .components()
            .get_id(std::any::TypeId::of::<bevy::window::Window>())
            .is_none()
    );
    Ok(())
}

fn normalized_shortcut(
    target: Target,
    repeat: bool,
    super_key: bool,
) -> Result<oteryn_input_actions::NormalizedInputEvent, Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let modifiers = Modifiers::one(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    Ok(NormalizedInputEvent::Key {
        code: KeyCode::new(match target {
            Target::BattleInstinct => 5,
            Target::Conditions => 6,
            Target::Rest => 21,
        })?,
        state: ButtonState::Pressed,
        modifiers: if super_key {
            modifiers.with(Modifier::Super)
        } else {
            modifiers
        },
        repeat,
    })
}
#[test]
fn condition_details_shortcuts_use_normalized_owner_input() -> TestResult {
    for target in Target::ALL {
        let (mut app, link, mut commands, _outbox) = app()?;
        for (repeat, super_key, blocked) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let event = normalized_shortcut(target, repeat, super_key)?;
            let mut release = event.clone();
            if let oteryn_input_actions::NormalizedInputEvent::Key { state, .. } = &mut release {
                *state = oteryn_input_actions::ButtonState::Released;
            }
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .route_input(&[event], blocked);
            assert!(
                !link.condition_details_snapshot(target).open(),
                "{target:?}: repeat={repeat}, super={super_key}, blocked={blocked}"
            );
            assert!(commands.try_recv().is_err());
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .route_input(&[release], false);
        }
        let event = normalized_shortcut(target, false, false)?;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(std::slice::from_ref(&event), false);
        if target == Target::BattleInstinct {
            assert!(!link.condition_details_snapshot(target).open());
            continue;
        }
        request(&mut commands, target)?;
        assert!(link.condition_details_snapshot(target).open());
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(std::slice::from_ref(&event), false);
        assert!(commands.try_recv().is_err());
        assert!(link.condition_details_snapshot(target).open());
    }
    Ok(())
}

fn escape_key() -> bevy::input::keyboard::KeyboardInput {
    bevy::input::keyboard::KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: bevy::input::keyboard::Key::Escape,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

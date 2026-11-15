use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    combat_input::{CombatCommand, PlayCommand},
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, ui_widgets::Activate};
use oteryn_session::{
    ActorPosition, ActorVitals, EntityDetail, EntityKind, EntityRef, StepDirection,
    WorldSpatialEntity,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn row(generation: u64) -> WorldSpatialEntity {
    WorldSpatialEntity {
        motion: None,
        display_name: None,
        entity: EntityRef {
            identity: [2; 16],
            generation,
        },
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 42,
        },
    }
}
type AppResult = Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
>;
pub(super) fn app() -> AppResult {
    app_with_vitals(Some(ActorVitals {
        health: 42,
        max_health: 100,
        mana: 10,
        max_mana: 20,
        ..default()
    }))
}
fn app_with_vitals(vitals: Option<ActorVitals>) -> AppResult {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>();
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    outbox.bevy_hud_fixture(&[row(7)], vitals);
    host.phase = Phase::Playing;
    host.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    app.insert_non_send(host);
    let mut entities = Vec::new();
    for _ in 0..11 {
        entities.push(app.world_mut().spawn(Node::default()).id());
    }
    app.insert_resource(super::super::GameRegions {
        vitals: entities[0],
        minimap: entities[1],
        equipment: entities[2],
        containers: entities[3],
        battle: entities[4],
        skills: entities[5],
        vip: entities[6],
        analyzers: entities[7],
        chat: entities[8],
        bars: entities[9],
        world: entities[10],
    });
    app.add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
#[test]
fn battle_activate_reaches_real_command_owner_and_uses_server_vitals() -> TestResult {
    let (mut app, mut commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("attack row")?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        matches!(commands.try_recv(),Ok(PlayCommand::Battle(CombatCommand::Attack(target))) if target==row(7).entity)
    );
    app.update();
    assert!(
        app.world().get::<HudAction>(button).is_some(),
        "pending updates retain the live target control"
    );
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|value| value.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("42/100"));
    assert!(text.contains("10/20"));
    assert!(!text.contains("7907"));
    Ok(())
}
#[test]
fn stale_or_modal_battle_actions_do_not_reach_current_session() -> TestResult {
    let (mut app, mut commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("attack row")?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.world_mut()
        .get_mut::<HudAction>(button)
        .ok_or("action")?
        .command = Command::Attack(row(6).entity);
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    {
        let mut action = app
            .world_mut()
            .get_mut::<HudAction>(button)
            .ok_or("action")?;
        action.command = Command::Attack(row(7).entity);
        action.generation = 999;
    }
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn absent_vitals_show_unknown_instead_of_claiming_full_health() -> TestResult {
    let (mut app, _commands, _outbox) = app_with_vitals(None)?;
    app.update();
    let labels = app
        .world_mut()
        .query::<(&VitalLabel, &Text)>()
        .iter(app.world())
        .map(|(_, text)| text.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["—", "—"]);
    for (_, node) in app
        .world_mut()
        .query::<(&VitalFill, &Node)>()
        .iter(app.world())
    {
        assert_eq!(node.width, percent(0.));
    }
    Ok(())
}

#[test]
fn analyser_selection_uses_retained_session_counters() -> TestResult {
    let (mut app, _commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(
                action.command,
                Command::Analyser(oteryn_client_ui::analyser::Tool::Damage)
            )
            .then_some(entity)
        })
        .ok_or("damage analyser selector")?;
    app.world_mut().trigger(Activate { entity: button });
    let host = app.world().non_send::<HostOwner>();
    let panel = host.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(panel.tool, oteryn_client_ui::analyser::Tool::Damage);
    assert_eq!(panel.kills, 23);
    Ok(())
}

#[test]
fn inventory_modal_blocks_prior_battle_focus_enter_and_direct_activation() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
    for keyboard in [true, false] {
        let (mut app, mut commands, _outbox) = app()?;
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        super::super::modal::install(&mut app);
        let button = app
            .world_mut()
            .query::<(Entity, &HudAction)>()
            .iter(app.world())
            .find_map(|(entity, action)| {
                matches!(action.command, Command::Attack(_)).then_some(entity)
            })
            .ok_or("battle row")?;
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(button, FocusCause::Navigated);
        app.world_mut()
            .spawn((super::super::GameModal, Node::default(), GlobalZIndex(80)));
        if keyboard {
            let window = app.world_mut().spawn_empty().id();
            let key = KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            };
            app.world_mut()
                .trigger(FocusedInput::new(button, key, window));
            app.world_mut().flush();
        } else {
            app.world_mut().trigger(Activate { entity: button });
        }
        assert!(
            commands.try_recv().is_err(),
            "background battle input escaped the local modal fence"
        );
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
    }
    Ok(())
}

#[test]
fn activating_current_battle_target_stops_attacking() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_hud_target(row(7).entity);
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .battle_panel()
            .combat
            .and_then(|combat| combat.target),
        Some(row(7).entity)
    );
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("battle target")?;
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Battle(CombatCommand::Stop)
    );
    Ok(())
}

#[test]
fn secondary_battle_click_opens_retained_creature_context() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, mut commands, outbox) = app()?;
    crate::play::PlayLink::bevy_items_fixture(&outbox, None, None, false, false);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?
        .with_entities(vec![row(7)])?;
    let entity = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("battle row")?;
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
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    assert!(app.world().non_send::<HostOwner>().dialogs.active());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn shift_battle_click_looks_without_attacking() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, mut commands, outbox) = app()?;
    crate::play::PlayLink::bevy_items_fixture(&outbox, None, None, false, false);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?
        .with_entities(vec![row(7)])?;
    app.init_resource::<ButtonInput<KeyCode>>();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    let entity = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("battle row")?;
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
    assert!(
        matches!(commands.try_recv()?, PlayCommand::WorldContext(command)
        if command.action == oteryn_client_ui::world_interaction::menu::Action::Look)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn battle_and_player_health_use_percentage_colours() -> TestResult {
    let (mut app, _commands, outbox) = app()?;
    for (percent, rgb) in [
        (100, [0, 188, 0]),
        (92, [80, 161, 80]),
        (60, [161, 161, 0]),
        (30, [191, 10, 10]),
        (8, [145, 15, 15]),
        (3, [133, 12, 12]),
    ] {
        let mut enemy = row(7);
        if let EntityDetail::Actor { health_percent, .. } = &mut enemy.detail {
            *health_percent = percent;
        }
        outbox.bevy_hud_fixture(
            &[enemy],
            Some(ActorVitals {
                health: u32::from(percent),
                max_health: 100,
                mana: 10,
                max_mana: 20,
                ..default()
            }),
        );
        app.update();
        let expected = Color::srgb_u8(rgb[0], rgb[1], rgb[2]);
        assert_eq!(
            app.world_mut()
                .query::<(&battle::HealthFill, &BackgroundColor)>()
                .iter(app.world())
                .map(|(_, color)| color.0)
                .collect::<Vec<_>>(),
            vec![expected]
        );
        assert_eq!(
            app.world_mut()
                .query::<(&VitalFill, &BackgroundColor)>()
                .iter(app.world())
                .filter(|(fill, _)| !fill.0)
                .map(|(_, color)| color.0)
                .collect::<Vec<_>>(),
            vec![expected]
        );
    }
    Ok(())
}

#[test]
fn compact_status_projects_the_settings_owner() -> TestResult {
    let (mut app, _, _) = app()?;
    for compact in [false, true] {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .compact_status = compact;
        app.update();
        let owner = app.world().non_send::<HostOwner>();
        let play = owner.play.as_ref().ok_or("play")?;
        let expected = if compact {
            oteryn_client_ui::spell_action_bar::vitals_text(play.link.actor_vitals().as_ref())
        } else {
            play.view.hud_status()
        };
        assert!(
            app.world_mut()
                .query::<(&InfoText, &Text)>()
                .iter(app.world())
                .any(|(_, text)| text.0 == expected)
        );
    }
    Ok(())
}

#[test]
fn parity_a_battle_chord_and_stacked_row_preserve_exact_target() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerPress},
        pointer::{Location, PointerButton, PointerId},
    };
    for gesture in 0..3 {
        let chord = gesture == 1;
        let (mut app, mut commands, outbox) = app()?;
        crate::play::PlayLink::bevy_items_fixture(&outbox, None, None, false, false);
        let target = row(7);
        let mut stacked = row(8);
        stacked.entity.identity = [3; 16];
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?
            .with_entities(vec![target, stacked])?;
        let entity = app
            .world_mut()
            .query::<(Entity, &HudAction)>()
            .iter(app.world())
            .find_map(|(entity, action)| {
                matches!(action.command, Command::Attack(id) if id == target.entity)
                    .then_some(entity)
            })
            .ok_or("row")?;
        let pointer = Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        );
        if chord {
            app.init_resource::<ButtonInput<MouseButton>>();
            // InputPlugin may publish both releases before picking dispatches these presses.
            for button in [MouseButton::Left, MouseButton::Right] {
                let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
                mouse.press(button);
                mouse.release(button);
            }
            for button in [PointerButton::Primary, PointerButton::Secondary] {
                app.world_mut().trigger(PointerPress {
                    entity,
                    pointer: pointer.clone(),
                    button,
                    hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                    count: 1,
                });
            }
        } else if gesture == 0 {
            app.init_resource::<ButtonInput<KeyCode>>();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ShiftLeft);
        }
        for button in if chord {
            vec![PointerButton::Primary, PointerButton::Secondary]
        } else if gesture == 0 {
            vec![PointerButton::Primary]
        } else {
            vec![PointerButton::Secondary]
        } {
            app.world_mut().trigger(PointerClick {
                entity,
                pointer: pointer.clone(),
                button,
                hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            });
        }
        if gesture == 2 {
            assert!(app.world().non_send::<HostOwner>().dialogs.active());
        } else {
            assert!(
                matches!(commands.try_recv()?, PlayCommand::WorldContext(command) if command.target == oteryn_client_ui::world_interaction::menu::Target::Creature(target.entity) && command.action == oteryn_client_ui::world_interaction::menu::Action::Look)
            );
        }
        assert!(
            commands.try_recv().is_err(),
            "chord releases must not attack or open a second menu"
        );
    }
    Ok(())
}

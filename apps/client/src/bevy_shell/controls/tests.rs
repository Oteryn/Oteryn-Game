use super::*;
#[test]
fn configured_preference_utilities_reach_settings_owner() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{CustomAction, UtilityAction};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers};
    let mut owner = HostOwner::new(None)?;
    let (link, _commands, _outbox) = crate::play::play_channel();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    link.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut preferences = owner.settings.preferences.clone();
    preferences.add_hotkey_profile("Zeta")?;
    owner.settings.apply(preferences);
    for action in [
        UtilityAction::ToggleCreatureHuds,
        UtilityAction::ToggleFpsLagIndicator,
        UtilityAction::NextHotkeyPreset,
    ] {
        let before = owner.settings.preferences.clone();
        let mut preferences = before.clone();
        preferences.custom_hotkeys.remove("CTRL+SHIFT+F9");
        preferences.assign_custom_hotkey("CTRL+SHIFT+F9", CustomAction::Utility { action })?;
        owner.settings.apply(preferences);
        owner.route_input(
            &[NormalizedInputEvent::Key {
                code: KeyCode::new(66)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                repeat: false,
            }],
            false,
        );
        let after = &owner.settings.preferences;
        match action {
            UtilityAction::ToggleCreatureHuds => {
                assert_eq!(after.creature_bars, !before.creature_bars)
            }
            UtilityAction::ToggleFpsLagIndicator => {
                assert_ne!(after.performance.show_fps, before.performance.show_fps)
            }
            UtilityAction::NextHotkeyPreset => assert_ne!(
                after.hotkey_contexts.selected,
                before.hotkey_contexts.selected
            ),
            _ => return Err("unexpected utility".into()),
        }
        owner.route_input(&[NormalizedInputEvent::DeviceLost], false);
    }
    Ok(())
}
use crate::play::{PlayView, play_channel};
use oteryn_input_actions::{
    ButtonState, Modifiers, MouseButton, PointerCoordinate, PointerDelta, PointerMotion,
    PointerPosition,
};
use oteryn_renderer::TileCoord;

fn install(host: &mut HostOwner, mut view: PlayView, link: crate::play::PlayLink) {
    view.use_shared_frontend();
    link.use_shared_frontend();
    host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
}

#[test]
fn autowalk_bevy_left_ground_release_sends_native_target() -> Result<(), Box<dyn std::error::Error>>
{
    let mut host = HostOwner::new(None)?;
    let view = PlayView::new(TileCoord::new(100, 100), -3, &[])?;
    let tile = crate::input::click_tile(view.scene().view(), 48.0, 48.0).ok_or("map tile")?;
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_target_walk_selected();
    install(&mut host, view, link);
    host.route_input(
        &[
            NormalizedInputEvent::PointerMoved {
                position: PointerPosition::new(
                    PointerCoordinate::new(48)?,
                    PointerCoordinate::new(48)?,
                ),
                motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
            },
            NormalizedInputEvent::MouseButton {
                button: MouseButton::PRIMARY,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
            },
        ],
        false,
    );
    host.route_input(
        &[NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
        }],
        false,
    );
    let play = host.play.as_mut().ok_or("play")?;
    play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::TargetWalk(oteryn_session::ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: -3
        })
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn autowalk_bevy_wasd_and_arrows_cancel_before_manual_step()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::KeyCode;
    for code in [
        KeyCode::ARROW_UP,
        KeyCode::ARROW_RIGHT,
        KeyCode::ARROW_DOWN,
        KeyCode::ARROW_LEFT,
        KeyCode::new(26)?,
        KeyCode::new(7)?,
        KeyCode::new(22)?,
        KeyCode::new(4)?,
    ] {
        let mut host = HostOwner::new(None)?;
        let mut prefs = host.settings.preferences.clone();
        prefs.controls.movement = crate::play::MovementKeys::ArrowsAndWasd;
        host.settings.apply(prefs);
        let mut view = PlayView::new(TileCoord::new(100, 100), -3, &[])?;
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_target_walk_selected();
        outbox.battle_publish(
            Some(oteryn_session::ActorCombatState {
                target: Some(oteryn_session::EntityRef {
                    identity: [7; 16],
                    generation: 1,
                }),
                modes: oteryn_session::FightModes::DEFAULT,
                in_fight: true,
            }),
            Some(-3),
            &[],
        );
        view.primary_click(TileCoord::new(98, 100), &link)?;
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(matches!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::TargetWalk(_)
        ));
        view.primary_click(TileCoord::new(97, 100), &link)?;
        install(&mut host, view, link);
        host.route_input(
            &[NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            }],
            false,
        );
        let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
            return Err("stop before step".into());
        };
        assert_eq!(stop.payload, [16, 1]);
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
        assert!(matches!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::Step(_)
        ));
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn manual_turn_keeps_the_active_attack() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{KeyCode, Modifier};
    for (name, modifier) in [
        ("CTRL", Modifier::Control),
        ("ALT", Modifier::Alt),
        ("SHIFT", Modifier::Shift),
    ] {
        let mut host = HostOwner::new(None)?;
        let mut prefs = host.settings.preferences.clone();
        prefs.rotation_modifier = name.into();
        host.settings.apply(prefs);
        let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
        view.set_preferences(host.settings.preferences.clone())?;
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_target_walk_selected();
        outbox.battle_publish(
            Some(oteryn_session::ActorCombatState {
                target: Some(oteryn_session::EntityRef {
                    identity: [7; 16],
                    generation: 1,
                }),
                modes: oteryn_session::FightModes::DEFAULT,
                in_fight: true,
            }),
            Some(-7),
            &[],
        );
        install(&mut host, view, link);
        host.route_input(
            &[NormalizedInputEvent::Key {
                code: KeyCode::ARROW_UP,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(modifier),
                repeat: false,
            }],
            false,
        );
        let crate::combat_input::PlayCommand::ServerMechanic(turn) = commands.try_recv()? else {
            return Err("turn must preserve combat".into());
        };
        assert_eq!(turn.payload, [26, 2, 8, 1]);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn alt_arrows_on_visible_minimap_do_not_step_or_cancel_walking()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{KeyCode, Modifier};
    let mut host = HostOwner::new(None)?;
    assert!(host.settings.preferences.panel_visible("minimap"));
    let mut view = PlayView::new(TileCoord::new(100, 100), -3, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_target_walk_selected();
    view.primary_click(TileCoord::new(98, 100), &link)?;
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(matches!(commands.try_recv()?, crate::combat_input::PlayCommand::TargetWalk(_)));
    let goal = link.target_walk_state().goal.ok_or("active walk goal")?;
    install(&mut host, view, link);
    for code in [
        KeyCode::ARROW_RIGHT,
        KeyCode::ARROW_LEFT,
        KeyCode::ARROW_UP,
        KeyCode::ARROW_DOWN,
    ] {
        host.route_input(
            &[NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Alt),
                repeat: false,
            }],
            false,
        );
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
        assert!(commands.try_recv().is_err());
        assert_eq!(play.link.target_walk_state().goal, Some(goal));
    }
    Ok(())
}

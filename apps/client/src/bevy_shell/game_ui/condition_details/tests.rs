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
    .init_resource::<bevy::input::ButtonInput<KeyCode>>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
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
        toggle(app.world().non_send::<HostOwner>(), target);
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
        toggle(app.world().non_send::<HostOwner>(), target);
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
    toggle(app.world().non_send::<HostOwner>(), target);
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
    assert!(include_str!("../mod.rs").contains("condition_details::install(app);"));
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
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::ShiftLeft);
        keys.press(target.shortcut());
        app.update();
        assert!(link.condition_details_snapshot(target).open());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!link.condition_details_snapshot(target).open());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
    }
    Ok(())
}

#[test]
fn condition_details_refreshes_open_windows_and_stops_after_close() -> TestResult {
    let (mut app, link, mut commands, outbox) = app()?;
    for target in Target::ALL {
        toggle(app.world().non_send::<HostOwner>(), target);
        request(&mut commands, target)?;
        publish(&outbox, target)?;
        app.update();
        app.world_mut()
            .resource_mut::<State>()
            .window(target)
            .refreshed = Some(Instant::now() - Duration::from_secs(2));
        app.update();
        request(&mut commands, target)?;
        app.update();
        assert!(commands.try_recv().is_err());
        let close = button(&mut app, target, Action::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        publish(&outbox, target)?;
        app.update();
        assert!(!link.condition_details_snapshot(target).open());
        app.world_mut()
            .resource_mut::<State>()
            .window(target)
            .refreshed = Some(Instant::now() - Duration::from_secs(2));
        app.update();
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

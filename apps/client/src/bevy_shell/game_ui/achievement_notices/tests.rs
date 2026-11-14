use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, ui_widgets::Activate};
use oteryn_client_ui::achievement_notices::{
    AchievementEarned, AchievementNoticesPanel as Panel, AchievementWatermark, notices_panel_text,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Panel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|p| p.link.achievement_notices_panel())
        .unwrap_or_default()
}
fn toggle(app: &mut App) {
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
}
fn close(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query_filtered::<Entity, With<Close>>()
        .iter(app.world())
        .next()
        .ok_or("close absent".into())
}
#[test]
fn achievement_notices_window_shows_server_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    for selected in [false, true] {
        outbox.bevy_achievement_notices_snapshot(selected, None);
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|t| t.0 == notices_panel_text(&panel(&app)))
        );
    }
    outbox.bevy_achievement_notices_snapshot(
        true,
        Some(AchievementWatermark {
            fact_count: 42,
            total_points: 123,
        }),
    );
    outbox.bevy_achievement_notices_earned(AchievementEarned {
        key: "oteryn:achievement/server-award".into(),
        name: "Server award".into(),
        watermark: AchievementWatermark {
            fact_count: 43,
            total_points: 126,
        },
    });
    app.update();
    let expected = panel(&app);
    assert_eq!(expected.recent.len(), 1);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == notices_panel_text(&expected))
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn achievement_notices_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    toggle(&mut app);
    let before = panel(&app);
    let button = close(&mut app)?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(!panel(&app).open);
    let mut expected = before.clone();
    expected.open = false;
    assert_eq!(panel(&app), expected);
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), expected);
    // A stale control must stay consumed even if the window is reopened before projection.
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    let reopened = panel(&app);
    assert!(reopened.open);
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), reopened);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn achievement_notices_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let button = close(&mut app)?;
    outbox.bevy_achievement_notices_snapshot(
        true,
        Some(AchievementWatermark {
            fact_count: 5,
            total_points: 10,
        }),
    );
    let changed = panel(&app);
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), changed);
    app.update();
    let button = close(&mut app)?;
    app.world_mut()
        .get_mut::<Close>(button)
        .ok_or("close")?
        .generation += 1;
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), changed);
    app.world_mut()
        .get_mut::<Close>(button)
        .ok_or("close")?
        .generation -= 1;
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), changed);
    app.world_mut().despawn(modal);
    app.update();
    let (replacement, _receiver, events) = play_channel();
    events.bevy_achievement_notices_snapshot(true, changed.watermark);
    replacement.use_shared_frontend();
    let snapshot = replacement.achievement_notices_binding();
    assert!(replacement.achievement_notices_set_open_bound(&snapshot, true));
    let previous = std::mem::replace(
        &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .link,
        replacement,
    );
    let previous_binding = previous.achievement_notices_binding();
    let current = panel(&app);
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(panel(&app), current);
    outbox.bevy_achievement_notices_end();
    assert_eq!(previous.achievement_notices_panel(), Panel::default());
    assert!(!previous.achievement_notices_set_open_bound(&previous_binding, false));
    assert!(
        !previous.achievement_notices_set_open_bound(&previous.achievement_notices_binding(), true)
    );
    assert_eq!(panel(&app), current);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn achievement_notices_window_registered() -> TestResult {
    assert!(include_str!("../mod.rs").contains("achievement_notices::install(app);"));
    let (mut app, mut commands, _) = fixture()?;
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.achievement_notices").then_some(e))
        .ok_or("toolbar absent")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(panel(&app).open);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(!panel(&app).open);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(panel(&app).open);
    let button = close(&mut app)?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(!panel(&app).open);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut().write_message(key(KeyCode::KeyM));
    app.update();
    assert!(panel(&app).open);
    app.world_mut().write_message(key(KeyCode::KeyM));
    app.update();
    assert!(!panel(&app).open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn key(code: KeyCode) -> bevy::input::keyboard::KeyboardInput {
    bevy::input::keyboard::KeyboardInput {
        key_code: code,
        logical_key: bevy::input::keyboard::Key::Character("m".into()),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

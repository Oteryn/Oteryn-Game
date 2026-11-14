use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn support_evidence_window_registered() -> TestResult {
    assert!(include_str!("../mod.rs").contains("support_evidence::install(app)"));
    let (link, mut requests, _outbox) = play_channel();
    link.use_shared_frontend();
    let directory = std::env::temp_dir().join(format!("u52-window-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join("approved-survey.txt"),
        "Reference: owner\nExact approved survey",
    )?;
    link.queue_support_at(&directory);
    assert!(matches!(
        requests.try_recv()?,
        PlayCommand::SupportDelivery(_, _)
    ));
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
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
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, crate::bevy_shell::ui::Control::SupportEvidence).then_some(entity)
        })
        .ok_or("support toolbar missing")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().overlay.is_some());
    assert!(requests.try_recv().is_err(), "opening Help must not submit");
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Reference: owner\nExact approved survey")
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == crate::play::help_text(crate::play::HelpTopic::Client))
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| control.evidence.is_none().then_some(entity))
        .ok_or("close missing")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().overlay.is_none());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(SHORTCUT);
    app.update();
    assert!(app.world().resource::<State>().overlay.is_some());
    assert!(requests.try_recv().is_err());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(SHORTCUT);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(SHORTCUT);
    app.update();
    assert!(app.world().resource::<State>().overlay.is_none());
    assert!(requests.try_recv().is_err());
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

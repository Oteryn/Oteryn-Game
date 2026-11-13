use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct File(PathBuf);
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn app() -> Result<(App, File), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "oteryn-public-audit-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_non_send(HostOwner::new(None)?)
        .insert_resource(AuditOutput::new(Some(path.clone())))
        .add_systems(Update, write);
    let mut window = Window::default();
    window.resolution.set_scale_factor_override(Some(2.));
    app.world_mut().spawn((window, PrimaryWindow));
    Ok((app, File(path)))
}
fn node(app: &mut App, name: &str, parent: Option<Entity>, position: Vec2, size: Vec2) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            Name::new(name.to_owned()),
            Node::default(),
            ComputedNode { size, ..default() },
            UiGlobalTransform::from_translation(position),
            InheritedVisibility::VISIBLE,
        ))
        .id();
    if let Some(parent) = parent {
        app.world_mut().entity_mut(entity).insert(ChildOf(parent));
    }
    entity
}
fn read(app: &mut App, path: &File) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    app.update();
    Ok(serde_json::from_slice(&std::fs::read(&path.0)?)?)
}
#[test]
fn named_public_bounds_filter_hidden_zero_unknown_and_private_widget_contents() -> TestResult {
    let (mut app, path) = app()?;
    let root = node(
        &mut app,
        "options.dialog",
        None,
        Vec2::new(400., 300.),
        Vec2::new(600., 500.),
    );
    let field = node(
        &mut app,
        "graphics.fullscreen",
        Some(root),
        Vec2::new(240., 160.),
        Vec2::new(20., 10.),
    );
    app.world_mut().entity_mut(field).insert((
        Text::new("PRIVATE_WIDGET_CONTENT"),
        bevy::text::EditableText::new("PRIVATE_WIDGET_CONTENT"),
    ));
    node(
        &mut app,
        "options.page.Graphics",
        Some(root),
        Vec2::new(100., 120.),
        Vec2::new(90., 24.),
    );
    node(
        &mut app,
        "options.apply",
        Some(root),
        Vec2::new(200., 500.),
        Vec2::new(60., 20.),
    );
    node(
        &mut app,
        "options.apply",
        Some(root),
        Vec2::new(280., 500.),
        Vec2::new(60., 20.),
    );
    let disabled = node(
        &mut app,
        "graphics.vsync",
        Some(root),
        Vec2::new(240., 180.),
        Vec2::new(20., 10.),
    );
    app.world_mut()
        .entity_mut(disabled)
        .insert(bevy::ui::InteractionDisabled);
    node(
        &mut app,
        "graphics.world_zoom",
        Some(root),
        Vec2::ZERO,
        Vec2::ZERO,
    );
    node(
        &mut app,
        "options.unapproved.PRIVATE_NAME_CONTENT",
        Some(root),
        Vec2::ZERO,
        Vec2::splat(20.),
    );
    node(&mut app, "sound.master", None, Vec2::ZERO, Vec2::splat(20.));
    let hidden = node(&mut app, "hidden", Some(root), Vec2::ZERO, Vec2::splat(20.));
    app.world_mut()
        .get_mut::<Node>(hidden)
        .ok_or("hidden parent")?
        .display = Display::None;
    node(
        &mut app,
        "options.reset",
        Some(hidden),
        Vec2::ZERO,
        Vec2::splat(20.),
    );
    let hidden_visibility = node(
        &mut app,
        "hidden visibility",
        Some(root),
        Vec2::ZERO,
        Vec2::splat(20.),
    );
    app.world_mut()
        .entity_mut(hidden_visibility)
        .insert(Visibility::Hidden);
    node(
        &mut app,
        "options.cancel",
        Some(hidden_visibility),
        Vec2::ZERO,
        Vec2::splat(20.),
    );
    let inherited = node(
        &mut app,
        "graphics.ambient",
        Some(root),
        Vec2::ZERO,
        Vec2::splat(20.),
    );
    app.world_mut()
        .entity_mut(inherited)
        .insert(InheritedVisibility::HIDDEN);
    let exit = node(
        &mut app,
        "exit.dialog",
        None,
        Vec2::new(400., 300.),
        Vec2::new(800., 600.),
    );
    node(
        &mut app,
        "exit.panel",
        Some(exit),
        Vec2::new(400., 300.),
        Vec2::new(340., 140.),
    );
    node(
        &mut app,
        "exit.yes",
        Some(exit),
        Vec2::new(400., 340.),
        Vec2::new(80., 20.),
    );
    node(
        &mut app,
        "exit.no",
        Some(exit),
        Vec2::new(490., 340.),
        Vec2::new(80., 20.),
    );
    let snapshot = read(&mut app, &path)?;
    let controls = snapshot["controls"].as_array().ok_or("controls")?;
    for id in [
        "options.dialog",
        "options.page.Graphics",
        "options.apply",
        "graphics.fullscreen",
        "graphics.vsync",
        "exit.dialog",
        "exit.panel",
        "exit.yes",
        "exit.no",
    ] {
        assert_eq!(
            controls.iter().filter(|entry| entry["id"] == id).count(),
            1,
            "public control {id}"
        );
    }
    for id in [
        "graphics.world_zoom",
        "options.reset",
        "options.cancel",
        "graphics.ambient",
        "sound.master",
    ] {
        assert!(
            controls.iter().all(|entry| entry["id"] != id),
            "hidden/unscoped {id}"
        );
    }
    let field = controls
        .iter()
        .find(|entry| entry["id"] == "graphics.fullscreen")
        .ok_or("field bounds")?;
    assert_eq!(
        field["bounds_physical"],
        serde_json::json!([230., 155., 20., 10.])
    );
    assert_eq!(
        field["bounds_logical"],
        serde_json::json!([115., 77.5, 10., 5.])
    );
    assert_eq!(
        controls
            .iter()
            .find(|entry| entry["id"] == "graphics.vsync")
            .ok_or("vsync")?["enabled"],
        false
    );
    assert!(
        controls
            .iter()
            .all(|entry| entry.get("text_metrics").is_none())
    );
    let bytes = std::fs::read_to_string(&path.0)?;
    assert!(!bytes.contains("PRIVATE_WIDGET_CONTENT"));
    assert!(!bytes.contains("PRIVATE_NAME_CONTENT"));
    Ok(())
}
#[test]
fn full_options_bounds_win_over_legacy_graphics_and_hide_legacy_footer() -> TestResult {
    let (mut app, path) = app()?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    let root = node(
        &mut app,
        "options.dialog",
        None,
        Vec2::new(400., 300.),
        Vec2::new(600., 500.),
    );
    node(
        &mut app,
        "graphics.fullscreen",
        Some(root),
        Vec2::new(250., 150.),
        Vec2::new(20., 20.),
    );
    node(
        &mut app,
        "options.apply",
        Some(root),
        Vec2::new(400., 500.),
        Vec2::new(60., 20.),
    );
    for control in [Control::Fullscreen, Control::Apply] {
        app.world_mut().spawn((
            control,
            ComputedNode {
                size: Vec2::splat(99.),
                ..default()
            },
            UiGlobalTransform::default(),
        ));
    }
    let snapshot = read(&mut app, &path)?;
    let controls = snapshot["controls"].as_array().ok_or("controls")?;
    let fullscreen: Vec<_> = controls
        .iter()
        .filter(|entry| entry["id"] == "graphics.fullscreen")
        .collect();
    assert_eq!(fullscreen.len(), 1);
    assert_eq!(
        fullscreen[0]["bounds_physical"],
        serde_json::json!([240., 140., 20., 20.])
    );
    app.insert_resource(crate::bevy_shell::game_ui::options::Installed);
    let snapshot = read(&mut app, &path)?;
    let controls = snapshot["controls"].as_array().ok_or("controls")?;
    assert!(controls.iter().all(|entry| entry["id"] != "graphics.apply"));
    assert!(controls.iter().any(|entry| entry["id"] == "options.apply"));
    Ok(())
}

#[test]
fn step_receipt_requires_an_actual_reply_and_resets_with_the_play_view() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_renderer::TileCoord;
    use oteryn_session::{
        ActorPosition, AppliedDelta, CommandStatus, SessionEvent, StepDisposition, StepOutcome,
        WorldSpatialObservation,
    };
    let (mut app, path) = app()?;
    let (link, _commands, outbox) = play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: PlayView::new(TileCoord::new(100, 100), 0, &[])?,
            link,
        });
        owner.phase = Phase::Playing;
    }
    let before = read(&mut app, &path)?;
    assert!(
        before["play_observation"].is_object(),
        "playing needs an observation"
    );
    assert_eq!(
        before["play_observation"]["generation"],
        before["generation"]
    );
    assert!(before["play_observation"]["last_step"].is_null());
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner
            .play
            .as_mut()
            .ok_or("play")?
            .view
            .arrow(crate::input::StepDir::North);
        owner.poll([720, 528]);
    }
    outbox.push(vec![SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: 10,
        base_revision: 1,
        new_revision: 2,
        value: WorldSpatialObservation {
            content_generation: [0; 32],
            actor_position: ActorPosition {
                x: 101,
                y: 100,
                floor: 0,
            },
        },
    })]);
    app.world_mut().non_send_mut::<HostOwner>().poll([720, 528]);
    let pushed = read(&mut app, &path)?;
    assert_eq!(
        pushed["play_observation"]["map_center"],
        serde_json::json!([101, 100, 0])
    );
    assert!(pushed["play_observation"]["last_step"].is_null());
    for (index, (status, disposition, accepted, moved)) in [
        (
            CommandStatus::Rejected,
            StepDisposition::Rejected,
            false,
            false,
        ),
        (
            CommandStatus::Accepted,
            StepDisposition::Blocked,
            true,
            false,
        ),
        (CommandStatus::Accepted, StepDisposition::Moved, true, true),
    ]
    .into_iter()
    .enumerate()
    {
        crate::play::audit_step_reply(
            &outbox,
            StepOutcome {
                command_id: 40 + index as u64,
                status,
                disposition,
                result_server_sequence: 50 + index as u64,
                world_spatial_delta: moved.then_some(AppliedDelta {
                    server_sequence: 60 + index as u64,
                    base_revision: 2,
                    new_revision: 3,
                    value: WorldSpatialObservation {
                        content_generation: [0; 32],
                        actor_position: ActorPosition {
                            x: 102,
                            y: 100,
                            floor: 0,
                        },
                    },
                }),
                world_object_overlay_delta: None,
                gate_message_id: None,
            },
        );
        app.world_mut().non_send_mut::<HostOwner>().poll([720, 528]);
        let snapshot = read(&mut app, &path)?;
        assert_eq!(
            snapshot["play_observation"]["last_step"],
            serde_json::json!({
                "reply_count": index + 1,
                "command_id": 40 + index,
                "result_server_sequence": 50 + index,
                "accepted": accepted,
                "moved": moved,
            })
        );
    }
    let (link, _commands, _outbox) = play_channel();
    app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(200, 200), 0, &[])?,
        link,
    });
    let replaced = read(&mut app, &path)?;
    assert!(replaced["play_observation"]["last_step"].is_null());
    assert_eq!(
        replaced["play_observation"]["map_center"],
        serde_json::json!([200, 200, 0])
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    assert!(read(&mut app, &path)?["play_observation"].is_null());
    Ok(())
}

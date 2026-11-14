use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{
    cyclopedia_tracker::{
        TrackerAction, TrackerEntry, TrackerRequest, TrackerSnapshot, decode_request,
        encode_snapshot,
    },
    raw_feature::GatedCommandResult,
};
use oteryn_session::CommandStatus;
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
    outbox.bevy_cyclopedia_trackers_initialize();
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
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    open(app.world().non_send::<HostOwner>());
    app.update();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_tracker_panel())
}
fn publish(outbox: &Outbox, boss: bool, keys: &[(&str, u32, bool)]) -> TestResult {
    let payload = encode_snapshot(&TrackerSnapshot {
        boss,
        entries: keys
            .iter()
            .map(|(key, kills, completed)| TrackerEntry {
                creature_key: (*key).into(),
                kill_count: *kills,
                thresholds: [5, 20, 60],
                completed: *completed,
            })
            .collect(),
    })
    .ok_or("snapshot")?;
    outbox.bevy_cyclopedia_trackers_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
    Ok(())
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn field(app: &mut App, kind: Field, value: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(e, f)| (*f == kind).then_some(e))
        .ok_or("field")?;
    app.world_mut()
        .entity_mut(entity)
        .insert(EditableText::new(value));
    app.update();
    // Filters reduce after text editing; project that owner change before the next click.
    app.update();
    Ok(())
}
#[test]
fn cyclopedia_trackers_window_shows_server_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(panel(&app)?.bestiary_snapshot.is_none());
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Widget(false))?;
    publish(
        &outbox,
        false,
        &[("oteryn:rat", 4, false), ("oteryn:dragon", 60, true)],
    )?;
    let PlayCommand::Feature(read) = commands.try_recv()? else {
        return Err("Bestiary read".into());
    };
    assert_eq!(
        (read.kind, decode_request(&read.payload)),
        (383, Some(TrackerRequest::Read))
    );
    app.update();
    click(&mut app, Kind::Widget(true))?;
    let PlayCommand::Feature(read) = commands.try_recv()? else {
        return Err("Bosstiary read".into());
    };
    assert_eq!(
        (read.kind, decode_request(&read.payload)),
        (384, Some(TrackerRequest::Read))
    );
    publish(&outbox, true, &[("oteryn:boss", 18, false)])?;
    app.update();
    let texts: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("oteryn:rat") && t.contains("4 / 60") && t.contains("5 / 20 / 60"))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("oteryn:boss") && t.contains("18 / 60"))
    );
    field(&mut app, Field::Search(false), "rat")?;
    assert_eq!(
        oteryn_client_ui::cyclopedia_tracker::tracker_rows(&panel(&app)?, false).len(),
        1
    );
    click(&mut app, Kind::Sort(false, TrackerSort::Kills))?;
    click(&mut app, Kind::Hide(true))?;
    assert_eq!(panel(&app)?.sort, TrackerSort::Kills);
    assert!(panel(&app)?.boss_hide_completed);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn cyclopedia_trackers_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    field(&mut app, Field::Key, "oteryn:rat")?;
    click(&mut app, Kind::Request(Intent::Add))?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("feature".into());
    };
    assert_eq!((c.capability, c.kind), (197, 383));
    assert_eq!(
        decode_request(&c.payload),
        Some(TrackerRequest::Change {
            action: TrackerAction::Add,
            creature_key: "oteryn:rat".into()
        })
    );
    assert!(panel(&app)?.bestiary_snapshot.is_none());
    assert!(panel(&app)?.pending);
    click(&mut app, Kind::Request(Intent::Add))?;
    assert!(commands.try_recv().is_err());
    publish(&outbox, false, &[("oteryn:rat", 4, false)])?;
    app.update();
    click(&mut app, Kind::Request(Intent::Remove))?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Cancel)?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Request(Intent::Remove))?;
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("remove".into());
    };
    assert_eq!(
        decode_request(&c.payload),
        Some(TrackerRequest::Change {
            action: TrackerAction::Remove,
            creature_key: "oteryn:rat".into()
        })
    );
    assert_eq!(
        panel(&app)?
            .bestiary_snapshot
            .as_ref()
            .map(|s| s.entries.len()),
        Some(1)
    );
    publish(&outbox, false, &[])?;
    app.update();
    click(&mut app, Kind::Boss)?;
    click(&mut app, Kind::Request(Intent::Read))?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("read".into());
    };
    assert_eq!((c.capability, c.kind), (197, 384));
    assert_eq!(decode_request(&c.payload), Some(TrackerRequest::Read));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn cyclopedia_trackers_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    field(&mut app, Field::Key, "oteryn:rat")?;
    click(&mut app, Kind::Request(Intent::Remove))?;
    field(&mut app, Field::Key, "oteryn:dragon")?;
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Cancel)?;
    let old = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_trackers_binding();
    let (other, mut other_commands, other_outbox) = play_channel();
    other_outbox.bevy_cyclopedia_trackers_initialize();
    assert!(other.cyclopedia_trackers_dispatch_bound(
        &other.cyclopedia_trackers_binding(),
        OwnerAction::Toggle,
        false
    ));
    assert_eq!(other.cyclopedia_tracker_panel(), old.panel);
    assert!(!other.cyclopedia_trackers_dispatch_bound(&old, OwnerAction::Toggle, false));
    assert!(other_commands.try_recv().is_err());
    publish(&outbox, false, &[("oteryn:old_session", 1, false)])?;
    assert!(other.cyclopedia_tracker_panel().snapshot.is_none());
    app.update();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    let before = panel(&app)?;
    click(&mut app, Kind::Request(Intent::Add))?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    outbox.bevy_cyclopedia_trackers_end();
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    let ended = link.cyclopedia_tracker_panel();
    assert!(!link.cyclopedia_trackers_dispatch_bound(&old, OwnerAction::Toggle, false));
    assert_eq!(link.cyclopedia_tracker_panel(), ended);
    Ok(())
}
#[test]
fn cyclopedia_trackers_window_registered() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app)?.open);
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.cyclopedia_trackers").then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(21)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), false);
    app.update();
    assert!(!panel(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), false);
    app.update();
    assert!(panel(&app)?.open);
    Ok(())
}

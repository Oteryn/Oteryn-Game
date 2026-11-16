use super::*;
use crate::{
    bevy_shell::{
        owner::ActivePlay,
        ui::{Control as ToolbarControl, toolbar::open_toolbar},
    },
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_tracker::{
        TrackerAction, TrackerEntry, TrackerRequest, TrackerSnapshot, encode_domain_snapshot,
        encode_request, encode_snapshot,
    },
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_tracker_initialize();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        })
        .add_systems(
            Startup,
            (
                super::super::setup,
                |mut roots: Query<&mut Node, With<super::super::GameRoot>>| {
                    for mut node in &mut roots {
                        node.display = Display::Flex;
                    }
                },
            )
                .chain(),
        );
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn toolbar(app: &mut App, id: &'static str) -> TestResult {
    let opened = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        open_toolbar(&mut owner, ToolbarControl::Shortcut(id))
    };
    if !opened {
        return Err("toolbar entry refused".into());
    }
    app.update();
    Ok(())
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected feature command".into());
    };
    assert_eq!(command.capability, 197);
    Ok(command)
}
fn reply(app: &mut App, outbox: &Outbox, snapshot: &TrackerSnapshot) -> TestResult {
    outbox.bevy_cyclopedia_tracker_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_snapshot(snapshot).ok_or("snapshot")?,
    });
    app.update();
    Ok(())
}
fn tracker(boss: bool, entries: &[(&str, u32)]) -> TrackerSnapshot {
    TrackerSnapshot {
        boss,
        entries: entries
            .iter()
            .map(|&(key, kill_count)| TrackerEntry {
                creature_key: key.into(),
                kill_count,
                thresholds: [5, 25, 250],
                completed: false,
            })
            .collect(),
    }
}
fn change(action: TrackerAction, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(encode_request(&TrackerRequest::Change {
        action,
        creature_key: key.into(),
    })
    .ok_or("request")?)
}
fn click(app: &mut App, boss: bool, action: &Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.boss == boss && control.action == *action).then_some(entity)
        })
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}
fn shows(app: &mut App, wanted: &str) -> bool {
    texts(app).iter().any(|text| text == wanted)
}

#[test]
fn tracker_windows_open_from_toolbar_track_stop_and_follow_domain_pushes() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app, "bestiary_tracker")?;
    let read = command(&mut commands)?;
    assert_eq!((read.kind, read.payload.as_slice()), (383, &[][..]));
    assert!(shows(&mut app, "Bestiary Tracker"), "{:?}", texts(&mut app));
    assert!(shows(&mut app, "Waiting for the server"));
    reply(
        &mut app,
        &outbox,
        &tracker(false, &[("oteryn:creature.rat", 3)]),
    )?;
    assert!(shows(&mut app, "rat: 3/250 kills"), "{:?}", texts(&mut app));

    let field = app
        .world_mut()
        .query::<(Entity, &KeyField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (!field.0).then_some(entity))
        .ok_or("key field")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new(" oteryn:creature.wolf "));
    click(&mut app, false, &Action::Track)?;
    let add = command(&mut commands)?;
    assert_eq!(add.kind, 383);
    assert_eq!(
        add.payload,
        change(TrackerAction::Add, "oteryn:creature.wolf")?
    );
    reply(
        &mut app,
        &outbox,
        &tracker(
            false,
            &[("oteryn:creature.rat", 3), ("oteryn:creature.wolf", 0)],
        ),
    )?;
    assert!(
        shows(&mut app, "wolf: 0/250 kills"),
        "{:?}",
        texts(&mut app)
    );

    click(&mut app, false, &Action::Stop("oteryn:creature.rat".into()))?;
    let remove = command(&mut commands)?;
    assert_eq!(remove.kind, 383);
    assert_eq!(
        remove.payload,
        change(TrackerAction::Remove, "oteryn:creature.rat")?
    );
    reply(
        &mut app,
        &outbox,
        &tracker(false, &[("oteryn:creature.wolf", 0)]),
    )?;
    assert!(!shows(&mut app, "rat: 3/250 kills"));

    // Domain 98 carries both trackers after a credited kill.
    outbox.bevy_cyclopedia_tracker_push(
        encode_domain_snapshot(
            &tracker(false, &[("oteryn:creature.wolf", 7)]),
            &tracker(true, &[("oteryn:creature.ferumbras", 1)]),
        )
        .ok_or("domain snapshot")?,
    );
    app.update();
    assert!(
        shows(&mut app, "wolf: 7/250 kills"),
        "{:?}",
        texts(&mut app)
    );

    toolbar(&mut app, "bosstiary_tracker")?;
    let boss_read = command(&mut commands)?;
    assert_eq!(
        (boss_read.kind, boss_read.payload.as_slice()),
        (384, &[][..])
    );
    reply(
        &mut app,
        &outbox,
        &tracker(true, &[("oteryn:creature.ferumbras", 2)]),
    )?;
    assert!(shows(&mut app, "Bosstiary Tracker"));
    assert!(
        shows(&mut app, "ferumbras: 2/250 kills"),
        "{:?}",
        texts(&mut app)
    );
    assert!(shows(&mut app, "wolf: 7/250 kills"));

    click(&mut app, false, &Action::Close)?;
    app.update();
    assert!(!shows(&mut app, "Bestiary Tracker"));
    assert!(shows(&mut app, "Bosstiary Tracker"));
    assert!(commands.try_recv().is_err(), "closing sends nothing");
    Ok(())
}

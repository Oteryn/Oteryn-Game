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
    fixture_with(|_| {})
}
fn fixture_with(
    prepare: impl FnOnce(&mut HostOwner),
) -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_app(prepare, App::new())
}
fn fixture_app(
    prepare: impl FnOnce(&mut HostOwner),
    mut app: App,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_tracker_initialize();
    let mut owner = HostOwner::new(None)?;
    prepare(&mut owner);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
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
    // `finish` acts in PostUpdate; the window shows the result on the next frame.
    app.update();
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

/// The texts of one tracker window in display order (depth first over `Children`).
fn window_texts(app: &mut App, boss: bool) -> Vec<String> {
    let name = if boss {
        "bosstiary_tracker.window"
    } else {
        "bestiary_tracker.window"
    };
    let Some(root) = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, n)| (n.as_str() == name).then_some(entity))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            out.push(text.0.clone());
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    out
}
fn control_disabled(
    app: &mut App,
    boss: bool,
    action: &Action,
) -> Result<bool, Box<dyn std::error::Error>> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.boss == boss && control.action == *action).then_some(entity)
        })
        .ok_or("control absent")?;
    Ok(app.world().get::<InteractionDisabled>(entity).is_some())
}
fn refused(app: &mut App, outbox: &Outbox) {
    outbox.bevy_cyclopedia_tracker_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Rejected,
        payload: Vec::new(),
    });
    app.update();
}

#[test]
fn bestiary_tracker_key_editor_keeps_focus_and_text_across_domain_pushes() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app, "bestiary_tracker")?;
    command(&mut commands)?;
    reply(
        &mut app,
        &outbox,
        &tracker(false, &[("oteryn:creature.rat", 3)]),
    )?;
    let field = app
        .world_mut()
        .query::<(Entity, &KeyField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (!field.0).then_some(entity))
        .ok_or("key field")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("oteryn:creature.wo"));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, bevy::input_focus::FocusCause::Navigated);
    // A credited kill pushes domain 98 while the player is typing.
    outbox.bevy_cyclopedia_tracker_push(
        encode_domain_snapshot(
            &tracker(false, &[("oteryn:creature.rat", 4)]),
            &tracker(true, &[]),
        )
        .ok_or("domain snapshot")?,
    );
    app.update();
    assert!(shows(&mut app, "rat: 4/250 kills"), "{:?}", texts(&mut app));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert_eq!(
        app.world()
            .get::<EditableText>(field)
            .ok_or("key field despawned")?
            .value()
            .to_string(),
        "oteryn:creature.wo"
    );
    Ok(())
}

#[test]
fn opening_both_trackers_before_a_reply_reads_both_and_keeps_their_state_apart() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app, "bestiary_tracker")?;
    toolbar(&mut app, "bosstiary_tracker")?;
    let read = command(&mut commands)?;
    assert_eq!(read.kind, 383);
    assert!(commands.try_recv().is_err(), "one request in flight");
    // The Bosstiary window waits for its read, queued behind the Bestiary read.
    assert!(window_texts(&mut app, true).contains(&"Waiting for the server".to_owned()));
    reply(
        &mut app,
        &outbox,
        &tracker(false, &[("oteryn:creature.rat", 3)]),
    )?;
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
    assert!(
        window_texts(&mut app, true).contains(&"ferumbras: 2/250 kills".to_owned()),
        "{:?}",
        window_texts(&mut app, true)
    );

    // A Bosstiary request in flight leaves the Bestiary window usable.
    click(&mut app, true, &Action::Refresh)?;
    assert_eq!(command(&mut commands)?.kind, 384);
    assert!(control_disabled(&mut app, true, &Action::Refresh)?);
    assert!(!control_disabled(&mut app, false, &Action::Refresh)?);
    // A Bestiary refresh waits for it and then goes out.
    click(&mut app, false, &Action::Refresh)?;
    assert!(commands.try_recv().is_err());
    refused(&mut app, &outbox);
    let refused_text = "The server refused the tracker action".to_owned();
    assert!(window_texts(&mut app, true).contains(&refused_text));
    assert!(!window_texts(&mut app, false).contains(&refused_text));
    assert_eq!(command(&mut commands)?.kind, 383);
    reply(
        &mut app,
        &outbox,
        &tracker(false, &[("oteryn:creature.rat", 3)]),
    )?;
    assert!(window_texts(&mut app, true).contains(&refused_text));
    assert!(!window_texts(&mut app, false).contains(&refused_text));
    Ok(())
}

#[test]
fn tracker_sort_and_hide_completed_controls_reorder_and_filter_rows() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app, "bestiary_tracker")?;
    command(&mut commands)?;
    let mut snapshot = tracker(
        false,
        &[
            ("oteryn:creature.bear", 20),
            ("oteryn:creature.rat", 4),
            ("oteryn:creature.wolf", 250),
        ],
    );
    snapshot.entries[2].completed = true;
    reply(&mut app, &outbox, &snapshot)?;
    let rows = |app: &mut App| -> Vec<String> {
        window_texts(app, false)
            .into_iter()
            .filter(|text| text.contains("/250 kills"))
            .collect()
    };
    let (bear, rat, wolf) = (
        "bear: 20/250 kills",
        "rat: 4/250 kills",
        "wolf: 250/250 kills (complete)",
    );
    assert_eq!(rows(&mut app), [bear, rat, wolf]);
    click(&mut app, false, &Action::Sort(TrackerSort::Kills))?;
    assert_eq!(rows(&mut app), [wolf, bear, rat]);
    assert!(shows(&mut app, "Sort: kills"));
    // Kills left to the next stage: wolf none, rat 1 (to 5), bear 5 (to 25).
    click(&mut app, false, &Action::Sort(TrackerSort::Remaining))?;
    assert_eq!(rows(&mut app), [wolf, rat, bear]);
    click(&mut app, false, &Action::Sort(TrackerSort::Name))?;
    assert_eq!(rows(&mut app), [bear, rat, wolf]);
    click(&mut app, false, &Action::HideCompleted(true))?;
    assert_eq!(rows(&mut app), [bear, rat]);
    assert!(shows(&mut app, "Show completed"));
    click(&mut app, false, &Action::HideCompleted(false))?;
    assert_eq!(rows(&mut app), [bear, rat, wolf]);
    assert!(commands.try_recv().is_err(), "sorting is local");
    Ok(())
}

#[test]
fn open_tracker_windows_are_saved_in_the_layout_and_reopen_next_session() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    toolbar(&mut app, "bosstiary_tracker")?;
    assert_eq!(command(&mut commands)?.kind, 384);
    let preferences = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    assert!(preferences.panel_visible("bosstiary-tracker"));
    assert!(!preferences.panel_visible("bestiary-tracker"));

    // The next session starts with the saved layout and reopens the window.
    let (mut next, mut next_commands, _next_outbox) =
        fixture_with(|owner| owner.settings.apply(preferences))?;
    assert_eq!(command(&mut next_commands)?.kind, 384);
    next.update();
    assert!(shows(&mut next, "Bosstiary Tracker"));
    assert!(!shows(&mut next, "Bestiary Tracker"));
    click(&mut next, true, &Action::Close)?;
    assert!(!shows(&mut next, "Bosstiary Tracker"));
    assert!(
        !next
            .world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("bosstiary-tracker")
    );
    Ok(())
}

#[test]
fn acquired_bestiary_field_accepts_typed_creature_key_and_tracks_it() -> TestResult {
    use bevy::{
        asset::AssetPlugin,
        input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::{AcquireFocus, InputDispatchPlugin, tab_navigation::TabNavigationPlugin},
        text::{RemSize, TextPlugin},
        ui_widgets::TextInputPlugin,
        window::{PrimaryWindow, WindowPlugin},
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        InputDispatchPlugin,
        TabNavigationPlugin,
        WindowPlugin::default(),
        TextPlugin,
        TextInputPlugin,
    ))
    .init_resource::<UiScale>()
    .init_resource::<RemSize>()
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    let (mut app, mut commands, outbox) = fixture_app(|_| {}, app)?;
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/assets/FiraMono-subset.ttf"
            ))
            .to_vec(),
        ));
    app.world_mut().resource_mut::<Skins>().font = font;
    toolbar(&mut app, "bestiary_tracker")?;
    assert_eq!(command(&mut commands)?.kind, 383);
    reply(&mut app, &outbox, &tracker(false, &[]))?;
    let world = app.world_mut();
    let field = world
        .query_filtered::<Entity, With<KeyField>>()
        .single(world)?;
    let window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)?;
    world.trigger(AcquireFocus {
        focused_entity: field,
        window,
    });
    assert_eq!(world.resource::<InputFocus>().get(), Some(field));
    let creature = "oteryn:creature.wolf";
    world.write_message(KeyboardInput {
        key_code: KeyCode::KeyO,
        logical_key: Key::Character(creature.into()),
        state: ButtonState::Pressed,
        text: Some(creature.into()),
        repeat: false,
        window,
    });
    app.update();
    click(&mut app, false, &Action::Track)?;
    let add = command(&mut commands)?;
    assert_eq!(add.kind, 383);
    assert_eq!(add.payload, change(TrackerAction::Add, creature)?);
    Ok(())
}

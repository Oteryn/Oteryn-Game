use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    },
};
use oteryn_client_runtime::wheel_presentation as wire;
use oteryn_client_ui::wheel_allocation::{atelier, wheel};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn model() -> WheelPanel {
    let request = wire::Request {
        nonce: 1,
        wheel_revision: 2,
        atelier_revision: 3,
        points: [0; 36],
        vessels: [None; 4],
    };
    let mut p = WheelPanel {
        open: true,
        selected: true,
        synchronized: true,
        confirmed_wheel: Some(wheel::WheelAllocationReply {
            refused: false,
            revision: 2,
            points: [0; 36],
            vessels: Some([None; 4]),
        }),
        confirmed_atelier: Some(atelier::AtelierReply {
            refused: false,
            revision: 3,
            gems: vec![],
            grades: vec![],
            quality_quotes: vec![],
            grade_quotes: vec![],
        }),
        ..Default::default()
    };
    p.diagram.selected = true;
    p.diagram.last = Some(request.clone());
    p.diagram.reply = Some(wire::Reply {
        request,
        catalogue: "test@revision".into(),
        vocation: "knight".into(),
        available: 100,
        unlock_points: 100,
        removal_permitted: true,
        eligible: true,
        draft_valid: true,
        reason: String::new(),
        nodes: (0..36)
            .map(|i| wire::Node {
                domain: (i % 4) as u8,
                capacity: 50,
                minimum_points: 0,
                unlock_from: if i == 14 { vec![] } else { vec![14] },
                name: format!("Perk {}", i + 1),
                per_point: vec![],
                full_slot: vec![],
            })
            .collect(),
        revelations: vec![],
        current: vec![],
        draft: vec![],
    });
    p
}
fn publish(outbox: &Outbox, mut p: WheelPanel) {
    if let Some(reply) = &mut p.diagram.reply {
        reply.request.points = p.points;
        p.diagram.last = Some(reply.request.clone());
    }
    outbox.bevy_wheel_state(p);
}
fn fixture(panel: WheelPanel) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    publish(&outbox, panel);
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
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Result<WheelPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .wheel_panel())
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.kind == kind).then_some(entity))
        .ok_or("control absent".into())
}
fn click(app: &mut App, index: usize, remove: bool) -> TestResult {
    let entity = control(app, Kind::Node(index))?;
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
        button: if remove {
            PointerButton::Secondary
        } else {
            PointerButton::Primary
        },
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    Ok(())
}
#[test]
fn wheel_nodes_from_model() -> TestResult {
    let mut p = model();
    p.points[14] = 7;
    let (mut app, _commands, outbox) = fixture(p)?;
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter(|c| matches!(c.kind, Kind::Node(_)))
            .count(),
        36
    );
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(
        labels
            .iter()
            .any(|s| s.contains("Perk 15") && s.contains("7 / 50"))
    );
    assert!(
        labels
            .iter()
            .any(|s| s.contains("Promotion points: 100") && s.contains("Allocated: 7"))
    );
    assert!(
        !labels
            .iter()
            .any(|s| s == "Gem Atelier" || s == "Fragment Workshop")
    );
    let mut stale = panel(&app)?;
    stale.confirmed_wheel.as_mut().ok_or("wheel")?.revision += 1;
    publish(&outbox, stale);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter(|c| matches!(c.kind, Kind::Node(_)))
            .count(),
        0
    );
    Ok(())
}
#[test]
fn wheel_allocate_rules() -> TestResult {
    let (mut app, _commands, outbox) = fixture(model())?;
    click(&mut app, 0, false)?;
    assert_eq!(panel(&app)?.points[0], 0);
    click(&mut app, 14, false)?;
    assert_eq!(panel(&app)?.points[14], 1);
    let mut p = model();
    p.points[14] = 50;
    publish(&outbox, p);
    app.update();
    click(&mut app, 14, false)?;
    assert_eq!(panel(&app)?.points[14], 50);
    click(&mut app, 0, false)?;
    assert_eq!(panel(&app)?.points[0], 1);
    let mut p = model();
    p.points[14] = 50;
    p.points[0] = 50;
    publish(&outbox, p);
    app.update();
    click(&mut app, 1, false)?;
    assert_eq!(panel(&app)?.points[1], 0);
    let mut p = model();
    p.points[14] = 50;
    p.diagram.reply.as_mut().ok_or("reply")?.nodes[1].minimum_points = 101;
    publish(&outbox, p);
    app.update();
    click(&mut app, 1, false)?;
    assert_eq!(panel(&app)?.points[1], 0);
    let mut p = model();
    p.points[0] = 50;
    p.points[1] = 49;
    let r = p.diagram.reply.as_mut().ok_or("reply")?;
    r.nodes[0].unlock_from = vec![1];
    r.nodes[1].unlock_from = vec![0];
    publish(&outbox, p);
    app.update();
    click(&mut app, 1, false)?;
    assert_eq!(panel(&app)?.points[1], 49);
    Ok(())
}
#[test]
fn wheel_remove_rules() -> TestResult {
    let mut p = model();
    p.points[14] = 50;
    p.points[0] = 1;
    let (mut app, _commands, outbox) = fixture(p)?;
    click(&mut app, 14, true)?;
    assert_eq!(panel(&app)?.points[14], 50);
    click(&mut app, 0, true)?;
    assert_eq!(panel(&app)?.points[0], 0);
    let mut p = model();
    p.points[14] = 50;
    p.diagram.reply.as_mut().ok_or("reply")?.removal_permitted = false;
    publish(&outbox, p);
    app.update();
    click(&mut app, 14, true)?;
    assert_eq!(panel(&app)?.points[14], 50);
    let mut p = model();
    p.points[14] = 50;
    p.diagram.reply.as_mut().ok_or("reply")?.available = 1;
    publish(&outbox, p);
    app.update();
    click(&mut app, 14, true)?;
    assert_eq!(panel(&app)?.points[14], 49);
    publish(&outbox, model());
    app.update();
    click(&mut app, 14, true)?;
    assert_eq!(panel(&app)?.points[14], 0);
    Ok(())
}
#[test]
fn wheel_preset_intents() -> TestResult {
    let mut p = model();
    let mut build = model();
    build.points[14] = 9;
    p.preset_book.insert("Hunt", &build.build_code())?;
    let (mut app, mut commands, outbox) = fixture(p)?;
    let select = control(&mut app, Kind::Preset("Hunt".into()))?;
    app.world_mut().trigger(Activate { entity: select });
    let mut p = panel(&app)?;
    assert_eq!(p.points[14], 9);
    assert_eq!(p.preset_fields[0], "Hunt");
    let request = wire::Request {
        points: p.points,
        ..p.diagram.reply.as_ref().ok_or("reply")?.request.clone()
    };
    p.diagram.reply.as_mut().ok_or("reply")?.request = request.clone();
    p.diagram.last = Some(request);
    publish(&outbox, p);
    app.update();
    let save = control(&mut app, Kind::Save)?;
    app.world_mut().trigger(Activate { entity: save });
    app.world_mut().trigger(Activate { entity: save });
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("save command".into());
    };
    let request = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.action, 1);
    assert_eq!(request.points[14], 9);
    assert_eq!(request.expected_revision, 2);
    assert!(commands.try_recv().is_err());
    publish(&outbox, model());
    app.update();
    let reset = control(&mut app, Kind::Reset)?;
    app.world_mut().trigger(Activate { entity: reset });
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("reset command".into());
    };
    let request = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.action, 2);
    assert_eq!(request.points, [0; 36]);
    Ok(())
}
#[test]
fn wheel_shortcut_and_stale_controls_are_fenced() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut closed = model();
    closed.open = false;
    let (mut app, mut commands, outbox) = fixture(closed)?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(26)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    assert!(panel(&app)?.open);
    assert!(
        panel(&app)?.diagram.last.is_none(),
        "opening must refresh the live point budget and removal permission"
    );
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("wheel query".into());
    };
    let request = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.action, 0);
    publish(&outbox, model());
    app.update();
    let stale = control(&mut app, Kind::Node(14))?;
    let mut changed = model();
    changed.points[14] = 10;
    publish(&outbox, changed);
    let saved = app.world().get::<Control>(stale).ok_or("control")?.clone();
    perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &saved,
        Some(false),
    );
    assert_eq!(panel(&app)?.points[14], 10);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    click(&mut app, 14, false)?;
    assert_eq!(panel(&app)?.points[14], 10);
    Ok(())
}
#[test]
fn wheel_preset_shortcut_opens_closed_panel() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut closed = model();
    closed.open = false;
    let (mut app, mut commands, _outbox) = fixture(closed)?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(26)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    assert!(
        panel(&app)?.open,
        "the preset chord must open the Wheel panel"
    );
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("wheel query".into());
    };
    let request = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.action, 0);
    Ok(())
}
#[path = "presets_tests.rs"]
mod presets_tests;

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
    fixture_with_modal(panel, None)
}
fn fixture_with_modal(
    panel: WheelPanel,
    display: Option<Display>,
) -> Result<Fixture, Box<dyn std::error::Error>> {
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
    if let Some(display) = display {
        app.world_mut().spawn((
            GameModal,
            GlobalZIndex(999),
            Node {
                display,
                ..default()
            },
        ));
    }
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
fn wheel_ignores_hidden_modals() -> TestResult {
    for display in [Display::None, Display::Flex] {
        let mut pending_preview = model();
        pending_preview.diagram.reply = None;
        pending_preview.diagram.last = None;
        let (_app, mut commands, _outbox) = fixture_with_modal(pending_preview, Some(display))?;
        if display == Display::None {
            assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
        }
        assert!(commands.try_recv().is_err());
        let (mut app, mut commands, _outbox) = fixture_with_modal(model(), Some(display))?;
        let close = control(&mut app, Kind::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        assert_eq!(panel(&app)?.open, display == Display::Flex);
        assert!(commands.try_recv().is_err());
    }
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
    let mut allocated = model();
    allocated.points[14] = 9;
    allocated
        .confirmed_wheel
        .as_mut()
        .ok_or("allocation")?
        .points = allocated.points;
    publish(&outbox, allocated);
    app.update();
    let reset = control(&mut app, Kind::Reset)?;
    app.world_mut().trigger(Activate { entity: reset });
    assert!(
        commands.try_recv().is_err(),
        "Reset must only edit the draft"
    );
    let reset_draft = panel(&app)?;
    assert_eq!(reset_draft.points, [0; 36]);
    assert!(reset_draft.diagram.reset_requested);
    assert_eq!(
        reset_draft.confirmed_wheel.ok_or("allocation")?.points[14],
        9
    );

    app.update();
    let PlayCommand::Feature(preview) = commands.try_recv()? else {
        return Err("preview command".into());
    };
    let request = wire::Request::decode(&preview.payload).ok_or("preview request")?;
    assert_eq!(request.points, [0; 36]);
    let mut pending = panel(&app)?;
    let mut reply = pending.diagram.reply.clone().ok_or("preview metadata")?;
    reply.request = request;
    reply.revelations = (0..4)
        .map(|domain| wire::Revelation {
            domain,
            name: "Revelation".into(),
            stages: [25, 50, 100]
                .into_iter()
                .map(|minimum_points| wire::Stage {
                    minimum_points,
                    effects: vec![],
                })
                .collect(),
        })
        .collect();
    let result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        payload: reply.encode().ok_or("preview reply")?,
    };
    diagram::apply(&mut pending, &preview, &result);
    publish(&outbox, pending);
    app.update();
    assert!(commands.try_recv().is_err());
    let save = control(&mut app, Kind::Save)?;
    app.world_mut().trigger(Activate { entity: save });
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

#[test]
fn wheel_has_a_reachable_gem_atelier_entry() -> TestResult {
    let (mut app, _, _) = fixture(model())?;
    let label = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(e, t)| (t.0 == "Gem Atelier / Workshop").then_some(e))
        .ok_or("atelier entry missing")?;
    let button = app.world().get::<ChildOf>(label).ok_or("button")?.parent();
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Gem Atelier and fragment workshop"))
    );
    Ok(())
}

#[test]
fn atelier_reveal_uses_review_cancel_and_one_shot_confirm_through_bevy() -> TestResult {
    let mut p = model();
    p.confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .quality_quotes = vec![atelier::QualityQuote {
        quality: 0,
        reveal_gold: 25000,
        switch_gold: 5000,
        fragment_kind: 0,
        fragment_min: 10,
        fragment_max: 20,
    }];
    let (mut app, mut commands, _) = fixture(p)?;
    let activate_label = |app: &mut App, label: &str| -> TestResult {
        let entity = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .find_map(|(e, t)| (t.0 == label).then_some(e))
            .ok_or("atelier action missing")?;
        let entity = app.world().get::<ChildOf>(entity).ok_or("button")?.parent();
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    activate_label(&mut app, "Gem Atelier / Workshop")?;
    assert!(super::atelier::shared_button_catalogue(&mut app));
    activate_label(&mut app, "Reveal gem")?;
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("25000 gold"))
    );
    activate_label(&mut app, "Cancel review")?;
    assert!(commands.try_recv().is_err());
    activate_label(&mut app, "Reveal gem")?;
    activate_label(&mut app, "Confirm reviewed operation")?;
    let PlayCommand::Wheel(command) = commands.try_recv()? else {
        return Err("reviewed atelier command".into());
    };
    assert_eq!(command.kind, atelier::COMMAND_TYPE_ATELIER_MUTATE);
    let mutation = atelier::decode_mutate(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(mutation.expected_revision, 3);
    assert_eq!(mutation.mutation, atelier::Mutation::Reveal { quality: 0 });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn wheel_renders_confirmed_and_draft_perk_summary() -> TestResult {
    let mut p = model();
    let reply = p.diagram.reply.as_mut().ok_or("diagram")?;
    reply.current = vec!["Max health +5".into()];
    reply.draft = vec!["Max health +10".into()];
    let (mut app, _, _) = fixture(p)?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text
                .0
                .contains("Current allocation configured effects:\nMax health +5")
                && text.0.contains("Draft configured effects:\nMax health +10"))
    );
    Ok(())
}

fn activate_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(entity, text)| (text.0 == caption).then_some(entity))
        .ok_or_else(|| format!("missing {caption}"))?;
    let entity = app.world().get::<ChildOf>(entity).ok_or("button")?.parent();
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

#[test]
fn wheel_inspect_then_fill_and_empty_slice() -> TestResult {
    let mut initial = model();
    initial.slot = 0;
    let (mut app, _commands, outbox) = fixture(initial)?;
    activate_caption(&mut app, "Inspect Perk 15")?;
    let inspected = panel(&app)?;
    assert_eq!(inspected.slot, 14);
    assert_eq!(inspected.points, [0; 36]);
    activate_caption(&mut app, "Fill selected slice")?;
    assert_eq!(panel(&app)?.points[14], 50);
    let mut full = model();
    full.slot = 14;
    full.points[14] = 50;
    publish(&outbox, full);
    app.update();
    activate_caption(&mut app, "Empty selected slice")?;
    assert_eq!(panel(&app)?.points, [0; 36]);
    Ok(())
}

#[test]
fn atelier_filters_quality_and_locked_gems_through_buttons() -> TestResult {
    let mut p = model();
    let gems = &mut p.confirmed_atelier.as_mut().ok_or("atelier")?.gems;
    let mut labels = Vec::new();
    for (i, (quality, locked)) in [(0, true), (1, true), (0, false)].into_iter().enumerate() {
        let gem = atelier::GemView {
            id: [i as u8 + 1; 16],
            quality,
            domain: 0,
            locked,
            vessel: false,
            modifiers: vec![1],
        };
        labels.push(oteryn_client_ui::wheel_allocation::gem_label(&gem));
        gems.push(gem);
    }
    let (mut app, _commands, _) = fixture(p)?;
    activate_caption(&mut app, "Gem Atelier / Workshop")?;
    activate_caption(&mut app, "Quality filter: All")?;
    activate_caption(&mut app, "Show only locked gems")?;
    let shown = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| &text.0)
        .collect::<Vec<_>>();
    assert!(shown.contains(&&labels[0]));
    assert!(!shown.contains(&&labels[1]));
    assert!(!shown.contains(&&labels[2]));
    Ok(())
}

#[test]
fn wheel_opens_installed_gem_and_its_workshop_modifier() -> TestResult {
    use oteryn_client_runtime::gem_presentation as wire;
    let mut p = model();
    p.confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .gems
        .push(atelier::GemView {
            id: [1; 16],
            quality: 0,
            domain: 1,
            locked: false,
            vessel: true,
            modifiers: vec![9],
        });
    p.gems.reply = Some(wire::Reply {
        request: wire::Request {
            nonce: 4,
            revision: 3,
        },
        catalogue: "fixture".into(),
        vocation: "knight".into(),
        grades_current: true,
        qualified_gems: vec![[1; 16]],
        modifiers: vec![wire::Modifier {
            kind: 0,
            id: 9,
            name: "Fire Resistance".into(),
            effects: vec![wire::Effect {
                name: "Resistance".into(),
                target: "Fire".into(),
                unit: 0,
                values: [1000, 2000, 3000, 4000],
            }],
        }],
    });
    let mut waiting = p.clone();
    waiting.gems.pending = Some(wire::Request {
        nonce: 5,
        revision: 3,
    });
    let (mut waiting_app, _commands, _) = fixture(waiting)?;
    activate_caption(&mut waiting_app, "Inspect gem in vessel 2")?;
    assert!(
        !waiting_app
            .world_mut()
            .query::<&Text>()
            .iter(waiting_app.world())
            .any(|t| t.0 == "Back to Wheel"),
        "a pending request cannot open the wrong gem"
    );
    let (mut app, _commands, _) = fixture(p)?;
    activate_caption(&mut app, "Inspect gem in vessel 2")?;
    assert_eq!(panel(&app)?.vessel_slot, 1);
    assert_eq!(panel(&app)?.gem, Some([1; 16]));
    activate_caption(&mut app, "Workshop: Fire Resistance")?;
    assert_eq!(panel(&app)?.modifier, Some((0, 9)));
    activate_caption(&mut app, "Show only in-vessel modifiers")?;
    activate_caption(&mut app, "Workshop kind: All")?;
    activate_caption(&mut app, "Fire Resistance")?;
    activate_caption(&mut app, "Workshop kind: Basic")?;
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Fire Resistance" || t.0 == "Workshop: Fire Resistance")
    );
    let mut fields = app
        .world_mut()
        .query_filtered::<&mut bevy::text::EditableText, With<super::atelier::ModifierSearch>>();
    fields
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text("unknown modifier");
    activate_caption(&mut app, "Apply modifier search")?;
    assert!(
        panel(&app)?.gem.is_none(),
        "detail selection must follow filtered rows"
    );
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Lock / unlock gem")
    );
    Ok(())
}

#[test]
fn wheel_vessel_preset_restores_numbered_slot_through_replace_command() -> TestResult {
    let mut p = model();
    p.atelier_v2 = true;
    let gem = atelier::GemView {
        id: [8; 16],
        quality: 0,
        domain: 6,
        locked: false,
        vessel: true,
        modifiers: vec![],
    };
    p.confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .gems
        .push(gem);
    let mut slots = vec![
        atelier::v2::VesselSlotView {
            quality: 0,
            open: true,
            gem: None
        };
        12
    ];
    slots[6].gem = Some([8; 16]);
    p.vessel_board = Some(oteryn_client_ui::gem_atelier::VesselBoard {
        slots,
        wheel_revision: 2,
        ..Default::default()
    });
    let saved = p.named_build_code();
    p.vessel_board.as_mut().ok_or("board")?.slots[6].gem = None;
    p.confirmed_atelier.as_mut().ok_or("atelier")?.gems[0].vessel = false;
    p.preset_book.insert("Raid", &saved)?;
    let (mut app, mut commands, _) = fixture(p)?;
    activate_caption(&mut app, "Raid")?;
    activate_caption(&mut app, "Apply preset to vessel 7")?;
    let PlayCommand::Wheel(cmd) = commands.try_recv()? else {
        return Err("replace command".into());
    };
    assert_eq!(cmd.kind, 6213);
    Ok(())
}

#[test]
fn wheel_reports_gem_points_separately_from_ring_unlock_points() -> TestResult {
    let mut p = model();
    p.diagram.reply.as_mut().ok_or("reply")?.available = 105;
    let (mut app, _, outbox) = fixture(p)?;
    outbox.bevy_hud_skills_fixture()?;
    app.update();
    assert!(app.world_mut().query::<&Text>().iter(app.world()).any(|t| {
        t.0.contains("Level points: 0") && t.0.contains("Task and pilgrimage points: 100")
    }));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Gem grade bonuses: 5"))
    );
    Ok(())
}

#[test]
fn loading_a_saved_empty_legacy_vessel_clears_only_the_draft() -> TestResult {
    let mut p = model();
    let code = p.named_build_code();
    p.preset_book.insert("Empty vessels", &code)?;
    p.vessels[1] = Some([5; 16]);
    p.confirmed_wheel.as_mut().ok_or("wheel")?.vessels = Some(p.vessels);
    p.diagram.reply.as_mut().ok_or("diagram")?.request.vessels = p.vessels;
    p.confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .gems
        .push(atelier::GemView {
            id: [5; 16],
            domain: 1,
            quality: 0,
            vessel: true,
            locked: false,
            modifiers: vec![1],
        });
    let (mut app, _commands, _) = fixture(p)?;
    activate_caption(&mut app, "Empty vessels")?;
    let result = panel(&app)?;
    assert_eq!(result.vessels, [None; 4]);
    assert_eq!(
        result
            .confirmed_wheel
            .ok_or("wheel")?
            .vessels
            .ok_or("vessels")?[1],
        Some([5; 16])
    );
    Ok(())
}

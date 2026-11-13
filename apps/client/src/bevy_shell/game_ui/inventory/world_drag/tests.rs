use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_input_actions::{PointerCoordinate, PointerDelta, PointerMotion, PointerPosition};
use oteryn_session::{
    ActorPosition, CharacterInventory, EntityDetail, EntityKind, EntityRef, WorldSpatialEntity,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    HostOwner,
    InventoryUi,
    Gesture,
    WorldViewport,
    Vec2,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
);
fn fixture(counted: bool, actor: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, rx, outbox) = play_channel();
    let item = super::super::tests::entry(8, 9)?;
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: Some(super::super::tests::entry(9, 1)?),
            entries: vec![item],
            equipment: vec![],
        }),
        Some(Default::default()),
        counted,
        true,
    );
    let position = ActorPosition {
        x: 100,
        y: 100,
        floor: 0,
    };
    let ground = WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [1; 16],
            generation: 2,
        },
        kind: EntityKind::GroundItem,
        position,
        detail: EntityDetail::Object {
            item_definition_ref: 42,
            quantity: 9,
            item_handle: Some(7.try_into()?),
        },
    };
    let mut entities = vec![ground];
    if actor {
        entities.push(WorldSpatialEntity {
            display_name: None,
            entity: EntityRef {
                identity: [2; 16],
                generation: 3,
            },
            kind: EntityKind::Creature,
            position,
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        });
    }
    let mut owner = super::super::tests::host(link)?;
    let mut view = PlayView::new(TileCoord::new(100, 100), 0, &[])?.with_entities(entities)?;
    view.use_shared_frontend();
    let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(100, 100));
    let (w, h) = view.scene().view().viewport_px();
    owner.play.as_mut().ok_or("play")?.view = view;
    let viewport = WorldViewport {
        origin: UVec2::new(100, 50),
        size: UVec2::new(w + 20, h + 20),
    };
    let point = viewport.origin.as_vec2() + Vec2::new(x + 11., y + 11.);
    Ok((
        owner,
        InventoryUi::default(),
        Gesture::default(),
        viewport,
        point,
        rx,
    ))
}
fn button(state: ButtonState) -> NormalizedInputEvent {
    NormalizedInputEvent::MouseButton {
        button: MouseButton::PRIMARY,
        state,
        modifiers: Modifiers::NONE,
    }
}
fn motion(point: Vec2) -> Result<NormalizedInputEvent, oteryn_input_actions::InputError> {
    Ok(NormalizedInputEvent::PointerMoved {
        position: PointerPosition::new(
            PointerCoordinate::new(point.x as i32)?,
            PointerCoordinate::new(point.y as i32)?,
        ),
        motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
    })
}
fn step(
    owner: &mut HostOwner,
    ui: &mut InventoryUi,
    gesture: &mut Gesture,
    viewport: WorldViewport,
    point: Vec2,
    event: NormalizedInputEvent,
) -> TestResult {
    if matches!(
        event,
        NormalizedInputEvent::MouseButton {
            state: ButtonState::Pressed,
            ..
        }
    ) {
        gesture.picking.press(
            0,
            owner.generation(),
            Entity::PLACEHOLDER,
            picking::Source::World,
            point,
        );
    }
    let consumed = before(ui, gesture, 0, owner, viewport, Some(point), &event, true);
    let routed = if matches!(event, NormalizedInputEvent::PointerMoved { .. }) {
        let (w, h) = owner
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .scene()
            .view()
            .viewport_px();
        match viewport.scene_point(point, UVec2::new(w, h)) {
            Some(local) => motion(local)?,
            None => NormalizedInputEvent::DeviceLost,
        }
    } else {
        event.clone()
    };
    owner.route_input(&[routed], consumed);
    after_press(
        ui,
        gesture,
        0,
        owner,
        viewport,
        Some(point),
        &event,
        true,
        true,
    );
    Ok(())
}
fn start(
    owner: &mut HostOwner,
    ui: &mut InventoryUi,
    gesture: &mut Gesture,
    viewport: WorldViewport,
    point: Vec2,
) -> TestResult {
    step(owner, ui, gesture, viewport, point, motion(point)?)?;
    step(
        owner,
        ui,
        gesture,
        viewport,
        point,
        button(ButtonState::Pressed),
    )?;
    step(
        owner,
        ui,
        gesture,
        viewport,
        point + Vec2::new(8., 0.),
        motion(point + Vec2::new(8., 0.))?,
    )
}
#[test]
fn actual_normalized_world_drag_queues_ground_move_once_and_reset_does_not_abort_item_owner()
-> TestResult {
    let (mut owner, mut ui, mut gesture, viewport, point, mut rx) = fixture(false, false)?;
    start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .item_movement()
            .dragged
            .is_some()
    );
    let end = point + Vec2::new(48., 0.);
    note_world(&mut gesture);
    step(
        &mut owner,
        &mut ui,
        &mut gesture,
        viewport,
        end,
        button(ButtonState::Released),
    )?;
    let PlayCommand::MoveItem(request) = rx.try_recv()? else {
        return Err("ground move missing".into());
    };
    assert_eq!(request.source, 7.try_into()?);
    assert_eq!(request.count, None);
    assert_eq!(
        request.destination,
        ItemMoveDestination::Ground(oteryn_session::item_movement::WorldTilePosition {
            x: 101,
            y: 100,
            floor: 0
        })
    );
    step(
        &mut owner,
        &mut ui,
        &mut gesture,
        viewport,
        end,
        button(ButtonState::Released),
    )?;
    assert!(rx.try_recv().is_err());
    Ok(())
}
#[test]
fn ordinary_click_actor_overlap_outside_focus_modal_and_stale_generation_never_move() -> TestResult
{
    for scope in 0..7 {
        let (mut owner, mut ui, mut gesture, viewport, point, mut rx) = fixture(false, scope == 1)?;
        if scope == 0 {
            step(
                &mut owner,
                &mut ui,
                &mut gesture,
                viewport,
                point,
                motion(point)?,
            )?;
            step(
                &mut owner,
                &mut ui,
                &mut gesture,
                viewport,
                point,
                button(ButtonState::Pressed),
            )?;
        } else {
            start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
        }
        let end = if scope == 2 {
            Vec2::ZERO
        } else {
            point + Vec2::new(48., 0.)
        };
        if scope == 3 {
            before(
                &mut ui,
                &mut gesture,
                0,
                &owner,
                viewport,
                Some(end),
                &NormalizedInputEvent::FocusChanged { focused: false },
                false,
            );
        }
        if scope == 4 {
            owner.close();
        }
        if scope == 5 {
            before(
                &mut ui,
                &mut gesture,
                0,
                &owner,
                viewport,
                Some(end),
                &NormalizedInputEvent::DeviceLost,
                true,
            );
        }
        if scope == 6 {
            before(
                &mut ui,
                &mut gesture,
                0,
                &owner,
                viewport,
                Some(end),
                &motion(end)?,
                false,
            );
        }
        if owner.play.is_some() {
            step(
                &mut owner,
                &mut ui,
                &mut gesture,
                viewport,
                end,
                button(ButtonState::Released),
            )?;
        }
        assert!(
            !std::iter::from_fn(|| rx.try_recv().ok())
                .any(|command| matches!(command, PlayCommand::MoveItem(_))),
            "scope {scope}"
        );
    }
    Ok(())
}
#[test]
fn ground_stack_prompts_then_confirm_uses_real_count() -> TestResult {
    let (mut owner, mut ui, mut gesture, viewport, point, mut rx) = fixture(true, false)?;
    start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
    gesture.picking.drop_on(
        0,
        owner.generation(),
        Entity::PLACEHOLDER,
        picking::Destination::Slot(Slot {
            generation: owner.generation(),
            location: Location::Inventory,
            item: None,
            destination: Some(ItemMoveDestination::MainBackpack),
        }),
    );
    step(
        &mut owner,
        &mut ui,
        &mut gesture,
        viewport,
        Vec2::ZERO,
        button(ButtonState::Released),
    )?;
    assert!(rx.try_recv().is_err());
    let (item, destination) = owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .item_movement()
        .prompt
        .ok_or("prompt missing")?;
    let prompt = Prompt {
        generation: owner.generation(),
        item,
        destination,
    };
    ui.prompt = Some(prompt);
    ui.split_count = 4;
    ui.split_text = Some("4".into());
    super::super::amount::confirm(&owner, &mut ui, prompt);
    let PlayCommand::MoveItem(request) = rx.try_recv()? else {
        return Err("counted move missing".into());
    };
    assert_eq!(request.source, 7.try_into()?);
    assert_eq!(request.count, Some(4));
    assert_eq!(request.destination, ItemMoveDestination::MainBackpack);
    Ok(())
}

fn note_world(gesture: &mut Gesture) {
    gesture
        .picking
        .drop_on(0, 0, Entity::PLACEHOLDER, picking::Destination::World);
}
#[path = "tests/picking.rs"]
mod picking_tests;

#[path = "tests/bindings.rs"]
mod binding_tests;

#[path = "tests/hit_boundary.rs"]
mod hit_boundary_tests;

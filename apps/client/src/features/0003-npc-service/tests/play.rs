use super::*;
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, StepDirection};
#[test]
fn dialogue_selection_requires_visible_npc_and_current_floor() -> Result<(), BatchError> {
    let npc = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Npc,
        entity: EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut view = view()?.with_entities(vec![npc])?;
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(view.selected_npc(), Some(npc.entity));
    assert!(view.inputs.is_empty());
    let mut view = view.with_entities(vec![WorldSpatialEntity {
        kind: EntityKind::Creature,
        ..npc
    }])?;
    assert_eq!(view.selected_npc(), None);
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(view.selected_npc(), None);
    Ok(())
}

fn context_npc() -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Npc,
        entity: EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn pointer_on(
    view: &PlayView,
    tile: TileCoord,
) -> Result<oteryn_input_actions::NormalizedInputEvent, oteryn_input_actions::InputError> {
    use oteryn_input_actions::*;
    let [x, y] = view.scene().view().tile_to_screen(tile);
    Ok(NormalizedInputEvent::PointerMoved {
        position: PointerPosition::new(
            PointerCoordinate::new(x as i32 + 1)?,
            PointerCoordinate::new(y as i32 + 1)?,
        ),
        motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
    })
}
fn secondary() -> oteryn_input_actions::NormalizedInputEvent {
    use oteryn_input_actions::*;
    NormalizedInputEvent::MouseButton {
        button: MouseButton::SECONDARY,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
    }
}
#[test]
fn npc_context_without_capability_leaves_selection_untouched()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?.with_entities(vec![context_npc()])?;
    view.click(TileCoord::new(102, 199))?;
    let original = view.scene().target();
    let (link, _, _) = play_channel();
    let events = [pointer_on(&view, TileCoord::new(101, 200))?, secondary()];
    assert_eq!(link.npc_context_target(&events, &view), None);
    assert_eq!(view.scene().target(), original);
    Ok(())
}
#[test]
fn npc_context_misses_preserve_selection_and_hits_resolve_without_selecting()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?.with_entities(vec![
        context_npc(),
        WorldSpatialEntity {
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [5; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 99,
                y: 200,
                floor: 0,
            },
            ..context_npc()
        },
    ])?;
    let (link, _, outbox) = play_channel();
    mailbox::lock(&outbox.0).dialogue.selected = true;
    view.click(TileCoord::new(101, 200))?;
    let original = view.scene().target();
    for tile in [
        TileCoord::new(99, 200),
        TileCoord::new(102, 199),
        TileCoord::new(98, 200),
    ] {
        let events = [pointer_on(&view, tile)?, secondary()];
        assert_eq!(link.npc_context_target(&events, &view), None);
        assert_eq!(view.scene().target(), original);
    }
    view.click(TileCoord::new(102, 199))?;
    let original = view.scene().target();
    let events = [pointer_on(&view, TileCoord::new(101, 200))?, secondary()];
    assert_eq!(
        link.npc_context_target(&events, &view),
        Some(context_npc().entity)
    );
    assert_eq!(view.scene().target(), original);
    Ok(())
}
#[test]
fn npc_pointer_resets_on_focus_and_device_loss() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::NormalizedInputEvent;
    let view = view()?.with_entities(vec![context_npc()])?;
    let (link, _, outbox) = play_channel();
    mailbox::lock(&outbox.0).dialogue.selected = true;
    for reset in [
        NormalizedInputEvent::FocusChanged { focused: false },
        NormalizedInputEvent::DeviceLost,
    ] {
        let events = [
            pointer_on(&view, TileCoord::new(101, 200))?,
            reset,
            secondary(),
        ];
        assert_eq!(link.npc_context_target(&events, &view), None);
        assert_eq!(link.npc_context_target(&[secondary()], &view), None);
    }
    Ok(())
}

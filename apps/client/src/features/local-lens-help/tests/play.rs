use crate::play::{HudAction, HudPanel, PlayView, play_channel};
use oteryn_client_ui::lens_help;
use oteryn_renderer::TileCoord;
use oteryn_session::{ActorPosition, EntityDetail, EntityRef, StepDirection, WorldSpatialEntity};

const ROOT: [u16; 2] = [1280, 720];

fn creature_at(x: i32, y: i32) -> Result<WorldSpatialEntity, &'static str> {
    Ok(WorldSpatialEntity {
        entity: EntityRef {
            identity: [5; 16],
            generation: 1,
        },
        kind: oteryn_session::EntityKind::Creature,
        display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
        position: ActorPosition { x, y, floor: 0 },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    })
}

#[test]
fn lens_mode_toggles_and_a_click_shows_one_help_text_then_ends_the_mode() {
    let (link, _commands, _outbox) = play_channel();
    assert!(!link.lens_active());
    assert!(link.lens_toggle());
    assert!(link.lens_active());
    link.lens_click(Some("minimap"));
    assert!(!link.lens_active());
    assert_eq!(
        link.chat_panel().feedback.as_deref(),
        lens_help::help_text("minimap")
    );
    // A spot the lens knows nothing about ends the mode with its off notice.
    assert!(link.lens_toggle());
    link.lens_click(None);
    assert!(!link.lens_active());
    assert_eq!(
        link.chat_panel().feedback.as_deref(),
        Some(lens_help::mode_text(false))
    );
    // Toggling twice leaves it off.
    assert!(link.lens_toggle());
    assert!(!link.lens_toggle());
}

#[test]
fn lens_key_names_the_hud_pane_the_creature_and_the_world_under_the_point()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?
        .with_entities(vec![creature_at(101, 200)?])?;
    let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(101, 200));
    assert_eq!(view.lens_key([x + 1.0, y + 1.0], ROOT), Some("creature"));
    let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(100, 200));
    assert_eq!(view.lens_key([x + 1.0, y + 1.0], ROOT), Some("world"));
    // Outside the game window there is nothing to read about.
    assert_eq!(view.lens_key([-5.0, 3.0], ROOT), None);
    for (panel, key) in [
        (HudPanel::Resources, "resources"),
        (HudPanel::Combat, "combat"),
        (HudPanel::Conditions, "conditions"),
        (HudPanel::Monk, "monk"),
    ] {
        view.hud_action(HudAction::Visible(panel, true), ROOT)?;
        let rect = view
            .preferences()
            .effective_hud_layout()
            .rect(panel, ROOT, view.preferences().interface_scale)
            .ok_or("visible pane has a rect")?;
        let inside = [
            f32::from(rect[0]) + f32::from(rect[2]) / 2.0,
            f32::from(rect[1]) + f32::from(rect[3]) / 2.0,
        ];
        assert_eq!(view.lens_key(inside, ROOT), Some(key));
        assert!(lens_help::help_text(key).is_some());
    }
    Ok(())
}

#[test]
fn lens_gesture_never_arms_a_creature_attack_or_push() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{
        ButtonState, Modifiers, MouseButton, NormalizedInputEvent, PointerCoordinate, PointerDelta,
        PointerMotion, PointerPosition,
    };
    for (alt, delta) in [(false, 0), (false, 48), (true, 0), (true, 48)] {
        let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?
            .with_entities(vec![creature_at(101, 200)?])?;
        let (link, mut commands, _) = play_channel();
        let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(101, 200));
        let moved = NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(
                PointerCoordinate::new(x as i32 + 1)?,
                PointerCoordinate::new(y as i32 + 1)?,
            ),
            motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
        };
        let modifiers = if alt {
            Modifiers::one(oteryn_input_actions::Modifier::Alt)
        } else {
            Modifiers::NONE
        };
        let button = |state| NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state,
            modifiers,
        };
        assert!(link.lens_toggle());
        assert!(!view.attack_toggle_click(TileCoord::new(101, 200), &link));
        view.world_pointer_input(&link, &[moved, button(ButtonState::Pressed)])?;
        assert!(view.creature_drag.source.is_none());
        link.lens_click(Some("creature"));
        let moved = NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(
                PointerCoordinate::new(x as i32 + 1 + delta)?,
                PointerCoordinate::new(y as i32 + 1)?,
            ),
            motion: PointerMotion::new(PointerDelta::new(delta)?, PointerDelta::new(0)?),
        };
        view.world_pointer_input(&link, &[moved, button(ButtonState::Released)])?;
        assert!(view.inputs.is_empty());
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

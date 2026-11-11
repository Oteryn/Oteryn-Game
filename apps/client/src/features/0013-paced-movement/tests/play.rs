use super::*;

#[test]
fn arrow_key_steps_without_a_goal() -> Result<(), BatchError> {
    let mut view = view()?;
    assert_eq!(view.facing(), StepDir::South);
    view.arrow(StepDir::North);
    assert_eq!(view.next_step(), Some(StepDir::North));
    assert_eq!(view.next_step(), None);
    // The player turns to the step sent, moved or not.
    assert_eq!(view.facing(), StepDir::North);
    Ok(())
}

#[test]
fn sending_a_step_redraws_the_new_facing() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(view.own())?;
    let target = view.scene().target();
    assert!(target.is_some());
    view.arrow(StepDir::North);
    assert_eq!(view.send_step()?, Some(StepDir::North));
    assert_eq!(view.scene().facing(), StepDir::North);
    // Turning keeps the selection.
    assert_eq!(view.scene().target(), target);
    Ok(())
}

#[test]
fn arrow_step_outcome_moves_the_view_and_refusal_keeps_the_position() -> Result<(), BatchError> {
    let mut view = view()?;
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(99, 200, 0))?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.scene().view().origin(), TileCoord::new(92, 195));
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    let mut blocked = moved_to(99, 200, 0);
    blocked.disposition = StepDisposition::Blocked;
    blocked.world_spatial_delta = None;
    view.apply(&blocked)?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.next_step(), None);
    Ok(())
}

#[test]
fn one_step_is_in_flight_until_its_outcome() -> Result<(), BatchError> {
    let mut view = view()?;
    view.arrow(StepDir::North);
    assert_eq!(view.next_step(), Some(StepDir::North));
    view.arrow(StepDir::East);
    assert_eq!(view.next_step(), None);
    view.apply(&moved_to(100, 199, 0))?;
    assert_eq!(view.next_step(), Some(StepDir::East));
    Ok(())
}

#[test]
fn m40_keypad_diagonals_reach_wire_and_apply_only_authoritative_positions()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    use oteryn_protocol_oteryn::world_spatial::{
        CAPABILITY_DIAGONAL_MOVEMENT_V1, decode_step_intent, decode_step_intent_selected,
        encode_step_intent,
    };
    for (key, direction, wire, x, y) in [
        (97, StepDir::NorthEast, StepDirection::NorthEast, 101, 199),
        (91, StepDir::SouthEast, StepDirection::SouthEast, 101, 201),
        (89, StepDir::SouthWest, StepDirection::SouthWest, 99, 201),
        (95, StepDir::NorthWest, StepDirection::NorthWest, 99, 199),
    ] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(key)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::default(),
            repeat: false,
        };
        let input = crate::input::arrow_step(&[event]);
        assert_eq!(input, Some(direction));
        let mut view = view()?;
        view.arrow(input.ok_or("missing keypad intent")?);
        assert_eq!(view.send_step()?, Some(direction));
        assert_eq!(view.own(), TileCoord::new(100, 200));
        assert_eq!(view.scene().facing(), direction);
        let encoded = encode_step_intent(step_direction(direction));
        assert!(decode_step_intent(&encoded).is_err());
        assert_eq!(
            decode_step_intent_selected(&[CAPABILITY_DIAGONAL_MOVEMENT_V1], &encoded)
                .map_err(|error| format!("{error:?}"))?,
            wire
        );
        view.arrow(direction);
        assert_eq!(view.next_step(), None, "only one diagonal may be in flight");
        let mut refused = moved_to(100, 200, 0);
        refused.disposition = StepDisposition::Blocked;
        refused.world_spatial_delta = None;
        view.apply(&refused)?;
        assert_eq!(view.own(), TileCoord::new(100, 200));
        assert_eq!(view.next_step(), Some(direction));
        view.apply(&moved_to(x, y, 0))?;
        assert_eq!(view.own(), TileCoord::new(x, y));
        assert_eq!(view.next_step(), None);
    }
    Ok(())
}

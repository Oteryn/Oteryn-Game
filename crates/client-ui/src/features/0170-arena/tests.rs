use super::*;
use oteryn_client_runtime::{
    arena::*,
    raw_feature::{CommandStatus, GatedCommandResult},
};

fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}
#[test]
fn arena_controls_gate_pending_and_never_predict_progress() -> Result<(), String> {
    let mut p = ArenaPanel::default();
    assert!(request(&mut p, Intent::Catalogue).is_none());
    p.selected = true;
    for action in [
        ArenaAction::Status,
        ArenaAction::Enter,
        ArenaAction::Advance,
        ArenaAction::Leave,
    ] {
        let c = request(&mut p, Intent::Action(action)).ok_or("request")?;
        assert_eq!((c.capability, c.kind), (170, 436));
        assert_eq!(
            decode_match_request(&c.payload).map_err(|e| format!("{e:?}"))?,
            action
        );
        assert!(p.result.is_none());
        assert!(request(&mut p, Intent::Catalogue).is_none());
        assert!(!apply_result(&mut p, &reply(vec![8, 11])));
        assert!(p.pending.is_none());
        assert!(p.result.is_none());
    }
    for rank in 1..=3 {
        for category in 1..=4 {
            for choice in 1..=if category == 3 { 3 } else { 1 } {
                let action = ArenaAction::ClaimReward {
                    rank,
                    category,
                    choice,
                };
                let c = request(&mut p, Intent::Action(action)).ok_or("claim")?;
                assert_eq!(
                    decode_match_request(&c.payload).map_err(|e| format!("{e:?}"))?,
                    action
                );
                let mut denied = reply(vec![]);
                denied.status = CommandStatus::Rejected;
                assert!(!apply_result(&mut p, &denied));
            }
        }
    }
    assert!(
        request(
            &mut p,
            Intent::Action(ArenaAction::ClaimReward {
                rank: 0,
                category: 1,
                choice: 1
            })
        )
        .is_none()
    );
    Ok(())
}
#[test]
fn arena_catalogue_replay_and_refusal_render_only_valid_server_state() -> Result<(), String> {
    let mut p = ArenaPanel {
        selected: true,
        ..Default::default()
    };
    let c = request(&mut p, Intent::Catalogue).ok_or("catalogue")?;
    assert_eq!((c.kind, c.payload), (240, vec![]));
    assert!(apply_result(
        &mut p,
        &reply(encode_catalogue().map_err(|e| format!("{e:?}"))?)
    ));
    assert_eq!(p.challenges, SVARGROND_CHALLENGES);
    request(&mut p, Intent::Action(ArenaAction::Status)).ok_or("status")?;
    let result = ArenaResult {
        disposition: ArenaDisposition::InventoryFull,
        state: ArenaState {
            completed_rank: 1,
            claimed_masks: [5, 0, 0],
            ..Default::default()
        },
    };
    let mut r = reply(encode_match_result(&result).map_err(|e| format!("{e:?}"))?);
    r.status = CommandStatus::DuplicateReplay;
    assert!(apply_result(&mut p, &r));
    assert_eq!(p.result, Some(result));
    assert!(panel_text(&p).contains("Inventory full"));
    assert!(panel_text(&p).contains("Greenhorn: 1000 gold"));
    request(&mut p, Intent::Action(ArenaAction::Status)).ok_or("refresh")?;
    assert!(!apply_result(&mut p, &reply(vec![8, 1, 32, 11])));
    assert_eq!(p.result, Some(result));
    assert!(!apply_result(&mut p, &r)); // unsolicited result has no matching request
    Ok(())
}

#[test]
fn arena_open_binding_requires_control_shift_z_press()
-> Result<(), oteryn_input_actions::InputError> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(29)?;
    let both = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, both, false)]));
    assert!(!open_key(&[NormalizedInputEvent::Key {
        code: KeyCode::new(4)?,
        state: ButtonState::Pressed,
        modifiers: both,
        repeat: false,
    }]));
    for e in [
        event(ButtonState::Released, both, false),
        event(ButtonState::Pressed, both, true),
        event(
            ButtonState::Pressed,
            Modifiers::one(Modifier::Control),
            false,
        ),
        event(ButtonState::Pressed, Modifiers::one(Modifier::Shift), false),
        event(ButtonState::Pressed, both.with(Modifier::Alt), false),
    ] {
        assert!(!open_key(&[e]));
    }
    Ok(())
}

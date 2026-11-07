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

#[test]
fn arena_panel_exposes_tibiadrome_alongside_svargrond() {
    let panel = ArenaPanel::default();
    let text = panel_text(&panel);
    assert!(text.contains("Svargrond Arena"));
    assert!(
        text.contains("Tibiadrome"),
        "native arena panel omits Tibiadrome: {text}"
    );
}

// Fixed vectors from PR #3002 c64d5437's DromeResult encoder, fields 1..12.
const DROME_REPLY: &[u8] = &[
    8, 2, 16, 1, 24, 7, 32, 9, 40, 70, 48, 1, 56, 10, 64, 120, 72, 60, 80, 3, 88, 2, 96, 128, 128,
    128, 128, 8,
];

#[test]
fn drome_requests_are_gated_and_decode_only_the_expected_reply_family() -> Result<(), String> {
    let mut panel = ArenaPanel::default();
    assert!(
        request(
            &mut panel,
            Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
                DromeAction::Enter
            ))
        )
        .is_none()
    );
    panel.selected = true;
    let mut actions = vec![
        (DromeAction::Status, vec![8, 6, 16, 1]),
        (DromeAction::Enter, vec![8, 6, 16, 2]),
        (DromeAction::Leave, vec![8, 6, 16, 3]),
        (DromeAction::LowerStart, vec![8, 6, 16, 4]),
        (DromeAction::ClaimPotion, vec![8, 6, 16, 5]),
    ];
    for selector in 1..=18 {
        actions.push((DromeAction::Buy(selector), vec![8, 6, 16, 6, 24, selector]));
    }
    for (index, (action, bytes)) in actions.into_iter().enumerate() {
        let (control, selected) = if index < 5 {
            (200 + index, -1)
        } else {
            (205, (index - 5) as isize)
        };
        assert_eq!(
            drome_intent(control, selected),
            Some(Intent::Action(
                oteryn_client_runtime::arena::ArenaAction::Drome(action)
            ))
        );
        let before = panel.drome;
        let command = request(
            &mut panel,
            Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(action)),
        )
        .ok_or("Drome request")?;
        assert_eq!(
            (command.capability, command.kind, command.payload),
            (170, 436, bytes)
        );
        assert_eq!(panel.drome, before, "no predicted progression or reward");
        assert!(request(&mut panel, Intent::Catalogue).is_none());
        let mut result = reply(DROME_REPLY.to_vec());
        result.status = CommandStatus::DuplicateReplay;
        assert!(apply_result(&mut panel, &result));
        let state = panel.drome.ok_or("Drome reply")?;
        assert_eq!(
            (state.level, state.start, state.points, state.wave),
            (7, 9, 70, 10)
        );
        assert_eq!(
            (state.remaining, state.cooldown, state.rank, state.potions),
            (120, 60, 3, 2)
        );
        assert_eq!(state.rotation_ends, 0x8000_0000);
        assert!(panel.result.is_none());
        assert!(!apply_result(&mut panel, &result), "unsolicited reply");
    }
    for selector in [0, 19, 255] {
        assert!(
            request(
                &mut panel,
                Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
                    DromeAction::Buy(selector)
                ))
            )
            .is_none()
        );
        assert!(panel.pending.is_none());
    }
    let before = panel.drome;
    request(
        &mut panel,
        Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
            DromeAction::Status,
        )),
    )
    .ok_or("request")?;
    let svargrond = ArenaResult {
        disposition: ArenaDisposition::Applied,
        state: ArenaState::default(),
    };
    assert!(!apply_result(
        &mut panel,
        &reply(encode_match_result(&svargrond).map_err(|e| format!("{e:?}"))?)
    ));
    assert_eq!(panel.drome, before);
    request(&mut panel, Intent::Action(ArenaAction::Status)).ok_or("Svargrond")?;
    assert!(!apply_result(&mut panel, &reply(DROME_REPLY.to_vec())));
    assert!(panel.result.is_none());
    for disposition in 1..=8 {
        let mut bytes = DROME_REPLY.to_vec();
        bytes[3] = disposition;
        request(
            &mut panel,
            Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
                DromeAction::Status,
            )),
        )
        .ok_or("refresh")?;
        assert!(apply_result(&mut panel, &reply(bytes)));
        assert!(panel_text(&panel).contains(match disposition {
            1 => "Applied",
            2 => "Action unavailable",
            3 => "Premium and level 50",
            4 => "Re-entry cooldown active",
            5 => "Arena busy",
            6 => "content unavailable",
            7 => "insufficient drome points",
            _ => "Inventory full",
        }));
    }
    let before = panel.drome;
    request(
        &mut panel,
        Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
            DromeAction::Status,
        )),
    )
    .ok_or("request")?;
    let mut denied = reply(vec![]);
    denied.status = CommandStatus::Rejected;
    assert!(!apply_result(&mut panel, &denied));
    assert_eq!(panel.drome, before);
    assert!(panel.pending.is_none());
    assert!(panel_text(&panel).contains("server refused"));
    Ok(())
}

#[test]
fn drome_reply_refuses_malformed_oversized_and_wrong_version_data() {
    for bytes in [
        vec![],
        vec![8, 1, 16, 1],
        vec![8, 2, 16, 0],
        vec![8, 2, 16, 9],
        vec![8, 2, 16, 1, 48, 3],
        vec![8, 2, 16, 1, 8, 2],
        vec![8, 2, 16, 1, 104, 1],
        vec![8, 2, 16, 1, 0, 1],
        vec![10, 2, 16, 1],
        vec![8, 2, 16],
        vec![8, 2, 16, 1, 24, 128],
        vec![8, 2, 16, 1, 24, 255, 255, 255, 255, 16],
        vec![
            8, 2, 16, 1, 24, 128, 128, 128, 128, 128, 128, 128, 128, 128, 2,
        ],
        vec![
            8, 2, 16, 1, 24, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 0,
        ],
        vec![0; 129],
    ] {
        assert!(DromeReply::decode(&bytes).is_err(), "accepted {bytes:?}");
    }
    let mut max = vec![8, 2, 16, 1];
    for field in 3..=12 {
        max.push(field << 3);
        if field == 6 {
            max.push(2);
        } else {
            max.extend_from_slice(&[255, 255, 255, 255, 15]);
        }
    }
    let decoded = DromeReply::decode(&max);
    assert!(decoded.is_ok());
    assert!(decoded.is_ok_and(|state| state.points == u32::MAX));
    assert!(
        DromeReply::decode(&[8, 130, 0, 16, 1]).is_ok(),
        "legal padded varint"
    );
}

#[test]
fn drome_native_selection_requires_a_valid_control_and_reward() {
    for invalid in [-1, 18, isize::MAX] {
        assert!(drome_intent(205, invalid).is_none());
    }
    for invalid in [0, 199, 206, usize::MAX] {
        assert!(drome_intent(invalid, 0).is_none());
    }
}

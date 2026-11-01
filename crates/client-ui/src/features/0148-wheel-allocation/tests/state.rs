use super::*;

#[path = "resources.rs"]
mod resources;
#[path = "workshop.rs"]
mod workshop;

type Error = Box<dyn std::error::Error>;

fn wire<T>(result: Result<T, impl std::fmt::Debug>) -> Result<T, Error> {
    result.map_err(|error| format!("{error:?}").into())
}

fn occurrence(byte: u8) -> [u8; 16] {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn current(revision: u64) -> (wheel::WheelAllocationReply, atelier::AtelierReply) {
    (
        wheel::WheelAllocationReply {
            refused: false,
            revision,
            points: [0; 36],
            vessels: None,
        },
        atelier::AtelierReply {
            refused: false,
            revision,
            gems: vec![atelier::GemView {
                id: [17; 16],
                quality: 2,
                domain: 0,
                locked: false,
                vessel: false,
                modifiers: vec![31, 0, 5],
            }],
            grades: vec![atelier::GradeView {
                kind: 1,
                mod_id: 5,
                grade: 1,
            }],
            quality_quotes: (0..3)
                .map(|quality| atelier::QualityQuote {
                    quality,
                    reveal_gold: 123_456 + u64::from(quality),
                    switch_gold: 654_321,
                    fragment_kind: u8::from(quality == 2),
                    fragment_min: 9,
                    fragment_max: 11,
                })
                .collect(),
            grade_quotes: (0..2)
                .flat_map(|kind| {
                    (1..4).map(move |grade| atelier::GradeQuote {
                        kind,
                        grade,
                        gold: 222_222,
                        fragments: 777,
                    })
                })
                .collect(),
        },
    )
}

fn panel() -> WheelPanel {
    let (wheel, atelier) = current(4);
    let mut panel = WheelPanel {
        selected: true,
        synchronized: true,
        confirmed_wheel: Some(wheel),
        confirmed_atelier: Some(atelier),
        ..WheelPanel::default()
    };
    panel.select_gem(Some([17; 16]));
    panel
}

fn response(
    kind: u32,
    status: CommandStatus,
    payload: Vec<u8>,
    wheel: &wheel::WheelAllocationReply,
    atelier: &atelier::AtelierReply,
) -> Result<WheelResponse, Error> {
    Ok(WheelResponse {
        kind,
        status,
        payload,
        wheel: Some(wire(wheel::encode_wheel_reply(wheel))?),
        atelier: Some(wire(atelier::encode_reply(atelier))?),
        resources: None,
    })
}

#[test]
fn wheel_proposals_preserve_confirmed_state_and_encode_all_four_vessels() -> Result<(), Error> {
    let mut panel = panel();
    let confirmed = panel.confirmed_wheel.clone();
    assert!(!panel.set_points(36, 1));
    assert!(!panel.set_points(14, 51));
    assert!(panel.set_points(14, 50));
    assert!(panel.socket_selected());
    assert_eq!(panel.confirmed_wheel, confirmed);
    assert!(!panel.selected_gem().ok_or("gem")?.vessel);
    assert!(panel.dirty());
    let command = request(&mut panel, Intent::Save, occurrence(3)).ok_or("save")?;
    assert_eq!(command.kind, 161);
    assert!(command.mutation);
    let decoded = wire(wheel::decode_wheel_request(&command.payload))?;
    assert_eq!(decoded.action, 1);
    assert_eq!(decoded.expected_revision, 4);
    assert_eq!(decoded.occurrence, occurrence(3));
    assert_eq!(decoded.points[14], 50);
    assert_eq!(decoded.vessels, Some([Some([17; 16]), None, None, None]));
    assert!(request(&mut panel, Intent::Save, occurrence(4)).is_none());
    assert!(!panel.set_points(14, 0));
    assert_eq!(panel.confirmed_wheel, confirmed);
    Ok(())
}

#[test]
fn reset_and_unsocket_use_the_existing_allocation_owner() -> Result<(), Error> {
    let mut panel = panel();
    panel.confirmed_atelier.as_mut().ok_or("atelier")?.gems[0].vessel = true;
    panel.vessels[0] = Some([17; 16]);
    assert!(panel.clear_vessel());
    let command = request(&mut panel, Intent::Save, occurrence(1)).ok_or("unsocket")?;
    assert_eq!(
        wire(wheel::decode_wheel_request(&command.payload))?.vessels,
        Some([None; 4])
    );
    panel.pending = None;
    let command = request(&mut panel, Intent::Reset, occurrence(2)).ok_or("reset")?;
    let decoded = wire(wheel::decode_wheel_request(&command.payload))?;
    assert_eq!(decoded.action, 2);
    assert_eq!(decoded.points, [0; 36]);
    assert_eq!(decoded.vessels, None);
    Ok(())
}

#[test]
fn atelier_controls_send_only_intent_and_current_revision() -> Result<(), Error> {
    for (intent, mutation) in [
        (Intent::Reveal, atelier::Mutation::Reveal { quality: 2 }),
        (
            Intent::SwitchDomain,
            atelier::Mutation::SwitchDomain { gem: [17; 16] },
        ),
        (
            Intent::Dismantle,
            atelier::Mutation::Dismantle { gem: [17; 16] },
        ),
        (
            Intent::GradeUp,
            atelier::Mutation::GradeUp { kind: 1, mod_id: 5 },
        ),
    ] {
        let mut panel = panel();
        panel.quality = 2;
        panel.modifier = Some((1, 5));
        let command = request(&mut panel, intent, occurrence(7)).ok_or("mutation")?;
        assert_eq!(command.kind, 424);
        assert_eq!(
            wire(atelier::decode_mutate(&command.payload))?,
            atelier::MutateRequest {
                occurrence: occurrence(7),
                expected_revision: 4,
                mutation,
            }
        );
    }
    let mut panel = panel();
    let command = request(&mut panel, Intent::ToggleLock, occurrence(6)).ok_or("lock")?;
    assert_eq!(command.kind, 309);
    assert_eq!(
        wire(atelier::decode_set_lock(&command.payload))?,
        atelier::SetLockRequest {
            occurrence: occurrence(6),
            expected_revision: 4,
            gem: [17; 16],
            locked: true,
        }
    );
    Ok(())
}

#[test]
fn paid_actions_require_live_quotes_and_unresolved_changes_remain_blocked() -> Result<(), Error> {
    let mut unknown = WheelPanel::default();
    assert!(request(&mut unknown, Intent::Refresh, occurrence(1)).is_none());
    unknown.selected = true;
    assert!(request(&mut unknown, Intent::Reveal, occurrence(1)).is_none());
    assert!(request(&mut unknown, Intent::Refresh, occurrence(1)).is_some());
    let mut panel = panel();
    panel
        .confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .quality_quotes
        .clear();
    assert!(request(&mut panel, Intent::Reveal, occurrence(1)).is_none());
    assert!(request(&mut panel, Intent::SwitchDomain, occurrence(1)).is_none());
    panel
        .confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .grade_quotes
        .clear();
    assert!(request(&mut panel, Intent::GradeUp, occurrence(1)).is_none());
    assert!(panel.set_points(14, 5));
    assert!(request(&mut panel, Intent::ToggleLock, occurrence(1)).is_none());
    Ok(())
}

#[test]
fn live_queries_replace_historical_receipts_and_restore_current_vessels_and_prices()
-> Result<(), Error> {
    let mut panel = panel();
    request(&mut panel, Intent::Reveal, occurrence(1)).ok_or("reveal")?;
    let (_, mut historical) = current(1);
    historical.quality_quotes.clear();
    historical.grade_quotes.clear();
    historical.gems[0].modifiers = vec![0, 1, 2];
    let (mut wheel, mut atelier) = current(6);
    wheel.points[14] = 50;
    atelier.gems[0].vessel = true;
    atelier.grades[0].grade = 3;
    let response = response(
        424,
        CommandStatus::DuplicateReplay,
        wire(atelier::encode_reply(&historical))?,
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &response));
    assert_eq!(panel.confirmed_wheel, Some(wheel));
    assert_eq!(panel.confirmed_atelier, Some(atelier));
    assert_eq!(panel.vessels[0], Some([17; 16]));
    assert_eq!(panel.grade(1, 5), 3);
    assert!(panel.ready());
    assert!(!panel.dirty());
    assert!(panel.retry.is_none());
    Ok(())
}

#[test]
fn refused_changes_show_the_live_state_without_optimistic_commits() -> Result<(), Error> {
    let mut panel = panel();
    panel.set_points(14, 50);
    request(&mut panel, Intent::Save, occurrence(2)).ok_or("save")?;
    let (wheel, atelier) = current(4);
    let mut refusal = wheel.clone();
    refusal.refused = true;
    assert!(apply_result(
        &mut panel,
        &response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&refusal))?,
            &wheel,
            &atelier
        )?
    ));
    assert_eq!(panel.points, [0; 36]);
    assert!(panel.feedback.is_some_and(|text| text.contains("refused")));
    assert!(panel.retry.is_none());
    Ok(())
}

#[test]
fn unavailable_live_views_preserve_confirmed_state_and_explain_eligibility() -> Result<(), Error> {
    for refused_wheel in [false, true] {
        let mut panel = panel();
        let before = (
            panel.confirmed_wheel.clone(),
            panel.confirmed_atelier.clone(),
        );
        request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
        let (mut wheel, mut atelier) = current(0);
        wheel.refused = refused_wheel;
        atelier.refused = !refused_wheel;
        let reply = response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&wheel))?,
            &wheel,
            &atelier,
        )?;
        assert!(!apply_result(&mut panel, &reply));
        assert_eq!(
            (&panel.confirmed_wheel, &panel.confirmed_atelier),
            (&before.0, &before.1)
        );
        assert!(!panel.ready());
        assert!(!panel.synchronized);
        assert!(
            panel
                .feedback
                .is_some_and(|message| message.contains("level, promotion and Premium"))
        );
        assert!(request(&mut panel, Intent::Reveal, occurrence(1)).is_none());
        assert!(request(&mut panel, Intent::Refresh, [0; 16]).is_some());
    }
    Ok(())
}

#[test]
fn stale_or_incomplete_live_queries_never_partially_replace_the_view() -> Result<(), Error> {
    let mut panel = panel();
    let before = (
        panel.confirmed_wheel.clone(),
        panel.confirmed_atelier.clone(),
    );
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (mut wheel, atelier) = current(3);
    wheel.points[14] = 25;
    let stale = response(
        161,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    assert!(!apply_result(&mut panel, &stale));
    assert_eq!(
        (&panel.confirmed_wheel, &panel.confirmed_atelier),
        (&before.0, &before.1)
    );
    assert!(!panel.ready());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (wheel, atelier) = current(5);
    let mut incomplete = response(
        161,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    incomplete.atelier = Some(vec![255]);
    assert!(!apply_result(&mut panel, &incomplete));
    assert_eq!(
        (&panel.confirmed_wheel, &panel.confirmed_atelier),
        (&before.0, &before.1)
    );
    assert!(!panel.synchronized);
    Ok(())
}

#[test]
fn uncertain_outcome_retains_exact_request_across_refresh_and_retry() -> Result<(), Error> {
    let mut panel = panel();
    let command = request(&mut panel, Intent::Reveal, occurrence(1)).ok_or("reveal")?;
    let (wheel, atelier) = current(5);
    let expired = response(
        424,
        CommandStatus::DuplicateOutcomeExpired,
        vec![],
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &expired));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    assert!(request(&mut panel, Intent::Reveal, occurrence(2)).is_none());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    assert!(apply_result(
        &mut panel,
        &response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&wheel))?,
            &wheel,
            &atelier
        )?
    ));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    let retry = request(&mut panel, Intent::Retry, occurrence(99)).ok_or("retry")?;
    assert_eq!(retry, command);
    assert!(apply_result(
        &mut panel,
        &response(
            424,
            CommandStatus::DuplicateReplay,
            wire(atelier::encode_reply(&atelier))?,
            &wheel,
            &atelier
        )?
    ));
    assert!(panel.retry.is_none());
    assert!(panel.ready());
    Ok(())
}

#[test]
fn panel_displays_server_quotes_and_the_matching_modifier_grade() {
    let mut panel = panel();
    panel.quality = 2;
    panel.modifier = Some((1, 5));
    let text = panel_text(&panel);
    assert!(text.contains("123458 gold"));
    assert!(text.contains("654321 gold"));
    assert!(text.contains("9–11 supreme fragments"));
    assert!(text.contains("222222 gold and 777 supreme fragments"));
    assert!(modifier_label(&panel, 1, 5).contains("Grade II"));
    assert!(slot_label(&panel, 14).contains("Green"));
    assert!(slot_label(&panel, 20).contains("Blue"));
}

#[test]
fn modifier_labels_distinguish_weaknesses_and_paid_spell_variants() {
    let panel = panel();
    assert!(modifier_label(&panel, 0, 7).contains("Holy Resistance + Death Weakness"));
    assert!(modifier_label(&panel, 0, 15).contains("Fire Resistance + Earth Weakness"));
    assert!(modifier_label(&panel, 0, 9).contains("Fire Resistance + Earth Resistance"));
    let cooldown = modifier_label(&panel, 1, 7);
    let damage = modifier_label(&panel, 1, 8);
    let critical = modifier_label(&panel, 1, 9);
    assert!(cooldown.contains("Cooldown"));
    assert!(damage.contains("Damage"));
    assert!(critical.contains("Critical damage"));
    assert_ne!(cooldown, damage);
    assert_ne!(damage, critical);
    // Crystal's executed Flurry fields are reversed relative to the source enum names.
    assert!(modifier_label(&panel, 1, 82).contains("Critical damage"));
    assert!(modifier_label(&panel, 1, 83).contains("Damage"));
}

#[test]
fn open_shortcut_requires_a_fresh_control_w_press() -> Result<(), Error> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(26)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let control = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, control, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    assert!(!open_key(&[event(ButtonState::Pressed, control, true)]));
    assert!(!open_key(&[event(ButtonState::Released, control, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        control.with(Modifier::Shift),
        false
    )]));
    Ok(())
}

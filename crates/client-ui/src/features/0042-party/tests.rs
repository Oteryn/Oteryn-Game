use super::*;

fn result(disposition: party::PartyDisposition) -> party::PartyResult {
    party::PartyResult {
        disposition,
        shared_xp_enabled: true,
        members: vec![party::PartyEntry {
            list: Vec::new(),
            handle: 7,
            name: "Leader".into(),
            leader: true,
            expires_at: 0,
            party_spell_remaining_ms: None,
        }],
        invitations: Vec::new(),
    }
}

#[test]
fn shared_xp_uses_existing_server_route() {
    let mut panel = PartyPanel {
        selected: true,
        chat_selected: true,
        state: Some(result(party::PartyDisposition::Ok)),
        ..PartyPanel::default()
    };
    let command = request(&mut panel, Intent::ToggleSharedExperience);
    assert_eq!(
        command.and_then(|command| command.shared_setting()),
        Some(false)
    );
    assert!(
        panel
            .state
            .as_ref()
            .is_some_and(|state| state.shared_xp_enabled)
    );
}

#[test]
fn malformed_operation_is_never_reported_as_confirmed() -> Result<(), party::PartyWireError> {
    let mut panel = PartyPanel {
        selected: true,
        pending: Some(PartyCommand::query()),
        ..PartyPanel::default()
    };
    apply_result(
        &mut panel,
        &PartyResponse {
            status: oteryn_client_runtime::party::CommandStatus::Accepted,
            operation: vec![],
            view: Some(party::encode_party_result(&result(
                party::PartyDisposition::Ok,
            ))?),
        },
    );
    assert!(!panel_text(&panel).contains("server confirmed"));
    Ok(())
}

#[test]
fn request_uses_party_codec_and_requires_selection() -> Result<(), &'static str> {
    let mut state = result(party::PartyDisposition::Ok);
    state.members.push(party::PartyEntry {
        list: Vec::new(),
        handle: 8,
        name: "Member".into(),
        leader: false,
        expires_at: 0,
        party_spell_remaining_ms: None,
    });
    let mut panel = PartyPanel {
        selected: true,
        own_character_name: "Leader".into(),
        state: Some(state),
        selected_member: Some(8),
        ..PartyPanel::default()
    };
    let command = request(&mut panel, Intent::Kick).ok_or("selected member makes a command")?;
    assert_eq!(
        party::decode_party_intent(&command.payload),
        Ok(party::PartyIntent::Kick(8))
    );
    Ok(())
}

#[test]
fn result_applies_view_and_surfaces_refusal() -> Result<(), party::PartyWireError> {
    let mut panel = PartyPanel {
        selected: true,
        pending: Some(PartyCommand::query()),
        ..PartyPanel::default()
    };
    let response = PartyResponse {
        status: oteryn_client_runtime::party::CommandStatus::Accepted,
        operation: party::encode_party_result(&result(party::PartyDisposition::NotLeader))?,
        view: Some(party::encode_party_result(&result(
            party::PartyDisposition::Ok,
        ))?),
    };
    assert!(apply_result(&mut panel, &response));
    assert!(panel_text(&panel).contains("only the party leader"));
    assert!(panel_text(&panel).contains("Leader: Leader"));
    Ok(())
}

#[test]
fn capability_gates_never_send_unavailable_requests() {
    let mut panel = PartyPanel::default();
    assert!(request(&mut panel, Intent::Refresh).is_none());
    panel.selected = true;
    assert!(request(&mut panel, Intent::ToggleSharedExperience).is_none());
    assert!(panel_text(&panel).contains("requires chat support"));
    assert!(panel.pending.is_none());
}

#[test]
fn shared_refusal_text_reaches_party_projection_only_for_a_shared_request() {
    let line = oteryn_client_runtime::chat::ChatLine::Private {
        speaker_name: "Game".into(),
        text: "Only the party leader can change shared experience.".into(),
    };
    let mut panel = PartyPanel::default();
    push_feedback(&mut panel, &line);
    assert!(panel.server_feedback.is_none());
    panel.awaiting_shared_feedback = true;
    push_feedback(&mut panel, &line);
    assert!(panel_text(&panel).contains("Only the party leader"));
    assert!(!panel.awaiting_shared_feedback);
}

#[test]
fn failed_refresh_preserves_the_authoritative_refusal() -> Result<(), party::PartyWireError> {
    let mut panel = PartyPanel {
        pending: Some(PartyCommand::query()),
        ..PartyPanel::default()
    };
    apply_result(
        &mut panel,
        &PartyResponse {
            status: oteryn_client_runtime::party::CommandStatus::Accepted,
            operation: party::encode_party_result(&party::PartyResult::refused(
                party::PartyDisposition::NotLeader,
            ))?,
            view: None,
        },
    );
    assert!(panel_text(&panel).contains("only the party leader"));
    Ok(())
}

#[test]
fn unknown_own_identity_cannot_manage_party_members() {
    for intent in [Intent::Kick, Intent::PassLeadership] {
        let mut panel = PartyPanel {
            selected: true,
            selected_member: Some(7),
            state: Some(result(party::PartyDisposition::Ok)),
            ..PartyPanel::default()
        };
        assert!(request(&mut panel, intent).is_none());
    }
}

#[test]
fn party_shortcut_does_not_steal_prey_or_weapon_proficiency()
-> Result<(), oteryn_input_actions::InputError> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut input = PartyInput::default();
    let event = |code, shift| -> Result<_, oteryn_input_actions::InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: if shift {
                Modifiers::one(Modifier::Control).with(Modifier::Shift)
            } else {
                Modifiers::one(Modifier::Control)
            },
            repeat: false,
        })
    };
    assert!(
        input
            .route(&[event(19, false)?, event(19, true)?])
            .is_empty()
    );
    assert_eq!(input.route(&[event(10, true)?]), vec![Action::Open]);
    Ok(())
}

#[test]
fn leadership_controls_follow_own_character_and_authoritative_role_changes()
-> Result<(), party::PartyWireError> {
    let mut state = result(party::PartyDisposition::Ok);
    state.members.push(party::PartyEntry {
        list: Vec::new(),
        handle: 8,
        name: "Member".into(),
        leader: false,
        expires_at: 0,
        party_spell_remaining_ms: None,
    });
    let mut panel = PartyPanel {
        selected: true,
        own_character_name: "Member".into(),
        selected_member: Some(7),
        state: Some(state.clone()),
        ..PartyPanel::default()
    };
    assert!(!panel.can_manage_member());
    assert!(request(&mut panel, Intent::PassLeadership).is_none());
    panel.own_character_name = "Leader".into();
    panel.selected_member = Some(8);
    assert!(panel.can_manage_member());
    assert!(request(&mut panel, Intent::PassLeadership).is_some());
    assert!(!panel.can_manage_member());
    for member in &mut state.members {
        member.leader = member.handle == 8;
    }
    apply_result(
        &mut panel,
        &PartyResponse {
            status: oteryn_client_runtime::party::CommandStatus::Accepted,
            operation: party::encode_party_result(&party::PartyResult::refused(
                party::PartyDisposition::Ok,
            ))?,
            view: Some(party::encode_party_result(&state)?),
        },
    );
    assert!(!panel.can_manage_member());
    assert!(request(&mut panel, Intent::Kick).is_none());
    Ok(())
}

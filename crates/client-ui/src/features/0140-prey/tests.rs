use super::*;

#[test]
fn prey_shortcut_preserves_control_p_for_vip_and_ignores_repeats()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(19)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let control = Modifiers::one(Modifier::Control);
    let chord = control.with(Modifier::Shift);
    assert!(!open_key(&[event(ButtonState::Pressed, control, false)]));
    assert!(open_key(&[event(ButtonState::Pressed, chord, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, chord, true)]));
    assert!(!open_key(&[event(ButtonState::Released, chord, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        chord.with(Modifier::Alt),
        false
    )]));
    Ok(())
}

fn slot(number: u8, count: u8) -> prey::Slot {
    prey::Slot {
        slot: number,
        state: 1,
        creatures: (0..count)
            .map(|index| format!("oteryn:creature.slot{number}_{index}"))
            .collect(),
        selected: String::new(),
        bonus: 0,
        grade: 0,
        percent: 0,
        hunting_seconds: 0,
        option: 0,
        free_reroll_at: 0,
        chosen_tier: 2,
        allowed_tier: 3,
        selection_epoch: 0,
    }
}

fn projection(count: u8) -> prey::Reply {
    prey::Reply {
        status: 0,
        wildcard_balance: 10,
        slots: (1..=3).map(|number| slot(number, count)).collect(),
        list_size: count,
        expansion_owned: count == 12,
        offered_today: Vec::new(),
        offered_next: String::new(),
    }
}

fn panel(count: u8) -> PreyPanel {
    PreyPanel {
        selected: true,
        confirmed: Some(projection(count)),
        ..PreyPanel::default()
    }
}

fn response(request: prey::Request, reply: &prey::Reply) -> Result<PreyResponse, &'static str> {
    Ok(PreyResponse {
        request,
        status: CommandStatus::Accepted,
        payload: prey::encode_reply(reply).map_err(|_| "reply")?,
        current: None,
    })
}

#[test]
fn tiers_use_server_allowance_and_do_not_redraw_or_change_before_confirmation()
-> Result<(), &'static str> {
    let mut panel = panel(9);
    let original = panel.confirmed.clone();
    assert!(request(&mut panel, Intent::SetTier { slot: 1, tier: 4 }).is_none());
    assert!(request(&mut panel, Intent::SetTier { slot: 1, tier: 0 }).is_none());
    let command = request(&mut panel, Intent::SetTier { slot: 1, tier: 3 }).ok_or("set tier")?;
    assert_eq!(panel.confirmed, original);
    assert!(request(&mut panel, Intent::Refresh).is_none());
    let mut confirmed = projection(9);
    confirmed.slots[0].chosen_tier = 3;
    let mut answer = response(command, &confirmed)?;
    answer.current = Some(answer.payload.clone());
    assert!(apply_result(&mut panel, &answer));
    assert_eq!(panel.slot(1).ok_or("slot")?.chosen_tier, 3);
    assert_eq!(
        panel.slot(1).ok_or("slot")?.creatures,
        original.ok_or("old")?.slots[0].creatures
    );
    Ok(())
}

#[test]
fn retained_tier_after_level_loss_is_selected_but_disabled() -> Result<(), &'static str> {
    let mut panel = panel(9);
    let state = panel.confirmed.as_mut().ok_or("reply")?;
    state.slots[0].chosen_tier = 5;
    state.slots[0].allowed_tier = 2;
    let choices = tier_choices(&panel, 1);
    assert_eq!(choices[4], ("5★".into(), true, false));
    assert!(choices[1].2);
    assert!(!choices[2].2);
    assert!(request(&mut panel, Intent::SetTier { slot: 1, tier: 5 }).is_none());
    Ok(())
}

#[test]
fn locked_and_unselected_slots_cannot_send_actions() -> Result<(), &'static str> {
    let mut panel = panel(9);
    panel.confirmed.as_mut().ok_or("reply")?.slots[1].state = 0;
    for intent in [
        Intent::ListReroll { slot: 2 },
        Intent::BonusReroll { slot: 2 },
        Intent::SetTier { slot: 2, tier: 1 },
        Intent::OfferedToday {
            slot: 2,
            after: String::new(),
        },
    ] {
        assert!(request(&mut panel, intent).is_none());
    }
    assert!(
        tier_choices(&panel, 2)
            .iter()
            .all(|(_, _, enabled)| !enabled)
    );
    panel.selected = false;
    assert!(request(&mut panel, Intent::Refresh).is_none());
    Ok(())
}

#[test]
fn expansion_changes_only_grid_rows_and_select_supports_twelfth_candidate()
-> Result<(), &'static str> {
    let mut panel = panel(9);
    assert_eq!(
        GridLayout::for_panel(&panel),
        GridLayout {
            columns: 3,
            rows: 3
        }
    );
    panel.confirmed = Some(projection(12));
    assert_eq!(
        GridLayout::for_panel(&panel),
        GridLayout {
            columns: 3,
            rows: 4
        }
    );
    assert!(panel.choose_candidate(1, 12));
    assert_eq!(
        request(&mut panel, Intent::Select { slot: 1 }),
        Some(prey::Request::Select { slot: 1, index: 12 })
    );
    assert_eq!(panel.confirmed.as_ref().ok_or("reply")?.slots.len(), 3);
    Ok(())
}

#[test]
fn offered_creatures_require_explicit_confirmation_and_bind_server_price()
-> Result<(), &'static str> {
    let mut panel = panel(9);
    let command = request(
        &mut panel,
        Intent::OfferedToday {
            slot: 1,
            after: String::new(),
        },
    )
    .ok_or("query")?;
    let mut reply = projection(9);
    reply.offered_today = vec![
        prey::OfferedCreature {
            creature: "oteryn:creature.dragon".into(),
            price_gold: 18000,
            occupied_slot: 0,
        },
        prey::OfferedCreature {
            creature: "oteryn:creature.rat".into(),
            price_gold: 800,
            occupied_slot: 2,
        },
    ];
    assert!(apply_result(&mut panel, &response(command, &reply)?));
    assert!(panel.confirm_offer(1).is_none()); // another slot's creature is disabled
    let confirmation = panel.confirm_offer(0).ok_or("confirmation")?;
    assert!(panel.pending.is_none()); // opening the dialog never charges or sends
    let mut changed = confirmation.clone();
    changed.price_gold += 1;
    assert!(request(&mut panel, Intent::ConfirmPickAgain(changed)).is_none());
    let sent = request(&mut panel, Intent::ConfirmPickAgain(confirmation)).ok_or("confirmed")?;
    assert_eq!(
        sent,
        prey::Request::PickAgain {
            slot: 1,
            creature: "oteryn:creature.dragon".into(),
            expected_price_gold: 18000
        }
    );
    assert!(panel.confirmation.is_none());
    Ok(())
}

#[test]
fn offered_pages_replace_a_bounded_page_and_maintain_cursor() -> Result<(), &'static str> {
    let mut panel = panel(12);
    for (after, creature, next) in [
        ("", "oteryn:creature.ant", "oteryn:creature.ant"),
        ("oteryn:creature.ant", "oteryn:creature.zebra", ""),
    ] {
        let query = request(
            &mut panel,
            Intent::OfferedToday {
                slot: 3,
                after: after.into(),
            },
        )
        .ok_or("query")?;
        let mut reply = projection(12);
        reply.offered_today = vec![prey::OfferedCreature {
            creature: creature.into(),
            price_gold: 1000,
            occupied_slot: 0,
        }];
        reply.offered_next = next.into();
        assert!(apply_result(&mut panel, &response(query, &reply)?));
        assert_eq!(panel.offered_today.len(), 1);
        assert_eq!(panel.offered_today[0].creature, creature);
        assert_eq!(panel.offered_next, next);
    }
    Ok(())
}

#[test]
fn receipt_replay_applies_current_projection_and_exhaustion_explains_no_charge()
-> Result<(), &'static str> {
    let mut panel = panel(9);
    let command = request(&mut panel, Intent::ListReroll { slot: 1 }).ok_or("reroll")?;
    let mut receipt = projection(9);
    receipt.status = prey::STATUS_NO_CANDIDATES;
    let mut current = projection(12);
    current.slots[0].chosen_tier = 3;
    let mut answer = response(command, &receipt)?;
    answer.status = CommandStatus::DuplicateReplay;
    answer.current = Some(prey::encode_reply(&current).map_err(|_| "current")?);
    assert!(apply_result(&mut panel, &answer));
    assert_eq!(panel.confirmed, Some(current));
    assert!(panel_text(&panel).contains("No gold was charged"));
    Ok(())
}

#[test]
fn unsolicited_and_malformed_replies_do_not_replace_confirmed_state() -> Result<(), &'static str> {
    let mut panel = panel(9);
    let old = panel.confirmed.clone();
    assert!(!apply_result(
        &mut panel,
        &response(prey::Request::Query, &projection(12))?
    ));
    let command = request(&mut panel, Intent::Refresh).ok_or("query")?;
    let mut answer = response(command, &projection(12))?;
    answer.payload = vec![0];
    assert!(!apply_result(&mut panel, &answer));
    assert_eq!(panel.confirmed, old);
    assert!(panel.feedback.is_some());
    Ok(())
}

#[test]
fn accepted_mutation_without_current_projection_requires_refresh_before_more_actions()
-> Result<(), &'static str> {
    for status in [CommandStatus::Accepted, CommandStatus::DuplicateReplay] {
        for current in [None, Some(vec![0])] {
            let mut panel = panel(9);
            assert!(panel.choose_candidate(1, 1));
            let command = request(&mut panel, Intent::ListReroll { slot: 1 }).ok_or("reroll")?;
            let mut answer = response(command, &projection(12))?;
            answer.status = status;
            answer.current = current;
            assert!(!apply_result(&mut panel, &answer));
            assert!(panel.pending.is_none());
            assert!(panel.confirmed.is_none());
            assert_eq!(panel.candidate, [None; 3]);
            assert!(panel_text(&panel).contains("Refresh"));
            assert!(
                tier_choices(&panel, 1)
                    .iter()
                    .all(|(_, _, enabled)| !enabled)
            );
            assert!(request(&mut panel, Intent::ListReroll { slot: 1 }).is_none());

            let refresh = request(&mut panel, Intent::Refresh).ok_or("recovery query")?;
            let current = projection(12);
            assert!(apply_result(&mut panel, &response(refresh, &current)?));
            assert_eq!(panel.confirmed, Some(current));
            assert!(panel.feedback.is_none());
            assert!(request(&mut panel, Intent::ListReroll { slot: 1 }).is_some());
        }
    }
    Ok(())
}

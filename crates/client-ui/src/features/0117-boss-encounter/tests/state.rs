use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn boss_encounter_gates_pending_and_projects_only_valid_server_data() {
    let mut panel = BossEncounterPanel::default();
    assert!(request(&mut panel, Intent::Refresh).is_none());
    panel.selected = true;
    let command = request(&mut panel, Intent::Refresh);
    assert_eq!(
        command.map(|c| (c.capability, c.kind, c.payload)),
        Some((117, 133, vec![16, 1]))
    );
    assert!(request(&mut panel, Intent::Refresh).is_none());
    let response = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![
            8, 100, 18, 7, 10, 3, b'a', b':', b'b', 16, 120, 34, 7, 10, 3, b'a', b':', b'b', 16, 3,
            40, 5,
        ],
    };
    assert!(apply_result(&mut panel, &response));
    let text = panel_text(&panel);
    assert!(text.contains("Boss points: 5"));
    assert!(text.contains("a:b: 3 kills"));
    assert!(text.contains("20 ms remaining"));
    let before = panel.snapshot.clone();
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![8]
        }
    ));
    assert_eq!(panel.snapshot, before);
    assert!(!panel.pending);
}

#[test]
fn boss_encounter_next_uses_earlier_cursor_to_avoid_skipping_either_list() {
    use oteryn_client_runtime::boss_encounter::Snapshot;
    let mut panel = BossEncounterPanel {
        selected: true,
        snapshot: Some(Snapshot {
            cooldown_next: Some("a:z".into()),
            progress_next: Some("a:m".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Next).map(|c| c.payload),
        Some(vec![10, 3, b'a', b':', b'm', 16, 1])
    );
    panel.pending = false;
    panel.snapshot = Some(Snapshot::default());
    assert!(request(&mut panel, Intent::Next).is_none());
    assert!(panel_text(&panel).contains("No boss records"));
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Rejected,
        payload: vec![],
    };
    assert!(!apply_result(&mut panel, &result));
    assert!(panel.feedback.is_some());
}

#[test]
fn boss_encounter_open_requires_exact_chord_and_ignores_repeat_release()
-> Result<(), oteryn_input_actions::InputError> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(13)?;
    let modifiers = Modifiers::NONE
        .with(Modifier::Control)
        .with(Modifier::Shift);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    for e in [
        event(ButtonState::Released, modifiers, false),
        event(ButtonState::Pressed, modifiers, true),
        event(ButtonState::Pressed, modifiers.with(Modifier::Alt), false),
        event(
            ButtonState::Pressed,
            Modifiers::NONE.with(Modifier::Control),
            false,
        ),
    ] {
        assert!(!open_key(&[e]));
    }
    Ok(())
}

#[test]
fn pc05_boss_activity_uses_existing_codec_and_keeps_confirmed_state() {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    panel.fields[0] = "boss:test".into();
    for (intent, kind) in [
        (Intent::Enter, 178),
        (Intent::Defeated, 179),
        (Intent::Timeout, 180),
    ] {
        let command = request(&mut panel, intent).unwrap_or_else(|| unreachable!());
        assert_eq!((command.capability, command.kind), (117, kind));
        assert_eq!(
            oteryn_client_runtime::boss_encounter::decode_boss_key(&command.payload)
                .map(|v| v.boss_key),
            Ok("boss:test".into())
        );
        assert!(request(&mut panel, intent).is_none());
        let reply = GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: vec![8, 2, 32, 13],
        };
        assert!(apply_result(&mut panel, &reply));
        assert!(panel_text(&panel).contains("Cooldown"));
        assert!(!panel.pending);
    }
    let confirmed = panel.summary.clone();
    assert!(request(&mut panel, Intent::Enter).is_some());
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: vec![255]
        }
    ));
    assert_eq!(panel.summary, confirmed);
    panel.fields[0] = "invalid".into();
    assert!(request(&mut panel, Intent::Enter).is_none());
    assert!(!panel.pending);
}

#[test]
fn pc05_boss_auxiliary_flows_validate_input_and_render_server_results() {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    panel.fields = ["oteryn:creature.test".into(), "2".into(), "3".into()];
    for (intent, kind, payload, reply) in [
        (Intent::HazardQuery, 169, vec![], vec![5, 2]),
        (Intent::HazardSelect, 169, vec![3], vec![5, 3]),
        (Intent::SlotsQuery, 491, vec![0], vec![0; 26]),
        (
            Intent::SlotEquip,
            491,
            [vec![1, 2], b"oteryn:creature.test".to_vec()].concat(),
            vec![0; 26],
        ),
        (Intent::SlotRemove, 491, vec![2, 2], vec![0; 26]),
        (Intent::Rewards, 340, vec![], vec![8, 0]),
    ] {
        let command = request(&mut panel, intent).unwrap_or_else(|| unreachable!());
        assert_eq!((command.kind, command.payload), (kind, payload));
        assert!(apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: reply
            }
        ));
        assert!(!panel.summary.is_empty());
    }
    assert!(panel_text(&panel).contains("Reward groups"));
    panel.fields[1] = "3".into();
    assert!(request(&mut panel, Intent::SlotEquip).is_none());
    panel.fields[2] = "13".into();
    assert!(request(&mut panel, Intent::HazardSelect).is_none());
}

#[test]
fn pc05_reward_chest_paging_uses_only_last_server_death_id() -> Result<(), String> {
    use oteryn_client_runtime::boss_encounter as wire;
    let entries = (1..=64)
        .map(|id| wire::BossRewardChestEntry {
            death: [id; 16],
            boss_key: "boss:test".into(),
            minted_at_unix_ms: 1,
            expires_at_unix_ms: 1 + wire::BOSS_REWARD_EXPIRY_MS,
            item_count: 1,
        })
        .collect::<Vec<_>>();
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut panel, Intent::RewardsNext).is_none());
    assert!(request(&mut panel, Intent::Rewards).is_some());
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::encode_reward_chest(100, &entries).map_err(|e| format!("{e:?}"))?,
    };
    assert!(apply_result(&mut panel, &result));
    let command = request(&mut panel, Intent::RewardsNext).ok_or("missing next page")?;
    assert_eq!(
        wire::decode_reward_chest_query(&command.payload),
        Ok(Some([64; 16]))
    );
    assert_eq!((command.capability, command.kind), (117, 340));
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            payload: vec![8, 100],
            ..result
        }
    ));
    assert!(request(&mut panel, Intent::RewardsNext).is_none());
    assert!(panel_text(&panel).contains("Reward groups: 0"));
    Ok(())
}

#[test]
fn cooldown_widget_counts_down_confirmed_deadlines_and_keeps_progress_out() {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![
                8, 100, 18, 7, 10, 3, b'a', b':', b'b', 16, 120, 34, 7, 10, 3, b'a', b':', b'b',
                16, 3, 40, 5
            ],
        }
    ));
    assert!(panel.cooldown_received_at.is_some());
    assert!(cooldown_text(&panel, 0).contains("a:b: 20 ms remaining"));
    assert!(cooldown_text(&panel, 15).contains("a:b: 5 ms remaining"));
    assert!(cooldown_text(&panel, u64::MAX).contains("a:b: Ready"));
    assert!(!cooldown_text(&panel, 0).contains("3 kills"));
    assert!(panel.snapshot.as_ref().is_some_and(|s| s.now == 100));
}

#[test]
fn cooldown_shortcut_requires_exact_modifiers_and_a_fresh_press()
-> Result<(), oteryn_input_actions::InputError> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(13)?;
    let modifiers = Modifiers::NONE
        .with(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    let event = |modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat,
    };
    assert!(cooldowns_key(&[event(modifiers, false)]));
    assert!(!cooldowns_key(&[event(modifiers, true)]));
    assert!(!cooldowns_key(&[event(
        modifiers.with(Modifier::Super),
        false
    )]));
    assert!(!cooldowns_key(&[event(
        Modifiers::NONE
            .with(Modifier::Control)
            .with(Modifier::Shift),
        false
    )]));
    Ok(())
}

#[test]
fn cooldown_widget_lists_the_next_available_boss_first() {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![
                8, 100, 18, 7, 10, 3, b'a', b':', b'a', 16, 125, 18, 7, 10, 3, b'b', b':', b'b',
                16, 105, 40, 5
            ],
        }
    ));
    let text = cooldown_text(&panel, 0);
    assert!(text.find("b:b:") < text.find("a:a:"));
}

#[test]
fn hazard_panel_shows_unlocked_levels_and_blocks_locked_selection() {
    let mut p = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::HazardQuery).is_some());
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![5, 2]
        }
    ));
    let text = panel_text(&p);
    assert!(text.contains("Selected level 2 of 5 unlocked"));
    assert!(text.contains("Enter"));
    p.fields[2] = "6".into();
    assert!(request(&mut p, Intent::HazardSelect).is_none());
    p.fields[2] = "5".into();
    assert!(request(&mut p, Intent::HazardSelect).is_some());
}

#[test]
fn bosstiary_browser_filters_and_selects_confirmed_bosses() {
    let mut panel = BossEncounterPanel {
        snapshot: Some(oteryn_client_runtime::boss_encounter::Snapshot {
            progress: vec![("boss:dragon".into(), 12), ("boss:rat".into(), 2)],
            ..Default::default()
        }),
        ..Default::default()
    };
    panel.boss_search = "DRAGON".into();
    panel.select_boss(false);
    assert_eq!(panel.fields[0], "boss:dragon");
    assert!(panel_text(&panel).contains("boss:dragon: 12 kills"));
    assert!(!panel_text(&panel).contains("boss:rat: 2 kills"));
    panel.pending = true;
    panel.boss_search = "rat".into();
    panel.select_boss(false);
    assert_eq!(panel.fields[0], "boss:dragon");
}

#[test]
fn boss_slot_selector_uses_server_unlocks_and_preserves_confirmed_bonus() -> Result<(), &'static str>
{
    use oteryn_client_runtime::boss_encounter::bosstiary_slots::BossSlotReply;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    let reply = BossSlotReply {
        status: 0,
        unlocked: 2,
        bonus: 25,
        points: 600,
        remove_fee: 1000,
        ready_at_ms: 900,
        bosses: [String::new(), "boss:dragon".into()],
    };
    assert!(request(&mut panel, Intent::SlotsQuery).is_some());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().ok_or("encode")?,
        }
    ));
    panel.select_slot();
    assert_eq!(panel.fields[1], "2");
    assert_eq!(panel.slots.as_ref(), Some(&reply));
    let text = panel_text(&panel);
    assert!(text.contains("Slot 1: Locked"));
    assert!(text.contains("Slot 2: boss:dragon"));
    assert!(text.contains("Confirmed bonus: 25"));
    assert!(request(&mut panel, Intent::SlotsQuery).is_some());
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: vec![255],
        }
    ));
    assert_eq!(panel.slots.as_ref(), Some(&reply));
    Ok(())
}

use super::*;

fn reply(status: u8) -> prey::Reply {
    prey::Reply {
        status,
        wildcard_balance: 4,
        list_size: 9,
        expansion_owned: false,
        offered_today: vec![],
        offered_next: String::new(),
        slots: vec![prey::Slot {
            slot: 1,
            state: 1,
            creatures: vec!["oteryn:rat".into()],
            selected: String::new(),
            bonus: 0,
            grade: 0,
            percent: 0,
            hunting_seconds: 0,
            option: 0,
            free_reroll_at: 12,
            chosen_tier: 1,
            allowed_tier: 1,
            selection_epoch: 0,
        }],
    }
}

#[test]
fn requests_cover_every_wire_action() {
    let mut panel = PreyPanel {
        selected: true,
        confirmed: Some(reply(0)),
        candidate: [Some(1), None, None],
        ..PreyPanel::default()
    };
    let expected = [
        prey::Request::Query,
        prey::Request::ListReroll { slot: 1 },
        prey::Request::Select { slot: 1, index: 1 },
        prey::Request::SelectAny {
            slot: 1,
            creature: "oteryn:dragon".into(),
        },
        prey::Request::BonusReroll { slot: 1 },
        prey::Request::SetOption { slot: 1, option: 2 },
    ];
    for (intent, expected) in [
        Intent::Refresh,
        Intent::ListReroll { slot: 1 },
        Intent::Select { slot: 1 },
        Intent::SelectAny {
            slot: 1,
            creature: "oteryn:dragon".into(),
        },
        Intent::BonusReroll { slot: 1 },
        Intent::SetOption { slot: 1, option: 2 },
    ]
    .into_iter()
    .zip(expected)
    {
        assert_eq!(request(&mut panel, intent), Some(expected));
        panel.pending = None;
    }
}

#[test]
fn result_replaces_state_and_exposes_refusals() -> Result<(), &'static str> {
    let mut panel = PreyPanel {
        selected: true,
        pending: Some(prey::Request::Query),
        ..PreyPanel::default()
    };
    assert!(apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Accepted,
            request: prey::Request::Query,
            payload: prey::encode_reply(&reply(0)).map_err(|_| "reply")?,
            current: None
        }
    ));
    assert!(panel_text(&panel).contains("oteryn:rat"));
    panel.pending = Some(prey::Request::Query);
    assert!(!apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Rejected,
            request: prey::Request::Query,
            payload: vec![],
            current: None
        }
    ));
    assert!(panel.feedback.is_some_and(|text| text.contains("refused")));
    panel.pending = Some(prey::Request::Query);
    assert!(!apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Accepted,
            request: prey::Request::Query,
            payload: prey::encode_reply(&reply(prey::STATUS_CONTENT_UNAVAILABLE))
                .map_err(|_| "reply")?,
            current: None
        }
    ));
    assert!(panel.feedback.is_some_and(|text| text.contains("loot")));
    Ok(())
}

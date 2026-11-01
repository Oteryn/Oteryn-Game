use super::*;

fn reply(status: u8) -> prey::Reply {
    prey::Reply {
        status,
        wildcard_balance: 4,
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
        }],
    }
}

#[test]
fn requests_cover_every_wire_action() {
    let mut panel = PreyPanel {
        selected: true,
        creature: "oteryn:dragon".into(),
        option: 2,
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
        Intent::ListReroll,
        Intent::Select,
        Intent::SelectAny,
        Intent::BonusReroll,
        Intent::SetOption,
    ]
    .into_iter()
    .zip(expected)
    {
        assert_eq!(
            request(&mut panel, intent).map(|value| value.request),
            Some(expected)
        );
        panel.pending = false;
    }
}

#[test]
fn result_replaces_state_and_exposes_refusals() {
    let mut panel = PreyPanel {
        selected: true,
        pending: true,
        ..PreyPanel::default()
    };
    assert!(apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Accepted,
            reply: Some(reply(0))
        }
    ));
    assert!(panel_text(&panel).contains("oteryn:rat"));
    assert!(!apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Rejected,
            reply: None
        }
    ));
    assert!(panel.feedback.is_some_and(|text| text.contains("refused")));
    assert!(!apply_result(
        &mut panel,
        &PreyResponse {
            status: CommandStatus::Accepted,
            reply: Some(reply(prey::STATUS_CONTENT_UNAVAILABLE))
        }
    ));
    assert!(panel.feedback.is_some_and(|text| text.contains("loot")));
}

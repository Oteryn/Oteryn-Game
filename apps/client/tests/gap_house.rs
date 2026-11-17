#[test]
fn gap_house_commands_are_owned_by_house_not_adjacent_panels()
-> Result<(), Box<dyn std::error::Error>> {
    let registration: serde_json::Value = serde_json::from_str(include_str!(
        "../../../crates/client-ui/src/features/0049-house/registrations/handlers.json"
    ))?;
    assert_eq!(
        registration["status"], "supported",
        "commands 56/57 have no house handler"
    );
    assert_eq!(registration["commands"], serde_json::json!([56]));
    assert_eq!(registration["capability"], 49);
    let query: serde_json::Value = serde_json::from_str(include_str!(
        "../../../crates/client-ui/src/features/0049-house/registrations/query.json"
    ))?;
    assert_eq!(query["status"], "supported");
    assert_eq!(query["commands"], serde_json::json!([57]));
    assert_eq!(query["capability"], 49);
    Ok(())
}
#[path = "../src/features/0049-house/controller.rs"]
mod controller;
#[test]
fn gap_house_all_action_selections() -> Result<(), &'static str> {
    use oteryn_client_runtime::house::{Action, Row};
    use oteryn_client_ui::house::Panel;
    let mut w = [1; 16];
    w[6] = 0x70;
    w[8] = 0x80;
    let mut p = Panel {
        selected: true,
        world: w,
        town: "oteryn:content.area.city.thais".into(),
        ..Panel::default()
    };
    p.page.rows.push(Row {
        house: "a".into(),
        ..Row::default()
    });
    p.page.cursor = Some("a".into());
    let actions = [
        None,
        None,
        Some(Action::Bid(20)),
        Some(Action::Guests(vec![])),
        Some(Action::Subowners(vec![])),
        Some(Action::Door {
            x: 1,
            y: 2,
            floor: 0,
            revision: 0,
            members: vec![],
        }),
        Some(Action::Kick(w)),
        None,
        Some(Action::Notice(300)),
        None,
    ];
    assert_eq!(controller::BUTTONS.len(), actions.len());
    for (i, a) in actions.into_iter().enumerate() {
        let c = controller::submit(&mut p, i, a)?;
        assert_eq!(c.kind, if i < 2 { 57 } else { 56 });
        p.apply(
            &c,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
                server_sequence: 1,
                payload: if i < 2 { vec![8, 2] } else { vec![8, 1, 16, 0] },
            },
        );
    }
    assert!(controller::submit(&mut p, 2, Some(Action::Leave)).is_err());
    Ok(())
}

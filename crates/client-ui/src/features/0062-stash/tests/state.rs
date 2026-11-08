use super::*;
use oteryn_client_runtime::stash::CommandStatus;

fn reply(outcome: stash::StashOutcome) -> StashResponse {
    StashResponse {
        status: CommandStatus::Accepted,
        reply: stash::StashReply {
            outcome,
            stowed_quantity: 0,
            transaction: None,
            page: 2,
            has_more: true,
            rows: vec![stash::StashRow {
                handle: [1; 16],
                definition_key: "item.apple".into(),
                definition_revision: "r1".into(),
                quantity: 7,
                migration_required: false,
            }],
        },
    }
}

#[test]
fn browse_and_withdraw_use_server_rows_and_render_refusals() -> Result<(), &'static str> {
    let mut panel = StashPanel {
        selected: true,
        ..StashPanel::default()
    };
    let browse = request(&mut panel, Intent::Refresh).ok_or("browse command")?;
    assert_eq!(
        stash::decode_stash_intent(&browse.payload)
            .map_err(|_| "browse payload")?
            .action,
        stash::StashAction::Browse
    );
    apply_result(&mut panel, &reply(stash::StashOutcome::Opened));
    let withdraw =
        request(&mut panel, Intent::Withdraw { row: 0, amount: 3 }).ok_or("withdraw command")?;
    assert_eq!(
        stash::decode_stash_intent(&withdraw.payload)
            .map_err(|_| "withdraw payload")?
            .action,
        stash::StashAction::Withdraw {
            row: [1; 16],
            amount: 3
        }
    );
    apply_result(&mut panel, &reply(stash::StashOutcome::Insufficient));
    assert!(panel_text(&panel).contains("not have enough"));
    assert!(panel_text(&panel).contains("item.apple"));
    Ok(())
}

#[test]
fn stow_rejects_handles_not_in_the_visible_carried_picker() {
    let mut panel = StashPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut panel, Intent::Stow(std::num::NonZeroU64::MIN)).is_none());
    assert!(!panel.pending);
}

#[test]
fn visible_carried_selection_stows_the_server_handle_and_blocks_repeated_input()
-> Result<(), &'static str> {
    let item = oteryn_client_runtime::depot::ItemEntry {
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let mut panel = StashPanel {
        selected: true,
        carried: vec![item],
        ..Default::default()
    };
    let handle = panel.carried[0].handle;
    let command = request(&mut panel, Intent::Stow(handle)).ok_or("stow command")?;
    assert_eq!(
        stash::decode_stash_intent(&command.payload)
            .map_err(|_| "payload")?
            .action,
        stash::StashAction::Stow(item.handle)
    );
    assert!(request(&mut panel, Intent::Stow(item.handle)).is_none());
    apply_result(&mut panel, &reply(stash::StashOutcome::Unsupported));
    assert!(panel_text(&panel).contains("cannot be stowed"));
    panel.carried.clear();
    assert!(request(&mut panel, Intent::Stow(item.handle)).is_none());
    panel.selected = false;
    panel.carried.push(item);
    assert!(request(&mut panel, Intent::Stow(item.handle)).is_none());
    Ok(())
}

#[test]
fn carried_click_rejects_a_changed_projection_and_invalid_row() {
    let item = oteryn_client_runtime::depot::ItemEntry {
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let mut panel = StashPanel {
        carried: vec![item],
        ..Default::default()
    };
    assert_eq!(
        carried_intent(&panel, &[item], 0),
        Some(Intent::Stow(item.handle))
    );
    assert_eq!(carried_intent(&panel, &[item], 1), None);
    panel.carried.clear();
    assert_eq!(carried_intent(&panel, &[item], 0), None);
}

#[test]
fn filtered_sorted_stash_withdraws_the_displayed_server_handle() -> Result<(), &'static str> {
    let mut panel = StashPanel {
        selected: true,
        query: "APPLE".into(),
        sort: Sort::Stock,
        ..Default::default()
    };
    for (handle, key, quantity) in [
        (1, "item.apple", 2),
        (2, "item.pear", 90),
        (3, "item.apple.green", 7),
    ] {
        panel.rows.push(stash::StashRow {
            handle: [handle; 16],
            definition_key: key.into(),
            definition_revision: "r1".into(),
            quantity,
            migration_required: false,
        });
    }
    assert_eq!(visible_rows(&panel), vec![2, 0]);
    let intent = visible_withdraw(&panel, 0, 3).ok_or("visible selection")?;
    let command = request(&mut panel, intent).ok_or("withdraw")?;
    assert_eq!(
        stash::decode_stash_intent(&command.payload)
            .map_err(|_| "wire")?
            .action,
        stash::StashAction::Withdraw {
            row: [3; 16],
            amount: 3
        }
    );
    panel.pending = false;
    panel.query = "no match".into();
    assert!(visible_withdraw(&panel, 0, 1).is_none());
    assert!(panel_text(&panel).contains("No matching items on this page"));
    Ok(())
}

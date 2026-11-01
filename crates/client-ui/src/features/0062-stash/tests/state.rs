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
